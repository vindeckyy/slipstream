//! Per-OS clipboard backends (Wayland data-control, Mutter, Win32) and the session coordinator.
//!
//! Public API stays at [`crate::host`] — this module is the implementation tree.

pub mod host;
