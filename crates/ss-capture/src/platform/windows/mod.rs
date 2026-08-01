//! Windows capture: DXGI helpers, IDD direct-push, and the synthetic NV12 source.
//!
//! Crate-root type re-exports keep `idd_push`'s `super::{CapturedFrame, …}` and `super::dxgi`
//! imports working after the move under `platform/`.

pub(crate) use crate::{CapturedFrame, Capturer, FramePayload, PixelFormat};

pub mod dxgi;
pub(crate) mod idd_push;
pub mod synthetic_nv12;

pub use idd_push::verify_is_wudfhost;
