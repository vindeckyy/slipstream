//! Host **update check** (design `host-update-from-web-console.md`, phase U0).
//!
//! This module answers one question for the console: *does a newer host release exist for
//! this box's channel* — by fetching the per-channel signed manifest and verifying it against
//! the Ed25519 keys pinned below. It deliberately contains **no apply code**: U0 ships check
//! everywhere; Linux package and source-rebuild apply legs use the same status surface.
//!
//! Shape: a process-wide cache + a lazy refresh. `GET /update/status` returns the cache and,
//! when it is older than [`AUTO_REFRESH_AFTER`], kicks a background refresh — the console
//! polls status anyway, so freshness needs no timer of its own. `POST /update/check` forces a
//! refresh, rate-limited to one per [`FORCE_MIN_INTERVAL`].
//!
//! Trust and failure rules live in [`manifest`]; the serial floor persisted here
//! (`update-state.json`) is what makes a replayed older manifest an *error*, not a silent
//! downgrade of our knowledge. `SLIPSTREAM_UPDATE_CHECK=0` disables all network activity —
//! status then reports `check_disabled` and carries whatever identity facts need no network.

pub(crate) mod detect;
pub(crate) mod jobs;
#[cfg(target_os = "linux")]
mod linux;
// The signed manifest's schema + validation live in `ss-update-check` (shared with the
// client's check). Re-exported so `manifest::…` call sites below are unchanged.
pub(crate) use ss_update_check::manifest;

use manifest::Manifest;
use ss_update_check::{FeedError, PublicKey};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The Ed25519 public keys this binary trusts for update manifests. The list itself lives in
/// `ss-update-check` (the client verifies the same manifest and must trust the same signers);
/// this alias keeps the host's call sites and the plan's vocabulary intact.
pub(crate) use ss_update_check::OFFICIAL_UPDATE_KEYS as UPDATE_KEYS;

/// A cache older than this is refreshed in the background on the next status read.
const AUTO_REFRESH_AFTER: Duration = Duration::from_secs(6 * 60 * 60);

/// Forced checks (`POST /update/check`) are rate-limited to one per this interval.
pub(crate) const FORCE_MIN_INTERVAL: Duration = Duration::from_secs(30);

/// A manifest whose publish serial is older than this is flagged stale in status — the
/// freeze-detection hint (design §3.2), not an error.
const STALE_AFTER: Duration = Duration::from_secs(45 * 24 * 60 * 60);

/// Update checks disabled by operator config (env or `host.env`).
pub(crate) fn check_disabled() -> bool {
    matches!(
        std::env::var("SLIPSTREAM_UPDATE_CHECK").as_deref(),
        Ok("0") | Ok("false") | Ok("off")
    )
}

/// One-click apply disabled by operator config — the host-side kill switch (design §4.2): the
/// apply route 409s and status reports `notify` even on kinds an apply leg exists for. The
/// check surface is unaffected.
pub(crate) fn apply_disabled() -> bool {
    matches!(
        std::env::var("SLIPSTREAM_UPDATE_APPLY").as_deref(),
        Ok("0") | Ok("false") | Ok("off")
    )
}

/// What the console may offer for this install: `full` (one-click apply), `staged` (apply +
/// reboot to finish — rpm-ostree), or `notify` (show the command). Linux legs additionally
/// require the packaged root helper AND the operator's group opt-in; pacman also the
/// root-owned full-sysupgrade config (design §5).
pub(crate) fn apply_support() -> &'static str {
    if apply_disabled() {
        return "notify";
    }
    let (kind, _) = detect::detect();
    match kind {
        #[cfg(target_os = "linux")]
        detect::InstallKind::Apt | detect::InstallKind::Dnf | detect::InstallKind::Sysext
            if linux::helper_installed() && linux::opted_in() =>
        {
            "full"
        }
        #[cfg(target_os = "linux")]
        detect::InstallKind::RpmOstree if linux::helper_installed() && linux::opted_in() => {
            "staged"
        }
        #[cfg(target_os = "linux")]
        detect::InstallKind::Pacman
            if linux::helper_installed() && linux::opted_in() && linux::pacman_opted_in() =>
        {
            "full"
        }
        // The Deck source rebuild is user-owned — no helper, no group.
        #[cfg(target_os = "linux")]
        detect::InstallKind::SteamosSource => "full",
        _ => "notify",
    }
}

