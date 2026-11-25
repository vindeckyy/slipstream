//! Mode-conflict **admission** (design: `design/display-management.md` §5.3, Stage 4). When a
//! *different* client connects while another client's session is already live, the `mode_conflict`
//! policy decides what happens — BEFORE the Welcome / RTSP launch, so the client gets an honest answer
//! instead of a mid-build failure:
//!
//! * `separate` — proceed on a fresh display at the requested mode (today's Linux multi-view / the
//!   default; no behavior change unconfigured).
//! * `join` — admit at the live display's mode (honest-downgrade: the Welcome carries the real mode).
//! * `steal` — signal the victim session(s)' stop flag(s), wait the release grace, then serve.
//! * `reject` — refuse with a typed handshake error naming the live mode + client.
//!
//! A **live-session registry** ([`register`]) lets the decision see the current sessions (identity +
//! mode + stop flag); each session registers once admitted and drops its [`LiveGuard`] on end. The
//! decision itself ([`decide`]) is pure over a session slice, so it is unit-tested exhaustively.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::policy::{self, ModeConflict};

/// A currently-live session, as admission sees it.
#[derive(Clone)]
pub struct LiveSession {
    id: u64,
    /// The owning client's cert fingerprint (`None` = anonymous / no client cert presented).
    pub identity: Option<[u8; 32]>,
    pub mode: (u32, u32, u32),
    /// The session's stop flag — signaled to preempt it on `steal`.
    pub stop: Arc<AtomicBool>,
    /// Short client label for `reject` messages.
    pub label: String,
}

/// The admission outcome for a connecting session.
#[derive(Debug)]
pub enum Admission {
    /// No conflict / `separate`: proceed on a fresh display at the requested mode.
    Separate,
    /// `join`: admit at this (live) mode — share the existing display (honest-downgrade).
    Join((u32, u32, u32)),
    /// `steal`: signal these victim stop flags, wait the release grace, then proceed at the requested mode.
    Steal(Vec<Arc<AtomicBool>>),
    /// `reject`: refuse with this reason (host-busy + live mode + client label).
    Reject(String),
}

fn table() -> &'static Mutex<Vec<LiveSession>> {
    static T: OnceLock<Mutex<Vec<LiveSession>>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(Vec::new()))
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Two identities are the same client iff both are present and equal. Anonymous (`None`) never
/// matches — we can't prove it's the same client, so two anonymous clients are treated as distinct
/// (each conflicts), which is the safe side for `steal`/`reject`.
fn same_client(a: Option<[u8; 32]>, b: Option<[u8; 32]>) -> bool {
    matches!((a, b), (Some(x), Some(y)) if x == y)
}

/// The mode-conflict decision, pure over the live-session slice (so it's unit-testable). A conflict is
/// a live session owned by a DIFFERENT client — a same-client reconnect adopts / reconfigures its own
/// display and never conflicts (so it always resolves to `Separate` here and preempts downstream).
pub fn decide(
    conflict: ModeConflict,
    req_identity: Option<[u8; 32]>,
    live: &[LiveSession],
) -> Admission {
    let others: Vec<&LiveSession> = live
        .iter()
        .filter(|s| !same_client(s.identity, req_identity))
        .collect();
    if others.is_empty() {
        return Admission::Separate; // no other client is live → no conflict
    }
    match conflict {
        ModeConflict::Separate => Admission::Separate,
        // Join at the OLDEST other session's mode (the established "primary" the desktop is built on).
        ModeConflict::Join => Admission::Join(others[0].mode),
        ModeConflict::Steal => {
            Admission::Steal(others.iter().map(|s| Arc::clone(&s.stop)).collect())
        }
        ModeConflict::Reject => {
            let v = others[0];
            Admission::Reject(format!(
                "host busy: streaming {}x{}@{} to {}",
                v.mode.0, v.mode.1, v.mode.2, v.label
            ))
        }
    }
}

/// The effective `mode_conflict` policy for this host: the console value, or `Separate` when no
/// value is configured. Shared by the native and GameStream admission paths.
pub fn effective_conflict() -> ModeConflict {
    policy::prefs()
        .configured_effective()
        .map(|e| e.mode_conflict)
        .unwrap_or(ModeConflict::Separate)
}

