//! GameStream video data-plane hot path (plan §W1 — carved out of [`super`]): packetize + paced
//! send threads and the encode→packetize loop ([`stream_body`]). `start`/`run` stay in [`super`]
//! and call in here after the RTSP client endpoint is learned.

// Every `unsafe` block in this file carries a `// SAFETY:` proof; enforce it.
#![deny(clippy::undocumented_unsafe_blocks)]

use super::super::video::{FrameType, VideoPacketizer};
use super::super::OnSessionLost;
use super::{gs_bit_depth, StreamConfig};
use crate::capture::Capturer;
use crate::encode;
use crate::send_pacing::percentile;
use anyhow::{Context, Result};
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// One frame's packets, handed from the encode thread to the send thread.
pub(super) type PacketBatch = Vec<Vec<u8>>;

/// Send `pkts` with as few syscalls as possible (`sendmmsg`, up to 64 per call). The socket is
/// connected, so no per-message address. Returns an error on the first send failure.
#[cfg(target_os = "linux")]
pub(super) fn sendmmsg_all(sock: &UdpSocket, pkts: &[Vec<u8>]) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    const CHUNK: usize = 64;
    let fd = sock.as_raw_fd();
    for chunk in pkts.chunks(CHUNK) {
        let mut iovs: Vec<libc::iovec> = chunk
            .iter()
            .map(|p| libc::iovec {
                iov_base: p.as_ptr() as *mut libc::c_void,
                iov_len: p.len(),
            })
            .collect();
        let mut hdrs: Vec<libc::mmsghdr> = iovs
            .iter_mut()
            .map(|iov| {
                // SAFETY: `libc::mmsghdr` is a plain `#[repr(C)]` struct of integers and raw
                // pointers, for which an all-zero bit pattern is valid (null pointers / zero
                // lengths); the fields we rely on (`msg_iov`, `msg_iovlen`) are overwritten on the
                // next two lines before the struct is handed to the kernel.
                let mut h: libc::mmsghdr = unsafe { std::mem::zeroed() };
                h.msg_hdr.msg_iov = iov;
                h.msg_hdr.msg_iovlen = 1;
                h
            })
            .collect();
        let mut off = 0usize;
        while off < hdrs.len() {
            // SAFETY: `fd` is `sock`'s live raw fd (`sock` outlives the call). `hdrs[off..]
            // .as_mut_ptr()` is a live slice of `(hdrs.len() - off)` `mmsghdr`s — exactly the count
            // passed — into which the kernel writes each `msg_len`. Each header's `msg_iov` points
            // into `iovs` (a local that outlives this call, with `msg_iovlen == 1` matching its one
            // entry) and each `iovec.iov_base` points into the `chunk` packet buffers (the caller's
            // `pkts`, alive for the call); the kernel only reads those payloads. Flags 0; the return
            // is error-/progress-checked before advancing `off`.
            let n = unsafe {
                libc::sendmmsg(fd, hdrs[off..].as_mut_ptr(), (hdrs.len() - off) as u32, 0)
            };
            if n < 0 {
                return Err(std::io::Error::last_os_error());
            }
            off += n as usize;
        }
    }
    Ok(())
}

/// Windows: coalesce each paced burst's equal-size packets into `WSASendMsg(UDP_SEND_MSG_SIZE)`
/// super-buffers (UDP Send Offload — the Windows analogue of Linux GSO), so a 16-packet burst is one
/// syscall instead of 16. Reuses the proven core USO primitive; it returns how many leading packets
/// it sent, and we send any remainder (USO off via `SLIPSTREAM_GSO=0`, a size-mixed burst, or a
/// frame's short final packet) with a per-packet `send`. The socket is connected.
#[cfg(target_os = "windows")]
pub(super) fn sendmmsg_all(sock: &UdpSocket, pkts: &[Vec<u8>]) -> std::io::Result<()> {
    let refs: Vec<&[u8]> = pkts.iter().map(|p| p.as_slice()).collect();
    let n = slipstream_core::transport::send_uso_all(sock, &refs)?;
    for p in &pkts[n..] {
        sock.send(p)?;
    }
    Ok(())
}

/// Portable fallback (other non-Linux dev builds, e.g. macOS — GameStream hosting never ships there):
/// one syscall per packet.
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub(super) fn sendmmsg_all(sock: &UdpSocket, pkts: &[Vec<u8>]) -> std::io::Result<()> {
    for p in pkts {
        sock.send(p)?;
    }
    Ok(())
}

