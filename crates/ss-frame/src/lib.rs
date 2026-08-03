//! The shared media-pipeline vocabulary (plan §W6): the frame + pixel-format types that capture
//! (producer) and encode (consumer) both speak, extracted into a leaf crate so `ss-capture` and
//! `ss-encode` depend on the vocabulary WITHOUT depending on each other. The GPU payloads pull
//! their heavy backends in from below: `FramePayload::Cuda` owns a [`ss_zerocopy::DeviceBuffer`],
//! `FramePayload::D3d11` a [`dxgi::D3d11Frame`].
//!
//! Alongside the vocabulary live the small pure helpers that ride the same capture-encode seam:
//! [`hdr`] (HDR static metadata / in-band SEI), [`metronome`] (the metronomic-stall detector),
//! [`thread_qos`] (per-thread scheduling QoS), [`session_tuning`] (Windows process session
//! tuning), and — on Windows — [`dxgi`] (the capture identity + D3D11 device creation).

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

#[cfg(target_os = "windows")]
pub use core::dxgi;
