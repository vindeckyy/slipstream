//! NVIDIA NvFBC capture — Phase 2 stub.
//!
//! SolarFlare loads `libnvidia-fbc.so` and grabs the desktop via NvFBC. Slipstream will grow the
//! same path later; this module only probes for the shared library and fails open with a clear
//! "not yet" so `auto` and the management API can skip it.

use anyhow::{bail, Result};
use std::path::Path;

use super::Capturer;

/// Well-known library names / paths NvFBC ships as on Linux.
const CANDIDATES: &[&str] = &[
    "libnvidia-fbc.so.1",
    "libnvidia-fbc.so",
    "/usr/lib/x86_64-linux-gnu/libnvidia-fbc.so.1",
    "/usr/lib64/libnvidia-fbc.so.1",
    "/usr/lib/libnvidia-fbc.so.1",
];

/// True when a `libnvidia-fbc.so*` looks present on disk (PATH/`ldconfig` not consulted deeply —
/// just the common install locations and a `LD_LIBRARY_PATH` walk). Does not `dlopen`.
pub fn probe_nvfbc() -> bool {
    for name in CANDIDATES {
        if Path::new(name).is_file() {
            return true;
        }
    }
    if let Some(ld) = std::env::var_os("LD_LIBRARY_PATH") {
        for dir in std::env::split_paths(&ld) {
            for name in ["libnvidia-fbc.so.1", "libnvidia-fbc.so"] {
                if dir.join(name).is_file() {
                    return true;
                }
            }
        }
    }
    false
}

/// Open an NvFBC desktop capturer. Phase 2: always errors.
pub fn open_nvfbc_desktop() -> Result<Box<dyn Capturer>> {
    let _ = probe_nvfbc();
    bail!("NvFBC desktop capture is not yet implemented (Phase 2)")
}
