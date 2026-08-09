//! Measured LAN/WAN transport states (latency Phase 6): the host classifies the link from
//! MEASURED samples (RTT, RTT variation, loss, capacity) — never from private-IP heuristics —
//! with hysteresis, and each state carries a policy the send path applies (FEC floor, pacing
//! factor, burst cap, queue-age limit).
//!
//! The state machine is pure and fully unit-tested here; the control task feeds it one
//! `TransportSample` per ~750 ms window (loss from client `LossReport`s, RTT from the QUIC
//! control connection, capacity from the speed-test probe bursts) and the send loop reads the
//! resulting [`TransportPolicy`] through shared atomics.

use std::time::Duration;

/// The measured link classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TransportState {
    /// Not yet classified (the initial state, and the middle band between `FastLan`/`Wan`).
    #[default]
    Auto,
    /// Clean low-latency LAN: tiny RTT, tiny jitter, negligible loss, ample headroom.
    FastLan,
    /// Constrained link (WAN/loaded): treat queue residency as latency.
    Wan,
    /// The link is degrading badly — deadline-limited, capacity-limited.
    Degraded,
}

impl TransportState {
    pub fn label(self) -> &'static str {
        match self {
            TransportState::Auto => "auto",
            TransportState::FastLan => "fast_lan",
            TransportState::Wan => "wan",
            TransportState::Degraded => "degraded",
        }
    }
}

/// The policy one measured state drives on the send path. All values are the Phase-6 initial
/// table; they are consumed per frame by the send loop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransportPolicy {
    /// Burst cap (bytes) for the microburst stage — `None` = the automatic default.
    pub burst_bytes: Option<usize>,
    /// Pacing multiplier over the live bitrate.
    pub pace_factor: f64,
    /// The FEC floor (percent) the adaptive controller may not go below.
    pub fec_floor_pct: u8,
    /// The queue-age limit, in frame periods, for the stale-deadline drop.
    pub queue_age_frames: f64,
}

impl TransportPolicy {
    /// The send path's default before any classification (`Auto`): the existing behavior.
    fn auto() -> TransportPolicy {
        TransportPolicy {
            burst_bytes: None,
            pace_factor: 3.0,
            fec_floor_pct: 1,
            queue_age_frames: 1.0,
        }
    }

    pub fn for_state(state: TransportState) -> TransportPolicy {
        match state {
            TransportState::Auto => TransportPolicy::auto(),
            TransportState::FastLan => TransportPolicy {
                burst_bytes: Some(128 * 1024),
                pace_factor: 3.0,
                fec_floor_pct: 1,
                queue_age_frames: 0.5,
            },
            TransportState::Wan => TransportPolicy {
                burst_bytes: Some(64 * 1024),
                pace_factor: 1.5,
                fec_floor_pct: 10,
                queue_age_frames: 1.0,
            },
            TransportState::Degraded => TransportPolicy {
                burst_bytes: None,
                pace_factor: 1.25,
                fec_floor_pct: 10,
                queue_age_frames: 1.0,
            },
        }
    }
}

/// The policy the send path reads per frame, published by the control task whenever the
/// transport state changes. All reads are single relaxed atomic loads — no lock on the hot path.
pub struct TransportPolicyShared {
    /// Burst cap in bytes; 0 = the automatic default (`None`).
    burst_bytes: std::sync::atomic::AtomicU64,
    /// Pacing multiplier over the live bitrate, ×100.
    pace_factor_x100: std::sync::atomic::AtomicU64,
    /// The adaptive-FEC floor, percent.
    fec_floor: std::sync::atomic::AtomicU8,
    /// The stale-deadline queue-age limit in frame periods, ×100.
    queue_age_x100: std::sync::atomic::AtomicU64,
}

impl Default for TransportPolicyShared {
    fn default() -> Self {
        Self::from_policy(TransportPolicy::auto())
    }
}