/// Resolve the admission decision for a connecting native session from the effective policy and
/// current live-session table.
pub fn admit(req_identity: Option<[u8; 32]>) -> Admission {
    let decision = {
        let live = table().lock().unwrap();
        decide(effective_conflict(), req_identity, &live)
    };
    decision
}

/// Pure core of [`preempt_same_identity`]: the stop flags of live sessions owned by the SAME client
/// as `req_identity` (its own zombies). Testable over a slice (the public fn locks the global table).
fn same_identity_stops(
    req_identity: Option<[u8; 32]>,
    live: &[LiveSession],
) -> Vec<Arc<AtomicBool>> {
    live.iter()
        .filter(|s| same_client(s.identity, req_identity))
        .map(|s| Arc::clone(&s.stop))
        .collect()
}

/// Preempt this reconnecting client's OWN still-live session(s). A client has at most one live
/// session, so a new connection from an already-registered identity is a **reconnect** — the old
/// session is a zombie whose QUIC idle timer hasn't fired yet (an unwanted disconnect is only
/// declared dead after `max_idle_timeout`, ~seconds later). Return its stop flag(s) so the caller
/// signals them and waits the release grace: the zombie tears its display down, which (keep-alive on)
/// lingers, and THIS reconnect **reuses** that kept display instead of landing on a fresh SECOND one
/// (the "thrown onto a second display while the old one keeps streaming" bug). Anonymous (`None`)
/// never matches — same limitation as `steal`/`reject`. Call this BEFORE [`admit`] and before this
/// session registers itself, so it only ever signals a *prior* session's flag, never its own.
pub fn preempt_same_identity(req_identity: Option<[u8; 32]>) -> Vec<Arc<AtomicBool>> {
    same_identity_stops(req_identity, &table().lock().unwrap())
}

/// Register a now-admitted, live session; the returned guard removes it on drop (session end). Call
/// AFTER [`admit`] (so a session never conflicts with itself) and once the mode + stop flag are known.
pub fn register(
    identity: Option<[u8; 32]>,
    mode: (u32, u32, u32),
    stop: Arc<AtomicBool>,
    label: String,
) -> LiveGuard {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    table().lock().unwrap().push(LiveSession {
        id,
        identity,
        mode,
        stop,
        label,
    });
    LiveGuard { id }
}

/// RAII handle: removes its live-session entry from the registry on drop (session end).
pub struct LiveGuard {
    id: u64,
}

impl Drop for LiveGuard {
    fn drop(&mut self) {
        table().lock().unwrap().retain(|s| s.id != self.id);
    }
}

/// The result of an atomic admit-and-claim.
///
/// This exists because admission and registration used to be two separate table
/// accesses with the whole session bring-up between them: `preempt_same_identity` +
/// `admit` ran in the handshake, and [`register`] ran only once the data-plane pipeline
/// was up. A burst of simultaneous Hellos from one identity therefore all read an empty
/// (or stale) table, all resolved to `Separate`, and each landed on its own display —
/// the exact outcome preemption exists to prevent (observed live: one client holding
/// three KWin outputs).
pub struct Claimed {
    /// What the connecting session should do, resolved against the tenants that were
    /// already present. Never computed against this claim itself.
    pub decision: Admission,
    /// The mode this claim is RECORDED at: the requested mode, or the live display's mode when
    /// [`decide`] resolved [`Admission::Join`]. The caller must stream at this mode.
    ///
    /// Resolved HERE, under the same lock as the insert, because the alternative - inserting the
    /// requested mode and correcting it with [`set_mode`] afterwards - leaves a window in which the
    /// table advertises a mode the session is not going to use, and a concurrent claim resolving
    /// `join`/`reject` against it reads the wrong one.
    pub mode: (u32, u32, u32),
    /// This identity's own prior sessions (a genuine reconnect's zombies). The caller
    /// signals these AFTER this call returns, so the table lock is never held across a
    /// sleep.
    pub own_zombies: Vec<Arc<AtomicBool>>,
    /// The live entry this claim owns. Dropping it removes the entry, so a pipeline that
    /// fails after the claim is registered cannot lock the client out of its identity.
    pub guard: LiveGuard,
}

