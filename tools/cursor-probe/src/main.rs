//! Cursor-as-metadata on-glass probe (see Cargo.toml). Run inside the target user session:
//!
//! ```sh
//! DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$(id -u)/bus cursor-probe [--seconds 20] [--embedded] [--gpu]
//! ```
//!
//! `--embedded` A/Bs the pre-channel path (compositor embeds the pointer, no metadata expected);
//! `--gpu` takes the zero-copy dmabuf negotiation a real session uses instead of the CPU mmap path.

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    linux::run()
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("cursor-probe is Linux-only (compositor virtual outputs)");
    std::process::exit(1);
}
