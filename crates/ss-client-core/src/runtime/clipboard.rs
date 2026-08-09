//! Clipboard integration for the Linux session client.
//!
//! The host-side clipboard transport remains available through `ss-clipboard`. The desktop
//! session client currently exposes the shared API without claiming a local clipboard backend.

use slipstream_core::client::NativeClient;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Keep the session hook stable until the Linux desktop clipboard backend is wired in.
pub fn run(client: Arc<NativeClient>, stop: Arc<AtomicBool>) {
    let _ = (client, stop);
}

/// Copy-link integration is deferred until the Linux desktop clipboard backend is available.
pub fn set_text(_text: &str) {}
