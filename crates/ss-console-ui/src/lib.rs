//! The Skia console UI (slipstream-planning `linux-client-rearchitecture.md` §6): an
//! [`Overlay`](ss_presenter::overlay::Overlay) implementation rendering on the
//! PRESENTER's Vulkan device into offscreen RGBA images the presenter composites as one
//! premultiplied quad. Skia never touches the swapchain, and nothing here runs while
//! the overlay has nothing to show — the §6.1 invariants live or die in this crate.
//!
//! The console is a full couch shell now — home (host carousel), the game library
//! coverflow, settings, add-host, and PIN pairing, with screen transitions, per-pad
//! button glyphs, and a controller keyboard (suppressed on Steam Deck, where Steam's
//! own keyboard types through SDL text input) — plus the in-stream chrome: stats OSD,
//! capture hint, start banner.
//!
//! Layout: sources live under `ui/` (screens, shell, widgets, theme, anim, …).
//! Crate-root module names stay stable via `#[path]` so `ss_console_ui::*` pubs do not
//! move.

// Unsafe-proof program: every `unsafe {}` in the Skia/Vulkan overlay carries a `// SAFETY:` proof.
#![deny(clippy::undocumented_unsafe_blocks)]

#[cfg(target_os = "linux")]
#[path = "ui/anim.rs"]
mod anim;
#[cfg(target_os = "linux")]
#[path = "ui/glyphs.rs"]
mod glyphs;
#[cfg(target_os = "linux")]
#[path = "ui/library.rs"]
pub mod library;
#[cfg(target_os = "linux")]
#[path = "ui/model.rs"]
pub mod model;
#[cfg(target_os = "linux")]
#[path = "ui/screens.rs"]
mod screens;
#[cfg(target_os = "linux")]
#[path = "ui/shell.rs"]
mod shell;
#[cfg(target_os = "linux")]
#[path = "ui/skia_overlay.rs"]
mod skia_overlay;
#[cfg(target_os = "linux")]
#[path = "ui/theme.rs"]
mod theme;
#[cfg(target_os = "linux")]
#[path = "ui/widgets.rs"]
mod widgets;

#[cfg(target_os = "linux")]
pub use library::{LibraryGame, LibraryPhase, LibraryShared};
#[cfg(target_os = "linux")]
pub use model::{ConsoleBus, ConsoleCmd, ConsoleShared, HostRow, PairPhase, WakeStatus};
#[cfg(target_os = "linux")]
pub use shell::ConsoleOptions;
#[cfg(target_os = "linux")]
pub use skia_overlay::{ConsoleEntry, ConsoleHandles, SkiaOverlay};
