//! The native audio plane (plan §W1 — carved out of the [`super`] module): desktop capture → Opus
//! (48 kHz, 5 ms, CBR — the same tuning as the GameStream path) → `AUDIO_MAGIC` QUIC datagrams, at
//! the negotiated channel count. The encoder ([`NativeAudioEnc`]) and the capture/encode/send loop
//! ([`audio_thread`]) are gated to linux/windows (libopus + a real capturer); other targets get the
//! stub, so a dev build streams video-only rather than failing to compile.

use super::*;
use slipstream_core::fec::{generate_parity, AudioFecData, AUDIO_GROUP_LEN, AUDIO_MAX_PARITY};

/// Opus encoder for the native audio plane: a plain stereo encoder (the live-validated,
/// byte-identical path) or a libopus *multistream* encoder for 5.1/7.1, both behind one
/// `encode_float`. Surround uses the safe `opus::MSEncoder` (no `audiopus_sys`).
#[cfg(any(target_os = "linux", target_os = "windows"))]
enum NativeAudioEnc {
    Stereo(opus::Encoder),
    Surround(opus::MSEncoder),
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
impl NativeAudioEnc {
    /// Build the encoder for `channels` (2/6/8), hard-CBR + RESTRICTED_LOWDELAY like the
    /// GameStream path; bitrate from the shared layout table (stereo keeps the validated 128 kbps).
    fn new(channels: u8) -> Result<NativeAudioEnc, opus::Error> {
        if channels == 2 {
            let mut e = opus::Encoder::new(
                crate::audio::SAMPLE_RATE,
                opus::Channels::Stereo,
                opus::Application::LowDelay,
            )?;
            e.set_bitrate(opus::Bitrate::Bits(128_000)).ok();
            e.set_vbr(false).ok();
            Ok(NativeAudioEnc::Stereo(e))
        } else {
            let l = slipstream_core::audio::layout_for(channels, false);
            let mut e = opus::MSEncoder::new(
                crate::audio::SAMPLE_RATE,
                l.streams,
                l.coupled,
                l.mapping,
                opus::Application::LowDelay,
            )?;
            e.set_bitrate(opus::Bitrate::Bits(l.bitrate)).ok();
            e.set_vbr(false).ok();
            Ok(NativeAudioEnc::Surround(e))
        }
    }

