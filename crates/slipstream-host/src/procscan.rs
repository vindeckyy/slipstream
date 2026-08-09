//! Finding a launched game's processes, from the signals its store gave us
//! ([`crate::library::DetectSpec`]).
//!
//! Read-only by construction: the host enumerates processes and reads metadata it already has
//! permission to see. No ptrace, no injection, no handles held open. It looks only at processes
//! owned by its **own uid**.
//!
//! ### The two rules
//!
//! 1. **Never adopt a process that predates the launch.** A player may already have the game open
//!    when a session starts; treating that instance as "this session's game" would let a session end
//!    kill a process it never started. Candidates are filtered by start time against
//!    [`launch_stamp`], taken before anything spawns.
//! 2. **Never trust a bare pid.** Pids are recycled, and a lease can outlive its game by a grace
//!    window, so every remembered process carries its start time and is re-verified against it
//!    ([`Scanner::alive`]) before it is counted as running, or signalled.
//!
//! The scanner reads `/proc` and exposes one small surface to [`crate::gamelease`].

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::Scanner;

/// A process the matcher adopted: its pid plus a start stamp that pins that pid to *this* process,
/// so a recycled pid can never be mistaken for it.
///
/// `start` is opaque and only ever compared for equality against a later read of the same pid — its
/// units are Linux process-start clock ticks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcRef {
    pub pid: u32,
    pub start: u64,
}

/// Tolerance on the "started after the launch" test, in seconds.
///
/// The launch stamp is taken just before the spawn, but start times are quantized (~10 ms on Linux)
/// and a launcher can race ahead of the host's own bookkeeping, so an exact comparison would
/// occasionally reject the real game. Two seconds is far below the time any launcher takes to bring a
/// game up, so it cannot let a *pre-existing* instance through.
pub const START_SLACK_SECS: f64 = 2.0;

/// The reference instant for adopting a launch's processes, in seconds on Linux's process-start
/// timeline. It is only compared against a process's own start time, never interpreted as a wall
/// clock or persisted.
///
/// Call it **before** anything spawns; see [`crate::gamelease::LeaseRequest::launch_stamp`]. `None`
/// when the platform has no matcher (macOS) or the clock could not be read, which disables the
/// start-time filter rather than rejecting everything.
pub fn launch_stamp() -> Option<f64> {
    Scanner::system().now_stamp()
}

/// An out-of-band opinion on whether a spec's game is still running, independent of the process scan.
///
/// Consulted **only to veto** declaring a game gone — never to declare it running, and never as the
/// primary signal. `Some(true)` = something else believes it is up, so hold off; `Some(false)` = that
/// something agrees it is gone; `None` = no opinion available, which is the common case.
///
/// Linux has none by design: Steam's launch reaper is both the sharpest signal and already a process,
/// so it is covered by the scan itself.
pub fn running_hint(spec: &crate::library::DetectSpec) -> Option<bool> {
    let _ = spec;
    None
}
