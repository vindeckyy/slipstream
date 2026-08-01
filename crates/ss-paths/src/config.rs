use std::path::PathBuf;

/// The host config dir (host identity, pairing state, mgmt token, library) — created on demand.
/// Linux: `$XDG_CONFIG_HOME/slipstream` or `~/.config/slipstream`. Windows: `%ProgramData%\slipstream`
/// (machine-wide — the SYSTEM service and the interactive user share ONE dir that survives logout).
/// `SLIPSTREAM_CONFIG_DIR` overrides on both platforms (used by the Windows service config / tests).
pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("SLIPSTREAM_CONFIG_DIR").filter(|s| !s.is_empty()) {
        return PathBuf::from(dir);
    }
    // Windows: %ProgramData% (e.g. C:\ProgramData\slipstream) — machine-wide, SYSTEM-readable,
    // persists across user logout, correct for a SYSTEM service. Falls back to %APPDATA% then CWD.
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("ProgramData")
        .or_else(|| std::env::var_os("APPDATA"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    #[cfg(not(target_os = "windows"))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("slipstream")
}
