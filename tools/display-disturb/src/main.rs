//! `display-disturb` — deterministic display-stack disturbance generator (Windows).
//!
//! The vdisplay stall-immunity bench (design: slipstream-planning
//! `design/vdisplay-disturbance-immunity.md`) needs both stall classes reproducible on demand,
//! without waiting for a standby TV or a monitor-tool storm:
//!
//! * `ddc` — Class 2: DDC/CI traffic through the win32k → dxgkrnl → miniport I2C path, exactly
//!   what Twinkle-Tray/PowerDisplay-class tools emit after every HPD blip (one VCP read ≈ 100 ms,
//!   a capabilities string up to ~1 s — serialized per physical I2C bus). Requires a physical
//!   monitor; virtual displays expose no DDC handle.
//! * `modeset` — Class 1: a same-mode `ChangeDisplaySettingsExW(CDS_RESET)` re-commit — a
//!   Level-Two modeset-class DDI entry that idles the whole adapter ("the graphics hardware is
//!   idle") without changing anything Win32-visible.
//!
//! Every operation prints `epoch_ms op target duration_ms result` so stalls in a concurrent
//! stream's host.log correlate line-for-line. The per-op duration is itself measurement: it is
//! the I2C/modeset service time the GPU driver spent, per disturbance.
//!
//! Usage: `display-disturb ddc [--interval-ms 2000] [--caps] [--vcp 0x10]`
//!        `display-disturb modeset [--interval-ms 2000]`

// Unsafe-proof program: every `unsafe {}` in this tool carries a `// SAFETY:` proof.
#![deny(clippy::undocumented_unsafe_blocks)]

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("display-disturb is Windows-only (it exercises the WDDM display stack).");
    std::process::exit(2);
}

#[cfg(target_os = "windows")]
mod win;

#[cfg(target_os = "windows")]
fn main() {
    win::main()
}
