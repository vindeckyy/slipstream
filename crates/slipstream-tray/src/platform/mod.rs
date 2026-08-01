//! Platform-specific tray backends (Windows Win32 / Linux StatusNotifierItem).

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(windows)]
pub mod win;
#[cfg(windows)]
pub mod win_theme;
