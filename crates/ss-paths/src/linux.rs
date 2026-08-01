use std::path::PathBuf;

/// The shared path of the file where the gamescope backend relays the nested session's
/// `LIBEI_SOCKET` (gamescope's EIS server) for the input injector: `$XDG_RUNTIME_DIR/
/// slipstream-gamescope-ei` (per-user 0700), or `/tmp/…` when the runtime dir is unset. It is a
/// **contract shared** by the gamescope producer (`ss-vdisplay`, which writes it under the session
/// env lock) and the libei consumer (`ss-inject`, which reads it after the session env is applied) —
/// a leaf so neither subsystem crate has to reach into the other (plan §W6). Linux-only.
pub fn gamescope_ei_socket_file() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR").filter(|s| !s.is_empty()) {
        Some(rt) => PathBuf::from(rt).join("slipstream-gamescope-ei"),
        None => PathBuf::from("/tmp/slipstream-gamescope-ei"),
    }
}
