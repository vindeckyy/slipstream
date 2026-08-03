//! The unified latency policy every Linux encoder backend applies (latency Phase 4): one
//! internal [`LatencyProfile`] resolved once per process and threaded through the libav,
//! direct-NVENC, and Vulkan Video config authors, so a `LowLatency` session means the SAME
//! encoder contract on every vendor. The profile is internal plumbing for now; the host
//! configuration surface that selects it lands with the performance profile (Phase 7).
//!
//! `Balanced` is the existing behavior — the crate already ships the low-latency RC contract
//! (CBR, zero B-frames, no lookahead, ~1-frame VBV) as its default; `LowLatency` pins the
//! stronger constraints (1-frame VBV ceiling, depth-one encoder input ring, no CPU colour
//! conversion preference, capability-gated sub-frame output) and forbids deepening the pipeline
//! to hide a slow backend.

use std::sync::OnceLock;

/// The latency policy for one encode session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LatencyProfile {
    /// The existing default contract: CBR, zero B-frames, no lookahead, ~1-frame VBV (env-tuned).
    #[default]
    Balanced,
    /// The pinned low-latency contract (see the module docs and [`ProfileConfig`]).
    LowLatency,
}

impl LatencyProfile {
    /// The process-wide profile: `SLIPSTREAM_LATENCY_PROFILE=low_latency` opts in; anything
    /// else (including unset) is `Balanced`. Parsed once.
    pub fn current() -> LatencyProfile {
        *PROFILE.get_or_init(Self::from_env)
    }

    /// The pure env parse [`current`](Self::current) caches — split out so tests can exercise
    /// the parsing without racing the process-wide `OnceLock`.
    pub(crate) fn from_env() -> LatencyProfile {
        if std::env::var("SLIPSTREAM_LATENCY_PROFILE").as_deref() == Ok("low_latency") {
            LatencyProfile::LowLatency
        } else {
            LatencyProfile::Balanced
        }
    }

    /// The resolved encoder contract for this profile.
    pub fn config(self) -> ProfileConfig {
        ProfileConfig {
            zero_b_frames: true,
            no_lookahead: true,
            // `LowLatency` pins the VBV to one frame; `Balanced` keeps the env-tuned default
            // (also 1 frame unless `SLIPSTREAM_VBV_FRAMES` says otherwise).
            vbv_frames: match self {
                LatencyProfile::LowLatency => 1.0,
                LatencyProfile::Balanced => crate::vbv_frames_env(),
            },
            // `LowLatency` forbids the encoder input ring from hiding backend slowness behind
            // extra depth; `Balanced` leaves the capturer's natural depth in place.
            max_input_depth: match self {
                LatencyProfile::LowLatency => 1,
                LatencyProfile::Balanced => usize::MAX,
            },
            // `LowLatency` uses sub-frame output only when the encoder explicitly advertises it
            // (ordered slice output); `Balanced` may use whatever the backend supports.
            subframe_capability_gated: self == LatencyProfile::LowLatency,
            // `LowLatency` prefers surfaces the hardware can consume directly (no CPU colour
            // conversion when a zero-copy path exists); `Balanced` accepts the capture path's
            // own choice. Enforced by the capture negotiation (Phase 3 diagnostics surface it).
            prefer_zero_copy: self == LatencyProfile::LowLatency,
        }
    }

    /// A dropped inter-frame reference requires a recovery point (RFI or IDR) before dependent
    /// frames resume — true for every profile (both planes already re-anchor on drop; this is
    /// the contract's name, and Phase 5's `recovery_required` session state carries it).
    pub fn recovery_after_reference_drop(self) -> bool {
        true
    }
}

/// The concrete encoder contract a profile demands. Every field is a constraint the encoder
/// config authors must honor; fields that carry no vendor meaning for a given backend are no-ops
/// there by construction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProfileConfig {
    /// Zero B-frames (no reordering).
    pub zero_b_frames: bool,
    /// No look-ahead (no hidden frame queue).
    pub no_lookahead: bool,
    /// The VBV/HRD window in frames.
    pub vbv_frames: f64,
    /// The encoder input-ring depth ceiling (1 = never pipeline inside the backend).
    pub max_input_depth: usize,
    /// Sub-frame (slice) output is used only when the backend advertises ordered sub-frame
    /// capability — never assumed.
    pub subframe_capability_gated: bool,
    /// Prefer surfaces the hardware path can consume directly over CPU colour conversion.
    pub prefer_zero_copy: bool,
}

static PROFILE: OnceLock<LatencyProfile> = OnceLock::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_balanced() {
        unsafe {
            std::env::remove_var("SLIPSTREAM_LATENCY_PROFILE");
        }
        assert_eq!(LatencyProfile::from_env(), LatencyProfile::Balanced);
    }

    #[test]
    fn low_latency_pins_the_contract() {
        unsafe {
            std::env::set_var("SLIPSTREAM_LATENCY_PROFILE", "low_latency");
        }
        let profile = LatencyProfile::from_env();
        let cfg = profile.config();
        assert_eq!(profile, LatencyProfile::LowLatency);
        assert!(cfg.zero_b_frames);
        assert!(cfg.no_lookahead);
        assert_eq!(cfg.vbv_frames, 1.0);
        assert_eq!(cfg.max_input_depth, 1);
        assert!(cfg.subframe_capability_gated);
        assert!(cfg.prefer_zero_copy);
        assert!(profile.recovery_after_reference_drop());
        unsafe {
            std::env::remove_var("SLIPSTREAM_LATENCY_PROFILE");
        }
    }

    #[test]
    fn balanced_keeps_the_existing_shape() {
        unsafe {
            std::env::remove_var("SLIPSTREAM_LATENCY_PROFILE");
            std::env::remove_var("SLIPSTREAM_VBV_FRAMES");
        }
        let cfg = LatencyProfile::from_env().config();
        assert!(cfg.zero_b_frames, "the existing contract already has B-frames off");
        assert!(cfg.no_lookahead);
        assert_eq!(cfg.vbv_frames, 1.0, "env-default VBV is one frame");
        assert_eq!(cfg.max_input_depth, usize::MAX, "balanced leaves depth alone");
        assert!(!cfg.subframe_capability_gated);
        assert!(!cfg.prefer_zero_copy);
    }

    #[test]
    fn unknown_profile_value_falls_back_to_balanced() {
        unsafe {
            std::env::set_var("SLIPSTREAM_LATENCY_PROFILE", "warp-speed");
        }
        assert_eq!(LatencyProfile::from_env(), LatencyProfile::Balanced);
        unsafe {
            std::env::remove_var("SLIPSTREAM_LATENCY_PROFILE");
        }
    }
}
