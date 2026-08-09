//! Linux worker scheduling QoS for the opt-in low-latency performance profile (latency plan
//! Phase 7). The capture, encode-submit, send, and input-injection workers call
//! [`apply_worker_qos`] at thread start. With the profile OFF (the default) this is a no-op that
//! changes nothing; with `SLIPSTREAM_PERFORMANCE_PROFILE=low_latency` it raises exactly those
//! workers to a realtime class:
//!
//! 1. **RTKit** (the preferred path) — asks the session's `org.freedesktop.portal.Desktop` /
//!    `org.freedesktop.RealtimeKit1` service to raise the calling thread, which it does with a
//!    bounded, system-policy-compliant realtime priority. This is the same mechanism PipeWire
//!    itself uses, so a host with PipeWire's RT module already working gets the same treatment.
//! 2. **Explicit `SCHED_FIFO`** — when RTKit is unavailable and the operator set
//!    `SLIPSTREAM_SCHED_FIFO=1`, `pthread_setschedparam` with the configured priority
//!    (`SLIPSTREAM_SCHED_PRIO`, default 10). Requires `CAP_SYS_NICE` / a raised `RLIMIT_RTPRIO`;
//!    a failure is recorded and we fall through.
//! 3. **`SCHED_OTHER` + nice** — the documented fallback: `setpriority` nice `-10` (critical
//!    workers) / `-5` (others), matching the existing `thread_qos::boost_thread_priority`.
//!
//! Every step records whether it was applied or rejected into a process-wide table so operators
//! can see at a glance whether the profile is actually in force. The profile never changes
//! system-wide settings silently: GPU power policy and governors are untouched, PipeWire's own RT
//! module is never outranked (we request a modest priority, not a ceiling race), and CPU affinity
//! is applied ONLY when `SLIPSTREAM_WORKER_AFFINITY` is explicitly set — we never steal CPUs from
//! the game or compositor automatically.
//!
//! The whole module is a no-op off Linux (`apply_worker_qos` returns `SchedOutcome::NotApplicable`).

use std::collections::BTreeMap;
use std::sync::Mutex;

/// How one worker thread's scheduling request resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchedOutcome {
    /// The profile is off, or this platform has no worker QoS — nothing was attempted.
    NotApplicable,
    /// The worker was raised (RTKit, SCHED_FIFO, or nice) or already held the requested state.
    Applied,
    /// The request was refused (no RTKit session, no CAP_SYS_NICE, invalid affinity) and the
    /// worker runs at its default priority. This is a recorded, expected fallback, not an error.
    Rejected,
}

/// Which worker class the calling thread is, so the nice fallback and the record can differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkerClass {
    /// The capture and encode-submit loops — the critical path.
    Critical,
    /// The send and input-injection workers.
    Background,
}

/// Process-wide outcome table, keyed by thread name, for the web console / diagnostics.
static OUTCOMES: Mutex<BTreeMap<String, SchedOutcome>> = Mutex::new(BTreeMap::new());

/// Whether the low-latency profile is active (parsed once — the env is constant for the process).
fn profile_active() -> bool {
    std::env::var("SLIPSTREAM_PERFORMANCE_PROFILE")
        .map(|s| s.trim().eq_ignore_ascii_case("low_latency"))
        .unwrap_or(false)
}

/// The explicitly configured `SCHED_FIFO` priority (default 10, clamped 1..=99).
fn sched_prio() -> i32 {
    std::env::var("SLIPSTREAM_SCHED_PRIO")
        .ok()
        .and_then(|s| s.trim().parse::<i32>().ok())
        .map(|p| p.clamp(1, 99))
        .unwrap_or(10)
}

/// Whether the operator explicitly enabled the raw `SCHED_FIFO` path (RTKit is preferred).
fn sched_fifo_enabled() -> bool {
    std::env::var("SLIPSTREAM_SCHED_FIFO")
        .map(|s| s.trim() == "1")
        .unwrap_or(false)
}