/// One encoded frame handed from the encode loop to the packetizer thread: the frame's access
/// units (owned buffers, each with its frame type) plus the shared 90 kHz RTP timestamp. FEC
/// packetization runs on the packetizer thread — off the encode loop — so it never serializes
/// behind encode (measured ~3 ms/frame at 4K, which capped GameStream's frame rate well below what
/// the encoder alone can sustain).
pub(super) struct RawFrame {
    /// `(bitstream, type, wire frameIndex)` per AU. The stream loop assigns the index (it owns
    /// the numbering — see its `au_seq`), so the encoder's RFI bookkeeping stays 1:1 with what
    /// Moonlight sees across mid-stream encoder rebuilds.
    aus: Vec<(Vec<u8>, FrameType, u32)>,
    ts: u32,
}

/// Packetizer thread: turns each [`RawFrame`]'s access units into wire datagrams (data + Reed–Solomon
/// FEC parity shards) via the stateful [`VideoPacketizer`], then hands the batch to the paced sender.
/// It sits between encode and send so the FEC never blocks the encode loop. Backpressure: the hand-off
/// to the sender BLOCKS, so if the paced sender falls behind, the packetizer stalls and the
/// encode→packetizer queue fills — the encode loop then drops the newest frame (see the loop) rather
/// than stalling. Tallies goodput (bytes handed to the wire) into `goodput` for the encode loop's stats
/// window. Exits when either neighbor's channel closes (session teardown / client gone).
pub(super) fn spawn_packetizer(
    rx: std::sync::mpsc::Receiver<RawFrame>,
    tx: std::sync::mpsc::SyncSender<PacketBatch>,
    mut pk: VideoPacketizer,
    goodput: Arc<std::sync::atomic::AtomicU64>,
) -> Result<()> {
    std::thread::Builder::new()
        .name("slipstream-pkt".into())
        .spawn(move || {
            // Above-normal, like the send thread — this stage is on the per-frame critical path.
            crate::native::boost_thread_priority(false);
            while let Ok(frame) = rx.recv() {
                let mut batch: PacketBatch = Vec::new();
                for (au, ft, idx) in frame.aus {
                    batch.extend(pk.packetize(&au, ft, frame.ts, Some(idx)));
                }
                if batch.is_empty() {
                    continue;
                }
                let bytes: u64 = batch.iter().map(|p| p.len() as u64).sum();
                // Blocking send: propagates the paced sender's backpressure upstream (see above).
                if tx.send(batch).is_err() {
                    break; // sender exited (client gone)
                }
                goodput.fetch_add(bytes, std::sync::atomic::Ordering::Relaxed);
            }
        })
        .context("spawn packetizer thread")?;
    Ok(())
}

/// Dedicated send thread: one [`PacketBatch`] per frame arrives on `rx`; its packets go out in
/// `sendmmsg` chunks, paced so the frame's data spreads over ~3/4 of the frame interval — the
/// shared [`send_pacing`](crate::send_pacing) policy at the GameStream parameterization: no
/// microburst stage, a BOUNDED step count (≤ 12, chunk ≥ 16, see the policy's docs for the
/// "send queue full" history that bound guards), each step ending in a sleep toward its slice
/// of the fixed budget. On send failure (client gone) it ends the whole session via `on_lost` —
/// not just this thread: audio would otherwise keep streaming at the dead endpoint and the stale
/// launch state would wedge the next connect (see `AppState::end_session`).
pub(super) fn spawn_sender(
    sock: UdpSocket,
    rx: std::sync::mpsc::Receiver<PacketBatch>,
    frame_interval: Duration,
    running: Arc<AtomicBool>,
    on_lost: OnSessionLost,
) -> Result<()> {
    std::thread::Builder::new()
        .name("slipstream-send".into())
        .spawn(move || {
            // Transmit thread: above-normal, matching the native path's send thread (includes the
            // Windows session tuning/MMCSS this used to call directly; adds the Linux nice -5).
            crate::native::boost_thread_priority(false);
            let budget = frame_interval.mul_f32(0.75);
            let cfg = crate::send_pacing::PaceCfg {
                burst_bytes: None, // no microburst stage — the whole frame spreads
                chunk: crate::send_pacing::ChunkPolicy::Bounded {
                    min_chunk: 16,
                    max_steps: 12,
                },
                sleep_floor: Duration::from_micros(500),
            };
            let mut sent: u64 = 0;
            let mut dropped: u64 = 0;
            while let Ok(mut batch) = rx.recv() {
                // FEC test knob (SLIPSTREAM_VIDEO_DROP) — same knob the native plane honors.
                dropped += crate::send_pacing::inject_video_drop(&mut batch);
                if batch.is_empty() {
                    continue;
                }
                let r = crate::send_pacing::pace_frame(
                    &batch,
                    crate::send_pacing::PaceBudget::Fixed(budget),
                    &cfg,
                    |chunk| {
                        sendmmsg_all(&sock, chunk)?;
                        sent += chunk.len() as u64;
                        Ok::<(), std::io::Error>(())
                    },
                );
                if let Err(e) = r {
                    tracing::info!(error = %e, sent, "video: client unreachable — ending session");
                    running.store(false, Ordering::SeqCst);
                    on_lost();
                    return;
                }
            }
            tracing::debug!(sent, dropped, "video sender exiting");
        })
        .context("spawn send thread")?;
    Ok(())
}

