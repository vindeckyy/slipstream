//! SolarFlare-style private headless compositor spawner.
//!
//! Spawns an isolated Wayland session for headless hosts that have no live desktop.
//! Distinct from the [`super::gamescope`] VirtualDisplay backend (which creates a
//! client-sized capture output for an active stream): this module only brings up a
//! private compositor the host can then attach to.
//!
//! Backends: labwc (`WLR_BACKENDS=headless`), krfb-virtualmonitor (KWin), gamescope
//! `--headless`, and auto (KDE→krfb, else labwc; gamescope session→nested gamescope).
//! No hermes-kms.
//!
//! Controlled by `SLIPSTREAM_HEADLESS_COMPOSITOR` (`off` | `auto` | `labwc` | `krfb` |
//! `gamescope`). Dropping [`HeadlessSession`] tears the child / virtual output down.

use anyhow::{bail, Context, Result};
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Default mode for a private session when the caller does not name one.
const DEFAULT_W: u32 = 1920;
const DEFAULT_H: u32 = 1080;
const DEFAULT_HZ: u32 = 60;

/// How long to wait for a new Wayland socket / HEADLESS-1 output / XWayland display.
const DISCOVER_BUDGET: Duration = Duration::from_secs(5);
const DISCOVER_STEP: Duration = Duration::from_millis(100);

/// Graceful-then-force kill budget for labwc / gamescope process groups.
const STOP_BUDGET: Duration = Duration::from_secs(3);

/// Backend selection for a private headless compositor session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeadlessBackend {
    /// Pick from the live session: KWin+krfb → krfb, gamescope desktop → gamescope, else labwc.
    Auto,
    /// labwc with wlroots headless backend (`WLR_BACKENDS=headless`).
    Labwc,
    /// `krfb-virtualmonitor` against a running KWin session.
    Krfb,
    /// Nested / private `gamescope --headless`.
    Gamescope,
}

impl HeadlessBackend {
    /// Parse a `SLIPSTREAM_HEADLESS_COMPOSITOR` value (`auto` / `labwc` / `krfb` / `gamescope`).
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Self::Auto,
            "labwc" => Self::Labwc,
            "krfb" | "krfb-virtualmonitor" => Self::Krfb,
            "gamescope" => Self::Gamescope,
            _ => return None,
        })
    }

    /// Stable id matching the host-config / console select values.
    pub fn id(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Labwc => "labwc",
            Self::Krfb => "krfb",
            Self::Gamescope => "gamescope",
        }
    }

    /// Human label for UIs.
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Labwc => "labwc (wlroots)",
            Self::Krfb => "krfb-virtualmonitor",
            Self::Gamescope => "Gamescope",
        }
    }
}

/// A running private headless compositor (or a krfb virtual output). Dropping it stops the
/// child process group (labwc / gamescope) or removes the krfb output.
pub struct HeadlessSession {
    resolved: HeadlessBackend,
    child: Option<Child>,
    wayland_display: Option<String>,
    x11_display: Option<String>,
    output_name: String,
}

impl HeadlessSession {
    /// Backend actually started (never [`HeadlessBackend::Auto`]).
    pub fn backend(&self) -> HeadlessBackend {
        self.resolved
    }

    /// Absolute Wayland socket path, or `None` for krfb (uses the host session).
    pub fn wayland_display(&self) -> Option<&str> {
        self.wayland_display.as_deref()
    }

    /// Discovered XWayland `DISPLAY` (labwc), if any.
    pub fn x11_display(&self) -> Option<&str> {
        self.x11_display.as_deref()
    }

    /// Virtual output name (`HEADLESS-1`, `Slipstream-Headless`, …).
    pub fn output_name(&self) -> &str {
        &self.output_name
    }

    /// Child PID for labwc / gamescope, or `None` for krfb.
    pub fn pid(&self) -> Option<u32> {
        self.child.as_ref().map(|c| c.id())
    }