/// The opt-in instruction for status: this install COULD one-click apply (helper shipped)
/// but the operator hasn't joined the `slipstream-update` group yet.
pub(crate) fn opt_in_hint() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let (kind, _) = detect::detect();
        let capable = matches!(
            kind,
            detect::InstallKind::Apt
                | detect::InstallKind::Dnf
                | detect::InstallKind::Sysext
                | detect::InstallKind::RpmOstree
                | detect::InstallKind::Pacman
        );
        if capable && !apply_disabled() && linux::helper_installed() && !linux::opted_in() {
            return Some(linux::opt_in_hint());
        }
    }
    None
}

fn pinned_keys() -> Vec<PublicKey> {
    UPDATE_KEYS
        .iter()
        .filter(|k| !k.is_empty())
        .filter_map(|k| PublicKey::parse(k).ok())
        .collect()
}

// ---------------------------------------------------------------- runtime state

/// What the last successful refresh produced.
#[derive(Clone)]
pub(crate) struct Checked {
    pub manifest: Manifest,
    pub fetched_unix: u64,
}

#[derive(Default)]
struct Runtime {
    checked: Option<Checked>,
    last_error: Option<String>,
    /// The channel has nothing published (and never had, for us) — an expected state, kept
    /// out of `last_error` so a console never paints an empty feed as a broken host.
    not_published: bool,
    /// Refresh in flight (status kicks at most one).
    refreshing: bool,
    /// Wall-clock guard for the forced-check rate limit.
    last_forced: Option<Instant>,
    /// Last attempt of any kind — drives the auto-refresh cadence.
    last_attempt: Option<Instant>,
    /// The manifest version an `update.available` event was already emitted for, so a
    /// steady-state "newer exists" doesn't re-announce every 6 h.
    announced: Option<String>,
    /// The live apply job, when one is running (single-flight).
    job: Option<jobs::JobSnapshot>,
}

fn runtime() -> &'static Mutex<Runtime> {
    static RT: OnceLock<Mutex<Runtime>> = OnceLock::new();
    RT.get_or_init(|| Mutex::new(Runtime::default()))
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---------------------------------------------------------------- serial floor

/// Persisted anti-rollback state: the highest manifest serial ever accepted per channel.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct FloorFile {
    #[serde(default)]
    serial_floor: std::collections::BTreeMap<String, u64>,
}

fn state_path() -> PathBuf {
    ss_paths::config_dir().join("update-state.json")
}

fn load_floor(path: &Path, channel: &str) -> u64 {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<FloorFile>(&b).ok())
        .and_then(|f| f.serial_floor.get(channel).copied())
        .unwrap_or(0)
}

/// Raise (never lower) the floor; atomic tmp+rename so a power cut can't half-write it.
fn store_floor(path: &Path, channel: &str, serial: u64) {
    let mut file: FloorFile = std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let slot = file.serial_floor.entry(channel.to_string()).or_insert(0);
    if serial <= *slot {
        return;
    }
    *slot = serial;
    let Ok(bytes) = serde_json::to_vec_pretty(&file) else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, &bytes).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

// ---------------------------------------------------------------- refresh

/// Fetch + verify the channel manifest through the shared checker. Blocking — call from a
/// blocking thread.
fn fetch_manifest_blocking(channel: &str) -> Result<Manifest, FeedError> {
    ss_update_check::feed::fetch_manifest_blocking(
        &ss_update_check::feed::feed_base(),
        channel,
        &pinned_keys(),
        &format!(
            "slipstream-host/{} (update-check)",
            env!("SLIPSTREAM_VERSION")
        ),
    )
}

/// One full refresh: fetch, verify, enforce + raise the serial floor, update the cache,
/// announce a newly available release on the event bus. Returns the user-facing error on
/// failure (also cached for status).
pub(crate) fn refresh_blocking() -> Result<Checked, FeedError> {
    let (kind, channel) = detect::detect();
    let result = fetch_manifest_blocking(channel.as_str()).and_then(|m| {
        let path = state_path();
        let floor = load_floor(&path, channel.as_str());
        if m.serial < floor {
            return Err(FeedError::Failed(format!(
                "manifest serial {} is older than the last accepted {} — refusing rollback",
                m.serial, floor
            )));
        }
        store_floor(&path, channel.as_str(), m.serial);
        Ok(m)
    });

    let mut rt = runtime().lock().unwrap();
    rt.last_attempt = Some(Instant::now());
    rt.refreshing = false;
    match result {
        Ok(m) => {
            let checked = Checked {
                manifest: m,
                fetched_unix: now_unix(),
            };
            let newer = detect::is_newer(
                &checked.manifest.version,
                checked.manifest.ci_run,
                env!("SLIPSTREAM_VERSION"),
                channel,
            );
            if newer && rt.announced.as_deref() != Some(checked.manifest.version.as_str()) {
                rt.announced = Some(checked.manifest.version.clone());
                crate::events::emit(crate::events::EventKind::UpdateAvailable {
                    version: checked.manifest.version.clone(),
                    channel: channel.as_str().to_string(),
                    install_kind: kind.as_str().to_string(),
                });
            }
            rt.last_error = None;
            rt.not_published = false;
            rt.checked = Some(checked.clone());
            Ok(checked)
        }
        Err(e) => {
            let (last_error, not_published) = classify_failure(&e, rt.checked.is_some());
            rt.last_error = last_error;
            rt.not_published = not_published;
            Err(e)
        }
    }
}