    fn encode_float(&mut self, frame: &[f32], out: &mut [u8]) -> Result<usize, opus::Error> {
        match self {
            NativeAudioEnc::Stereo(e) => e.encode_float(frame, out),
            NativeAudioEnc::Surround(e) => e.encode_float(frame, out),
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
#[derive(Default)]
struct AudioPacer {
    next_send: Option<std::time::Instant>,
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
impl AudioPacer {
    const FRAME_INTERVAL: std::time::Duration = std::time::Duration::from_millis(5);

    fn wait_for_next(&mut self) {
        let scheduled = self.next_send.take();
        if let Some(deadline) = scheduled {
            let now = std::time::Instant::now();
            if deadline > now {
                std::thread::sleep(deadline - now);
            }
        }
        let now = std::time::Instant::now();
        self.next_send = Some(match scheduled {
            Some(deadline) => {
                let next = deadline + Self::FRAME_INTERVAL;
                if next > now {
                    next
                } else {
                    now + Self::FRAME_INTERVAL
                }
            }
            None => now + Self::FRAME_INTERVAL,
        });
    }
}

/// The audio thread: desktop capture → Opus (48 kHz, 5 ms, CBR — same tuning as the GameStream
/// path) → `AUDIO_MAGIC` datagrams, at the negotiated `channels` (2 stereo / 6 = 5.1 / 8 = 7.1,
/// canonical wire order FL FR FC LFE RL RR SL SR). QUIC already encrypts; no extra layer. The
/// capturer comes from (and returns to) the persistent slot — see [`AudioCapSlot`].
///
/// Latency plan Phase 8: the sink ring is capped by the measured transport state — 2 buffered
/// Opus frames on LAN, 3 on WAN — and stale audio is dropped during recovery instead of replayed
/// (an old frame is latency, not audio). Each Opus frame is one datagram (one frame per packet).
/// A periodic telemetry line records the negotiated quantum, ring occupancy, underruns, and
/// playout age — audio health is never inferred from the video latency statistic.
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub(super) fn audio_thread(
    conn: quinn::Connection,
    stop: Arc<AtomicBool>,
    audio_cap: AudioCapSlot,
    channels: u8,
    transport_policy: Arc<crate::transport_state::TransportPolicyShared>,
    audio_fec: bool,
) {
    use crate::audio::SAMPLE_RATE;
    const FRAME_MS: usize = 5;
    const SAMPLES_PER_FRAME: usize = SAMPLE_RATE as usize * FRAME_MS / 1000; // 240
                                                                             // Audio is latency-critical: raise this thread's priority (nice -10 on Linux) so a
                                                                             // CPU-saturating game can't deschedule capture → encode → send and starve the audio
                                                                             // pipeline (the same boost the capture/encode video threads get). Best-effort: silently
                                                                             // no-ops without CAP_SYS_NICE / RLIMIT_NICE.
    ss_frame::thread_qos::boost_thread_priority(true);
    // Phase 8 ring cap (in Opus frames): 2 on LAN, 3 on WAN — the standing-audio budget. The
    // cap is expanded for a larger PipeWire quantum so a capture burst is not truncated.
    const RING_CAP_LAN: usize = 2;
    const RING_CAP_WAN: usize = 3;
    let want = slipstream_core::audio::normalize_channels(channels);

    // Reuse the cached capturer ONLY when its channel count matches this session's; a stereo
    // capturer left by a prior session must not feed a 5.1/7.1 session (the encoder + the client's
    // decoder are sized for `want`, so a mismatched capturer would garble/desync the audio).
    // A FAILED first open does not end the session's audio: session start is peak endpoint churn
    // on Windows (the virtual-display attach and the wiring plan's own default-device flips race
    // the WASAPI activate — 0x80070002 mid-re-registration), so it enters the same
    // reopen-with-backoff loop a mid-session capture death does; audio then starts a few seconds
    // late instead of never.
    let capturer = match audio_cap.lock().unwrap().take() {
        Some(mut c) if c.channels() == want as u32 => {
            c.drain(); // discard audio captured between sessions (also re-claims routing)
            Some(c)
        }
        prev => {
            drop(prev); // wrong channel count (or none): clean teardown, open fresh at `want`
            match crate::audio::open_audio_capture(want as u32) {
                Ok(c) => Some(c),
                Err(e) => {
                    tracing::warn!(error = %format!("{e:#}"), "slipstream/1 audio failed to open — retrying in the background until it comes up");
                    None
                }
            }
        }
    };
    let mut enc = match NativeAudioEnc::new(want) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(error = %e, "opus encoder init failed — session continues without audio");
            if let Some(mut c) = capturer {
                c.idle(); // parked, not streaming — release the routing claim
                crate::audio::park_audio_capture(&audio_cap, c);
            }
            return;
        }
    };

    let frame_len = SAMPLES_PER_FRAME * want as usize;
    let mut acc: Vec<f32> = Vec::with_capacity(frame_len * 4);
    // Sized for the largest surround frame (7.1 HQ ≈ 1.3 KB at 5 ms); ample for normal quality.
    let mut opus_buf = vec![0u8; 4096];
    let mut seq: u32 = 0;
    // Optional linear gain for quiet capture sources (SLIPSTREAM_AUDIO_GAIN, default 1.0) —
    // the native-plane twin of the GameStream path's gain. Applied per frame before encode.
    let gain: f32 = std::env::var("SLIPSTREAM_AUDIO_GAIN")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|g: &f32| g.is_finite() && (0.0..=4.0).contains(g))
        .unwrap_or(1.0);
    // Phase 8: bounded ring of full Opus frames awaiting send (the state-dependent cap). When
    // the capturer delivers a burst, older frames are dropped (drop-oldest) so the standing
    // audio age stays inside the transport state's budget.
    let mut ring: std::collections::VecDeque<Vec<f32>> =
        std::collections::VecDeque::with_capacity(RING_CAP_WAN + 1);
    // Audio FEC (design/audio-resilience.md): when negotiated, buffer a whole group of
    // encoded frames and emit the group + its RS parity together. The group is the send-side
    // reorder window: data always lands before its parity (they're sent back-to-back), and a
    // lost 5 ms packet can be rebuilt by the client from the group's survivors + parity.
    // One group of 8 × 5 ms = 40 ms of added latency, only on the negotiated path.
    let mut fec: Option<FecSender> = audio_fec.then(|| FecSender::new(transport_policy.clone()));
    let mut pacer = AudioPacer::default();
    // Reopen-with-backoff: hold the capturer in an Option so a mid-session capture-thread death
    // (device unplug, daemon restart) — or a first open lost to session-start churn above —
    // reopens instead of muting the rest of a multi-hour session. A quiet sink is NOT a death —
    // `next_chunk` returns an empty chunk on its idle timeout — so only a genuine thread-ended
    // Err drops the capturer. Reopens are throttled by INJECTOR_REOPEN_BACKOFF. The Opus encoder
    // and the monotonic `seq` are kept across reopens (the client sees a gap, not a restart).
    let mut last_failed = capturer.is_none().then(std::time::Instant::now);
    let mut capturer = capturer;
    // A stuck Opus encoder would fail on every 5 ms frame (~200/s); power-of-two throttle the
    // warn so it can't flood stderr + the log ring while still surfacing that it's failing.
    let mut opus_encode_errs: u64 = 0;
    // Phase 8 telemetry accumulators + cadence.
    let underruns: u64 = 0;
    let mut overflow_dropped: u64 = 0;
    let mut last_telemetry = std::time::Instant::now();
    if capturer.is_some() {
        tracing::info!(
            channels = want,
            "slipstream/1 audio streaming (Opus 48 kHz, 5 ms datagrams)"
        );
    }
    'session: while !stop.load(Ordering::SeqCst) {
        if capturer.is_none() {
            if last_failed.is_some_and(|t| t.elapsed() < INJECTOR_REOPEN_BACKOFF) {
                std::thread::sleep(std::time::Duration::from_millis(200));
                continue;
            }
            match crate::audio::open_audio_capture(want as u32) {
                Ok(c) => {
                    tracing::info!("slipstream/1 audio capture reopened");
                    capturer = Some(c);
                    last_failed = None;
                    acc.clear(); // drop the partial frame straddling the gap
                    ring.clear(); // and any stale audio queued before the gap
                }
                Err(e) => {
                    tracing::debug!(error = %format!("{e:#}"), "audio reopen failed — will retry");
                    last_failed = Some(std::time::Instant::now());
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    continue;
                }
            }
        }
        let chunk = match capturer.as_mut().unwrap().next_chunk() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %format!("{e:#}"), "audio capture lost — reopening");
                capturer = None;
                last_failed = Some(std::time::Instant::now());
                ring.clear(); // stale audio dies with the capture — never replay it
                continue;
            }
        };
        acc.extend_from_slice(&chunk);
        let capture_frames = acc.len() / frame_len;
        let base_cap = if transport_policy.fec_floor() >= 10 {
            RING_CAP_WAN
        } else {
            RING_CAP_LAN
        };
        // PipeWire may deliver one large quantum even when the stream requested 5 ms. Admit the
        // complete burst, then pace it onto the wire at one Opus frame every 5 ms.
        let cap = base_cap.max(capture_frames);
        while acc.len() >= frame_len {
            let frame: Vec<f32> = acc.drain(..frame_len).collect();
            ring.push_back(frame);
            while ring.len() > cap {
                ring.pop_front();
                overflow_dropped += 1;
            }
        }
        // Drain the ring into Opus datagrams — one Opus frame per packet, in order.
        while let Some(frame) = ring.pop_front() {
            let pts_ns = now_ns();
            // Apply the operator gain (1.0 = no-op) before encoding.
            let encoded = if gain != 1.0 {
                let scaled: Vec<f32> = frame.iter().map(|s| s * gain).collect();
                enc.encode_float(&scaled, &mut opus_buf)
            } else {
                enc.encode_float(&frame, &mut opus_buf)
            };
            match encoded {
                Ok(n) => {
                    // FEC path: buffer into the group, emit data + parity when full.
                    // Plain path: emit the datagram immediately.
                    let ok = if let Some(f) = &mut fec {
                        f.push(&conn, seq, pts_ns, &opus_buf[..n], &mut pacer)
                    } else {
                        pacer.wait_for_next();
                        let d = slipstream_core::quic::encode_audio_datagram(
                            seq,
                            pts_ns,
                            &opus_buf[..n],
                        );
                        if conn.send_datagram(d.into()).is_err() {
                            false
                        } else {
                            true
                        }
                    };
                    if !ok {
                        break 'session; // connection gone
                    }
                    seq = seq.wrapping_add(1);
                }
                Err(e) => {
                    opus_encode_errs += 1;
                    if opus_encode_errs.is_power_of_two() {
                        tracing::warn!(
                            error = %e,
                            count = opus_encode_errs,
                            "opus encode failed — dropping audio frame"
                        );
                    }
                }
            }
        }
        // Phase 8: periodic audio-health line — quantum, ring occupancy, underruns, playout
        // age. The ring is normally empty here (drained above), so `ring_len` is the standing
        // burst left after a congested drain; playout age is approximated from the overflow
        // drops and the current frame age. Recorded separately from video latency.
        if last_telemetry.elapsed() >= std::time::Duration::from_secs(5) {
            let tel = capturer.as_ref().map(|c| c.telemetry()).unwrap_or_default();
            let (fec_frames, fec_parity) = fec
                .as_ref()
                .map(|f| (f.frames_buffered, f.parity_sent))
                .unwrap_or((0, 0));
            tracing::info!(
                quantum_ms = format_args!("{:.1}", tel.quantum_ms),
                ring_len = ring.len(),
                ring_samples = tel.ring_samples,
                underruns = underruns.saturating_add(tel.underruns),
                overflow_dropped = overflow_dropped.saturating_add(tel.overflow_dropped),
                playout_age_ms = format_args!("{:.1}", tel.playout_age_ms),
                fec_frames = fec_frames,
                fec_parity = fec_parity,
                "audio health (Phase 8)"
            );
            last_telemetry = std::time::Instant::now();
        }
    }
    // Flush any partial FEC group at session end so the last few packets aren't stranded
    // (a group is emitted only when full; the tail would otherwise be dropped on teardown).
    if let Some(f) = &mut fec {
        let _ = f.flush(&conn, &mut pacer);
    }
    // Park the live capturer for the next session (None if it died and never reopened),
    // releasing its session-scoped routing claim (Linux: the default sink moves back;
    // Windows: dropped, restoring the operator's default playback device).
    if let Some(mut c) = capturer {
        c.idle();
        crate::audio::park_audio_capture(&audio_cap, c);
    }
}

