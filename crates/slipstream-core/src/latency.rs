//! Opt-in per-frame latency telemetry for the host streaming path: one machine-readable JSONL
//! record per captured video frame (the measurement contract the later latency phases build on).
//! Disabled unless the `SLIPSTREAM_LATENCY_ARTIFACT` env var names an output file; when set, the
//! native send thread appends one JSON object per line, flushed after every record so a crash
//! never loses the frames that already left the socket.
//!
//! Phase 1a coverage: the pipeline's existing delivery anchors plus the new send-side stamps
//! (enqueue → dequeue → first/last packet out). The capture-stage fields
//! (`cap_cb_entry_ns` … `convert_end_ns`) and the failure flags are declared up front for
//! schema stability and emit `0`/`false` until the phases that measure them land. Every field
//! follows the same 0-semantics: `0` (or `false`) = unavailable / not measured.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use zerocopy::FromBytes;

/// One captured frame's host-pipeline timings — the payload of one JSONL record. All timestamps
/// are wall-clock nanoseconds since the UNIX epoch ([`now_ns`]); `0` = unavailable / not
/// measured yet in this phase. `capture_backend` is the capturer's static backend name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameTimings {
    /// Capture callback entry (host-side ring read start). Not measured in Phase 1a.
    pub cap_cb_entry_ns: u64,
    /// Frame producer (capture backend) completion. Not measured in Phase 1a.
    pub producer_ns: u64,
    /// Import-fence wait start. Not measured in Phase 1a.
    pub fence_wait_start_ns: u64,
    /// Import-fence wait end. Not measured in Phase 1a.
    pub fence_wait_end_ns: u64,
    /// GPU import (dmabuf/DXGI texture) done. Not measured in Phase 1a.
    pub import_end_ns: u64,
    /// Colour convert done. Not measured in Phase 1a.
    pub convert_end_ns: u64,
    /// CPU row-copy (de-pad) completion — split from `convert_end_ns` so the copy and the
    /// conversion are recorded separately on the CPU fallback path (Phase 3).
    pub depad_end_ns: u64,
    /// Cursor composition completion (Phase 3).
    pub cursor_end_ns: u64,
    /// Producer metadata flags (`SPA_META_Header` flags; 0 = the producer supplied none).
    pub source_meta_flags: u32,
    /// Producer timestamp when the compositor stamps one (`SPA_META_Header.pts`, its clock
    /// domain — meaningful only relative to the graph latency recorded in the session header).
    pub source_meta_pts_ns: u64,
    /// Capture delivery (the frame's wire pts anchor): the stamp `CapturedFrame.pts_ns` carried.
    pub publish_ns: u64,
    /// Encoder submit (the capture/encode loop's `submit_indexed` entry stamp).
    pub encode_submit_ns: u64,
    /// First encoded packet (AU/slice chunk) polled off the encoder.
    pub first_enc_pkt_ns: u64,
    /// Last encoded packet polled off the encoder (== `first_enc_pkt_ns` on a single-AU poll).
    pub last_enc_pkt_ns: u64,
    /// The AU handed to the send channel (right before `send_msg_until_stop`).
    pub enqueue_ns: u64,
    /// The AU pulled off the send channel by the send thread (recv_timeout Ok).
    pub dequeue_ns: u64,
    /// First wire packet handed to the socket (pacing-loop first send call).
    pub first_sent_ns: u64,
    /// Last wire packet handed to the socket (pacing-loop last send call).
    pub last_sent_ns: u64,
    /// The wire `frame_index` this AU was sealed with.
    pub frame_id: u32,
    /// The wire pts (== `publish_ns` for a fresh frame).
    pub pts_ns: u64,
    /// FEC/parity packets among `total_packets`.
    pub fec_packets: u32,
    /// Packets actually handed to the send path (post loss-injection).
    pub total_packets: u32,
    /// The frame's send spread (`PaceStat::spread_us`).
    pub pace_spread_us: u32,
    /// The kernel's send-queue occupancy estimate at dequeue time (bytes; `SIOCOUTQ`).
    pub kernel_queue_bytes: u64,
    /// The capturer's backend name (e.g. "pipewire-portal", "kms").
    pub capture_backend: &'static str,
    /// How this frame's capture was sampled: `"arrival_wait"` (the loop slept on the producer's
    /// wake edge) or `"fixed_tick"` (the legacy fixed-cadence sampling, or an arrival-incapable
    /// backend). Records the GameStream arrival-driven rollout phase (Phase 2).
    pub sampling: &'static str,
    /// The streaming protocol that produced the record: `"slipstream1"` or `"gamestream"`.
    pub transport: &'static str,
    /// The frame was a re-encoded hold (no fresh capture). Not set in Phase 1a (always false).
    pub stale: bool,
    /// The send channel was ever Full for this frame (send_msg_until_stop retried).
    pub backpressure: bool,
    /// A capture fence timed out. Not measured in Phase 1a (always false).
    pub fence_timeout: bool,
    /// The frame was dropped for recovery. Not measured in Phase 1a (always false).
    pub recovery_drop: bool,
}

