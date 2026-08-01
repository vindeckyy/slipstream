//! Grouped update-check modules (detect / feed / manifest / sig / version).
//!
//! Re-exported at the crate root so `ss_update_check::{detect,feed,...}` and the flat
//! `ss_update_check::*` type aliases stay stable.

pub mod detect;
pub mod feed;
pub mod manifest;
pub mod sig;
pub mod version;
