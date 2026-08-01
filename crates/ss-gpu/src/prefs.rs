use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Operator GPU-selection mode: `Auto` (env substring, else max VRAM — today's behavior) or
/// `Manual` (an explicit GPU chosen in the web console).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuMode {
    #[default]
    Auto,
    Manual,
}

/// Stable identity of the manually preferred GPU (see [`GpuInfo::id`] for why not LUID/index).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreferredGpu {
    pub vendor_id: u32,
    pub device_id: u32,
    #[serde(default)]
    pub occurrence: u32,
    /// Adapter name at the time of selection — the last-resort matcher and the label the API
    /// shows when the preferred GPU is currently absent.
    #[serde(default)]
    pub name: String,
}

/// The persisted GPU preference (`<config>/gpu-settings.json`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuPreference {
    #[serde(default)]
    pub mode: GpuMode,
    /// `Some` when `mode == Manual` (kept when switching back to Auto so the console can offer
    /// "return to your previous manual pick").
    #[serde(default)]
    pub gpu: Option<PreferredGpu>,
}

/// The preference store: in-memory current value + its JSON file. Mirrors `native_pairing`'s
/// persistence discipline (private dir, secret-file temp write + atomic rename, in-memory
/// rollback if the disk write fails).
pub struct GpuPrefStore {
    path: PathBuf,
    cur: Mutex<GpuPreference>,
}

impl GpuPrefStore {
    /// Load the store from `path` (missing/corrupt file ⇒ default Auto, with a warning for the
    /// corrupt case — never fail host startup over a settings file).
    pub fn load_from(path: PathBuf) -> Self {
        let cur = match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<GpuPreference>(&bytes) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "gpu-settings.json unreadable — using default (Auto)");
                    GpuPreference::default()
                }
            },
            Err(_) => GpuPreference::default(),
        };
        GpuPrefStore {
            path,
            cur: Mutex::new(cur),
        }
    }

    pub fn get(&self) -> GpuPreference {
        self.cur.lock().unwrap().clone()
    }

    /// Persist + apply a new preference. The in-memory value only changes if the disk write
    /// succeeds, so a full disk can't leave memory and file disagreeing.
    pub fn set(&self, pref: GpuPreference) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            ss_paths::create_private_dir(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        ss_paths::write_secret_file(&tmp, &serde_json::to_vec_pretty(&pref)?)?;
        std::fs::rename(&tmp, &self.path)?;
        *self.cur.lock().unwrap() = pref;
        Ok(())
    }
}

/// The process-wide preference store (config-dir file), loaded once on first access — the same
/// global-accessor shape as [`ss_host_config::config`], because selection happens deep inside
/// capture/encode setup where no app state is threaded.
pub fn prefs() -> &'static GpuPrefStore {
    static STORE: OnceLock<GpuPrefStore> = OnceLock::new();
    STORE.get_or_init(|| GpuPrefStore::load_from(ss_paths::config_dir().join("gpu-settings.json")))
}