/// Split a failed refresh into `(last_error, not_published)` — the two are never both set.
///
/// An empty channel is only benign while we have never seen a manifest for it. Once a check
/// has succeeded, the same 404 means the feed LOST a document it used to serve, which is a
/// regression that must stay a loud error rather than being excused as "nothing yet".
fn classify_failure(e: &FeedError, had_manifest: bool) -> (Option<String>, bool) {
    if e.is_not_published() && !had_manifest {
        (None, true)
    } else {
        (Some(e.to_string()), false)
    }
}

/// The status handler's read: current cache + errors, kicking a background refresh when the
/// cache is cold and checks are enabled.
pub(crate) fn snapshot_and_maybe_refresh() -> Snapshot {
    let mut kick = false;
    let snap = {
        let mut rt = runtime().lock().unwrap();
        let cold = rt
            .last_attempt
            .map(|t| t.elapsed() >= AUTO_REFRESH_AFTER)
            .unwrap_or(true);
        if cold && !rt.refreshing && !check_disabled() {
            rt.refreshing = true;
            rt.last_attempt = Some(Instant::now());
            kick = true;
        }
        Snapshot {
            checked: rt.checked.clone(),
            last_error: rt.last_error.clone(),
            not_published: rt.not_published,
            job: rt.job.clone(),
            last_result: jobs::read_result(&jobs::result_path()),
        }
    };
    if kick {
        // Fire-and-forget; the console's next poll reads the outcome.
        tokio::task::spawn_blocking(|| {
            let _ = refresh_blocking();
        });
    }
    snap
}

/// A forced check (`POST /update/check`): rate-limited, blocking until the refresh finishes.
pub(crate) async fn force_check() -> Result<Snapshot, ForceError> {
    if check_disabled() {
        return Err(ForceError::Disabled);
    }
    {
        let mut rt = runtime().lock().unwrap();
        if let Some(t) = rt.last_forced {
            if t.elapsed() < FORCE_MIN_INTERVAL {
                return Err(ForceError::TooSoon);
            }
        }
        rt.last_forced = Some(Instant::now());
        rt.refreshing = true;
    }
    let _ = tokio::task::spawn_blocking(refresh_blocking).await;
    let rt = runtime().lock().unwrap();
    Ok(Snapshot {
        checked: rt.checked.clone(),
        last_error: rt.last_error.clone(),
        not_published: rt.not_published,
        job: rt.job.clone(),
        last_result: jobs::read_result(&jobs::result_path()),
    })
}

pub(crate) enum ForceError {
    Disabled,
    TooSoon,
}

// ---------------------------------------------------------------- apply

/// Why an apply request was refused (mapped to 409s by the API layer).
pub(crate) enum ApplyError {
    /// This install kind has no one-click leg (or the operator kill switch is on) — the
    /// console shows the command instead.
    Unsupported,
    /// `SLIPSTREAM_UPDATE_APPLY=0`.
    Disabled,
    /// An apply is already running.
    JobRunning,
    /// A stream is live and the request didn't say `force`.
    SessionActive,
    /// No verified manifest announces something newer.
    NothingToApply,
}

