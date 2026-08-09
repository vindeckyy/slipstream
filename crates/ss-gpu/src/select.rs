use crate::enumerate::enumerate;
use crate::prefs::{prefs, GpuMode, GpuPreference, PreferredGpu};
use crate::types::GpuInfo;
#[cfg(target_os = "linux")]
use crate::types::VENDOR_NVIDIA;
#[cfg(target_os = "linux")]
use std::path::PathBuf;
use std::sync::Mutex;

/// Why a GPU was selected — surfaced by the mgmt API so the console can explain the decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickSource {
    /// The operator's manual preference matched a present GPU.
    Preference,
    /// `SLIPSTREAM_RENDER_ADAPTER` substring matched.
    Env,
    /// Auto: max dedicated VRAM, with the Linux render node as the fallback.
    Auto,
    /// A manual preference is set but that GPU is absent — fell back to auto so the host keeps
    /// streaming (logged; the console shows the fallback).
    PreferenceMissing,
}

impl PickSource {
    pub fn tag(self) -> &'static str {
        match self {
            PickSource::Preference => "preference",
            PickSource::Env => "env",
            PickSource::Auto => "auto",
            PickSource::PreferenceMissing => "preference_missing",
        }
    }
}

/// A resolved selection: the GPU the next session's pipeline will be created on, and why.
#[derive(Clone, Debug)]
pub struct SelectedGpu {
    pub info: GpuInfo,
    pub source: PickSource,
}

/// Find the manually preferred GPU in the inventory. Match order: exact stable identity
/// (vendor, device, occurrence) → same model (vendor, device; a twin renumbered) → exact name
/// (ids changed across a driver/firmware quirk but the marketing name survived).
pub fn find_preferred(gpus: &[GpuInfo], want: &PreferredGpu) -> Option<usize> {
    gpus.iter()
        .position(|g| {
            g.vendor_id == want.vendor_id
                && g.device_id == want.device_id
                && g.occurrence == want.occurrence
        })
        .or_else(|| {
            gpus.iter()
                .position(|g| g.vendor_id == want.vendor_id && g.device_id == want.device_id)
        })
        .or_else(|| {
            if want.name.is_empty() {
                return None;
            }
            gpus.iter().position(|g| g.name == want.name)
        })
}

/// Pure selection over an inventory: **manual preference > env substring > max VRAM**. Returns
/// the index into `gpus` plus the reason. `None` only when `gpus` is empty. A set-but-unmatched
/// env substring falls through to max-VRAM (same outcome as env unset — deliberately more robust
/// than the old `resolve_render_adapter_luid`, which returned *no* adapter on a stale substring).
pub fn pick(
    gpus: &[GpuInfo],
    pref: &GpuPreference,
    env_substr: Option<&str>,
) -> Option<(usize, PickSource)> {
    let mut preference_missing = false;
    if pref.mode == GpuMode::Manual {
        if let Some(want) = &pref.gpu {
            match find_preferred(gpus, want) {
                Some(i) => return Some((i, PickSource::Preference)),
                None => preference_missing = true,
            }
        }
    }
    if let Some(sub) = env_substr.filter(|s| !s.is_empty()) {
        let sub = sub.to_ascii_lowercase();
        if let Some(i) = gpus
            .iter()
            .position(|g| g.name.to_ascii_lowercase().contains(&sub))
        {
            return Some((i, PickSource::Env));
        }
    }
    let i = gpus
        .iter()
        .enumerate()
        .max_by_key(|(_, g)| g.vram_bytes)
        .map(|(i, _)| i)?;
    Some((
        i,
        if preference_missing {
            PickSource::PreferenceMissing
        } else {
            PickSource::Auto
        },
    ))
}

/// The GPU the next session will run on. Mirrors the encode dispatch for display: a
/// matched manual preference wins; otherwise NVIDIA-presence → the NVIDIA GPU, else the GPU that
/// owns the VAAPI render node. (The *authoritative* Linux switches stay in `encode::open_video` /
/// [`linux_render_node`] — this is the console's view of them.)
#[cfg(target_os = "linux")]
pub fn selected_gpu() -> Option<SelectedGpu> {
    let gpus = enumerate();
    let pref = prefs().get();
    let mut preference_missing = false;
    if pref.mode == GpuMode::Manual {
        if let Some(want) = &pref.gpu {
            match find_preferred(&gpus, want) {
                Some(i) => {
                    return Some(SelectedGpu {
                        info: gpus.into_iter().nth(i)?,
                        source: PickSource::Preference,
                    })
                }
                None => preference_missing = true,
            }
        }
    }
    let source = if preference_missing {
        PickSource::PreferenceMissing
    } else {
        PickSource::Auto
    };
    if linux_nvidia_present() {
        if let Some(i) = gpus.iter().position(|g| g.vendor_id == VENDOR_NVIDIA) {
            return Some(SelectedGpu {
                info: gpus.into_iter().nth(i)?,
                source,
            });
        }
    }
    let node = linux_render_node();
    let i = gpus
        .iter()
        .position(|g| g.handle.render_node.as_deref() == Some(node.as_path()))
        .unwrap_or(0);
    Some(SelectedGpu {
        info: gpus.into_iter().nth(i)?,
        source,
    })
}

