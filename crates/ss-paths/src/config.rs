use std::path::PathBuf;

/// The host config dir (host identity, pairing state, mgmt token, library) — created on demand.
/// Linux: `$XDG_CONFIG_HOME/slipstream` or `~/.config/slipstream`.
/// Windows: `%APPDATA%\slipstream` (roaming) — per-user, ACL-restricted by the OS.
/// `SLIPSTREAM_CONFIG_DIR` overrides the default on every platform.
pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("SLIPSTREAM_CONFIG_DIR").filter(|s| !s.is_empty()) {
        return PathBuf::from(dir);
    }
    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA").map(PathBuf::from) {
            return appdata.join("slipstream");
        }
        if let Some(profile) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
            return profile.join("AppData").join("Roaming").join("slipstream");
        }
        PathBuf::from(".").join("slipstream")
    }
    #[cfg(not(windows))]
    {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("slipstream")
    }
}
