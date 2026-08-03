//! Opt-in per-frame latency artifact (`SLIPSTREAM_CLIENT_ARTIFACT=<path>`): one JSONL record
//! per received frame — receive → decode → present → display stage stamps, present-wait
//! capability, clock-sync state, and drop counters. Unset env var = no file, zero hot-path
//! cost beyond the stamps the session already keeps.
//!
//! Schema v1, kind `client_frame` (fixed field order — bench scripts match on it). The writer
//! is self-contained (hand-rolled JSON; the probe tree has no serde) so other clients can lift
//! the module verbatim.

use std::io::Write;

/// Presenter present-wait capability snapshot (`VK_KHR_present_wait`), reported per record so
/// bench scripts can correlate on-glass timing support with the driver. The probe client has
/// no Vulkan device and passes [`PresentWaitInfo::unavailable`]; GUI clients wire the
/// ss-presenter probe (`ss_presenter::vk::present_wait_capabilities`).
#[derive(Clone, Debug)]
pub struct PresentWaitInfo {
    /// True when the presenter's device enables `VK_KHR_present_id` + `VK_KHR_present_wait`.
    pub available: bool,
    /// Physical-device driver name (`vkGetPhysicalDeviceProperties::deviceName`); empty when
    /// no device was reachable.
    pub driver: String,
}

impl PresentWaitInfo {
    /// The no-Vulkan default: the client never reached a presenter device.
    pub fn unavailable() -> PresentWaitInfo {
        PresentWaitInfo {
            available: false,
            driver: String::new(),
        }
    }
}

/// One `client_frame` record (schema v1). Fields the probe cannot measure today stay 0/false
/// until later telemetry phases fill them.
pub struct FrameRecord {
    pub frame_id: u32,
    pub pts_ns: u64,
    pub received_ns: u64,
    pub decoded_ns: u64,
    pub decode_queue_displacement: u64,
    pub presenter_queue_displacement: u64,
    pub displayed_ns: u64,
    pub display_timing_valid: bool,
    pub present_mode: String,
    pub clock_offset_ns: i64,
    pub best_rtt_us: u64,
    pub clock_uncertainty_us: u64,
    pub resync_age_us: u64,
    pub drops_network: u64,
    pub drops_decode: u64,
    pub drops_presenter: u64,
    pub drops_display: u64,
}

/// The opt-in JSONL writer. Created via [`ClientArtifact::open`]; a write error disables the
/// artifact (best-effort — it must never affect the stream).
pub struct ClientArtifact {
    out: std::io::BufWriter<std::fs::File>,
    present_wait: PresentWaitInfo,
    failed: bool,
}

impl ClientArtifact {
    /// Open the artifact at `SLIPSTREAM_CLIENT_ARTIFACT` when set; `None` otherwise. An
    /// unreadable path warns and degrades to `None` — the env var is opt-in and a bad path
    /// must never fail the stream. `present_wait` feeds the per-record `present_wait_*`
    /// fields (the probe client has no Vulkan device and passes
    /// [`PresentWaitInfo::unavailable`]).
    pub fn open(present_wait: PresentWaitInfo) -> Option<ClientArtifact> {
        let path = std::env::var("SLIPSTREAM_CLIENT_ARTIFACT").ok()?;
        let file = match std::fs::File::create(&path) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!(
                    path = %path,
                    error = %e,
                    "SLIPSTREAM_CLIENT_ARTIFACT: cannot open artifact — disabled"
                );
                return None;
            }
        };
        Some(ClientArtifact {
            out: std::io::BufWriter::new(file),
            present_wait,
            failed: false,
        })
    }

    /// Emit one JSONL record for a received frame.
    pub fn record_frame(&mut self, r: &FrameRecord) {
        if self.failed {
            return;
        }
        let pw = &self.present_wait;
        let line = format!(
            "{{\"v\":1,\"kind\":\"client_frame\",\"frame_id\":{},\"pts_ns\":{},\"received_ns\":{},\
             \"decoded_ns\":{},\"decode_queue_displacement\":{},\"presenter_queue_displacement\":{},\
             \"displayed_ns\":{},\"display_timing_valid\":{},\"present_mode\":\"{}\",\
             \"present_wait_available\":{},\"present_wait_driver\":\"{}\",\"clock_offset_ns\":{},\
             \"best_rtt_us\":{},\"clock_uncertainty_us\":{},\"resync_age_us\":{},\"drops_network\":{},\
             \"drops_decode\":{},\"drops_presenter\":{},\"drops_display\":{}}}",
            r.frame_id,
            r.pts_ns,
            r.received_ns,
            r.decoded_ns,
            r.decode_queue_displacement,
            r.presenter_queue_displacement,
            r.displayed_ns,
            r.display_timing_valid,
            json_escape(&r.present_mode),
            pw.available,
            json_escape(&pw.driver),
            r.clock_offset_ns,
            r.best_rtt_us,
            r.clock_uncertainty_us,
            r.resync_age_us,
            r.drops_network,
            r.drops_decode,
            r.drops_presenter,
            r.drops_display,
        );
        if let Err(e) = writeln!(self.out, "{line}") {
            tracing::warn!(error = %e, "SLIPSTREAM_CLIENT_ARTIFACT: write failed — disabled");
            self.failed = true;
        }
    }
}

impl Drop for ClientArtifact {
    fn drop(&mut self) {
        let _ = self.out.flush();
    }
}

/// JSON-escape a string field (quotes/backslashes/control chars — driver names and present
/// modes come from the OS/driver and can contain anything).
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}