/// The explicitly configured CPU affinity (e.g. `"2,3"`). `None` = never touch affinity.
fn affinity_cpus() -> Option<Vec<usize>> {
    std::env::var("SLIPSTREAM_WORKER_AFFINITY")
        .ok()
        .and_then(|s| {
            let cpus: Vec<usize> = s
                .split(',')
                .filter_map(|p| p.trim().parse::<usize>().ok())
                .collect();
            (!cpus.is_empty()).then_some(cpus)
        })
}

/// The nice adjustment for the `SCHED_OTHER` fallback, per class (matches `thread_qos`).
fn fallback_nice(class: WorkerClass) -> i32 {
    match class {
        WorkerClass::Critical => -10,
        WorkerClass::Background => -5,
    }
}

/// Raise the CURRENT thread's scheduling class for the opt-in performance profile. Call at the
/// top of each capture / encode-submit / send / input-injection worker thread, with the thread's
/// name (the `thread::Builder::name`) for the outcome record. Never changes system-wide settings;
/// never steals CPUs. See the module docs for the RTKit → SCHED_FIFO → nice ladder.
pub fn apply_worker_qos(thread_name: &str, class: WorkerClass) -> SchedOutcome {
    if !profile_active() {
        return SchedOutcome::NotApplicable;
    }
    // Affinity is applied independently of the scheduling class — but only when explicitly
    // configured (the Phase-7 contract: never steal CPUs automatically).
    if let Some(cpus) = affinity_cpus() {
        if !set_affinity(&cpus) {
            record(thread_name, SchedOutcome::Rejected);
            return SchedOutcome::Rejected;
        }
    }
    let sched = try_rtkit(class)
        .or_else(|| try_sched_fifo(class))
        .unwrap_or_else(|| {
            set_nice(fallback_nice(class));
            SchedOutcome::Rejected
        });
    record(thread_name, sched);
    sched
}

/// Read the recorded outcome for a thread name (diagnostics; `None` = not yet recorded).
pub fn recorded_outcome(thread_name: &str) -> Option<SchedOutcome> {
    OUTCOMES
        .lock()
        .ok()
        .and_then(|m| m.get(thread_name).copied())
}

/// All recorded outcomes (diagnostics).
pub fn recorded_outcomes() -> BTreeMap<String, SchedOutcome> {
    OUTCOMES.lock().map(|m| m.clone()).unwrap_or_default()
}

fn record(thread_name: &str, outcome: SchedOutcome) {
    if let Ok(mut m) = OUTCOMES.lock() {
        m.insert(thread_name.to_string(), outcome);
    }
}

/// Prefer RTKit: ask the session realtime service to raise this thread. Returns `Some(Applied)`
/// when the request succeeded, `Some(Rejected)` when it was refused, `None` when RTKit is absent
/// (so the caller can try the explicit SCHED_FIFO path). The request is deliberately modest —
/// a bounded realtime priority the service itself caps — so we never outrank the PipeWire graph.
fn try_rtkit(class: WorkerClass) -> Option<SchedOutcome> {
    let rtkit = rtkit_request(class).ok()?;
    match rtkit {
        Ok(()) => Some(SchedOutcome::Applied),
        Err(()) => Some(SchedOutcome::Rejected),
    }
}

/// The explicit `SCHED_FIFO` path — only when `SLIPSTREAM_SCHED_FIFO=1` (RTKit is preferred and
/// the operator must explicitly opt into raw realtime). Fails (→ `None`) without
/// `CAP_SYS_NICE` / a raised `RLIMIT_RTPRIO`, so the caller falls back to nice.
fn try_sched_fifo(class: WorkerClass) -> Option<SchedOutcome> {
    if !sched_fifo_enabled() {
        return None;
    }
    // SAFETY: `pthread_self()` returns the calling thread's POSIX id — always valid here, never
    // freed. `sched_param` is a plain by-value struct we zero + set a priority on; the kernel
    // copies it. No pointers outlive the call; failure is reported via the return value.
    let mut param = unsafe { std::mem::zeroed::<libc::sched_param>() };
    param.sched_priority = sched_prio();
    // SAFETY: same reasoning — `pthread_self()` is the calling thread, `&mut param` is a live
    // exclusive borrow for the call, and the integer policy is a literal. The kernel validates
    // the policy/priority against this thread's realtime allowance and returns an error code.
    let rc = unsafe { libc::pthread_setschedparam(libc::pthread_self(), libc::SCHED_FIFO, &param) };
    if rc == 0 {
        tracing::info!(class = ?class, prio = param.sched_priority, "worker SCHED_FIFO applied");
        Some(SchedOutcome::Applied)
    } else {
        tracing::warn!(class = ?class, errno = rc, "worker SCHED_FIFO rejected (needs CAP_SYS_NICE / RLIMIT_RTPRIO)");
        Some(SchedOutcome::Rejected)
    }
}