/// Start the Linux apply pipeline. The request carries no version, URL, or channel. Everything
/// comes from the verified cached manifest.
pub(crate) fn start_apply(force: bool, session_active: bool) -> Result<(), ApplyError> {
    if apply_disabled() {
        return Err(ApplyError::Disabled);
    }
    let (kind, channel) = detect::detect();
    let linux_leg = matches!(
        kind,
        detect::InstallKind::Apt
            | detect::InstallKind::Dnf
            | detect::InstallKind::Sysext
            | detect::InstallKind::RpmOstree
            | detect::InstallKind::Pacman
            | detect::InstallKind::SteamosSource
    );
    if !linux_leg {
        return Err(ApplyError::Unsupported);
    }
    #[cfg(target_os = "linux")]
    if linux_leg && kind != detect::InstallKind::SteamosSource {
        // The Deck source rebuild is user-owned and needs no root helper; every other Linux
        // leg goes through it.
        if !linux::helper_installed() {
            return Err(ApplyError::Unsupported);
        }
        // The pacman leg additionally requires the root-owned full-sysupgrade opt-in — the
        // helper enforces it too; refusing here keeps the console honest before any spawn.
        if kind == detect::InstallKind::Pacman && !linux::pacman_opted_in() {
            return Err(ApplyError::Unsupported);
        }
    }
    if session_active && !force {
        return Err(ApplyError::SessionActive);
    }

    let (target_version, serial) = {
        let mut rt = runtime().lock().unwrap();
        if rt.job.is_some() {
            return Err(ApplyError::JobRunning);
        }
        // A fresh intent with the old version is still an apply in flight. Do not start a second
        // one under it.
        if matches!(
            jobs::reconcile(
                jobs::read_intent(&jobs::intent_path()),
                env!("SLIPSTREAM_VERSION"),
                now_unix()
            ),
            jobs::Reconciled::StillApplying
        ) {
            return Err(ApplyError::JobRunning);
        }
        let Some(checked) = rt.checked.as_ref() else {
            return Err(ApplyError::NothingToApply);
        };
        let newer = detect::is_newer(
            &checked.manifest.version,
            checked.manifest.ci_run,
            env!("SLIPSTREAM_VERSION"),
            channel,
        );
        if !newer {
            return Err(ApplyError::NothingToApply);
        }
        let version = checked.manifest.version.clone();
        let serial = checked.manifest.serial;
        rt.job = Some(jobs::JobSnapshot {
            target_version: version.clone(),
            stage: "applying",
            received_bytes: 0,
            total_bytes: None,
            started_unix: now_unix(),
        });
        (version, serial)
    };

    tokio::task::spawn_blocking(move || {
        let stage = |s: &'static str| {
            let mut rt = runtime().lock().unwrap();
            if let Some(job) = rt.job.as_mut() {
                job.stage = s;
            }
        };
        let run = if detect::detect().0 == detect::InstallKind::SteamosSource {
            linux::run_apply_steamos(&target_version, serial, &stage)
        } else {
            linux::run_apply(&target_version, serial, &stage)
        };
        let outcome = run.map(|()| PostApply::Done);
        match outcome {
            Ok(PostApply::Done) => {
                runtime().lock().unwrap().job = None;
            }
            Err((stage_name, error)) => {
                let record = jobs::ResultRecord {
                    ok: false,
                    from: env!("SLIPSTREAM_VERSION").into(),
                    to: target_version.clone(),
                    finished_unix: now_unix(),
                    stage: Some(stage_name.into()),
                    error: Some(error),
                    staged: false,
                };
                let _ = jobs::write_json_atomic(&jobs::result_path(), &record);
                runtime().lock().unwrap().job = None;
            }
        }
    });
    Ok(())
}

/// What an apply leg leaves behind after the helper or source rebuild finishes.
#[allow(dead_code)]
enum PostApply {
    /// The leg finished in-process or queued the systemd restart.
    Done,
}

