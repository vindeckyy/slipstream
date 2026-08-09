//! DRM connector force-off — Linux stand-in for the EXPERIMENTAL `pnp_disable_monitors` axis.
//!
//! On Linux, Exclusive topology already drops physical outputs from the compositor;
//! this module goes one step further by writing `off` to `/sys/class/drm/<connector>/status` for
//! connected non-virtual connectors, so the kernel stops servicing link probes. Restore writes
//! `detect` (or `on` as a fallback). Crash safety: connector names are journaled under the host
//! config dir and re-enabled on [`startup_recover`].

use std::fs;
use std::path::{Path, PathBuf};

fn journal_path() -> PathBuf {
    ss_paths::config_dir().join("drm-forced-off-connectors.json")
}

fn read_journal() -> Vec<String> {
    match fs::read(journal_path()) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn write_journal(names: &[String]) {
    let path = journal_path();
    if names.is_empty() {
        let _ = fs::remove_file(&path);
        return;
    }
    if let Some(dir) = path.parent() {
        let _ = ss_paths::create_private_dir(dir);
    }
    if let Err(e) = fs::write(&path, serde_json::to_vec_pretty(names).unwrap_or_default()) {
        tracing::warn!(error = %e, "DRM force-off: could not write the crash-recovery journal");
    }
}

fn is_virtual_connector(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("virtual") || n.contains("vrt") || n.contains("slipstream")
}

fn is_internal_panel(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("edp") || n.contains("lvds") || n.contains("dsi")
}

/// Connected DRM connectors under `/sys/class/drm`, excluding virtual and (by default) internal
/// eDP/LVDS panels — those rarely cause the standby-TV HPD class this axis targets.
fn connected_external(base: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(base) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        // Connector dirs look like `card1-DP-1`, not bare `card1` / `renderD128`.
        if !name.contains('-') {
            continue;
        }
        if is_virtual_connector(&name) || is_internal_panel(&name) {
            continue;
        }
        let status = e.path().join("status");
        let Ok(s) = fs::read_to_string(&status) else {
            continue;
        };
        if s.trim() == "connected" {
            out.push(name);
        }
    }
    out
}

fn write_status(connector: &str, value: &str) -> bool {
    let path = Path::new("/sys/class/drm").join(connector).join("status");
    match fs::write(&path, value) {
        Ok(()) => {
            tracing::info!(connector, value, "DRM force-off: wrote connector status");
            true
        }
        Err(e) => {
            tracing::warn!(
                connector,
                value,
                error = %e,
                "DRM force-off: could not write connector status (need write access to sysfs?)"
            );
            false
        }
    }
}

/// Force-off every connected external DRM connector. Returns the connector names that were
/// successfully forced (for journaling / restore). Call after Exclusive topology has dropped
/// physicals from the desktop, or for the standby-TV selector in any topology.
pub fn force_off_connected_external() -> Vec<String> {
    let targets = connected_external(Path::new("/sys/class/drm"));
    if targets.is_empty() {
        tracing::info!(
            "DRM force-off: no connected external connectors — the pnp_disable_monitors axis \
             did nothing"
        );
        return Vec::new();
    }
    let mut forced = Vec::new();
    for name in targets {
        if write_status(&name, "off") {
            forced.push(name);
        }
    }
    if !forced.is_empty() {
        let mut journal = read_journal();
        for n in &forced {
            if !journal.contains(n) {
                journal.push(n.clone());
            }
        }
        write_journal(&journal);
    }
    forced
}

/// Restore connectors previously forced off (write `detect`, then `on` if needed).
pub fn restore(connectors: &[String]) {
    if connectors.is_empty() {
        return;
    }
    for name in connectors {
        if !write_status(name, "detect") {
            let _ = write_status(name, "on");
        }
    }
    let mut journal = read_journal();
    journal.retain(|n| !connectors.contains(n));
    write_journal(&journal);
    tracing::info!(reenabled = ?connectors, "DRM force-off: restored connectors");
}

/// Re-enable leftovers from a previous crash/kill before any new session touches topology.
pub fn startup_recover() {
    let leftover = read_journal();
    if leftover.is_empty() {
        return;
    }
    tracing::warn!(
        connectors = ?leftover,
        "DRM force-off: re-enabling connectors left forced-off by a previous session"
    );
    restore(&leftover);
}