impl TransportPolicyShared {
    /// The session's initial policy: the `Auto` table seeded with the operator env overrides
    /// (`SLIPSTREAM_PACE_FACTOR`, `SLIPSTREAM_PACE_BURST_KB`) — those stay the defaults until a
    /// measured transport state publishes its own table.
    pub fn from_env() -> TransportPolicyShared {
        let mut p = TransportPolicy::auto();
        if let Some(v) = std::env::var("SLIPSTREAM_PACE_FACTOR")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|f| f.is_finite() && *f >= 0.0)
        {
            p.pace_factor = v;
        }
        if let Some(kb) = std::env::var("SLIPSTREAM_PACE_BURST_KB")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
        {
            p.burst_bytes = Some(kb * 1024);
        }
        TransportPolicyShared::from_policy(p)
    }

    pub fn from_policy(p: TransportPolicy) -> TransportPolicyShared {
        TransportPolicyShared {
            burst_bytes: std::sync::atomic::AtomicU64::new(p.burst_bytes.unwrap_or(0) as u64),
            pace_factor_x100: std::sync::atomic::AtomicU64::new((p.pace_factor * 100.0) as u64),
            fec_floor: std::sync::atomic::AtomicU8::new(p.fec_floor_pct),
            queue_age_x100: std::sync::atomic::AtomicU64::new((p.queue_age_frames * 100.0) as u64),
        }
    }

    /// Publish a new policy (called on a state transition).
    pub fn apply(&self, p: TransportPolicy) {
        self.burst_bytes.store(
            p.burst_bytes.unwrap_or(0) as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
        self.pace_factor_x100.store(
            (p.pace_factor * 100.0) as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
        self.fec_floor
            .store(p.fec_floor_pct, std::sync::atomic::Ordering::Relaxed);
        self.queue_age_x100.store(
            (p.queue_age_frames * 100.0) as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    pub fn burst_bytes(&self) -> Option<usize> {
        let b = self.burst_bytes.load(std::sync::atomic::Ordering::Relaxed) as usize;
        (b > 0).then_some(b)
    }

    pub fn pace_factor(&self) -> f64 {
        self.pace_factor_x100
            .load(std::sync::atomic::Ordering::Relaxed) as f64
            / 100.0
    }

    pub fn fec_floor(&self) -> u8 {
        self.fec_floor.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// The stale-deadline queue-age limit in frame periods.
    pub fn queue_age_frames(&self) -> f64 {
        self.queue_age_x100
            .load(std::sync::atomic::Ordering::Relaxed) as f64
            / 100.0
    }
}

/// One measurement window fed to the machine (~750 ms of transport observation).
#[derive(Clone, Copy, Debug, Default)]
pub struct TransportSample {
    /// Smoothed RTT, milliseconds.
    pub rtt_ms: f64,
    /// RTT variation, milliseconds.
    pub rtt_var_ms: f64,
    /// Loss in parts per million.
    pub loss_ppm: u64,
    /// Measured capacity, bps (0 = not yet measured).
    pub capacity_bps: u64,
    /// The session's current target bitrate, bps.
    pub target_bps: u64,
}

/// The hysteresis rules (Phase 6):
/// - `Auto` → `FastLan` after 3 seconds when ALL of: smoothed RTT ≤ 8 ms, RTT variation ≤ 2 ms,
///   loss < 0.1%, measured capacity ≥ 3× target bitrate.
/// - `Auto` (or `FastLan`) → `Wan` after TWO consecutive 750 ms windows when ANY of: RTT ≥ 20 ms,
///   variation ≥ 5 ms, loss ≥ 0.5%, capacity < 2× target.
/// - `Wan` → `FastLan` only through the middle band: the state holds for 5 seconds while the
///   link sits between the two thresholds, then re-evaluates.
/// - `Degraded` is entered when the link fails even the WAN envelope (loss ≥ 2% or RTT ≥ 50 ms
///   sustained); its policy is deadline-limited.
#[derive(Debug)]
pub struct TransportStateMachine {
    state: TransportState,
    /// Milliseconds spent meeting the FastLan conditions (cleared by any miss).
    fastlan_ms: u64,
    /// Consecutive windows meeting any WAN condition.
    wan_windows: u32,
    /// Milliseconds spent in the middle band (between the thresholds) while `Wan`-ish.
    hold_ms: u64,
    last_sample: TransportSample,
}

impl Default for TransportStateMachine {
    fn default() -> Self {
        Self {
            state: TransportState::Auto,
            fastlan_ms: 0,
            wan_windows: 0,
            hold_ms: 0,
            last_sample: TransportSample::default(),
        }
    }
}

impl TransportStateMachine {
    pub fn state(&self) -> TransportState {
        self.state
    }

    /// The policy for the current state.
    pub fn policy(&self) -> TransportPolicy {
        TransportPolicy::for_state(self.state)
    }

    const WINDOW_MS: u64 = 750;
    const FASTLAN_MS: u64 = 3000;
    const WAN_WINDOWS: u32 = 2;
    const HOLD_MS: u64 = 5000;

    fn fastlan_ok(s: &TransportSample) -> bool {
        s.rtt_ms <= 8.0
            && s.rtt_var_ms <= 2.0
            && s.loss_ppm < 1000
            && s.capacity_bps > 0
            && s.capacity_bps >= s.target_bps.saturating_mul(3)
    }

    fn wan_ok(s: &TransportSample) -> bool {
        s.rtt_ms >= 20.0
            || s.rtt_var_ms >= 5.0
            || s.loss_ppm >= 5000
            || (s.capacity_bps > 0 && s.capacity_bps < s.target_bps.saturating_mul(2))
    }

    fn degraded_ok(s: &TransportSample) -> bool {
        s.loss_ppm >= 20_000 || s.rtt_ms >= 50.0
    }

    /// In the middle band: neither fast-LAN-clean nor WAN-worthy.
    fn middle_band(s: &TransportSample) -> bool {
        !Self::fastlan_ok(s) && !Self::wan_ok(s)
    }

    /// Feed one measurement window (≈750 ms of transport observation) and return the new state
    /// when it changed.
    pub fn feed(&mut self, sample: TransportSample, elapsed: Duration) -> TransportState {
        self.last_sample = sample;
        let win_ms = elapsed.as_millis().max(1) as u64;

        // Degraded is the emergency state: leave it only on a sustained recovery window.
        if self.state == TransportState::Degraded {
            if Self::fastlan_ok(&sample) || Self::middle_band(&sample) {
                self.state = TransportState::Auto;
                self.fastlan_ms = 0;
                self.wan_windows = 0;
                self.hold_ms = 0;
            }
            return self.state;
        }

        if Self::degraded_ok(&sample) {
            self.state = TransportState::Degraded;
            self.fastlan_ms = 0;
            self.wan_windows = 0;
            self.hold_ms = 0;
            return self.state;
        }

        match self.state {
            TransportState::Auto => {
                // FastLan needs 3 s of clean conditions; Wan needs 2 consecutive bad windows.
                if Self::fastlan_ok(&sample) {
                    self.fastlan_ms += win_ms;
                    self.wan_windows = 0;
                    if self.fastlan_ms >= Self::FASTLAN_MS {
                        self.state = TransportState::FastLan;
                        self.fastlan_ms = 0;
                    }
                } else {
                    self.fastlan_ms = 0;
                    if Self::wan_ok(&sample) {
                        self.wan_windows += 1;
                        if self.wan_windows >= Self::WAN_WINDOWS {
                            self.state = TransportState::Wan;
                            self.wan_windows = 0;
                        }
                    } else {
                        self.wan_windows = 0;
                    }
                }
            }
            TransportState::FastLan => {
                if !Self::fastlan_ok(&sample) {
                    // Fall out of FastLan into the middle band / Wan with the same rules.
                    self.fastlan_ms = 0;
                    if Self::wan_ok(&sample) {
                        self.wan_windows += 1;
                        if self.wan_windows >= Self::WAN_WINDOWS {
                            self.state = TransportState::Wan;
                            self.wan_windows = 0;
                        }
                    } else {
                        self.wan_windows = 0;
                        // Middle band: hold the previous state for 5 s before re-evaluating.
                        self.hold_ms += win_ms;
                        if self.hold_ms >= Self::HOLD_MS {
                            self.state = TransportState::Auto;
                            self.hold_ms = 0;
                        }
                    }
                }
            }
            TransportState::Wan => {
                // Hold for 5 s in the middle band, then drop to Auto for a fresh evaluation.
                if Self::middle_band(&sample) {
                    self.hold_ms += win_ms;
                    if self.hold_ms >= Self::HOLD_MS {
                        self.state = TransportState::Auto;
                        self.hold_ms = 0;
                        self.wan_windows = 0;
                    }
                } else if Self::fastlan_ok(&sample) {
                    self.hold_ms = 0;
                    self.fastlan_ms += win_ms;
                    if self.fastlan_ms >= Self::FASTLAN_MS {
                        self.state = TransportState::FastLan;
                        self.fastlan_ms = 0;
                    }
                } else {
                    self.hold_ms = 0;
                    self.fastlan_ms = 0;
                }
            }
            TransportState::Degraded => unreachable!("handled above"),
        }
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean(rtt_ms: f64, target: u64) -> TransportSample {
        TransportSample {
            rtt_ms,
            rtt_var_ms: 1.0,
            loss_ppm: 10,
            capacity_bps: target * 4,
            target_bps: target,
        }
    }

    fn wan(rtt_ms: f64, target: u64) -> TransportSample {
        TransportSample {
            rtt_ms,
            rtt_var_ms: 8.0,
            // ≥ 0.5% (the WAN threshold) but well under the 2% Degraded line.
            loss_ppm: 6_000,
            capacity_bps: target * 3,
            target_bps: target,
        }
    }

    const WIN: Duration = Duration::from_millis(750);

    #[test]
    fn auto_enters_fastlan_after_three_seconds_of_clean_samples() {
        let mut m = TransportStateMachine::default();
        for _ in 0..3 {
            assert_eq!(m.feed(clean(5.0, 10_000_000), WIN), TransportState::Auto);
        }
        // 4th window crosses 3 s.
        assert_eq!(m.feed(clean(5.0, 10_000_000), WIN), TransportState::FastLan);
    }

    #[test]
    fn fastlan_requires_capacity_headroom() {
        let mut m = TransportStateMachine::default();
        let mut s = clean(5.0, 10_000_000);
        s.capacity_bps = 0; // not yet measured
        for _ in 0..8 {
            assert_eq!(m.feed(s, WIN), TransportState::Auto);
        }
        s.capacity_bps = 10_000_000 * 2; // only 2× — still not FastLan
        for _ in 0..8 {
            assert_eq!(m.feed(s, WIN), TransportState::Auto);
        }
    }

    #[test]
    fn wan_enters_after_two_consecutive_bad_windows() {
        let mut m = TransportStateMachine::default();
        assert_eq!(m.feed(wan(30.0, 10_000_000), WIN), TransportState::Auto);
        assert_eq!(m.feed(wan(30.0, 10_000_000), WIN), TransportState::Wan);
    }

    #[test]
    fn wan_requires_consecutive_windows() {
        let mut m = TransportStateMachine::default();
        assert_eq!(m.feed(wan(30.0, 10_000_000), WIN), TransportState::Auto);
        assert_eq!(m.feed(clean(5.0, 10_000_000), WIN), TransportState::Auto);
        assert_eq!(m.feed(wan(30.0, 10_000_000), WIN), TransportState::Auto);
        assert_eq!(m.feed(wan(30.0, 10_000_000), WIN), TransportState::Wan);
    }

    #[test]
    fn middle_band_holds_the_previous_state_for_five_seconds() {
        let mut m = TransportStateMachine::default();
        for _ in 0..4 {
            m.feed(clean(5.0, 10_000_000), WIN);
        }
        assert_eq!(m.state(), TransportState::FastLan);
        // Now a middle-band sample (12 ms RTT — between the thresholds, loss clean).
        let mut mid = clean(12.0, 10_000_000);
        mid.rtt_var_ms = 3.0;
        for _ in 0..6 {
            // 4.5 s — still held.
            assert_eq!(m.feed(mid, WIN), TransportState::FastLan);
        }
        // 7th middle window crosses 5 s → Auto.
        assert_eq!(m.feed(mid, WIN), TransportState::Auto);
    }

    #[test]
    fn degraded_enters_on_sustained_badness_and_recovers() {
        let mut m = TransportStateMachine::default();
        let mut s = clean(5.0, 10_000_000);
        s.loss_ppm = 50_000;
        assert_eq!(m.feed(s, WIN), TransportState::Degraded);
        // A clean window recovers to Auto.
        assert_eq!(m.feed(clean(5.0, 10_000_000), WIN), TransportState::Auto);
    }

    #[test]
    fn policy_table_matches_the_phase_6_contract() {
        let fast = TransportPolicy::for_state(TransportState::FastLan);
        assert_eq!(fast.burst_bytes, Some(128 * 1024));
        assert_eq!(fast.pace_factor, 3.0);
        assert_eq!(fast.fec_floor_pct, 1);
        assert_eq!(fast.queue_age_frames, 0.5);

        let wan = TransportPolicy::for_state(TransportState::Wan);
        assert!(wan.burst_bytes.unwrap_or(0) <= 64 * 1024);
        assert_eq!(wan.fec_floor_pct, 10);
        assert_eq!(wan.queue_age_frames, 1.0);
    }
}