/// Send-side audio FEC group accumulator (design/audio-resilience.md).
///
/// Buffers encoded audio frames into a group of [`AUDIO_GROUP_LEN`] (8 × 5 ms = 40 ms), then
/// emits the whole group as data datagrams followed by the group's RS parity datagram(s).
/// The parity is generated with the shared [`ErasureCoder`](slipstream_core::fec::ErasureCoder)
/// (GF(2¹⁶) Leopard-RS — the same engine as the video plane), 2 shards on WAN, 1 on LAN (the
/// transport state's FEC floor decides). The group is the send-side reorder window: parity
/// always trails its data, so the client can rebuild a lost packet from the survivors.
#[cfg(any(target_os = "linux", target_os = "windows"))]
struct FecSender {
    /// The current group's frames: `(seq, pts_ns, opus payload)`.
    frames: Vec<(u32, u64, Vec<u8>)>,
    /// Wrapping group id, incremented per emitted group.
    group_id: u8,
    /// The shared RS coder (reused; the encode path allocates only the parity shards).
    coder: Box<dyn slipstream_core::fec::ErasureCoder>,
    /// Transport-state FEC floor (percent) — decides 1 vs 2 parity shards per group.
    transport_policy: Arc<crate::transport_state::TransportPolicyShared>,
    /// Parity datagrams emitted (telemetry: proof FEC is earning its keep).
    parity_sent: u64,
    /// Frames buffered into groups (telemetry).
    frames_buffered: u64,
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
impl FecSender {
    fn new(transport_policy: Arc<crate::transport_state::TransportPolicyShared>) -> Self {
        Self {
            frames: Vec::with_capacity(AUDIO_GROUP_LEN),
            group_id: 0,
            coder: slipstream_core::fec::audio_coder(),
            transport_policy,
            parity_sent: 0,
            frames_buffered: 0,
        }
    }