impl FrameTimings {
    /// A zeroed record for one frame from `capture_backend` (0/false fields = unavailable).
    pub fn new(capture_backend: &'static str) -> FrameTimings {
        FrameTimings {
            capture_backend,
            sampling: "fixed_tick",
            transport: "slipstream1",
            ..FrameTimings::default()
        }
    }
}

/// Append-only JSONL latency artifact. One JSON object per line; the buffered writer is flushed
/// after every record so a crash leaves the frames that already left the socket readable. The
/// file is opened in append mode — a resumed stream extends the same artifact.
pub struct LatencyArtifact {
    w: BufWriter<File>,
}

impl LatencyArtifact {
    /// Open (creating if missing) the artifact at `path`, appending from the current end.
    pub fn open(path: impl AsRef<Path>) -> io::Result<LatencyArtifact> {
        let f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(LatencyArtifact {
            w: BufWriter::new(f),
        })
    }

    /// The `SLIPSTREAM_LATENCY_ARTIFACT` gate: `Some` when the env var names a path that opened,
    /// `None` when it is unset/empty — or unopenable (warned once, the stream continues without
    /// an artifact).
    pub fn from_env() -> Option<LatencyArtifact> {
        let path = std::env::var("SLIPSTREAM_LATENCY_ARTIFACT").ok()?;
        if path.is_empty() {
            return None;
        }
        match Self::open(&path) {
            Ok(a) => Some(a),
            Err(e) => {
                tracing::warn!(
                    path,
                    error = %e,
                    "SLIPSTREAM_LATENCY_ARTIFACT: cannot open — latency artifact disabled"
                );
                None
            }
        }
    }

    /// One session-header record (`kind: "host_header"`): the artifact schema version plus the
    /// session's identity. Written once when the artifact arms, before the first frame record.
    pub fn write_header(
        &mut self,
        capture_backend: &str,
        codec: &str,
        client: &str,
        width: u32,
        height: u32,
        fps: u32,
    ) -> io::Result<()> {
        let w = &mut self.w;
        write!(w, "{{\"kind\":\"host_header\",\"v\":1,\"capture_backend\":")?;
        write_json_str(w, capture_backend)?;
        write!(w, ",\"codec\":")?;
        write_json_str(w, codec)?;
        write!(w, ",\"client\":")?;
        write_json_str(w, client)?;
        writeln!(w, ",\"width\":{width},\"height\":{height},\"fps\":{fps}}}")?;
        w.flush()
    }

