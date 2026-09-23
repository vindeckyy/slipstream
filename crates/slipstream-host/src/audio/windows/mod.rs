//! Windows audio backends: WASAPI loopback capture (system sound) for the host→client
//! plane. The virtual microphone (client mic → host apps) needs a virtual audio driver
//! and lands with the packaging todo — until then it fails loudly (see `audio.rs`).
#[cfg(target_os = "windows")]
mod wasapi;

#[cfg(target_os = "windows")]
pub use wasapi::WasapiLoopback;