    /// Prefix `cmd` with `WAYLAND_DISPLAY=` / `DISPLAY=` for labwc / gamescope; krfb returns
    /// `cmd` unchanged (it runs on the host session).
    pub fn wrap_cmd(&self, cmd: &str) -> String {
        if self.resolved == HeadlessBackend::Krfb {
            return cmd.to_string();
        }
        let Some(wl) = self.wayland_display.as_deref() else {
            return cmd.to_string();
        };
        match self.x11_display.as_deref() {
            Some(x11) => format!("WAYLAND_DISPLAY={wl} DISPLAY={x11} {cmd}"),
            None => format!("WAYLAND_DISPLAY={wl} {cmd}"),
        }
    }
}

impl Drop for HeadlessSession {
    fn drop(&mut self) {
        match self.resolved {
            HeadlessBackend::Krfb => stop_krfb(&self.output_name),
            HeadlessBackend::Labwc | HeadlessBackend::Gamescope => {
                if let Some(child) = self.child.take() {
                    stop_process_group(child);
                }
            }
            HeadlessBackend::Auto => {}
        }
    }
}

/// Enumerate backends as `(id, label, available)` for the console / management API.
pub fn available() -> Vec<(String, String, bool)> {
    let labwc_ok = which("labwc").is_some() && which("wlr-randr").is_some();
    let krfb_ok = which("krfb-virtualmonitor").is_some();
    let gamescope_ok = which("gamescope").is_some();
    let any = labwc_ok || krfb_ok || gamescope_ok;
    [
        HeadlessBackend::Auto,
        HeadlessBackend::Labwc,
        HeadlessBackend::Krfb,
        HeadlessBackend::Gamescope,
    ]
    .into_iter()
    .map(|b| {
        let ok = match b {
            HeadlessBackend::Auto => any,
            HeadlessBackend::Labwc => labwc_ok,
            HeadlessBackend::Krfb => krfb_ok,
            HeadlessBackend::Gamescope => gamescope_ok,
        };
        (b.id().to_string(), b.label().to_string(), ok)
    })
    .collect()
}

/// Start a private headless compositor session at 1920×1080@60.
pub fn start(backend: HeadlessBackend) -> Result<HeadlessSession> {
    start_at(backend, DEFAULT_W, DEFAULT_H, DEFAULT_HZ)
}

/// Start at an explicit mode (same backends as [`start`]).
pub fn start_at(
    backend: HeadlessBackend,
    width: u32,
    height: u32,
    refresh_hz: u32,
) -> Result<HeadlessSession> {
    if width == 0 || height == 0 {
        bail!("headless compositor: invalid dimensions {width}x{height}");
    }
    let resolved = resolve(backend);
    tracing::info!(
        requested = backend.id(),
        resolved = resolved.id(),
        width,
        height,
        refresh_hz,
        "headless compositor: starting"
    );
    match resolved {
        HeadlessBackend::Labwc => start_labwc(width, height, refresh_hz),
        HeadlessBackend::Krfb => start_krfb(width, height, refresh_hz),
        HeadlessBackend::Gamescope => start_gamescope(width, height, refresh_hz),
        HeadlessBackend::Auto => unreachable!("resolve never returns Auto"),
    }
}

fn resolve(backend: HeadlessBackend) -> HeadlessBackend {
    match backend {
        HeadlessBackend::Auto => {
            if is_kwin_running() && which("krfb-virtualmonitor").is_some() {
                tracing::info!("headless compositor: detected KWin, using krfb-virtualmonitor");
                HeadlessBackend::Krfb
            } else if is_gamescope_running() && which("gamescope").is_some() {
                tracing::info!("headless compositor: detected Gamescope, using gamescope --headless");
                HeadlessBackend::Gamescope
            } else {
                tracing::info!("headless compositor: using labwc backend");
                HeadlessBackend::Labwc
            }
        }
        other => other,
    }
}

/// Detection helpers must only read env / `WAYLAND_DISPLAY`. Searching PATH for a binary is a
/// false-positive trap (binary installed ≠ that compositor is the live session).
fn is_kwin_running() -> bool {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let lower = desktop.to_ascii_lowercase();
    lower.contains("kde") || lower.contains("plasma")
}

