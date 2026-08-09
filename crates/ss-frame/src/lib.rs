//! Shared frame and pixel-format types for the Linux capture and encode paths, plus helpers for
//! HDR metadata, stall detection, thread QoS, and session tuning.

// Unsafe-proof program: every `unsafe {}` / `unsafe impl` must carry a `// SAFETY:` proof.
#![deny(clippy::undocumented_unsafe_blocks)]

mod core;

pub use core::frame::*;

pub use core::hdr;
pub use core::metronome;
pub use core::session_tuning;
pub use core::thread_qos;

#[cfg(target_os = "linux")]
pub use core::worker_qos;
