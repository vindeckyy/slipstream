//! OS-specific capture backends. Grouped here for structural divergence; crate-root shims in
//! `lib.rs` keep `ss_capture::dxgi`, `ss_capture::pwinit`, and the Linux helper re-exports stable.
//!
//! Re-exports of crate-root types let `linux/` keep its existing `super::…` imports without a
//! capture-logic rewrite (same pattern as `windows/mod.rs` for IDD-push).

#[cfg(target_os = "linux")]
pub(crate) use crate::{
    note_hdr_capture_failed, CapturedFrame, Capturer, DmabufFrame, FramePayload, HdrSource,
    PixelFormat, ZeroCopyPolicy,
};

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;