    /// Whether the current group is full and ready to emit.
    fn full(&self) -> bool {
        self.frames.len() >= AUDIO_GROUP_LEN
    }

    /// Parity shards for this group: 2 on WAN (FEC floor ≥ 10), 1 on LAN.
    fn parity_count(&self) -> usize {
        if self.transport_policy.fec_floor() >= 10 {
            AUDIO_MAX_PARITY
        } else {
            1
        }
    }

    /// Emit the buffered group: data datagrams (in order), then the parity datagram(s).
    /// Returns `false` if the connection is gone (the caller ends the session).
    fn flush(&mut self, conn: &quinn::Connection, pacer: &mut AudioPacer) -> bool {
        if self.frames.is_empty() {
            return true;
        }
        let parity_count = self.parity_count().min(AUDIO_MAX_PARITY);
        // Compute parity over the group's payloads BEFORE moving them out.
        let data: Vec<AudioFecData> = self
            .frames
            .iter()
            .map(|(seq, _, d)| AudioFecData {
                seq: *seq,
                data: d.clone(),
            })
            .collect();
        let parity = generate_parity(self.coder.as_ref(), &data, parity_count);
        let group_id = self.group_id;
        // Data datagrams, in order.
        for (seq, pts_ns, opus) in self.frames.iter() {
            pacer.wait_for_next();
            let d = slipstream_core::quic::encode_audio_datagram_fec(
                *seq,
                *pts_ns,
                opus,
                group_id,
                parity_count as u8,
            );
            if conn.send_datagram(d.into()).is_err() {
                return false;
            }
        }
        // Parity datagram: header (seq 0, pts 0 — meaningless for parity), the concatenated
        // shards as the payload, then the FEC tail with kind = AUDIO_FEC_PARITY.
        if let Ok(parity) = parity {
            if !parity.is_empty() {
                let mut d = slipstream_core::quic::encode_audio_datagram(0, 0, &parity.concat());
                d.push(group_id);
                d.push(parity_count as u8);
                d.push(slipstream_core::quic::AUDIO_FEC_PARITY);
                if conn.send_datagram(d.into()).is_err() {
                    return false;
                }
                self.parity_sent += 1;
            }
        }
        // Advance the group id and reset.
        self.group_id = self.group_id.wrapping_add(1);
        self.frames.clear();
        true
    }

    /// Add one encoded frame to the current group; flushes + emits when the group fills.
    /// Returns `false` if the connection is gone.
    fn push(
        &mut self,
        conn: &quinn::Connection,
        seq: u32,
        pts_ns: u64,
        opus: &[u8],
        pacer: &mut AudioPacer,
    ) -> bool {
        self.frames.push((seq, pts_ns, opus.to_vec()));
        self.frames_buffered += 1;
        if self.full() {
            self.flush(conn, pacer)
        } else {
            true
        }
    }
}

/// Stub — slipstream/1 audio needs Linux (PipeWire capture + libopus); non-Linux dev builds
/// run sessions without it, same as when the capturer fails to open.
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub(super) fn audio_thread(
    _conn: quinn::Connection,
    _stop: Arc<AtomicBool>,
    _audio_cap: AudioCapSlot,
    _channels: u8,
    _transport_policy: Arc<crate::transport_state::TransportPolicyShared>,
    _audio_fec: bool,
) {
    tracing::warn!("slipstream/1 audio requires Linux or Windows — session continues without it");
}