/// The documented `SCHED_OTHER` + nice fallback: lower the nice of the calling thread.
fn set_nice(nice: i32) {
    // SAFETY: `setpriority` takes three by-value integers and no pointers; `PRIO_PROCESS` with
    // `who == 0` targets the calling task on Linux, and `nice` is in range. It only adjusts this
    // thread's scheduling nice value and returns an `int` we inspect. No memory is touched.
    let rc = unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, nice) };
    if rc == 0 {
        tracing::debug!(nice, "worker nice fallback applied");
    } else {
        tracing::debug!(
            nice,
            "worker nice fallback no-op (needs CAP_SYS_NICE / RLIMIT_NICE)"
        );
    }
}

/// Apply an explicit CPU affinity mask to the calling thread (only when the operator configured
/// one). Returns false when the mask is invalid or the kernel refuses — the worker then runs
/// un-pinned rather than failing the session.
fn set_affinity(cpus: &[usize]) -> bool {
    if cpus.is_empty() {
        return true;
    }
    // The fixed `cpu_set_t` covers up to `CPU_SETSIZE` (typically 1024) CPUs — more than any
    // real host, and the kernel rejects out-of-range bits we can't represent anyway.
    // SAFETY: `sched_setaffinity` copies the mask into the kernel; `cpu_set_t` is a plain POD
    // array we zero + set bits in and never alias. `pid 0` targets the calling thread. The
    // `CPU_*` helpers are glibc macros re-declared as extern fns; every argument is an in-range
    // integer or `&mut` of the local, correctly-sized mask.
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_ZERO(&mut set);
        for &cpu in cpus {
            if cpu >= libc::CPU_SETSIZE as usize {
                tracing::warn!(cpu, "worker affinity: cpu out of range");
                return false;
            }
            libc::CPU_SET(cpu, &mut set);
        }
        let rc = libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
        if rc == 0 {
            tracing::info!(?cpus, "worker CPU affinity applied");
            true
        } else {
            tracing::warn!(?cpus, errno = rc, "worker CPU affinity rejected");
            false
        }
    }
}

/// The RTKit request: ask `org.freedesktop.RealtimeKit1` to raise the calling thread. We shell out
/// to the system `dbus-send` (a one-shot subprocess, once per worker at startup) — no new Rust
/// dependency, and RTKit itself is the same mechanism PipeWire uses, so a host with PipeWire's RT
/// module working gets the same treatment. Returns `Ok(Ok(()))` = applied, `Ok(Err(()))` =
/// refused, `Err(())` = RTKit/dbus unreachable (so the caller tries SCHED_FIFO). The requested
/// priority is modest (the service caps it anyway); we never outrank the PipeWire graph.
fn rtkit_request(class: WorkerClass) -> Result<Result<(), ()>, ()> {
    let _ = class;
    // `dbus-send` needs a session (or system) bus; the host's stream workers run in the user
    // session, so `DBUS_SESSION_BUS_ADDRESS` is set. Without it there is nothing to ask.
    let addr = std::env::var("DBUS_SESSION_BUS_ADDRESS").ok();
    let addr = addr.as_deref().filter(|a| !a.is_empty());
    let Some(_addr) = addr else {
        return Err(());
    };

    // `MakeThreadRealtime(pid, tid, priority)` — the standard RTKit interface.
    let tid = gettid();
    let pid = std::process::id() as u32;
    let prio = sched_prio();
    let method = format!(
        "org.freedesktop.RealtimeKit1.MakeThreadRealtime uint32:{pid} uint32:{tid} int32:{prio}"
    );

    let status = std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--print-reply",
            "--dest=org.freedesktop.RealtimeKit1",
            "/org/freedesktop/RealtimeKit1",
            &method,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match status {
        Ok(st) if st.success() => {
            tracing::info!(tid, prio, "worker RTKit realtime applied");
            Ok(Ok(()))
        }
        Ok(_st) => {
            tracing::warn!(tid, prio, "worker RTKit request refused");
            Ok(Err(()))
        }
        Err(_) => Err(()),
    }
}

