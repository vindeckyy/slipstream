//! `slipstream-session` — the Vulkan session binary (slipstream-planning
//! `linux-client-rearchitecture.md`, Phase 1: the software-path presenter MVP, which IS
//! the power-user CLI build).
//!
//! One stream session per invocation: `--connect host[:port]` (+ `--fp HEX`,
//! `--launch id`, `--fullscreen`), exits when the session ends. Reads the same identity
//! / known-hosts / settings stores as the Linux desktop shell, so pairing in either entry
//! point makes the other connect silently. `--pair <PIN> --connect host` runs the ceremony here,
//! with no window and no toolkit, for machines that have only a shell.
//!
//! Stdout is the machine interface (the shell↔session contract): `{"ready":true}` after
//! the first presented frame, `stats:` lines per 1 s window, one `{"error": …}` /
//! `{"ended": …}` JSON line on the way out. Logs go to stderr. Exit codes: 0 clean end,
//! 2 connect failed, 3 trust rejected / pairing required, 4 presenter init failed.
#![forbid(unsafe_code)]

#[cfg(target_os = "linux")]
mod session;

#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(session::run())
}

/// This stub keeps `cargo build --workspace` green elsewhere (the Mac client lives in
/// clients/apple).
#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("slipstream-session is available on Linux");
    std::process::exit(2);
}