    /// One `kind: "host_frame"` record per captured frame, keys exactly the
    /// [`FrameTimings`] field names. Flushed per record (crash visibility).
    pub fn write_frame(&mut self, t: &FrameTimings) -> io::Result<()> {
        let w = &mut self.w;
        write!(
            w,
            "{{\"kind\":\"host_frame\",\"v\":1,\"cap_cb_entry_ns\":{},\"producer_ns\":{},\"fence_wait_start_ns\":{},\"fence_wait_end_ns\":{},\"import_end_ns\":{},\"convert_end_ns\":{},\"depad_end_ns\":{},\"cursor_end_ns\":{},\"source_meta_flags\":{},\"source_meta_pts_ns\":{},\"publish_ns\":{},\"encode_submit_ns\":{},\"first_enc_pkt_ns\":{},\"last_enc_pkt_ns\":{},\"enqueue_ns\":{},\"dequeue_ns\":{},\"first_sent_ns\":{},\"last_sent_ns\":{},\"frame_id\":{},\"pts_ns\":{},\"fec_packets\":{},\"total_packets\":{},\"pace_spread_us\":{},\"kernel_queue_bytes\":{},\"capture_backend\":",
            t.cap_cb_entry_ns,
            t.producer_ns,
            t.fence_wait_start_ns,
            t.fence_wait_end_ns,
            t.import_end_ns,
            t.convert_end_ns,
            t.depad_end_ns,
            t.cursor_end_ns,
            t.source_meta_flags,
            t.source_meta_pts_ns,
            t.publish_ns,
            t.encode_submit_ns,
            t.first_enc_pkt_ns,
            t.last_enc_pkt_ns,
            t.enqueue_ns,
            t.dequeue_ns,
            t.first_sent_ns,
            t.last_sent_ns,
            t.frame_id,
            t.pts_ns,
            t.fec_packets,
            t.total_packets,
            t.pace_spread_us,
            t.kernel_queue_bytes,
        )?;
        write_json_str(w, t.capture_backend)?;
        write!(w, ",\"sampling\":")?;
        write_json_str(w, t.sampling)?;
        write!(w, ",\"transport\":")?;
        write_json_str(w, t.transport)?;
        writeln!(
            w,
            ",\"stale\":{},\"backpressure\":{},\"fence_timeout\":{},\"recovery_drop\":{}}}",
            t.stale, t.backpressure, t.fence_timeout, t.recovery_drop
        )?;
        w.flush()
    }

    /// Flush the buffered records to disk (no-op right after a record — already flushed).
    pub fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}

impl Drop for LatencyArtifact {
    fn drop(&mut self) {
        let _ = self.w.flush();
    }
}