/// The encode → packetize loop, over a borrowed capturer. Sending runs on a dedicated thread
/// (see [`spawn_sender`]) so a send spike can never stall capture/encode.
#[allow(clippy::too_many_arguments)]
pub(super) fn stream_body(
    // `&mut Box` (not `&mut dyn`) so a mid-stream capture-loss rebuild can SWAP the capturer in place.
    capturer: &mut Box<dyn Capturer>,
    // Re-open the video source on capture loss (virtual-display path → follow a Desktop<->Game switch);
    // `None` for the portal/synthetic source, which has nothing to re-detect (propagate the error).
    rebuild: Option<&dyn Fn() -> Result<Box<dyn Capturer>>>,
    // The capture hands the encoder cursor bitmaps to composite (cursor-as-metadata negotiated
    // because the resolved backend blends — see the callers). `false` = the pointer is embedded
    // in the pixels (or absent), so the encoder is asked to composite nothing.
    cursor_blend: bool,
    sock: &UdpSocket,
    cfg: StreamConfig,
    running: &Arc<AtomicBool>,
    force_idr: &AtomicBool,
    rfi_range: &std::sync::Mutex<Option<(i64, i64)>>,
    // Shared stats recorder. The encode loop reads `stats.is_armed()` per frame to decide whether
    // to accumulate the per-stage split, then emits a `StatsSample` at its 1 s aggregation boundary.
    stats: &Arc<crate::stats_recorder::StatsRecorder>,
    // Short client label (peer IP) seeded into the capture meta on the first armed registration.
    client_label: &str,
    // Whole-session teardown, handed to the send thread's client-unreachable detection.
    on_lost: &OnSessionLost,
) -> Result<()> {
    // The first frame establishes the authoritative size/format for the encoder.
    let mut frame = capturer.next_frame().context("capture first frame")?;
    if frame.width != cfg.width || frame.height != cfg.height {
        tracing::warn!(
            captured = ?(frame.width, frame.height),
            negotiated = ?(cfg.width, cfg.height),
            "captured size != negotiated size — Moonlight expects the negotiated size; resize the output"
        );
    }
    let mut enc = encode::open_video(
        cfg.codec,
        frame.format,
        frame.width,
        frame.height,
        cfg.fps,
        cfg.bitrate_kbps as u64 * 1000,
        frame.is_cuda(),
        // 8-bit SDR, or 10-bit when the captured frame is HDR (P010) — see `gs_bit_depth`.
        gs_bit_depth(frame.format),
        // GameStream/Moonlight stays 4:2:0 — stock Moonlight clients can't decode 4:4:4, and the
        // Windows IDD-push capturer can't yet deliver full-chroma frames. 4:4:4 is slipstream/1-native only.
        encode::ChromaFormat::Yuv420,
        // True only when THIS session's capture negotiated cursor-as-metadata — which the
        // callers grant only where the resolved backend composites (`cursor_blend_capable`).
        cursor_blend,
        // The client's requested slices-per-frame (its DECODER's ceiling) — see `StreamConfig`.
        cfg.slices,
    )
    .context("open video encoder for stream")?;
    // Tell the encoder how deep the capturer lets it pipeline. Without this an in-place backend
    // (Windows direct-NVENC, which encodes the capturer's textures with no CopyResource) bounds
    // itself by an env cap instead of the ring it is actually reading, and the capturer rotates a
    // texture out from under a live encode — torn/mixed frames, never an error. The backend now
    // also fails safe when nobody tells it, but pass the REAL depth: `idd_depth` is configurable
    // and a deeper ring is free pipelining the fallback would forfeit.
    enc.set_input_ring_depth(capturer.pipeline_depth().max(1));
    // FEC overhead percent (Sunshine default 20). Override with SLIPSTREAM_FEC_PCT (0 = data-only).
    let fec_pct: u8 = std::env::var("SLIPSTREAM_FEC_PCT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);
    let pk = VideoPacketizer::new(cfg.packet_size, fec_pct, cfg.min_fec);

    // Pace at the client's negotiated frame rate, re-encoding the last captured frame when the
    // compositor produced no new one. Compositors only emit frames on damage, so a static or
    // slow-updating desktop would otherwise starve the client into a "network too slow" abort.
    // Re-encoding an unchanged frame is cheap — NVENC emits a near-empty P-frame. The upper
    // bound just guards against an absurd client request (the encoder is opened at `cfg.fps`).
    let target_fps = cfg.fps.clamp(1, 240);
    let frame_interval = Duration::from_secs_f64(1.0 / target_fps as f64);
    let mut fps_count: u32 = 0;
    let mut fps_t = Instant::now();
    let stream_start = Instant::now();
    let mut sent_batches: u64 = 0;
    let mut dropped_batches: u64 = 0;

    // Three-stage pipeline so FEC packetization never blocks encode: `encode loop → [raw AUs] →
    // packetizer (FEC/RS) → [wire batch] → paced sender`, each stage on its own thread joined by a
    // depth-2 bounded queue. Depth 2 means a slow stage can buffer one frame while the next is
    // produced; beyond that the NEWEST frame is dropped (the client recovers via FEC/RFI) rather than
    // stalling the encode loop. Backpressure chains up: a slow sender blocks the packetizer, which
    // fills the encode→packetizer queue, which makes the encode loop drop — encode itself never
    // waits. Goodput (bytes handed to the wire) is tallied by the packetizer into `goodput`, read at
    // the encode loop's 1 s stats boundary (the old inline batch-byte sum moved with packetization).
    let goodput = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let (batch_tx, batch_rx) = std::sync::mpsc::sync_channel::<PacketBatch>(2);
    spawn_sender(
        sock.try_clone().context("clone video socket")?,
        batch_rx,
        Duration::from_secs_f64(1.0 / target_fps as f64),
        running.clone(),
        on_lost.clone(),
    )?;
    let (raw_tx, raw_rx) = std::sync::mpsc::sync_channel::<RawFrame>(2);
    spawn_packetizer(raw_rx, batch_tx, pk, goodput.clone())?;

    // Per-stage timing (SLIPSTREAM_PERF=1): max µs/stage per second + unique vs re-encoded frames,
    // to pinpoint stalls. `unique` counts genuinely-new captured frames (vs re-encoded holds).
    let perf = ss_host_config::config().perf;
    let (mut mx_cap, mut mx_enc, mut mx_pkt, mut mx_send, mut uniq) =
        (0u128, 0u128, 0u128, 0u128, 0u32);
    // Web-console stats accumulation (active when `perf` OR a capture is armed): per-stage vectors
    // for p50/p99, the goodput bytes queued to the sender this window, the previous window's
    // dropped-frame count for delta computation, and the registration id cached on the first sample.
    let codec_name = cfg.codec.label();
    let mut sid: Option<u32> = None;
    let (mut v_cap, mut v_enc, mut v_pkt, mut v_send): (Vec<u32>, Vec<u32>, Vec<u32>, Vec<u32>) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut last_dropped_batches: u64 = 0;
    // Absolute next-frame deadline — the single pacing clock for the loop.
    let mut next_frame = Instant::now();
    // RFI capability is fixed for the session (probed at encoder open). Query it once so the
    // recovery path skips the always-`false` invalidate call on encoders without NVENC RFI and
    // forces a keyframe directly instead.
    let mut supports_rfi = enc.caps().supports_rfi;

    // Bound consecutive capture-loss rebuilds (a delivered frame clears the counter) so a permanently
    // dead source can't loop forever — it ends the stream after the cap, falling back to a reconnect.
    const MAX_REBUILDS: u32 = 5;
    let mut rebuilds: u32 = 0;
    // Encode-stall recovery, the GameStream twin of the native path's ladder (native/stream.rs,
    // `reset_stalled_encoder`): a submit/poll failure or a silent stall rebuilds the encoder in
    // place — bounded — instead of ending the stream. The backends deliberately turn a wedged
    // GPU into a bounded error so the caller can do exactly this; without the ladder here, every
    // such error cost a Moonlight client a full disconnect/reconnect. `last_au_at` feeds the
    // silent-wedge watchdog below (backends whose non-blocking poll returns `None` forever
    // instead of erroring); every received AU clears the reset budget.
    const MAX_ENCODER_RESETS: u32 = 5;
    let mut encoder_resets: u32 = 0;
    let mut last_au_at = Instant::now();

    // Coalesce forced keyframes. Under loss Moonlight spams IDR/RFI requests; on an encoder without
    // RFI (VAAPI/AMD — `supports_rfi=false`) each one becomes a full IDR, so an un-coalesced request
    // stream turns EVERY frame into a 4K IDR, saturates the send path, and collapses the session
    // instead of recovering. One fresh IDR already resolves all pending loss, so after emitting one
    // we ignore further keyframe requests for a short in-flight window (~2 frames). NVENC
    // ref-invalidation (cheap, no IDR spike) is never rate-limited — only full keyframes are.
    let keyframe_coalesce = frame_interval * 2;
    let mut last_keyframe: Option<Instant> = None;
    // A frame dropped at the pipeline head (below) breaks the reference chain for the following
    // P-frames: the client never receives it, but the encoder advanced its references past it, and —
    // packetization being downstream now — a dropped frame consumes no frameIndex for the client to
    // detect the gap. So the host re-anchors itself: a drop arms a keyframe on the next iteration,
    // routed through the same coalesce gate as client IDR requests so a burst of drops (congestion)
    // can't become an IDR storm.
    let mut recover_after_drop = false;
    // The stream's wire frameIndex numbering, owned HERE (the index of the next AU handed to the
    // packetizer thread; a dropped-at-the-queue frame consumes none). A submission's future index
    // is `au_seq + enc_inflight` (AUs are emitted FIFO, one per submission); passing it to
    // `Encoder::submit_indexed` keeps the encoder's RFI bookkeeping 1:1 with Moonlight's frame
    // numbers across the in-place encoder rebuild above (an internal counter would desync there).
    // A pipeline-head drop desyncs the prediction by the dropped AU count for the frames already
    // in flight — bounded and self-healing: the drop arms `recover_after_drop`, whose forced IDR
    // resets the encoder's reference state (stale LTR/DPB bookkeeping dies with it).
    let mut au_seq: u32 = 0;
    let mut enc_inflight: u32 = 0;

    while running.load(Ordering::SeqCst) {
        let tick = Instant::now();
        // Measure per-stage timing when `SLIPSTREAM_PERF` is set OR a web-console stats capture is
        // armed (cheap Relaxed atomic, re-read each frame).
        let measure = perf || stats.is_armed();
        // Advance to the freshest captured frame if one arrived; otherwise reuse the last.
        match capturer.try_latest() {
            Ok(Some(f)) => {
                frame = f;
                uniq += 1;
                rebuilds = 0; // a delivered frame clears the consecutive-loss counter
            }
            Ok(None) => {} // no new frame — reuse the last (static/idle desktop)
            Err(e) => {
                // The capture source went away — the compositor was torn down on a Desktop<->Game
                // switch, or the virtual output was removed. On the virtual-display path, re-detect the
                // now-live compositor and re-attach IN PLACE (the send thread + packetizer + socket +
                // RTP clock all survive), then force an IDR so Moonlight resyncs — so the stream FOLLOWS
                // the switch with no client reconnect. Build the new source BEFORE dropping the old.
                // Bounded by a counter + a ~40s budget; on exhaustion, end the stream (Moonlight
                // reconnect). The portal/synthetic path has no rebuild closure → propagate as before.
                let Some(rebuild) = rebuild else {
                    return Err(e).context("capture frame");
                };
                rebuilds += 1;
                if rebuilds > MAX_REBUILDS {
                    return Err(e).context("capture lost — rebuild attempts exhausted");
                }
                tracing::warn!(error = %format!("{e:#}"), rebuild = rebuilds,
                    "gamestream: capture lost — rebuilding source in place (following a session switch)");
                let rebuild_deadline = Instant::now() + Duration::from_secs(40);
                let new_cap = loop {
                    match rebuild() {
                        Ok(c) => break c,
                        Err(e2) => {
                            if !running.load(Ordering::SeqCst) || Instant::now() >= rebuild_deadline
                            {
                                return Err(e2)
                                    .context("capture lost — no source within the rebuild budget");
                            }
                            tracing::warn!(error = %format!("{e2:#}"),
                                "gamestream: source not up yet — retrying");
                            std::thread::sleep(Duration::from_millis(500));
                        }
                    }
                };
                *capturer = new_cap;
                capturer.set_active(true);
                frame = capturer.next_frame().context("first frame after rebuild")?;
                // Re-open the encoder for the new source (same negotiated WxH → same SPS profile) and
                // force an IDR so Moonlight resyncs on the first emitted AU.
                enc = encode::open_video(
                    cfg.codec,
                    frame.format,
                    frame.width,
                    frame.height,
                    cfg.fps,
                    cfg.bitrate_kbps as u64 * 1000,
                    frame.is_cuda(),
                    gs_bit_depth(frame.format),
                    encode::ChromaFormat::Yuv420, // GameStream stays 4:2:0
                    cursor_blend,                 // same capture cursor mode — see the first open
                    cfg.slices,                   // client slicing ceiling — see the first open
                )
                .context("reopen encoder after rebuild")?;
                // A rebuilt encoder starts unconfigured — same reason as the first open above.
                enc.set_input_ring_depth(capturer.pipeline_depth().max(1));
                supports_rfi = enc.caps().supports_rfi;
                enc.request_keyframe();
                last_keyframe = Some(Instant::now());
                next_frame = Instant::now();
                // The old encoder died with its in-flight submissions — their AUs will never
                // arrive, so the numbering prediction restarts at `au_seq` (the fresh encoder's
                // reference state is empty, so the reused predictions meet no stale bookkeeping).
                enc_inflight = 0;
                tracing::info!("gamestream: source rebuilt — stream continues");
                continue;
            }
        }
        let t_cap = tick.elapsed();
        // Honor a client recovery request. Prefer reference-frame invalidation (the encoder
        // re-references an older still-valid frame — no costly IDR spike); if the encoder can't
        // invalidate (range too old, or no NVENC RFI) it returns false and we force a keyframe.
        // A prior pipeline drop needs a fresh keyframe to re-anchor the reference chain (see
        // below). Consumed only when the keyframe is actually EMITTED (in the coalesce gate) —
        // read-and-clear here let the gate swallow the request for good.
        let mut want_keyframe = recover_after_drop;
        if let Some((first, last)) = rfi_range.lock().unwrap().take() {
            // Prefer reference-frame invalidation when the encoder supports it (no costly IDR
            // spike); otherwise — or if the range is too old to invalidate — fall back to a keyframe.
            // Sanity-cap the range first: wider than RFI_MAX_RANGE exceeds any encoder's reference
            // history (or is a phantom range from a desynced counter) — keyframe, never a
            // force-reference that could ship corruption as a clean frame.
            let width = (last as u32).wrapping_sub(first as u32);
            if width > slipstream_core::packet::RFI_MAX_RANGE
                || !(supports_rfi && enc.invalidate_ref_frames(first, last))
            {
                want_keyframe = true;
            }
        }
        // An explicit IDR request (or a rangeless RFI) asks for a keyframe so the client resyncs
        // immediately instead of waiting for the next GOP boundary.
        if force_idr.swap(false, Ordering::SeqCst) {
            want_keyframe = true;
        }
        // Coalesce: emit at most one forced keyframe per in-flight window, so a burst of recovery
        // requests during one loss event doesn't turn every frame into a full IDR (see above).
        if want_keyframe {
            let now = Instant::now();
            let emit = match last_keyframe {
                Some(t) => now.duration_since(t) >= keyframe_coalesce,
                None => true,
            };
            if emit {
                enc.request_keyframe();
                last_keyframe = Some(now);
                // A drop-recovery request is satisfied by an EMITTED keyframe, not by being
                // read: coalesced away it would be lost — never retried — leaving duplicate wire
                // indices in the encoder's reference table for a later RFI to anchor on (the
                // stale-anchor case rfi.rs exists to prevent). Keep it armed until this point.
                recover_after_drop = false;
            } else {
                tracing::debug!("video: keyframe request coalesced (IDR still in flight)");
            }
        }
        if let Err(e) = enc.submit_indexed(&frame, au_seq.wrapping_add(enc_inflight)) {
            // The input half of an encode stall (see native/stream.rs): rebuild the encoder in
            // place instead of ending the stream. A backend without an in-place rebuild
            // (`reset` = false) or an exhausted budget still fails the session, with the cause.
            encoder_resets += 1;
            if encoder_resets > MAX_ENCODER_RESETS || !enc.reset() {
                tracing::error!(
                    error = %format!("{e:#}"),
                    resets = encoder_resets,
                    "encoder did not recover after repeated in-place rebuilds — ending the \
                     stream (see the error above for the cause)"
                );
                return Err(e).context("encoder submit");
            }
            // The owed AUs died with the discarded encoder state; numbering restarts at `au_seq`,
            // and the rebuilt encoder's reference state is empty so the reused predictions meet
            // no stale bookkeeping (same reasoning as the capture rebuild above). The IDR
            // bypasses the coalesce gate: a rebuilt encoder MUST resync the client.
            enc_inflight = 0;
            enc.request_keyframe();
            last_keyframe = Some(Instant::now());
            last_au_at = Instant::now();
            tracing::warn!(error = %format!("{e:#}"), reset = encoder_resets,
                max = MAX_ENCODER_RESETS,
                "encoder submit failed — encoder rebuilt in place, forcing an IDR");
            // Real backoff between attempts, not a frame period: five instant retries burn out
            // inside one driver hiccup (the native ladder's 2026-07 field lesson).
            let backoff =
                frame_interval.max(Duration::from_millis(100u64 << (encoder_resets - 1).min(4)));
            next_frame = Instant::now() + backoff;
            std::thread::sleep(backoff);
            continue;
        }
        enc_inflight = enc_inflight.wrapping_add(1);
        let t_enc = tick.elapsed();

        // 90 kHz RTP timestamp from wall-clock, so a variable capture rate stays correct.
        let ts = (stream_start.elapsed().as_secs_f64() * 90_000.0) as u32;
        // Drain the encoder's access units (owned buffers) — FEC/packetization runs on the
        // packetizer thread, off this loop, so it never serializes behind encode. Each AU is
        // stamped with its wire frameIndex here (`au_seq + position`); the numbering only
        // ADVANCES if the batch is actually enqueued below (a dropped batch consumes none).
        let mut aus: Vec<(Vec<u8>, FrameType, u32)> = Vec::new();
        // A poll error is the output half of an encode stall (e.g. a bounded fence timeout from
        // a wedged GPU) — carry it to the shared stall recovery below, after the AUs already
        // drained are handed off, instead of killing the session outright.
        let mut poll_err: Option<anyhow::Error> = None;
        loop {
            let au = match enc.poll() {
                Ok(Some(au)) => au,
                Ok(None) => break,
                Err(e) => {
                    poll_err = Some(e);
                    break;
                }
            };
            let ft = if au.keyframe {
                FrameType::Idr
            } else {
                FrameType::P
            };
            let idx = au_seq.wrapping_add(aus.len() as u32);
            aus.push((au.data, ft, idx));
            enc_inflight = enc_inflight.saturating_sub(1);
            // Every AU proves the encoder is alive.
            last_au_at = Instant::now();
            encoder_resets = 0;
        }
        let t_pkt = tick.elapsed();

        // Hand the frame's AUs to the pipeline; never block here. A full queue means the pipeline
        // (packetizer, or the paced sender behind it) is behind — drop this frame (FEC/RFI covers the
        // client) and keep encoding, so a downstream stall can never cap the encode rate.
        if !aus.is_empty() {
            let batch_len = aus.len() as u32;
            match raw_tx.try_send(RawFrame { aus, ts }) {
                Ok(()) => {
                    sent_batches += 1;
                    au_seq = au_seq.wrapping_add(batch_len);
                }
                Err(std::sync::mpsc::TrySendError::Full(_)) => {
                    dropped_batches += 1;
                    recover_after_drop = true; // re-anchor the reference chain on the next frame
                    if dropped_batches.is_power_of_two() {
                        tracing::warn!(
                            dropped_batches,
                            "video: pipeline queue full — frame dropped"
                        );
                    }
                }
                Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                    break; // packetizer/sender exited (client gone)
                }
            }
        }
        // Encode-stall recovery, the poll half (mirrors the native path's watchdog): an explicit
        // poll error, or no AU within the window while frames are owed — the silent wedge, where
        // a non-blocking poll returns `None` forever and nothing else ever errors. The window
        // scales with the frame interval so low-fps modes can't false-trip.
        let stall_window = Duration::from_secs(2).max(frame_interval * 8);
        if poll_err.is_some() || (enc_inflight > 0 && last_au_at.elapsed() >= stall_window) {
            let why = match &poll_err {
                Some(e) => format!("poll failed: {e:#}"),
                None => format!(
                    "no AU for {} ms with {} frame(s) owed",
                    last_au_at.elapsed().as_millis(),
                    enc_inflight
                ),
            };
            encoder_resets += 1;
            if encoder_resets > MAX_ENCODER_RESETS || !enc.reset() {
                return Err(poll_err.unwrap_or_else(|| anyhow::anyhow!("{why}")))
                    .context("encoder stalled — in-place rebuild unavailable or exhausted");
            }
            enc_inflight = 0;
            enc.request_keyframe();
            last_keyframe = Some(Instant::now());
            last_au_at = Instant::now();
            tracing::warn!(reset = encoder_resets, max = MAX_ENCODER_RESETS, %why,
                "encode stall detected — encoder rebuilt in place, forcing an IDR");
            let backoff =
                frame_interval.max(Duration::from_millis(100u64 << (encoder_resets - 1).min(4)));
            next_frame = Instant::now() + backoff;
            std::thread::sleep(backoff);
            continue;
        }
        if measure {
            let t_send = tick.elapsed();
            let cap_us = t_cap.as_micros();
            let enc_us = (t_enc - t_cap).as_micros();
            // `poll` = drain the encoder's AUs; `enqueue` = hand-off to the pipeline. FEC/packetize
            // and the paced send now run on their own threads, off this loop — so both of these
            // should be small; if they aren't, the encode loop is being stalled by pipeline
            // backpressure (a full queue), which is the signal that a downstream stage can't keep up.
            let poll_us = (t_pkt - t_enc).as_micros();
            let enqueue_us = (t_send - t_pkt).as_micros();
            mx_cap = mx_cap.max(cap_us);
            mx_enc = mx_enc.max(enc_us);
            mx_pkt = mx_pkt.max(poll_us);
            mx_send = mx_send.max(enqueue_us);
            v_cap.push(cap_us as u32);
            v_enc.push(enc_us as u32);
            v_pkt.push(poll_us as u32);
            v_send.push(enqueue_us as u32);
        }

        fps_count += 1;
        if fps_t.elapsed() >= Duration::from_secs(1) {
            let secs = fps_t.elapsed().as_secs_f64();
            // Bytes handed to the wire this window, tallied by the packetizer thread (goodput).
            let win_bytes = goodput.swap(0, std::sync::atomic::Ordering::Relaxed);
            if perf {
                // Max µs/stage this second on the ENCODE loop: cap=drain channel, enc=submit
                // (zero-copy device copy + NVENC), pkt=poll (AU drain), send=enqueue to the pipeline.
                // FEC/packetize and the paced send run on their own threads now, so pkt/send here
                // should be near-zero — a nonzero value means encode is being stalled by pipeline
                // backpressure. `uniq`=new captured frames (vs re-encoded).
                tracing::info!(
                    fps = fps_count,
                    uniq,
                    enc_us = mx_enc,
                    pkt_us = mx_pkt,
                    send_us = mx_send,
                    cap_us = mx_cap,
                    "video: streaming (perf)"
                );
            } else {
                tracing::debug!(
                    fps = fps_count,
                    sent_batches,
                    dropped_batches,
                    "video: streaming"
                );
            }
            // Web-console capture: build the aggregated sample. The host send side exposes no
            // receiver-side packet loss / FEC-recovery / send-buffer EAGAIN counters, so those stay
            // 0 (not fabricated); `frames_dropped` is the per-frame pipeline-queue overflow delta.
            if stats.is_armed() {
                let capture_telemetry = capturer.telemetry();
                let capture_age_us = crate::stats_recorder::capture_age_us(
                    capture_telemetry.last_frame_ns,
                    crate::stats_recorder::unix_now_ns(),
                );
                let session_id = *sid.get_or_insert_with(|| {
                    stats.register_session(
                        "gamestream",
                        cfg.width,
                        cfg.height,
                        cfg.fps,
                        codec_name,
                        client_label,
                    )
                });
                let sample = crate::stats_recorder::StatsSample {
                    t_ms: 0, // stamped by push_sample from the capture's monotonic start
                    session_id,
                    stages: vec![
                        crate::stats_recorder::StageTiming {
                            name: "capture".into(),
                            p50_us: percentile(&mut v_cap, 0.50) as f32,
                            p99_us: percentile(&mut v_cap, 0.99) as f32,
                        },
                        crate::stats_recorder::StageTiming {
                            name: "encode".into(),
                            p50_us: percentile(&mut v_enc, 0.50) as f32,
                            p99_us: percentile(&mut v_enc, 0.99) as f32,
                        },
                        crate::stats_recorder::StageTiming {
                            name: "packetize".into(),
                            p50_us: percentile(&mut v_pkt, 0.50) as f32,
                            p99_us: percentile(&mut v_pkt, 0.99) as f32,
                        },
                        crate::stats_recorder::StageTiming {
                            name: "send".into(),
                            p50_us: percentile(&mut v_send, 0.50) as f32,
                            p99_us: percentile(&mut v_send, 0.99) as f32,
                        },
                    ],
                    fps: (uniq as f64 / secs) as f32,
                    repeat_fps: (fps_count.saturating_sub(uniq) as f64 / secs) as f32,
                    mbps: (win_bytes as f64 * 8.0 / secs / 1_000_000.0) as f32,
                    bitrate_kbps: cfg.bitrate_kbps,
                    frames_dropped: dropped_batches.saturating_sub(last_dropped_batches) as u32,
                    packets_dropped: 0,
                    send_dropped: 0,
                    fec_recovered: 0,
                    capture_age_us,
                    capture_age_over_limit: crate::stats_recorder::capture_age_over_limit(
                        capture_age_us,
                    ),
                    capture_backend: capturer.backend_name().to_string(),
                    capture_frames_published: capture_telemetry.frames_published,
                    capture_frames_overwritten: capture_telemetry.frames_overwritten,
                    capture_buffers_drained: capture_telemetry.buffers_drained,
                    capture_modifier: capture_telemetry.modifier,
                    capture_width: capture_telemetry.width,
                    capture_height: capture_telemetry.height,
                };
                stats.push_sample(session_id, sample);
            }
            mx_cap = 0;
            mx_enc = 0;
            mx_pkt = 0;
            mx_send = 0;
            uniq = 0;
            v_cap.clear();
            v_enc.clear();
            v_pkt.clear();
            v_send.clear();
            last_dropped_batches = dropped_batches;
            fps_count = 0;
            fps_t = Instant::now();
        }
        // Single pacing authority: hold a steady cadence at the target rate from an absolute
        // clock. No double-sleep. If a slow frame put us behind, resync to now rather than
        // bursting to catch up.
        next_frame += frame_interval;
        match next_frame.checked_duration_since(Instant::now()) {
            Some(d) => std::thread::sleep(d),
            None => next_frame = Instant::now(),
        }
    }
    Ok(())
}
