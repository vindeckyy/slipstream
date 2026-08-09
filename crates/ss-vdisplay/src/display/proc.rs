#![cfg(target_os = "linux")]

//! Linux child-process helpers with bounded execution.
//!
//! Compositor queries invoke helpers such as `kscreen-doctor`, `systemctl`, and `pw-dump`.
//! A helper can block while connecting to a compositor or session bus, so waiting for it without
//! a deadline can pin the session thread indefinitely.
//!
//! These wrappers poll a directly executed Linux child until the budget expires. A timed-out
//! child is killed and reaped before the caller receives `ErrorKind::TimedOut`.

use std::io::{Error, ErrorKind, Result};
use std::process::{Command, ExitStatus, Output};
use std::time::{Duration, Instant};

/// Poll interval while waiting for a child to exit.
const POLL: Duration = Duration::from_millis(20);

/// Run `command` to completion, killing and reaping it if it outlives `budget`.
///
/// Stdout and stderr remain configured by the caller, inherited by default. Use
/// [`output_within`] when the output needs to be captured.
pub(crate) fn status_within(command: &mut Command, budget: Duration) -> Result<ExitStatus> {
    let mut child = command.spawn()?;
    let deadline = Instant::now() + budget;
    loop {
        match child.try_wait()? {
            Some(status) => return Ok(status),
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(timed_out(command, budget));
            }
            None => std::thread::sleep(POLL),
        }
    }
}

/// Run `command` to completion and capture its stdout and stderr, killing and reaping it if it
/// outlives `budget`.
///
/// The output is read only after the child exits, so a helper that fills a pipe is still bounded
/// by the same deadline.
pub(crate) fn output_within(command: &mut Command, budget: Duration) -> Result<Output> {
    let mut child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    let deadline = Instant::now() + budget;
    loop {
        match child.try_wait()? {
            Some(_) => return child.wait_with_output(),
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(timed_out(command, budget));
            }
            None => std::thread::sleep(POLL),
        }
    }
}

fn timed_out(command: &Command, budget: Duration) -> Error {
    let program = command.get_program().to_string_lossy().to_string();
    tracing::warn!(
        program,
        budget_ms = budget.as_millis() as u64,
        "helper did not exit within its budget - killed and reaped it; treating it as a failed query"
    );
    Error::new(
        ErrorKind::TimedOut,
        format!("`{program}` did not exit within {budget:?}"),
    )
}

/// Return the calling process's real uid.
///
/// The display session uses it when deriving `/run/user/<uid>` and when filtering `/proc` entries.
pub(crate) fn current_uid() -> u32 {
    // SAFETY: parameterless POSIX call that always succeeds and touches no memory. It returns
    // the calling process's real uid without reading or retaining a pointer.
    unsafe { libc::getuid() }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A helper that never exits is killed at the budget and reported as timed out.
    #[test]
    fn a_hung_child_is_killed_at_the_budget() {
        let started = Instant::now();
        let err = status_within(Command::new("sleep").arg("30"), Duration::from_millis(150))
            .expect_err("must time out");
        assert_eq!(err.kind(), ErrorKind::TimedOut);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "must return at its budget, not the child's lifetime (took {:?})",
            started.elapsed()
        );
    }

    /// The normal path still returns a quick command's status and output.
    #[test]
    fn a_quick_child_returns_normally() {
        let st = status_within(&mut Command::new("true"), Duration::from_secs(5)).expect("ran");
        assert!(st.success());

        let out = output_within(
            Command::new("echo").arg("slipstream"),
            Duration::from_secs(5),
        )
        .expect("ran");
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "slipstream");
    }
}
