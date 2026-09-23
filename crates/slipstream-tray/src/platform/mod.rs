//! Status-icon tray backends: Linux StatusNotifierItem, Windows Shell_NotifyIcon.

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub mod windows;