fn is_gamescope_running() -> bool {
    if std::env::var("XDG_CURRENT_DESKTOP").ok().as_deref() == Some("gamescope") {
        return true;
    }
    std::env::var("WAYLAND_DISPLAY")
        .ok()
        .is_some_and(|d| d.contains("gamescope"))
}

fn start_krfb(width: u32, height: u32, refresh_hz: u32) -> Result<HeadlessSession> {
    let bin = which("krfb-virtualmonitor")
        .context("headless compositor: krfb-virtualmonitor not found in PATH")?;
    let output_name = "Slipstream-Headless".to_string();
    let out = Command::new(&bin)
        .args([
            "--name",
            &output_name,
            "--width",
            &width.to_string(),
            "--height",
            &height.to_string(),
            "--refresh",
            &refresh_hz.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("headless compositor: spawn {bin}"))?;
    if !out.status.success() {
        let detail = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        bail!(
            "headless compositor: krfb-virtualmonitor exited with {}: stderr={detail} stdout={stdout}",
            out.status
        );
    }
    tracing::info!(
        %output_name,
        width,
        height,
        refresh_hz,
        "headless compositor: krfb virtual output created"
    );
    Ok(HeadlessSession {
        resolved: HeadlessBackend::Krfb,
        child: None,
        wayland_display: None,
        x11_display: None,
        output_name,
    })
}

fn stop_krfb(output_name: &str) {
    if output_name.is_empty() {
        return;
    }
    let Some(bin) = which("krfb-virtualmonitor") else {
        return;
    };
    tracing::info!(%output_name, "headless compositor: removing krfb virtual output");
    let _ = Command::new(&bin)
        .args(["--remove", output_name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn start_labwc(width: u32, height: u32, refresh_hz: u32) -> Result<HeadlessSession> {
    let labwc = which("labwc").context("headless compositor: labwc not found in PATH")?;
    let wlr_randr =
        which("wlr-randr").context("headless compositor: wlr-randr not found in PATH")?;
    let run_dir = user_runtime_dir().context("headless compositor: cannot determine XDG_RUNTIME_DIR")?;

    tracing::info!(%labwc, %wlr_randr, run_dir = %run_dir.display(), "headless compositor: starting labwc");

    let before = list_prefixed(&run_dir, "wayland-");
    let mut child = spawn_session_leader(
        Command::new(&labwc)
            .env("WLR_BACKENDS", "headless")
            .env("WLR_RENDERER", "gles2")
            .env("WLR_HEADLESS_OUTPUTS", "1")
            .env("WLR_NO_HARDWARE_CURSORS", "1")
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY"),
    )
    .context("headless compositor: spawn labwc")?;

    let discovered = match discover_new_socket(&run_dir, "wayland-", &before) {
        Some(s) => s,
        None => {
            stop_process_group(child);
            bail!("headless compositor: could not discover new WAYLAND_DISPLAY socket");
        }
    };
    let wayland_display = run_dir.join(&discovered).to_string_lossy().into_owned();
    tracing::info!(%wayland_display, "headless compositor: discovered WAYLAND_DISPLAY");

    // Poll for HEADLESS-1 via wlr-randr against the new socket.
    let deadline = Instant::now() + DISCOVER_BUDGET;
    let mut output_ready = false;
    while Instant::now() < deadline {
        let Ok(out) = Command::new(&wlr_randr)
            .env("WAYLAND_DISPLAY", &wayland_display)
            .env("XDG_RUNTIME_DIR", &run_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
        else {
            std::thread::sleep(DISCOVER_STEP);
            continue;
        };
        if String::from_utf8_lossy(&out.stdout).contains("HEADLESS-1") {
            tracing::info!("headless compositor: HEADLESS-1 output detected");
            output_ready = true;
            break;
        }
        std::thread::sleep(DISCOVER_STEP);
    }
    if !output_ready {
        stop_process_group(child);
        bail!("headless compositor: HEADLESS-1 output not detected in time");
    }

    // Best-effort mode apply (labwc headless may already be at a default).
    let mode = format!("{width}x{height}@{refresh_hz}Hz");
    let _ = Command::new(&wlr_randr)
        .args(["--output", "HEADLESS-1", "--mode", &mode])
        .env("WAYLAND_DISPLAY", &wayland_display)
        .env("XDG_RUNTIME_DIR", &run_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // XWayland is optional.
    let before_x11 = list_prefixed(Path::new("/tmp/.X11-unix"), "X");
    let x11_deadline = Instant::now() + Duration::from_secs(2);
    let mut x11_display = None;
    while Instant::now() < x11_deadline {
        if let Some(x) = list_prefixed(Path::new("/tmp/.X11-unix"), "X")
            .into_iter()
            .find(|n| !before_x11.iter().any(|b| b == n))
        {
            // Socket name is `X0`, `X1`, … → DISPLAY `:` + digits.
            let x11 = format!(":{}", x.trim_start_matches('X'));
            tracing::info!(%x11, "headless compositor: discovered XWayland display");
            x11_display = Some(x11);
            break;
        }
        // Bail early if labwc already died.
        match child.try_wait() {
            Ok(Some(status)) => {
                bail!("headless compositor: labwc exited early with {status}");
            }
            Ok(None) => {}
            Err(e) => bail!("headless compositor: labwc wait failed: {e}"),
        }
        std::thread::sleep(DISCOVER_STEP);
    }
    if x11_display.is_none() {
        tracing::info!("headless compositor: XWayland not detected, labwc is ready");
    }

    Ok(HeadlessSession {
        resolved: HeadlessBackend::Labwc,
        child: Some(child),
        wayland_display: Some(wayland_display),
        x11_display,
        output_name: "HEADLESS-1".to_string(),
    })
}

fn start_gamescope(width: u32, height: u32, refresh_hz: u32) -> Result<HeadlessSession> {
    let bin = which("gamescope").context("headless compositor: gamescope not found in PATH")?;
    let run_dir = user_runtime_dir().context("headless compositor: cannot determine XDG_RUNTIME_DIR")?;

    let before = list_prefixed(&run_dir, "wayland-");
    // Keep the compositor alive with a no-op nested client; the host launches real apps later
    // via [`HeadlessSession::wrap_cmd`] against the discovered socket.
    let child = spawn_session_leader(
        Command::new(&bin)
            .args([
                "--headless",
                "--prefer-vk-device",
                "-W",
                &width.to_string(),
                "-H",
                &height.to_string(),
                "-r",
                &refresh_hz.to_string(),
                "--",
                "sh",
                "-c",
                "sleep infinity",
            ])
            .env("WLR_NO_HARDWARE_CURSORS", "1")
            .env_remove("DISPLAY")
            .env_remove("WAYLAND_DISPLAY"),
    )
    .context("headless compositor: spawn gamescope")?;

    let discovered = match discover_new_socket(&run_dir, "wayland-", &before) {
        Some(s) => s,
        None => {
            stop_process_group(child);
            bail!("headless compositor: could not discover gamescope Wayland socket");
        }
    };
    let wayland_display = run_dir.join(&discovered).to_string_lossy().into_owned();
    tracing::info!(%wayland_display, "headless compositor: gamescope WAYLAND_DISPLAY");

    Ok(HeadlessSession {
        resolved: HeadlessBackend::Gamescope,
        child: Some(child),
        wayland_display: Some(wayland_display),
        x11_display: None,
        output_name: "HEADLESS-1".to_string(),
    })
}

/// Spawn with a new session (`setsid`) and stdio → `/dev/null`, so Drop can signal the whole
/// process group (`kill(-pid, …)`).
fn spawn_session_leader(cmd: &mut Command) -> io::Result<Child> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: `setsid` only creates a new session/process group for this child between fork and
    // exec; it touches no shared Rust state. Required so Drop can `kill(-pid, SIGTERM)` the tree.
    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    cmd.spawn()
}

fn stop_process_group(mut child: Child) {
    let pid = child.id() as i32;
    if pid <= 0 {
        return;
    }
    tracing::info!(pid, "headless compositor: stopping process group");
    // SAFETY: `pid` is the session-leader PID we spawned with `setsid`; negating it addresses
    // that process group. `SIGTERM`/`SIGKILL` are valid signals.
    unsafe {
        libc::kill(-pid, libc::SIGTERM);
    }
    let deadline = Instant::now() + STOP_BUDGET;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    tracing::warn!(?status, "headless compositor: child exited non-zero");
                }
                return;
            }
            Ok(None) if Instant::now() >= deadline => break,
            Ok(None) => std::thread::sleep(DISCOVER_STEP),
            Err(e) => {
                tracing::warn!(error = %e, "headless compositor: wait failed");
                return;
            }
        }
    }
    tracing::info!(pid, "headless compositor: sending SIGKILL");
    unsafe {
        libc::kill(-pid, libc::SIGKILL);
    }
    let _ = child.wait();
}

fn discover_new_socket(dir: &Path, prefix: &str, before: &[String]) -> Option<String> {
    let deadline = Instant::now() + DISCOVER_BUDGET;
    while Instant::now() < deadline {
        if let Some(s) = list_prefixed(dir, prefix)
            .into_iter()
            .find(|n| !before.iter().any(|b| b == n))
        {
            return Some(s);
        }
        std::thread::sleep(DISCOVER_STEP);
    }
    None
}

fn list_prefixed(dir: &Path, prefix: &str) -> Vec<String> {
    let Ok(rd) = fs::read_dir(dir) else {
        return Vec::new();
    };
    rd.filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            name.starts_with(prefix).then_some(name)
        })
        .collect()
}

