//! slipstream-tray — a small per-user system-tray companion for the slipstream host service.
//!
//! Shows at a glance whether the host is running, stopped, degraded, or failed, and offers one-click
//! access to the web console and Linux service controls.
//!
//! Status comes from two sources. The systemd user unit determines process state, then the host's
//! loopback-only unauthenticated `GET /api/v1/local/summary` supplies streaming details.
// Unsafe-proof program: every `unsafe {}` in the tray carries a `// SAFETY:` proof.
#![deny(clippy::undocumented_unsafe_blocks)]
#[cfg(target_os = "linux")]
mod platform;
#[cfg(target_os = "linux")]
mod status;

#[cfg(target_os = "linux")]
use platform::linux;

/// CLI configuration (hand-rolled parse, house style). The mgmt address/port default to the
/// host's defaults; they are flags so an operator who moved `--mgmt-bind` can
/// adjust the autostart command line instead.
pub struct Args {
    /// Ask an already-running tray instance to exit.
    pub quit: bool,
    /// Launched from the desktop autostart entry; the package installs the entry for every desktop
    /// user.
    pub autostart: bool,
    /// Management API address to poll (loopback only).
    pub mgmt_addr: String,
    pub mgmt_port: u16,
    /// Web console port for the "Open web console" action.
    pub web_port: u16,
}

impl Default for Args {
    fn default() -> Self {
        Args {
            quit: false,
            autostart: false,
            mgmt_addr: "127.0.0.1".into(),
            mgmt_port: 47990,
            web_port: 47992,
        }
    }
}

fn parse_args() -> anyhow::Result<Args> {
    let mut args = Args::default();
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut value = |flag: &str| {
            it.next()
                .ok_or_else(|| anyhow::anyhow!("{flag} needs a value"))
        };
        match a.as_str() {
            "--quit" => args.quit = true,
            "--autostart" => args.autostart = true,
            "--mgmt-addr" => args.mgmt_addr = value("--mgmt-addr")?,
            "--mgmt-port" => args.mgmt_port = value("--mgmt-port")?.parse()?,
            "--web-port" => args.web_port = value("--web-port")?.parse()?,
            "--version" | "-V" => {
                println!("slipstream-tray {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            other => anyhow::bail!(
                "unknown argument '{other}'\n\nUSAGE:\n    slipstream-tray [--autostart] [--quit] \
                 [--mgmt-addr <IP>] [--mgmt-port <N>] [--web-port <N>]"
            ),
        }
    }
    Ok(args)
}

fn main() -> anyhow::Result<()> {
    let args = parse_args()?;
    run(args)
}

#[cfg(target_os = "linux")]
fn run(args: Args) -> anyhow::Result<()> {
    linux::run(args)
}

#[cfg(not(target_os = "linux"))]
fn run(_args: Args) -> anyhow::Result<()> {
    anyhow::bail!("slipstream-tray supports Linux hosts only")
}
