//! Host config-dir + owner-private file helpers — a leaf crate so the subsystem crates
//! (`ss-media`, `ss-vdisplay`) and the orchestrator can all reach them WITHOUT depending on the
//! `gamestream` module they used to live in (plan §2.4 / §W6: the secret helpers were shared
//! vocabulary parked above their consumers in the junk drawer). Pure std + `tracing`; no I/O stack.
//!
//! - [`config_dir`] resolves the per-host config directory (`XDG_CONFIG_HOME`, `SLIPSTREAM_CONFIG_DIR` override).
//! - [`create_private_dir`] makes it owner-private (0700).
//! - [`write_secret_file`] writes an owner-only secret (0600).
#![forbid(unsafe_code)]

mod config;
mod secret;

#[cfg(target_os = "linux")]
mod linux;

pub use config::config_dir;
pub use secret::{create_private_dir, write_secret_file};

#[cfg(target_os = "linux")]
pub use linux::gamescope_ei_socket_file;
