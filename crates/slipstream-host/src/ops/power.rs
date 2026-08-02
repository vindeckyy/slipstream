//! Operator power controls: restart or stop this **host process** (not the OS).
//!
//! Used by `POST /api/v1/host/restart` and `POST /api/v1/host/shutdown`. Restart prefers the
//! service manager (systemd user unit / Windows SCM helper), then falls back to re-exec of the
//! same binary + argv. Shutdown always exits this process without asking a supervisor to start
//! it again.

use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// When true, [`schedule_restart`] / [`schedule_shutdown`] record success but do not spawn or exit.
/// Mgmt route tests set this so hitting the endpoints cannot kill the test process.
static SIDE_EFFECTS_DISABLED: AtomicBool = AtomicBool::new(false);

/// Delay after accepting the HTTP response before the process exits or the supervisor takes over.
const ACTION_DELAY: Duration = Duration::from_millis(400);

#[cfg(test)]
pub(crate) fn disable_side_effects_for_test() {
    SIDE_EFFECTS_DISABLED.store(true, Ordering::SeqCst);
}

/// Schedule a process restart after a short delay so the HTTP 202 can flush.
pub(crate) fn schedule_restart() -> Result<(), String> {
    if SIDE_EFFECTS_DISABLED.load(Ordering::SeqCst) {
        tracing::info!("power: restart requested (side effects disabled for test)");
        return Ok(());
    }
    match try_supervisor_restart() {
        Ok(true) => {
            tracing::info!("power: restart scheduled via service manager");
            Ok(())
        }
        Ok(false) => {
            tracing::info!("power: no service manager ownership — re-exec fallback");
            schedule_reexec()
        }
        Err(e) => Err(e),
    }
}

/// Schedule a graceful process stop (no supervisor start).
pub(crate) fn schedule_shutdown() {
    if SIDE_EFFECTS_DISABLED.load(Ordering::SeqCst) {
        tracing::info!("power: shutdown requested (side effects disabled for test)");
        return;
    }
    tracing::info!("power: shutdown scheduled");
    std::thread::Builder::new()
        .name("ss-power-shutdown".into())
        .spawn(|| {
            std::thread::sleep(ACTION_DELAY);
            graceful_exit();
        })
        .expect("spawn shutdown thread");
}

/// Try to bounce via the installed service unit/service. Returns `Ok(true)` if that path was
/// taken, `Ok(false)` if no supervisor owns this install (caller should re-exec).
fn try_supervisor_restart() -> Result<bool, String> {
    #[cfg(target_os = "linux")]
    {
        let active = Command::new("systemctl")
            .args(["--user", "is-active", "slipstream-host.service"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !active {
            return supervisor_decision(false, false, "");
        }
        let status = Command::new("systemctl")
            .args(["--user", "--no-block", "restart", "slipstream-host.service"])
            .status()
            .map_err(|e| format!("systemctl restart slipstream-host: {e}"))?;
        supervisor_decision(
            true,
            status.success(),
            &format!("systemctl restart slipstream-host failed with {status}"),
        )
    }
    #[cfg(target_os = "windows")]
    {
        // Match `windows/service.rs` SERVICE_NAME. Spawn a breakaway helper so this process can
        // die under `sc stop` while the helper brings it back.
        const SERVICE_NAME: &str = "SlipstreamHost";
        let probe = Command::new("sc.exe")
            .args(["query", SERVICE_NAME])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !probe {
            return Ok(false);
        }
        let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
        use std::os::windows::process::CommandExt as _;
        const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        Command::new(&exe)
            .args(["service", "restart"])
            .creation_flags(CREATE_BREAKAWAY_FROM_JOB | CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("spawn service restart helper: {e}"))?;
        Ok(true)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Ok(false)
    }
}

fn supervisor_decision(
    active: bool,
    restart_succeeded: bool,
    failure: &str,
) -> Result<bool, String> {
    if !active {
        Ok(false)
    } else if restart_succeeded {
        Ok(true)
    } else {
        Err(failure.to_owned())
    }
}

fn schedule_reexec() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let parent = std::process::id();

    // The replacement must not bind until this process has released the ports. Spawning the
    // binary immediately races the parent (Address already in use on --mgmt-bind).
    #[cfg(unix)]
    {
        let mut cmd = Command::new("sh");
        // $0 / $@ are the exe + serve args passed after -c.
        let wait =
            format!("while kill -0 {parent} 2>/dev/null; do sleep 0.05; done; exec \"$0\" \"$@\"");
        // The replacement must survive the terminal/service session that owns the parent. Closing
        // its inherited stdio avoids a PTY hangup, and a fresh session prevents the terminal's
        // process group from taking the waiter down with the host.
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        use std::os::unix::process::CommandExt as _;
        // SAFETY: this hook runs in the freshly forked child before `exec`; `setsid` only detaches
        // that child from the parent's session and does not touch shared Rust state.
        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        cmd.arg("-c").arg(wait).arg(&exe);
        for a in &args {
            cmd.arg(a);
        }
        cmd.spawn()
            .map_err(|e| format!("spawn replacement host waiter: {e}"))?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        // Wait-Process until the parent exits, then Start-Process the same command line.
        let mut ps = String::from(format!(
            "Wait-Process -Id {parent} -ErrorAction SilentlyContinue; Start-Process -FilePath "
        ));
        ps.push('\'');
        ps.push_str(&exe.display().to_string().replace('\'', "''"));
        ps.push('\'');
        if !args.is_empty() {
            ps.push_str(" -ArgumentList ");
            let joined: Vec<String> = args
                .iter()
                .map(|a| {
                    let s = a.to_string_lossy();
                    format!("'{}'", s.replace('\'', "''"))
                })
                .collect();
            ps.push_str(&joined.join(","));
        }
        Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", &ps])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_BREAKAWAY_FROM_JOB | CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("spawn replacement host waiter: {e}"))?;
    }
    #[cfg(not(any(unix, windows)))]
    {
        let mut child = Command::new(&exe);
        child.args(&args);
        child
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        child
            .spawn()
            .map_err(|e| format!("spawn replacement host: {e}"))?;
    }

    std::thread::Builder::new()
        .name("ss-power-reexec-exit".into())
        .spawn(|| {
            std::thread::sleep(ACTION_DELAY);
            graceful_exit();
        })
        .map_err(|e| format!("spawn re-exec exit thread: {e}"))?;
    Ok(())
}

fn graceful_exit() {
    // Prefer SIGTERM on Unix so the existing takeover-restore handler in `native` runs.
    #[cfg(unix)]
    {
        let pid = std::process::id() as i32;
        // SAFETY: kill(getpid(), SIGTERM) is well-defined and only signals this process.
        let rc = unsafe { libc::kill(pid, libc::SIGTERM) };
        if rc == 0 {
            // Handler exits; if it somehow doesn't, fall through after a beat.
            std::thread::sleep(Duration::from_secs(2));
        } else {
            tracing::warn!("power: SIGTERM failed — exiting directly");
        }
    }
    crate::vdisplay::restore_takeover_now();
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::supervisor_decision;

    #[test]
    fn inactive_supervisor_selects_reexec_fallback() {
        assert_eq!(supervisor_decision(false, false, "unused"), Ok(false));
    }

    #[test]
    fn active_supervisor_selects_managed_restart() {
        assert_eq!(supervisor_decision(true, true, "unused"), Ok(true));
    }

    #[test]
    fn active_supervisor_failure_stays_an_error() {
        assert_eq!(
            supervisor_decision(true, false, "restart failed"),
            Err("restart failed".to_owned())
        );
    }
}