fn user_runtime_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg));
        }
    }
    let uid = crate::proc::current_uid();
    if uid > 0 {
        return Some(PathBuf::from(format!("/run/user/{uid}")));
    }
    None
}

fn which(name: &str) -> Option<String> {
    let path = crate::with_env_lock(|| std::env::var("PATH").ok())?;
    for dir in path.split(':').filter(|d| !d.is_empty()) {
        let cand = Path::new(dir).join(name);
        let Ok(md) = fs::metadata(&cand) else {
            continue;
        };
        if md.is_file() && md.permissions().mode() & 0o111 != 0 {
            return Some(cand.to_string_lossy().into_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_backend_ids() {
        assert_eq!(HeadlessBackend::parse("auto"), Some(HeadlessBackend::Auto));
        assert_eq!(HeadlessBackend::parse("LABWC"), Some(HeadlessBackend::Labwc));
        assert_eq!(
            HeadlessBackend::parse("krfb-virtualmonitor"),
            Some(HeadlessBackend::Krfb)
        );
        assert_eq!(
            HeadlessBackend::parse("gamescope"),
            Some(HeadlessBackend::Gamescope)
        );
        assert_eq!(HeadlessBackend::parse("off"), None);
        assert_eq!(HeadlessBackend::parse("hermes-kms"), None);
    }

    #[test]
    fn available_lists_four_backends() {
        let list = available();
        assert_eq!(list.len(), 4);
        assert_eq!(list[0].0, "auto");
        assert_eq!(list[1].0, "labwc");
        assert_eq!(list[2].0, "krfb");
        assert_eq!(list[3].0, "gamescope");
    }

    #[test]
    fn wrap_cmd_krfb_passthrough() {
        let s = HeadlessSession {
            resolved: HeadlessBackend::Krfb,
            child: None,
            wayland_display: None,
            x11_display: None,
            output_name: "Slipstream-Headless".into(),
        };
        assert_eq!(s.wrap_cmd("foo"), "foo");
    }

    #[test]
    fn wrap_cmd_labwc_prefixes_env() {
        let s = HeadlessSession {
            resolved: HeadlessBackend::Labwc,
            child: None,
            wayland_display: Some("/run/user/1000/wayland-2".into()),
            x11_display: Some(":2".into()),
            output_name: "HEADLESS-1".into(),
        };
        assert_eq!(
            s.wrap_cmd("steam"),
            "WAYLAND_DISPLAY=/run/user/1000/wayland-2 DISPLAY=:2 steam"
        );
    }
}
