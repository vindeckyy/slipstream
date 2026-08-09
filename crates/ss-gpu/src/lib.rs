//! GPU inventory + operator GPU preference for multi-GPU hosts (web-console GPU selection).
//!
//! Three concerns, one module:
//! - **Enumeration** ([`enumerate`]): Linux `/dev/dri/renderD*` nodes with sysfs PCI ids.
//! - **Preference** ([`prefs`]): the operator's persisted auto/manual choice
//!   (`<config>/gpu-settings.json`, written by the mgmt API). A manual preference is stored by
//!   stable PCI vendor, device, occurrence, and name.
//! - **Selection** ([`selected_gpu`] / [`pick`]): the one place that turns (inventory, preference,
//!   `SLIPSTREAM_RENDER_ADAPTER`) into the render/encode GPU. Precedence is manual preference,
//!   environment substring, then automatic selection.
//!
//! A preference change applies to the **next session**: selection is read at capture/encode setup
//! (the encoder-backend dispatch and codec probes), a
//! running session keeps the device it opened on. [`session_begin`]/[`active`] record which GPU a
//! live session actually encodes on, for the console's "in use" display.

// Unsafe-proof program: every `unsafe {}` in this leaf carries a `// SAFETY:` proof.
#![deny(clippy::undocumented_unsafe_blocks)]

mod enumerate;
mod prefs;
mod select;
mod types;