/// Wall-clock nanoseconds since the UNIX epoch — the artifact's clock (same source as the wire
/// pts). `0` on a clock failure (before-epoch SystemTime).
pub fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Whether the `SLIPSTREAM_LATENCY_ARTIFACT` gate is set (mirrors [`LatencyArtifact::from_env`]
/// without opening a file). The host uses it once per session to skip the per-frame stamp work
/// entirely when the artifact is off — with it unset, the only added hot-path cost is the two
/// send-stamp clock reads in the pacing loop.
pub fn latency_artifact_enabled() -> bool {
    std::env::var("SLIPSTREAM_LATENCY_ARTIFACT")
        .is_ok_and(|p| !p.is_empty())
}

/// Count the FEC/parity packets among sealed wire packets: a packet is parity when its
/// [`crate::packet::PacketHeader`] shard index sits at/after the block's data-shard count
/// (the packetizer emits parity shards at `data_shards + r`). Unknown-shaped packets (too
/// short for a header) count as data. The host calls this only when the artifact is armed.
pub fn fec_packet_count(packets: &[&[u8]]) -> u32 {
    packets
        .iter()
        .filter(|p| {
            crate::packet::PacketHeader::read_from_bytes(
                p.get(..crate::packet::HEADER_LEN).unwrap_or(&[]),
            )
            .is_ok_and(|h| h.shard_index >= h.data_shards)
        })
        .count() as u32
}

/// Write `s` as a JSON string with the minimal escaping a fixed internal schema needs.
fn write_json_str(w: &mut impl Write, s: &str) -> io::Result<()> {
    write!(w, "\"")?;
    for c in s.chars() {
        match c {
            '"' => write!(w, "\\\"")?,
            '\\' => write!(w, "\\\\")?,
            '\n' => write!(w, "\\n")?,
            '\r' => write!(w, "\\r")?,
            '\t' => write!(w, "\\t")?,
            c if (c as u32) < 0x20 => write!(w, "\\u{:04x}", c as u32)?,
            c => write!(w, "{c}")?,
        }
    }
    write!(w, "\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn test_path(tag: &str) -> std::path::PathBuf {
        let unique = now_ns();
        std::env::temp_dir().join(format!("slipstream-latency-{tag}-{unique}.jsonl"))
    }

    fn read_all(path: &std::path::Path) -> String {
        let mut s = String::new();
        std::fs::File::open(path)
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        s
    }

    /// The frame record's fixed key set, in emission order (kind/v first, then the field list).
    const FRAME_KEYS: [&str; 33] = [
        "\"kind\":\"host_frame\"",
        "\"v\":1",
        "\"cap_cb_entry_ns\":",
        "\"producer_ns\":",
        "\"fence_wait_start_ns\":",
        "\"fence_wait_end_ns\":",
        "\"import_end_ns\":",
        "\"convert_end_ns\":",
        "\"depad_end_ns\":",
        "\"cursor_end_ns\":",
        "\"source_meta_flags\":",
        "\"source_meta_pts_ns\":",
        "\"publish_ns\":",
        "\"encode_submit_ns\":",
        "\"first_enc_pkt_ns\":",
        "\"last_enc_pkt_ns\":",
        "\"enqueue_ns\":",
        "\"dequeue_ns\":",
        "\"first_sent_ns\":",
        "\"last_sent_ns\":",
        "\"frame_id\":",
        "\"pts_ns\":",
        "\"fec_packets\":",
        "\"total_packets\":",
        "\"pace_spread_us\":",
        "\"kernel_queue_bytes\":",
        "\"capture_backend\":",
        "\"sampling\":",
        "\"transport\":",
        "\"stale\":",
        "\"backpressure\":",
        "\"fence_timeout\":",
        "\"recovery_drop\":",
    ];

    #[test]
    fn frame_record_emits_all_fields_with_zero_semantics() {
        let path = test_path("shape");
        let mut a = LatencyArtifact::open(&path).unwrap();
        let mut t = FrameTimings::new("pipewire-portal");
        t.frame_id = 42;
        t.pts_ns = 123_456_789;
        t.publish_ns = 111;
        t.encode_submit_ns = 222;
        t.first_enc_pkt_ns = 333;
        t.last_enc_pkt_ns = 333;
        t.enqueue_ns = 444;
        t.dequeue_ns = 555;
        t.first_sent_ns = 666;
        t.last_sent_ns = 777;
        t.pace_spread_us = 88;
        t.fec_packets = 2;
        t.total_packets = 10;
        t.backpressure = true;
        a.write_frame(&t).unwrap();
        drop(a);

        let out = read_all(&path);
        let _ = std::fs::remove_file(&path);
        // One line, one JSON object, no trailing junk.
        let line = out.trim_end_matches('\n');
        assert!(!line.contains('\n'), "one record per line: {out:?}");
        assert!(line.starts_with('{') && line.ends_with('}'), "object shape: {line}");
        // Every key present, in order.
        let mut pos = 0;
        for k in FRAME_KEYS {
            let at = line[pos..].find(k).unwrap_or_else(|| panic!("missing {k} in {line}"));
            pos += at + k.len();
        }
        // Field values round-trip; unset fields emit 0/false (the 0-semantics contract).
        for (k, want) in [
            ("\"frame_id\":", "42"),
            ("\"pts_ns\":", "123456789"),
            ("\"publish_ns\":", "111"),
            ("\"encode_submit_ns\":", "222"),
            ("\"first_enc_pkt_ns\":", "333"),
            ("\"last_enc_pkt_ns\":", "333"),
            ("\"enqueue_ns\":", "444"),
            ("\"dequeue_ns\":", "555"),
            ("\"first_sent_ns\":", "666"),
            ("\"last_sent_ns\":", "777"),
            ("\"pace_spread_us\":", "88"),
            ("\"fec_packets\":", "2"),
            ("\"total_packets\":", "10"),
            ("\"backpressure\":", "true"),
        ] {
            let at = line.find(k).unwrap();
            let rest = &line[at + k.len()..];
            let val: String = rest.chars().take_while(|c| *c != ',' && *c != '}').collect();
            assert_eq!(val, want, "{k} in {line}");
        }
        for (k, v) in [
            ("\"cap_cb_entry_ns\":", "0"),
            ("\"producer_ns\":", "0"),
            ("\"fence_wait_start_ns\":", "0"),
            ("\"fence_wait_end_ns\":", "0"),
            ("\"import_end_ns\":", "0"),
            ("\"convert_end_ns\":", "0"),
            ("\"depad_end_ns\":", "0"),
            ("\"cursor_end_ns\":", "0"),
            ("\"source_meta_flags\":", "0"),
            ("\"source_meta_pts_ns\":", "0"),
            ("\"kernel_queue_bytes\":", "0"),
            ("\"stale\":", "false"),
            ("\"fence_timeout\":", "false"),
            ("\"recovery_drop\":", "false"),
        ] {
            assert!(
                line.contains(k) && line[line.find(k).unwrap() + k.len()..].starts_with(v),
                "{k} should be {v} in {line}"
            );
        }
        assert!(line.contains("\"capture_backend\":\"pipewire-portal\""));
        assert!(
            line.contains("\"sampling\":\"fixed_tick\""),
            "default sampling marker: {line}"
        );
        assert!(
            line.contains("\"transport\":\"slipstream1\""),
            "default transport marker: {line}"
        );
    }

    #[test]
    fn sampling_and_transport_markers_round_trip() {
        let path = test_path("markers");
        let mut a = LatencyArtifact::open(&path).unwrap();
        let mut t = FrameTimings::new("kms");
        t.sampling = "arrival_wait";
        t.transport = "gamestream";
        a.write_frame(&t).unwrap();
        drop(a);
        let out = read_all(&path);
        let _ = std::fs::remove_file(&path);
        assert!(out.contains("\"sampling\":\"arrival_wait\""));
        assert!(out.contains("\"transport\":\"gamestream\""));
    }

    #[test]
    fn appends_one_record_per_line() {
        let path = test_path("append");
        let mut a = LatencyArtifact::open(&path).unwrap();
        a.write_frame(&FrameTimings::new("synthetic")).unwrap();
        a.write_frame(&FrameTimings::new("synthetic")).unwrap();
        drop(a);
        let out = read_all(&path);
        let _ = std::fs::remove_file(&path);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2, "two frames → two lines: {out:?}");
        assert!(lines[0].starts_with("{\"kind\":\"host_frame\""));
        assert!(lines[1].starts_with("{\"kind\":\"host_frame\""));
    }

    #[test]
    fn header_precedes_frame_records() {
        let path = test_path("header");
        let mut a = LatencyArtifact::open(&path).unwrap();
        a.write_header("pipewire-portal", "hevc", "loopback-client", 3840, 2160, 60)
            .unwrap();
        a.write_frame(&FrameTimings::new("pipewire-portal")).unwrap();
        drop(a);
        let out = read_all(&path);
        let _ = std::fs::remove_file(&path);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("{\"kind\":\"host_header\",\"v\":1"));
        assert!(lines[0].contains("\"codec\":\"hevc\""));
        assert!(lines[0].contains("\"client\":\"loopback-client\""));
        assert!(lines[0].contains("\"width\":3840,\"height\":2160,\"fps\":60}"));
        assert!(lines[1].starts_with("{\"kind\":\"host_frame\""));
    }

    #[test]
    fn from_env_gates_on_the_env_var() {
        let path = test_path("env");
        unsafe {
            std::env::set_var("SLIPSTREAM_LATENCY_ARTIFACT", &path);
        }
        let mut a = LatencyArtifact::from_env().expect("var set to a writable path → artifact");
        a.write_frame(&FrameTimings::new("synthetic")).unwrap();
        drop(a);
        unsafe {
            std::env::remove_var("SLIPSTREAM_LATENCY_ARTIFACT");
        }
        assert!(
            LatencyArtifact::from_env().is_none(),
            "unset → disabled"
        );
        let out = read_all(&path);
        let _ = std::fs::remove_file(&path);
        assert!(out.starts_with("{\"kind\":\"host_frame\""));

        unsafe {
            std::env::set_var("SLIPSTREAM_LATENCY_ARTIFACT", "");
        }
        assert!(
            LatencyArtifact::from_env().is_none(),
            "empty path → disabled"
        );
        unsafe {
            std::env::remove_var("SLIPSTREAM_LATENCY_ARTIFACT");
        }
    }

    #[test]
    fn now_ns_is_a_recent_epoch_clock() {
        assert!(now_ns() > 1_600_000_000_000_000_000, "past 2020-09-13");
        assert!(now_ns() > 0);
    }
}