/// The manually preferred GPU, only when `mode == Manual` **and** it is currently present.
/// The Linux encode dispatch consults this (auto mode keeps today's NVIDIA-presence behavior
/// exactly).
pub fn manual_selection() -> Option<GpuInfo> {
    let pref = prefs().get();
    if pref.mode != GpuMode::Manual {
        return None;
    }
    let want = pref.gpu?;
    let gpus = enumerate();
    let i = find_preferred(&gpus, &want)?;
    gpus.into_iter().nth(i)
}

/// The VAAPI/DRM render node for this host: matched manual preference > `SLIPSTREAM_RENDER_NODE`
/// (a deliberate live env read — see `config.rs` module docs) > `/dev/dri/renderD128`.
#[cfg(target_os = "linux")]
pub fn linux_render_node() -> PathBuf {
    if let Some(g) = manual_selection() {
        if let Some(node) = g.handle.render_node {
            return node;
        }
    }
    std::env::var("SLIPSTREAM_RENDER_NODE")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/dev/dri/renderD128"))
}

/// NVIDIA-presence probe (same device-node check as `encode::nvidia_present` — duplicated two
/// lines rather than widening that private fn's visibility).
#[cfg(target_os = "linux")]
fn linux_nvidia_present() -> bool {
    std::path::Path::new("/dev/nvidiactl").exists() || std::path::Path::new("/dev/nvidia0").exists()
}

/// A cache key that changes whenever the *selection* changes (preference edits included), for the
/// per-GPU probe caches that were process-lifetime
/// `OnceLock`s back when selection was env-only.
pub fn selection_key() -> String {
    match selected_gpu() {
        Some(sel) => sel.info.id,
        None => String::new(),
    }
}

// ---------------------------------------------------------------------------------------------
// Live "in use" record
// ---------------------------------------------------------------------------------------------

/// What a live session encodes on — the console's "currently used GPU".
#[derive(Clone, Debug)]
pub struct ActiveGpu {
    /// Stable id of the GPU ([`GpuInfo::id`]; empty for the CPU/software path) so a UI can match
    /// it against the inventory.
    pub id: String,
    pub name: String,
    pub vendor_id: u32,
    /// The encode backend the session opened (`nvenc` / `amf` / `qsv` / `vaapi` / `software`).
    pub backend: &'static str,
}

struct ActiveState {
    gpu: ActiveGpu,
    sessions: u32,
}

static ACTIVE: Mutex<Option<ActiveState>> = Mutex::new(None);

/// RAII marker for one live encode session; dropping it decrements the session count. Held by the
/// encoder wrapper `open_video` returns, so the count is correct by construction (every successful
/// open is paired with a drop).
pub struct ActiveSession(());

impl Drop for ActiveSession {
    fn drop(&mut self) {
        let mut st = ACTIVE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(st) = st.as_mut() {
            st.sessions = st.sessions.saturating_sub(1);
        }
    }
}

/// Record a session opening on `gpu`. Concurrent sessions share the selection, so the latest record
/// wins and a counter tracks liveness.
pub fn session_begin(gpu: ActiveGpu) -> ActiveSession {
    let mut st = ACTIVE.lock().unwrap_or_else(|e| e.into_inner());
    let sessions = st.as_ref().map(|s| s.sessions).unwrap_or(0) + 1;
    *st = Some(ActiveState { gpu, sessions });
    ActiveSession(())
}

/// The GPU live sessions encode on + how many sessions hold it. `Some` with `sessions == 0` means
/// "last used, idle now" — the mgmt API distinguishes the two.
pub fn active() -> Option<(ActiveGpu, u32)> {
    ACTIVE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .map(|s| (s.gpu.clone(), s.sessions))
}
