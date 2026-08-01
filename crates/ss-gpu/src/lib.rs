//! GPU inventory + operator GPU preference for multi-GPU hosts (web-console GPU selection).
//!
//! Three concerns, one module:
//! - **Enumeration** ([`enumerate`]): the machine's hardware GPUs — DXGI adapters on Windows
//!   (WARP/Basic-Render and indirect-display/display-only adapters filtered out — an IddCx
//!   adapter like our own ss-vdisplay mirrors its render GPU's whole DXGI identity and would
//!   list every GPU twice), `/dev/dri/renderD*` + sysfs PCI ids on Linux, empty elsewhere.
//!   Compiled on every platform so the management endpoints (and the checked-in OpenAPI
//!   document) are identical everywhere.
//! - **Preference** ([`prefs`]): the operator's persisted auto/manual choice
//!   (`<config>/gpu-settings.json`, written by the mgmt API). A manual preference is stored by
//!   *stable identity* — PCI vendor:device + occurrence + name — NOT by LUID (Windows LUIDs are
//!   reassigned every boot) or adapter index (enumeration order can change across driver updates).
//! - **Selection** ([`selected_gpu`] / [`pick`]): the one place that turns (inventory, preference,
//!   `SLIPSTREAM_RENDER_ADAPTER`) into the render/encode GPU. Precedence: **manual preference >
//!   env substring > auto (max dedicated VRAM)**, with graceful fall-through — a preferred GPU
//!   that vanished (unplugged eGPU, disabled iGPU) logs a warning and auto-selects so the host
//!   keeps streaming, and the mgmt API surfaces the fallback instead of hiding it.
//!
//! A preference change applies to the **next session**: selection is read at capture/encode setup
//! (`win_adapter::resolve_render_adapter_luid`, the encoder-backend dispatch, the codec probes), a
//! running session keeps the device it opened on. [`session_begin`]/[`active`] record which GPU a
//! live session actually encodes on, for the console's "in use" display.

// Unsafe-proof program: every `unsafe {}` in this leaf carries a `// SAFETY:` proof.
#![deny(clippy::undocumented_unsafe_blocks)]

/// Which kernel adapter types ([`D3DKMT_ADAPTERTYPE`] bit-field words) never belong in the GPU
/// inventory. Pure bit math on every platform so the classification is unit-tested with words
/// captured from real hardware; only the Windows [`enumerate`] consumes it at runtime.
///
/// [`D3DKMT_ADAPTERTYPE`]: https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/d3dkmdt/ns-d3dkmdt-_d3dkmt_adaptertype
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
mod adapter_type;
mod enumerate;
/// Kernel-side adapter-type query (`D3DKMTQueryAdapterInfo(KMTQAITYPE_ADAPTERTYPE)`) via raw
/// gdi32 FFI — the `windows` crate's `Wdk_*` bindings aren't enabled, and one 3-call query
/// doesn't justify them.
#[cfg(target_os = "windows")]
mod kmt;
mod prefs;
mod select;
mod types;

pub use enumerate::enumerate;
pub use prefs::{prefs, GpuMode, GpuPrefStore, GpuPreference, PreferredGpu};
#[cfg(target_os = "linux")]
pub use select::linux_render_node;
#[cfg(target_os = "windows")]
pub use select::resolve_render_adapter_luid;
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

    /// `D3DKMT_ADAPTERTYPE` words captured on a real host (RTX 4090 + ss-vdisplay + Raphael
    /// iGPU, 2026-07): the IDD ghost and Basic Render hide, the real GPUs stay.
    #[test]
    fn adapter_type_hides_idd_ghost_keeps_real_gpus() {
        assert!(adapter_type::hidden(0x0342)); // ss-vdisplay ghost twin: indirect + display-only
        assert!(!adapter_type::hidden(0x031b)); // NVIDIA GeForce RTX 4090
        assert!(!adapter_type::hidden(0x2323)); // AMD Radeon(TM) Graphics (Raphael iGPU)
        assert!(adapter_type::hidden(0x0105)); // Microsoft Basic Render Driver (software)
    }

    /// End-to-end smoke of the ghost-twin filter + the raw D3DKMT FFI (struct layout, linking)
    /// on whatever GPUs this Windows machine has: nothing the filter would hide may survive
    /// [`enumerate`]. On a host with ss-vdisplay installed this actively exercises ghost
    /// exclusion; on a GPU-less CI runner it passes vacuously.
    #[cfg(target_os = "windows")]
    #[test]
    fn enumerate_excludes_non_render_adapters() {
        for g in enumerate() {
            let bits = kmt::adapter_type_bits(g.handle.luid_low, g.handle.luid_high);
            eprintln!(
                "enumerated: {} ({}) kmt_bits={}",
                g.name,
                g.id,
                bits.map_or("<query failed>".into(), |b| format!("{b:#x}")),
            );
            if let Some(bits) = bits {
                assert!(
                    !adapter_type::hidden(bits),
                    "{} ({}) should have been filtered (bits {bits:#x})",
                    g.name,
                    g.id
                );
            }
        }
    }
}
