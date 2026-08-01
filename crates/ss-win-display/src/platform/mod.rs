//! Windows display-topology helpers (CCD/GDI, PnP, display-change watch, input-desktop binding).

#[cfg(target_os = "windows")]
pub mod display_events;
/// Bind display-config writes to the input desktop so a UAC / lock screen can't refuse them.
#[cfg(target_os = "windows")]
pub(crate) mod input_desktop;
#[cfg(target_os = "windows")]
pub mod monitor_devnode;
/// Cross-crate "topology churn in flight" latch (pure std — no Windows surface, so unconditionally
/// compiled and unit-tested on every platform).
pub mod topology_churn;
#[cfg(target_os = "windows")]
pub mod win_display;