pub use enumerate::enumerate;
pub use prefs::{prefs, GpuMode, GpuPrefStore, GpuPreference, PreferredGpu};
#[cfg(target_os = "linux")]
pub use select::linux_render_node;
pub use select::{
    active, find_preferred, manual_selection, pick, selected_gpu, selection_key, session_begin,
    ActiveGpu, ActiveSession, PickSource, SelectedGpu,
};
pub use types::{vendor_tag, GpuHandle, GpuInfo, VENDOR_AMD, VENDOR_INTEL, VENDOR_NVIDIA};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::assign_ids;

    fn gpu(vendor: u32, device: u32, name: &str, vram_gb: u64) -> GpuInfo {
        GpuInfo {
            id: String::new(),
            name: name.into(),
            vendor_id: vendor,
            device_id: device,
            occurrence: 0,
            vram_bytes: vram_gb * 1024 * 1024 * 1024,
            handle: GpuHandle::default(),
        }
    }

    /// The dev-box shape: NVIDIA dGPU + Intel Arc iGPU.
    fn hybrid() -> Vec<GpuInfo> {
        let mut v = vec![
            gpu(VENDOR_INTEL, 0x7d55, "Intel(R) Arc(TM) Graphics", 0),
            gpu(VENDOR_NVIDIA, 0x2c05, "NVIDIA GeForce RTX 5070 Ti", 16),
        ];
        assign_ids(&mut v);
        v
    }

    fn manual(vendor: u32, device: u32, occurrence: u32, name: &str) -> GpuPreference {
        GpuPreference {
            mode: GpuMode::Manual,
            gpu: Some(PreferredGpu {
                vendor_id: vendor,
                device_id: device,
                occurrence,
                name: name.into(),
            }),
        }
    }

    #[test]
    fn auto_picks_max_vram() {
        let (i, src) = pick(&hybrid(), &GpuPreference::default(), None).unwrap();
        assert_eq!(i, 1);
        assert_eq!(src, PickSource::Auto);
    }

    #[test]
    fn manual_preference_beats_env_and_vram() {
        let pref = manual(VENDOR_INTEL, 0x7d55, 0, "Intel(R) Arc(TM) Graphics");
        let (i, src) = pick(&hybrid(), &pref, Some("nvidia")).unwrap();
        assert_eq!(i, 0);
        assert_eq!(src, PickSource::Preference);
    }

    #[test]
    fn env_substring_beats_vram_and_is_case_insensitive() {
        let mut gpus = vec![
            gpu(VENDOR_NVIDIA, 0x2c05, "NVIDIA GeForce RTX 5070 Ti", 16),
            gpu(VENDOR_INTEL, 0x7d55, "Intel(R) Arc(TM) Graphics", 0),
        ];
        assign_ids(&mut gpus);
        let (i, src) = pick(&gpus, &GpuPreference::default(), Some("ARC")).unwrap();
        assert_eq!(i, 1);
        assert_eq!(src, PickSource::Env);
    }

    #[test]
    fn unmatched_env_falls_back_to_max_vram() {
        let (i, src) = pick(&hybrid(), &GpuPreference::default(), Some("radeon")).unwrap();
        assert_eq!(i, 1);
        assert_eq!(src, PickSource::Auto);
    }

    #[test]
    fn missing_preferred_gpu_falls_back_and_says_so() {
        let pref = manual(VENDOR_AMD, 0x744c, 0, "AMD Radeon RX 7900 XTX");
        let (i, src) = pick(&hybrid(), &pref, None).unwrap();
        assert_eq!(i, 1); // max VRAM
        assert_eq!(src, PickSource::PreferenceMissing);
    }

    #[test]
    fn preferred_matches_same_model_when_occurrence_gone() {
        // Stored occurrence 1 (was the second of two twins); only one twin remains.
        let mut gpus = vec![
            gpu(VENDOR_INTEL, 0x7d55, "Intel(R) Arc(TM) Graphics", 0),
            gpu(VENDOR_NVIDIA, 0x2c05, "NVIDIA GeForce RTX 5070 Ti", 16),
        ];
        assign_ids(&mut gpus);
        let pref = manual(VENDOR_NVIDIA, 0x2c05, 1, "NVIDIA GeForce RTX 5070 Ti");
        let (i, src) = pick(&gpus, &pref, None).unwrap();
        assert_eq!(i, 1);
        assert_eq!(src, PickSource::Preference);
    }

    #[test]
    fn preferred_matches_by_name_when_ids_changed() {
        let pref = manual(VENDOR_NVIDIA, 0xffff, 0, "Intel(R) Arc(TM) Graphics");
        let (i, src) = pick(&hybrid(), &pref, None).unwrap();
        assert_eq!(i, 0);
        assert_eq!(src, PickSource::Preference);
    }

    #[test]
    fn empty_inventory_selects_nothing() {
        assert!(pick(&[], &GpuPreference::default(), Some("nvidia")).is_none());
    }

    #[test]
    fn ids_disambiguate_twins() {
        let mut gpus = vec![
            gpu(VENDOR_NVIDIA, 0x2c05, "NVIDIA GeForce RTX 5070 Ti", 16),
            gpu(VENDOR_NVIDIA, 0x2c05, "NVIDIA GeForce RTX 5070 Ti", 16),
        ];
        assign_ids(&mut gpus);
        assert_eq!(gpus[0].id, "10de-2c05-0");
        assert_eq!(gpus[1].id, "10de-2c05-1");
    }

    #[test]
    fn store_round_trips_and_survives_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gpu-settings.json");
        let store = GpuPrefStore::load_from(path.clone());
        assert_eq!(store.get(), GpuPreference::default());
        let pref = manual(VENDOR_INTEL, 0x7d55, 0, "Intel(R) Arc(TM) Graphics");
        store.set(pref.clone()).unwrap();
        assert_eq!(store.get(), pref);
        // A fresh load sees the persisted value…
        assert_eq!(GpuPrefStore::load_from(path.clone()).get(), pref);
        // …and a corrupt file degrades to Auto instead of failing startup.
        std::fs::write(&path, b"{ not json").unwrap();
        assert_eq!(
            GpuPrefStore::load_from(path).get(),
            GpuPreference::default()
        );
    }

    #[test]
    fn session_counter_tracks_begin_and_drop() {
        // Serialize against other tests via the ACTIVE mutex being process-global: this is the
        // only test touching it.
        let a = session_begin(ActiveGpu {
            id: "10de-2c05-0".into(),
            name: "GPU A".into(),
            vendor_id: VENDOR_NVIDIA,
            backend: "nvenc",
        });
        let (gpu0, n0) = active().unwrap();
        assert_eq!((gpu0.name.as_str(), n0), ("GPU A", 1));
        let b = session_begin(ActiveGpu {
            id: "10de-2c05-0".into(),
            name: "GPU A".into(),
            vendor_id: VENDOR_NVIDIA,
            backend: "nvenc",
        });
        assert_eq!(active().unwrap().1, 2);
        drop(a);
        assert_eq!(active().unwrap().1, 1);
        drop(b);
        assert_eq!(active().unwrap().1, 0); // idle, last-used retained
    }

}
