use std::path::PathBuf;

/// The host config dir (host identity, pairing state, mgmt token, library) — created on demand.
/// Linux: `$XDG_CONFIG_HOME/slipstream` or `~/.config/slipstream`.
/// `SLIPSTREAM_CONFIG_DIR` overrides the default.
pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("SLIPSTREAM_CONFIG_DIR").filter(|s| !s.is_empty()) {
        return PathBuf::from(dir);
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("slipstream")
}