/// Atomically resolve admission for a connecting session AND insert its live entry.
///
/// One lock covers both the read (`decide` + own-zombie scan) and the write (the claim),
/// which is what makes the one-session-per-identity invariant true by construction
/// rather than by timing: a concurrent Hello either runs entirely before this one (and is
/// then seen here) or entirely after (and sees this claim). It does not serialise session
/// startup: the only work under the lock is scanning a table that holds one entry per
/// live session (tens at most) and a push. The 1500 ms preemption grace and every
/// pipeline step happen after the lock is released.
///
/// `mode` is the client's requested mode; callers whose decision is
/// [`Admission::Join`] overwrite it with the live mode before the display is acquired.
pub fn claim(
    conflict: ModeConflict,
    req_identity: Option<[u8; 32]>,
    mode: (u32, u32, u32),
    stop: Arc<AtomicBool>,
    label: String,
) -> Claimed {
    let mut live = table().lock().unwrap();
    // Decide against the table as it stands — never against the entry we are about to add.
    let decision = decide(conflict, req_identity, &live);
    let own_zombies = same_identity_stops(req_identity, &live);
    // Store the mode this session will actually serve, not the one it asked for: a `join` is an
    // honest downgrade, and a claim that recorded the REQUEST would mislead the next decision
    // (a concurrent claim resolving `join`/`reject` reads whatever is in the table).
    let claimed_mode = match &decision {
        Admission::Join(m) => *m,
        _ => mode,
    };
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    live.push(LiveSession {
        id,
        identity: req_identity,
        mode: claimed_mode,
        stop,
        label,
    });
    Claimed {
        decision,
        mode: claimed_mode,
        own_zombies,
        guard: LiveGuard { id },
    }
}

/// Whether the live entry that owns `stop` has been removed - i.e. that session has actually
/// released its display. A preempting session polls this instead of sleeping a fixed grace: the stop
/// flag is ADVISORY (a session inside its first pipeline build does not look at it until the build
/// returns, seconds later), so "signalled" and "released" are different instants, and acquiring a
/// display in between is exactly the second-display bug preemption exists to prevent.
pub fn released(stop: &Arc<AtomicBool>) -> bool {
    !table()
        .lock()
        .unwrap()
        .iter()
        .any(|s| Arc::ptr_eq(&s.stop, stop))
}

/// Update a claim's recorded mode once the session has negotiated one (e.g. after an
/// [`Admission::Join`] downgrade, or when the backend honours a different refresh).
pub fn set_mode(guard: &LiveGuard, mode: (u32, u32, u32)) {
    let mut live = table().lock().unwrap();
    if let Some(s) = live.iter_mut().find(|s| s.id == guard.id) {
        s.mode = mode;
    }
}

