//! Shared, UI-agnostic client plumbing, extracted verbatim from the GTK client
//! (design: slipstream-planning `linux-client-rearchitecture.md`, Phase 0) so the desktop
//! shells and the Vulkan session binary build on one Linux implementation.
//!
//! Nothing here may depend on a UI toolkit: the presenter contract is `session`'s
//! channels (`SessionHandle`) and `video`'s `DecodedImage` (RGBA bytes, dmabuf fds +
//! plane layout, or a decoded VkImage) — how frames reach the screen is the consumer's
//! business.
//!
//! Audio uses the PipeWire backend on Linux. `keymap` (evdev-keyed) stays in the client core;
//! the session path uses ss-presenter's SDL-scancode table as well.
//!
//! Layout: sources live under `media/` (decode + audio) and `runtime/` (session,
//! discovery, trust, …). Crate-root module names stay stable via `#[path]` so
//! `ss_client_core::video` and friends do not move.

// Unsafe-proof program: every `unsafe {}` / `unsafe impl` in this crate carries a `// SAFETY:`
// proof of why it is sound. This crate held ~91 unsafe items with NO enforcement while every
// other subsystem crate denied it — the decoders' `unsafe impl Send`s had a one-line aside
// instead of an argument precisely because nothing required one.
#![deny(clippy::undocumented_unsafe_blocks)]

#[cfg(target_os = "linux")]
#[path = "media/audio.rs"]
pub mod audio;
#[cfg(target_os = "linux")]
#[path = "runtime/discovery.rs"]
pub mod discovery;
#[cfg(target_os = "linux")]
#[path = "runtime/gamepad.rs"]
pub mod gamepad;
#[cfg(target_os = "linux")]
#[path = "runtime/keymap.rs"]
pub mod keymap;
#[cfg(target_os = "linux")]
#[path = "runtime/library.rs"]
pub mod library;
// The `slipstream://` grammar (design/client-deep-links.md §2): one parser/emitter for the
// shells, the session and the CLI, held to the Swift/Kotlin ports by a shared vector file.
#[cfg(target_os = "linux")]
#[path = "runtime/deeplink.rs"]
pub mod deeplink;
// The brain layer (design/client-architecture-split.md §3): what a connect is, the wake
// state machine every front-end drives, and the session spawn + stdout contract.
#[cfg(target_os = "linux")]
#[path = "runtime/orchestrate.rs"]
pub mod orchestrate;
// The host's OS-identity chain (mDNS `os=` TXT): sanitize + icon-walk order. Pure string
// logic, built everywhere (the Apple/Android ports mirror it rather than link it).
#[path = "runtime/os.rs"]
pub mod os;
// Client settings profiles: the override catalog + the one connect-time resolver
// (design/client-settings-profiles.md §4). Sits beside `trust`, which owns the host records
// the bindings live on.
#[cfg(target_os = "linux")]
#[path = "runtime/profiles.rs"]
pub mod profiles;
#[cfg(target_os = "linux")]
#[path = "runtime/session.rs"]
pub mod session;
#[cfg(target_os = "linux")]
#[path = "runtime/trust.rs"]
pub mod trust;
// "Is a newer client available, and can this box install it?" — the client half of the
// signed-manifest update check the host already runs (design: host-update-from-web-console.md).
#[cfg(target_os = "linux")]
#[path = "runtime/update.rs"]
pub mod update;
#[cfg(target_os = "linux")]
#[path = "media/video.rs"]
pub mod video;
#[cfg(target_os = "linux")]
#[path = "media/video_color.rs"]
mod video_color;
#[cfg(target_os = "linux")]
#[path = "media/video_software.rs"]
mod video_software;
// libav ownership helpers shared by the hardware decoders below (`AvBuffer`).
#[cfg(target_os = "linux")]
#[path = "media/video_libav.rs"]
mod video_libav;
#[cfg(target_os = "linux")]
#[path = "media/video_vaapi.rs"]
mod video_vaapi;
#[cfg(target_os = "linux")]
#[path = "media/video_vulkan.rs"]
mod video_vulkan;
// The OS-clipboard bridge for the shared clipboard (design/clipboard-and-file-transfer.md §5).
#[cfg(target_os = "linux")]
#[path = "runtime/clipboard.rs"]
pub mod clipboard;
#[cfg(all(target_os = "linux", feature = "pyrowave"))]
#[path = "media/video_pyrowave.rs"]
pub mod video_pyrowave;

#[path = "runtime/wol.rs"]
pub mod wol;
