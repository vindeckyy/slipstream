//! DRM/KMS plane capture — Phase 2 stub.
//!
//! SolarFlare captures the primary plane via DRM dumb buffers / dmabuf export. Slipstream will
//! grow the same path later; this module only probes for a usable card and fails open with a
//! clear "not yet" so `auto` and the management API can skip it without looking broken.

use anyhow::{bail, Result};
use std::path::Path;

use super::Capturer;

/// True when `/dev/dri/card0` (or `card1`) exists — a weak signal that KMS *could* work once
/// implemented. Does not check CAP_SYS_ADMIN / render-node permissions.
pub fn probe_kms() -> bool {
    Path::new("/dev/dri/card0").exists() || Path::new("/dev/dri/card1").exists()
}

/// Open a KMS desktop capturer. Phase 2: always errors.
pub fn open_kms_desktop() -> Result<Box<dyn Capturer>> {
    let _ = probe_kms();
    bail!("KMS desktop capture is not yet implemented (Phase 2)")
}