/// Number of entries in the live table. Test-only: the invariant under test is about the
/// table's own contents, and no production caller has a reason to read the count.
#[cfg(test)]
fn live_len() -> usize {
    table().lock().unwrap().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sess(identity: Option<u8>, mode: (u32, u32, u32)) -> LiveSession {
        LiveSession {
            id: 0,
            identity: identity.map(|n| {
                let mut f = [0u8; 32];
                f[0] = n;
                f
            }),
            mode,
            stop: Arc::new(AtomicBool::new(false)),
            label: "peer".into(),
        }
    }
    fn fp(n: u8) -> Option<[u8; 32]> {
        let mut f = [0u8; 32];
        f[0] = n;
        Some(f)
    }

    #[test]
    fn no_live_session_is_always_separate() {
        for c in [
            ModeConflict::Separate,
            ModeConflict::Join,
            ModeConflict::Steal,
            ModeConflict::Reject,
        ] {
            assert!(matches!(decide(c, fp(1), &[]), Admission::Separate));
        }
    }

    #[test]
    fn same_client_never_conflicts() {
        let live = [sess(Some(1), (2560, 1440, 60))];
        // Even under reject/steal, the SAME client (fp 1) reconnecting is not a conflict.
        assert!(matches!(
            decide(ModeConflict::Reject, fp(1), &live),
            Admission::Separate
        ));
        assert!(matches!(
            decide(ModeConflict::Steal, fp(1), &live),
            Admission::Separate
        ));
    }

    #[test]
    fn different_client_applies_policy() {
        let live = [sess(Some(1), (2560, 1440, 60))];
        assert!(matches!(
            decide(ModeConflict::Separate, fp(2), &live),
            Admission::Separate
        ));
        assert!(matches!(
            decide(ModeConflict::Join, fp(2), &live),
            Admission::Join((2560, 1440, 60))
        ));
        assert!(matches!(
            decide(ModeConflict::Steal, fp(2), &live),
            Admission::Steal(v) if v.len() == 1
        ));
        assert!(matches!(
            decide(ModeConflict::Reject, fp(2), &live),
            Admission::Reject(r) if r.contains("2560x1440@60")
        ));
    }

    #[test]
    fn two_anonymous_clients_conflict() {
        // Anonymous (None) can't be proven same-client, so a second anon client DOES conflict.
        let live = [sess(None, (1920, 1080, 60))];
        assert!(matches!(
            decide(ModeConflict::Reject, None, &live),
            Admission::Reject(_)
        ));
    }

    #[test]
    fn same_identity_stops_targets_own_zombie_only() {
        let live = [
            sess(Some(1), (2560, 1440, 60)), // this client's prior (zombie) session
            sess(Some(2), (1920, 1080, 60)), // a different client
        ];
        // Reconnecting as client 1 → its own zombie's stop is returned (to preempt), not client 2's.
        assert_eq!(same_identity_stops(fp(1), &live).len(), 1);
        // A client with no prior session (fp 3) has nothing of its own to preempt.
        assert_eq!(same_identity_stops(fp(3), &live).len(), 0);
        // Anonymous never matches — we can't prove it's the same client.
        assert_eq!(same_identity_stops(None, &live).len(), 0);
    }

    #[test]
    fn join_targets_the_oldest_other_session() {
        let live = [
            sess(Some(1), (3840, 2160, 60)), // oldest
            sess(Some(2), (1280, 720, 120)),
        ];
        assert!(matches!(
            decide(ModeConflict::Join, fp(3), &live),
            Admission::Join((3840, 2160, 60))
        ));
    }

    /// Serialises the tests that use the process-global live table.
    static CLAIM_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// The regression test for the concurrent-Hello race.
    ///
    /// Two Hellos for one identity claim at the same instant. Before the fix the table
    /// was only written once a pipeline was up, so both read an empty table: both got
    /// `Separate` with no zombies, and each created its own display. With the claim going
    /// in under the same lock as the decision, exactly one of them can be the primary;
    /// the other must see it as its own zombie.
    #[test]
    fn simultaneous_claims_admit_one_primary() {
        let _serial = CLAIM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(live_len(), 0, "table must start empty");

        let start = Arc::new(std::sync::Barrier::new(2));
        // The guards come back to THIS thread and are held there, so "both claims exist at
        // the same time" is asserted where both are provably alive — no reliance on
        // in-thread sampling relative to a barrier.
        let (tx, rx) = std::sync::mpsc::channel();
        let mut threads = Vec::new();
        for _ in 0..2 {
            let start = Arc::clone(&start);
            let tx = tx.clone();
            threads.push(std::thread::spawn(move || {
                start.wait();
                let claimed = claim(
                    ModeConflict::Separate,
                    fp(7),
                    (1920, 1080, 60),
                    Arc::new(AtomicBool::new(false)),
                    "peer".into(),
                );
                let saw_zombies = claimed.own_zombies.len();
                let separate = matches!(claimed.decision, Admission::Separate);
                tx.send((saw_zombies, separate, claimed.guard))
                    .expect("collector is alive");
            }));
        }
        drop(tx);
        let collected: Vec<(usize, bool, LiveGuard)> = rx.iter().collect();
        for t in threads {
            t.join().unwrap();
        }

        assert_eq!(collected.len(), 2, "both Hellos claimed");
        assert_eq!(live_len(), 2, "both claims are live at once");
        let results: Vec<(usize, bool)> = collected.iter().map(|(z, sep, _)| (*z, *sep)).collect();
        assert!(
            results.iter().all(|(_, sep)| *sep),
            "same-identity claims never conflict under Separate"
        );
        let primaries = results.iter().filter(|(z, _)| *z == 0).count();
        let secondaries = results.iter().filter(|(z, _)| *z == 1).count();
        assert_eq!(
            primaries, 1,
            "exactly one Hello may be the primary: {results:?}"
        );
        assert_eq!(secondaries, 1, "the other must see the first: {results:?}");
        drop(collected);
        assert_eq!(live_len(), 0, "both claims released");
    }

    /// A claim whose pipeline never comes up must not strand the identity: dropping the
    /// guard (the failure path in `serve_session`) removes the entry, and the client's
    /// next Hello is a clean primary again rather than its own zombie.
    #[test]
    fn failed_claim_is_reclaimable() {
        let _serial = CLAIM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(live_len(), 0);

        let failed = claim(
            ModeConflict::Separate,
            fp(8),
            (1280, 720, 60),
            Arc::new(AtomicBool::new(false)),
            "peer".into(),
        );
        assert_eq!(live_len(), 1);
        drop(failed); // the pipeline failed / the session returned early
        assert_eq!(live_len(), 0);

        let retry = claim(
            ModeConflict::Separate,
            fp(8),
            (1280, 720, 60),
            Arc::new(AtomicBool::new(false)),
            "peer".into(),
        );
        assert!(
            retry.own_zombies.is_empty(),
            "a failed claim must not leave a zombie behind"
        );
        assert!(matches!(retry.decision, Admission::Separate));
        drop(retry);
    }

    /// Documents the ordering that caused the bug: with two table accesses (admit, then
    /// register later) a second Hello cannot see the first, so both are admitted clean.
    /// This is what `claim` collapses into one access.
    #[test]
    fn pre_fix_ordering_admits_two_same_identity_hellos() {
        let _serial = CLAIM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(live_len(), 0);

        let first = admit(fp(9));
        let second = admit(fp(9));
        assert!(matches!(first, Admission::Separate));
        assert!(matches!(second, Admission::Separate));
        assert!(
            preempt_same_identity(fp(9)).is_empty(),
            "neither Hello has registered, so neither is visible to the other"
        );
        assert_eq!(live_len(), 0, "nothing was registered by admission alone");
    }

    /// A `join` claim must record the mode it will actually serve, at insert time.
    ///
    /// The defect this pins down: `claim` used to store the client's REQUESTED mode and the
    /// handshake corrected it with `set_mode` after the lock was released, so a concurrent claim
    /// in that window resolved `join`/`reject` against a mode no session was using.
    #[test]
    fn claim_records_the_join_resolved_mode_under_the_lock() {
        let _serial = CLAIM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(live_len(), 0);

        let primary = claim(
            ModeConflict::Separate,
            fp(1),
            (2560, 1440, 60),
            Arc::new(AtomicBool::new(false)),
            "primary".into(),
        );

        // A different client asks for a different mode; `join` downgrades it to the live one, and
        // the claim itself reports that mode back to the caller that must serve at it.
        let joined = claim(
            ModeConflict::Join,
            fp(2),
            (1280, 720, 120),
            Arc::new(AtomicBool::new(false)),
            "joiner".into(),
        );
        assert!(matches!(joined.decision, Admission::Join((2560, 1440, 60))));
        assert_eq!(
            joined.mode,
            (2560, 1440, 60),
            "the claim must report the mode it will serve"
        );

        // ...and the TABLE holds it: a third claim resolving against the table must see the join
        // mode, not the joiner's request. This is what the set_mode-after-unlock ordering broke.
        let third = claim(
            ModeConflict::Join,
            fp(3),
            (640, 480, 30),
            Arc::new(AtomicBool::new(false)),
            "third".into(),
        );
        assert!(
            matches!(third.decision, Admission::Join((2560, 1440, 60))),
            "the joiner's RESOLVED mode, not its request, is what the next claim sees: {:?}",
            third.decision
        );

        drop((primary, joined, third));
        assert_eq!(live_len(), 0);
    }

    /// `released` is the observable a preempting session waits on: it must stay false until the
    /// preempted session ends (its guard drops), not merely until its stop flag is set.
    #[test]
    fn released_tracks_the_guard_not_the_stop_flag() {
        let _serial = CLAIM_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(live_len(), 0);

        let stop = Arc::new(AtomicBool::new(false));
        let claimed = claim(
            ModeConflict::Separate,
            fp(4),
            (1920, 1080, 60),
            Arc::clone(&stop),
            "victim".into(),
        );
        assert!(!released(&stop), "the session is still live");

        // Signalling the stop flag is what a `steal`/reconnect does FIRST; it does not release
        // anything, and a preemptor must keep waiting.
        stop.store(true, Ordering::SeqCst);
        assert!(
            !released(&stop),
            "a signalled session that has not ended still holds its display"
        );

        drop(claimed);
        assert!(
            released(&stop),
            "the entry is gone, so the display is released"
        );
        assert!(
            released(&Arc::new(AtomicBool::new(false))),
            "an entry nobody owns is trivially released"
        );
    }
}
