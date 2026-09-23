//! Windows capture backends: Windows Graphics Capture (primary) + DXGI Desktop
//! Duplication (fallback), behind the shared [`Capturer`] trait.
//!
//! Both backends deliver CPU `Bgra` frames (8-bit SDR, cursor embedded) into a one-deep
//! overwriting slot (`common::FrameSlot`), so a stalled consumer costs intermediate frames
//! and is still handed the freshest one — the same contract as the Linux portal capturer.
//! GPU texture sharing (D3D11 → NVENC, the Windows analogue of the Linux zero-copy path)
//! lands with the encode todo; until then `telemetry()` reports the CPU path honestly.

// Re-exports of crate-root types let `windows/` keep `super::…` imports without a
// capture-logic rewrite (mirroring the Linux tree).
pub(crate) use crate::{
    capture_now_ns, CaptureTelemetry, CapturedFrame, Capturer, FramePayload, PixelFormat,
};

mod common;
mod dxgi;
mod wgc;

pub use dxgi::DxgiCapturer;
pub use wgc::WgcCapturer;