/// Boot-time reconciliation (design §4.2): close out an intent record left by a previous
/// apply. Called once from `mgmt::run` before the API serves.
pub(crate) fn reconcile_at_boot() {
    let path = jobs::intent_path();
    let intent = jobs::read_intent(&path);
    match jobs::reconcile(intent, env!("SLIPSTREAM_VERSION"), now_unix()) {
        jobs::Reconciled::None | jobs::Reconciled::StillApplying => {}
        jobs::Reconciled::Success(record) => {
            tracing::info!(from = %record.from, to = %record.to, "host update applied");
            let _ = jobs::write_json_atomic(&jobs::result_path(), &record);
            let _ = std::fs::remove_file(&path);
            crate::events::emit(crate::events::EventKind::UpdateApplied {
                from: record.from,
                to: record.to,
            });
        }
        jobs::Reconciled::Failed(record) => {
            tracing::warn!(
                from = %record.from,
                to = %record.to,
                error = record.error.as_deref().unwrap_or(""),
                "host update did NOT stick"
            );
            let _ = jobs::write_json_atomic(&jobs::result_path(), &record);
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// What status hands to the API layer.
pub(crate) struct Snapshot {
    pub checked: Option<Checked>,
    pub last_error: Option<String>,
    /// The channel simply has no release yet — mutually exclusive with `last_error`.
    pub not_published: bool,
    /// The live apply job, when one runs. When the process was restarted mid-apply this is
    /// `None` but a fresh intent still reads as in-flight — the API layer surfaces that via
    /// [`Snapshot::applying_from_intent`].
    pub job: Option<jobs::JobSnapshot>,
    /// Durable outcome of the most recent apply attempt.
    pub last_result: Option<jobs::ResultRecord>,
}

impl Snapshot {
    /// An apply is in flight even though no in-process job exists: a fresh intent record from
    /// a spawn that hasn't resolved (this process may be the OLD host in its last seconds, or
    /// a restarted host inside the grace window). The API surfaces it as a `restarting` job.
    pub(crate) fn applying_from_intent(&self) -> Option<jobs::IntentRecord> {
        if self.job.is_some() {
            return None;
        }
        let intent = jobs::read_intent(&jobs::intent_path())?;
        match jobs::reconcile(Some(intent.clone()), env!("SLIPSTREAM_VERSION"), now_unix()) {
            jobs::Reconciled::StillApplying => Some(intent),
            _ => None,
        }
    }

    /// The stale-feed hint: last successful check is fine but the manifest itself was
    /// published suspiciously long ago (freeze detection, design §3.2).
    pub(crate) fn stale(&self) -> bool {
        self.checked
            .as_ref()
            .map(|c| now_unix().saturating_sub(c.manifest.serial) > STALE_AFTER.as_secs())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_roundtrip_and_monotonicity() {
        let dir = std::env::temp_dir().join(format!("ss-update-floor-{}", std::process::id()));
        let path = dir.join("update-state.json");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(load_floor(&path, "stable"), 0);
        store_floor(&path, "stable", 100);
        assert_eq!(load_floor(&path, "stable"), 100);
        // Lowering is a no-op.
        store_floor(&path, "stable", 50);
        assert_eq!(load_floor(&path, "stable"), 100);
        // Channels are independent.
        store_floor(&path, "canary", 7);
        assert_eq!(load_floor(&path, "canary"), 7);
        assert_eq!(load_floor(&path, "stable"), 100);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_floor_file_reads_as_zero() {
        let dir = std::env::temp_dir().join(format!("ss-update-floor2-{}", std::process::id()));
        let path = dir.join("update-state.json");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, b"not json").unwrap();
        assert_eq!(load_floor(&path, "stable"), 0);
        // And writing over it recovers.
        store_floor(&path, "stable", 5);
        assert_eq!(load_floor(&path, "stable"), 5);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The console paints `last_error` as a failure and `not_published` as a plain sentence,
    /// so the two must never arrive together — and the benign reading must not survive a
    /// channel that has already served us a manifest.
    #[test]
    fn empty_channel_is_benign_only_until_a_manifest_has_been_seen() {
        let (err, not_published) = classify_failure(&FeedError::NotPublished, false);
        assert_eq!(err, None);
        assert!(not_published);

        // Same 404, but the feed used to answer — that is a regression, not "nothing yet".
        let (err, not_published) = classify_failure(&FeedError::NotPublished, true);
        assert_eq!(
            err.as_deref(),
            Some("no release has been published on this channel yet")
        );
        assert!(!not_published);

        // Every real failure stays an error whether or not we have a cached manifest.
        for had_manifest in [false, true] {
            let (err, not_published) = classify_failure(
                &FeedError::Failed("feed returned HTTP 500".into()),
                had_manifest,
            );
            assert_eq!(err.as_deref(), Some("feed returned HTTP 500"));
            assert!(!not_published);
        }
    }

    #[test]
    fn pinned_keys_skip_empty_rotation_slot() {
        let keys = pinned_keys();
        assert_eq!(keys.len(), 1, "one live key, one empty rotation slot");
    }

    #[test]
    fn stale_math() {
        let mk = |serial| Snapshot {
            checked: Some(Checked {
                manifest: manifest::parse_verified(
                    serde_json::to_vec(&serde_json::json!({
                        "schema": 1, "channel": "stable", "serial": serial,
                        "version": "0.23.0",
                        "notes_url": "https://github.com/vindeckyy/slipstream/releases",
                    }))
                    .unwrap()
                    .as_slice(),
                    "stable",
                )
                .unwrap(),
                fetched_unix: now_unix(),
            }),
            last_error: None,
            not_published: false,
            job: None,
            last_result: None,
        };
        assert!(!mk(now_unix()).stale());
        assert!(mk(now_unix() - STALE_AFTER.as_secs() - 10).stale());
    }
}