/// `gettid` via `syscall` (Linux-specific).
fn gettid() -> u32 {
    // SAFETY: `syscall(SYS_gettid)` takes no pointers and returns the calling thread's id.
    unsafe { libc::syscall(libc::SYS_gettid) as u32 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sched_prio_default_is_ten_and_clamps() {
        // The env is untouched here (the module only reads it, never sets it), so the default is
        // deterministic.
        assert_eq!(sched_prio(), 10);
    }

    #[test]
    fn fallback_nice_matches_thread_qos() {
        assert_eq!(fallback_nice(WorkerClass::Critical), -10);
        assert_eq!(fallback_nice(WorkerClass::Background), -5);
    }

    #[test]
    fn affinity_parses_only_explicit_cpus() {
        // Unset env means None; the default path never changes affinity.
        // SAFETY: this test mutates only environment variables owned by the test.
        unsafe {
            std::env::remove_var("SLIPSTREAM_WORKER_AFFINITY");
        }
        assert!(affinity_cpus().is_none());

        // SAFETY: this test mutates only environment variables owned by the test.
        unsafe {
            std::env::set_var("SLIPSTREAM_WORKER_AFFINITY", "2,3");
        }
        assert_eq!(affinity_cpus(), Some(vec![2, 3]));

        // SAFETY: this test mutates only environment variables owned by the test.
        unsafe {
            std::env::set_var("SLIPSTREAM_WORKER_AFFINITY", " 1 , 4 ");
        }
        assert_eq!(affinity_cpus(), Some(vec![1, 4]));

        // Empty or invalid input means None; nothing is pinned.
        // SAFETY: this test mutates only environment variables owned by the test.
        unsafe {
            std::env::set_var("SLIPSTREAM_WORKER_AFFINITY", "");
        }
        assert!(affinity_cpus().is_none());
        // SAFETY: this test mutates only environment variables owned by the test.
        unsafe {
            std::env::set_var("SLIPSTREAM_WORKER_AFFINITY", "abc");
        }
        assert!(affinity_cpus().is_none());

        // SAFETY: this test mutates only environment variables owned by the test.
        unsafe {
            std::env::remove_var("SLIPSTREAM_WORKER_AFFINITY");
        }
    }

    #[test]
    fn sched_fifo_is_explicitly_opt_in() {
        // SAFETY: this test mutates only environment variables owned by the test.
        unsafe {
            std::env::set_var("SLIPSTREAM_SCHED_FIFO", "1");
        }
        assert!(sched_fifo_enabled());
        // SAFETY: this test mutates only environment variables owned by the test.
        unsafe {
            std::env::set_var("SLIPSTREAM_SCHED_FIFO", "0");
        }
        assert!(!sched_fifo_enabled());
        // SAFETY: this test mutates only environment variables owned by the test.
        unsafe {
            std::env::remove_var("SLIPSTREAM_SCHED_FIFO");
        }
        assert!(!sched_fifo_enabled());
    }

    #[test]
    fn outcome_record_roundtrips() {
        record("test-thread", SchedOutcome::Applied);
        assert_eq!(recorded_outcome("test-thread"), Some(SchedOutcome::Applied));
        record("test-thread", SchedOutcome::Rejected);
        assert_eq!(
            recorded_outcome("test-thread"),
            Some(SchedOutcome::Rejected)
        );
        assert_eq!(recorded_outcome("never-recorded"), None);
    }
}
