//! `HostConfig` — the host's runtime knobs parsed ONCE from the environment, instead of the ~68 scattered
//! `env::var` reads recomputed at every call site (some up to 8×, which lets capture + encode silently
//! disagree on the resolved backend — plan §2.4). The service / launcher loads `host.env` into the process
//! environment before the host starts, and **for the knobs captured here the environment is constant for the
//! process lifetime**, so a lazily-parsed global is equivalent to "parsed once at startup".
//!
//! **Goal-1 stages 1-2**: stage 1 stood this up; stage 2 migrated the
//! genuinely-constant operator/dispatch knobs onto it (the dispatch-disagreement bug class:
//! `encoder_pref`, `render_adapter`, plus the plan-named `ten_bit`/`four_four_four` and the multi-site
//! `perf`/`compositor`/
//! `video_source`/`gamepad`). `SessionPlan` (stage 3) consumes it as the single owner of the
//! capture/topology/encoder decision.
//!
//! **What is deliberately NOT here (and must stay a live `env::var` read):**
//! - **Runtime-mutated session vars.** On Linux, `crate::vdisplay::apply_session_env` rewrites the process
//!   env on *every connect* so one host follows a Bazzite box across Gaming↔Desktop: `WAYLAND_DISPLAY`,
//!   `XDG_CURRENT_DESKTOP`, `XDG_RUNTIME_DIR`, `DBUS_SESSION_BUS_ADDRESS`, and the *derived* `SLIPSTREAM_*`
//!   vars `INPUT_BACKEND`, `GAMESCOPE_SESSION`/`GAMESCOPE_NODE`, `KWIN_VIRTUAL_PRIMARY`,
//!   `MUTTER_VIRTUAL_PRIMARY`, `FORCE_SHM` (+ `GAMESCOPE_APP` on the launch path). Parsing these once would
//!   freeze them at startup and silently break session-following — they are NOT constant.
//! - **Single-use local tuning** read exactly where it is used (no resolve-once benefit, and a parse with a
//!   call-site-local default/clamp): e.g. `FEC_PCT` (two *different* semantics — GameStream default-20 vs
//!   slipstream/1 `Option`/clamp-90), `VIDEO_DROP`, `VBV_FRAMES`, `SPLIT_ENCODE`, `PACE_BURST_KB`, the
//!   capture timing knobs, the `*_LIVE` test gates.
//! - **Path / genuinely-dynamic reads**: the config-dir resolution, `PATH` executable search, the
//!   env-forward-to-child loop, `SLIPSTREAM_MGMT_TOKEN`, `SLIPSTREAM_HOST_CMD`, `SLIPSTREAM_RENDER_NODE`.
//!
#![forbid(unsafe_code)]

mod config;
mod env;

use std::sync::OnceLock;

pub use config::{HostConfig, LatencyProfile, NetworkPolicy, PerformanceProfile, TEN_BIT_ENV};
pub use env::{default_on_gate, env_on, parse_env_on};

/// The process-wide host configuration, parsed once on first access.
pub fn config() -> &'static HostConfig {
    static CFG: OnceLock<HostConfig> = OnceLock::new();
    CFG.get_or_init(HostConfig::from_env)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(max_fps: Option<u32>) -> HostConfig {
        HostConfig {
            max_fps,
            ..Default::default()
        }
    }

    #[test]
    fn game_fps_caps_only_above_the_limit() {
        // Unset: every session rate passes through untouched — the default, and every existing
        // host. The game keeps rendering at the session's rate, exactly as it always did.
        for hz in [24, 30, 60, 120, 144, 240] {
            assert_eq!(cfg(None).game_fps(hz), hz);
        }
        // Set: capped above, exact at, untouched below. A session BELOW the limit keeps its own
        // rate — the knob is a ceiling on the game, not a target to render up to.
        let c = cfg(Some(60));
        assert_eq!(c.game_fps(120), 60);
        assert_eq!(c.game_fps(60), 60);
        assert_eq!(c.game_fps(30), 30);
        // An invalid rate stays invalid rather than being laundered into a real one.
        assert_eq!(c.game_fps(0), 0);
    }

    #[test]
    fn performance_profile_parses_exactly_low_latency() {
        assert_eq!(
            PerformanceProfile::parse("low_latency"),
            PerformanceProfile::LowLatency
        );
        assert_eq!(
            PerformanceProfile::parse("Low_Latency"),
            PerformanceProfile::LowLatency
        );
        assert_eq!(
            PerformanceProfile::parse(" low_latency "),
            PerformanceProfile::LowLatency
        );
        for v in ["", "off", "balanced", "warp"] {
            assert_eq!(PerformanceProfile::parse(v), PerformanceProfile::Off);
        }
    }

    #[test]
    fn latency_profile_parses_exactly_low_latency() {
        assert_eq!(
            LatencyProfile::parse("low_latency"),
            LatencyProfile::LowLatency
        );
        for v in ["", "balanced", "high"] {
            assert_eq!(LatencyProfile::parse(v), LatencyProfile::Balanced);
        }
    }

    #[test]
    fn network_policy_parses_lan_wan_auto() {
        assert_eq!(NetworkPolicy::parse("lan"), NetworkPolicy::Lan);
        assert_eq!(NetworkPolicy::parse("WAN"), NetworkPolicy::Wan);
        for v in ["", "auto", "fast", "0"] {
            assert_eq!(NetworkPolicy::parse(v), NetworkPolicy::Auto);
        }
    }

    #[test]
    fn ten_bit_env_key_is_slipstream_10bit() {
        // Runtime `HostConfig::from_env` must read this key (not SLIPSTREAM_TEN_BIT).
        assert_eq!(TEN_BIT_ENV, "SLIPSTREAM_10BIT");
    }

    #[test]
    fn default_on_policy_gates_match_runtime_ten_bit_444_chacha_hdr() {
        // Unset → on (the shipped default for SLIPSTREAM_10BIT / 444 / CHACHA20 / GAMESCOPE_HDR).
        assert!(default_on_gate(None));
        assert!(default_on_gate(Some("1")));
        assert!(default_on_gate(Some("true")));
        assert!(default_on_gate(Some(" yes ")));
        for off in ["0", "false", "off", "no", "OFF", " 0 "] {
            assert!(!default_on_gate(Some(off)), "expected off for {off:?}");
        }
    }

    #[test]
    fn parse_env_on_distinguishes_unset_from_off() {
        assert_eq!(parse_env_on(None), None);
        assert_eq!(parse_env_on(Some("0")), Some(false));
        assert_eq!(parse_env_on(Some("1")), Some(true));
    }
}
