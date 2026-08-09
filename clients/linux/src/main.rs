//! `slipstream-client` — the native Linux slipstream/1 desktop shell (relm4/libadwaita).
//!
//! Hosts, pairing, trust, settings, and the desktop library page. Every stream runs in the
//! spawned `slipstream-session` Vulkan binary; the shell never touches video.
#![forbid(unsafe_code)]

// The UI-agnostic plumbing lives in `ss-client-core`, shared with the session binary.
// Root re-exports keep every `crate::trust`-style path resolving unchanged.
#[cfg(target_os = "linux")]
pub use ss_client_core::{discovery, gamepad, library, os, trust, video, wol};

#[cfg(target_os = "linux")]
mod app;
#[cfg(target_os = "linux")]
mod ui;
// Root shims: preserve `crate::cli` / `crate::ui_hosts` paths used across the shell.
#[cfg(target_os = "linux")]
mod cli;
// "Create shortcut…" — the desktop-entry writer (design/client-deep-links.md §5).
#[cfg(target_os = "linux")]
mod shortcuts;
#[cfg(target_os = "linux")]
mod ui_hosts;
#[cfg(target_os = "linux")]
mod ui_library;
#[cfg(target_os = "linux")]
mod ui_settings;
#[cfg(target_os = "linux")]
mod ui_trust;

#[cfg(target_os = "linux")]
fn main() -> gtk::glib::ExitCode {
    app::run()
}

/// GTK4/SDL3 are Linux turf; this stub keeps `cargo build --workspace` green on macOS
/// (the Mac client lives in clients/apple).
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("slipstream-client is Linux-only — the macOS client lives in clients/apple");
    std::process::exit(2);
}
