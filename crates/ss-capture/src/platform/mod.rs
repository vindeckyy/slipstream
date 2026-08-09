//! OS-specific capture backends. Grouped here for structural divergence; crate-root shims in
//! `lib.rs` keeps the PipeWire and Linux helper re-exports stable.
//!
//! Re-exports of crate-root types let `linux/` keep its existing `super::…` imports without a
//! capture-logic rewrite.

#[cfg(target_os = "linux")]
pub(crate) use crate::{
    note_hdr_capture_failed, CaptureTelemetry, CapturedFrame, Capturer, DmabufFrame, FramePayload,
    HdrSource, PixelFormat, ZeroCopyPolicy,
};

#[cfg(target_os = "linux")]
pub mod linux;
