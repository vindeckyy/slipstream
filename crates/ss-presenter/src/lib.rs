//! The Vulkan session presenter: an SDL3 window and ash swapchain that present the shared
//! session pump's
//! decoded frames, captures input on the `ui_stream` state-machine contract, and reports
//! the unified stats window on stdout. No UI toolkit anywhere in the dependency tree.
//!
//! Three frame paths: software (`CpuFrame` RGBA staging upload), Vulkan Video (the
//! decoder's VkImage on THIS device — plane views + the CICP-driven CSC pass), and on
//! Linux additionally VAAPI hardware (NV12 dmabuf imported per-plane — `dmabuf.rs`),
//! all composited by a letterboxed blit. Devices without the import extensions, and any
//! import/present failure streak, demote the decoder to software via the session pump's
//! `force_software` contract, same as the GTK presenter.
//!
//! Builds on Linux with DRM-PRIME dmabuf import for hardware-decoded frames and
//! software fallback when the Vulkan decode path is unavailable.
//!
//! Layout: sources live under `present/` (including `present/vk/`). Crate-root module
//! names stay stable via `#[path]` so `ss_presenter::vk` and friends do not move.

// Unsafe-proof program: every `unsafe {}` in this crate carries a `// SAFETY:` proof.
#![deny(clippy::undocumented_unsafe_blocks)]

// THE VULKAN CONTRACT, stated once - most `// SAFETY:` proofs in this crate are an instance of it.
//
// Nearly every `unsafe` here is an `ash` call, which is `unsafe` because Vulkan is a C API, not
// because each call carries its own bespoke obligation. Three shapes recur, and only one of them
// has a real precondition worth restating per site:
//
//  * CREATE / ALLOCATE - `create_*`, `allocate_*`. The device is live (this type owns it), and the
//    `vk::*CreateInfo` builders are locals that outlive the synchronous call. The handle returned is
//    owned by the value being constructed and destroyed in its `Drop`.
//  * RECORD - `cmd_*`, `begin/end_command_buffer`, `update_descriptor_sets`. Recorded into a command
//    buffer this code owns and has begun, referencing handles it also owns. Nothing executes until
//    submit, so a recording error is not yet a memory error.
//  * DESTROY - `destroy_*`, `free_*`, `unmap_memory`. THIS is the one with a real obligation: the
//    GPU must not still be using the object. That is established on the path, not by the call - a
//    fence wait, a `queue_wait_idle`, or the swapchain having been retired - and the per-site proofs
//    say so, because getting it wrong is a use-after-free the type system cannot catch.
//
// A block doing something OUTSIDE these three shapes gets a real, specific proof; if you add one and
// find yourself writing "as above", it probably belongs in one of them.

#[path = "present/csc.rs"]
pub mod csc;
#[path = "present/cursor.rs"]
pub mod cursor;
#[path = "present/dmabuf.rs"]
pub mod dmabuf;
#[path = "present/input.rs"]
pub mod input;
#[path = "present/keymap_sdl.rs"]
pub mod keymap_sdl;
#[path = "present/overlay.rs"]
pub mod overlay;
#[path = "present/run.rs"]
mod run;
#[path = "present/touch.rs"]
pub mod touch;
#[path = "present/vk/mod.rs"]
pub mod vk;

pub use run::{run_browse, run_session, ActionOutcome, Outcome, SessionOpts};
