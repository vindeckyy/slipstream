//! Host clipboard backends (Linux Wayland data-control / Mutter direct, Windows
//! sequence-polling + eager writes) and the session coordinator.
//!
//! Public API stays at [`crate::host`] — this module is the implementation tree.

pub mod host;
