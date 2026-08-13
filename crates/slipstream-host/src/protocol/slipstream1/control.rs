//! The native `slipstream/1` mid-stream control task (plan §W1 — carved out of [`super`]'s
//! `serve_session`). After the handshake the control stream stays open for renegotiation and
//! speed tests; this task multiplexes the inbound client requests (`Reconfigure` /
//! `RequestKeyframe` / `RfiRequest` / `LossReport` / `SetBitrate` / `ProbeRequest` / `ClockProbe`)
//! with the outbound probe-result and mode-correction channels, handing every validated change to
//! the data-plane thread over the session's mpsc bridges.

use super::*;
use slipstream_core::quic::{ClipControl, ClipOffer, ClipState};
use ss_clipboard::ClipCoordCmd;

/// Run the control task for one live session. Owns the control streams (`serve_session` hands them
/// off after negotiation) plus every channel end that bridges to the data-plane thread, and the
/// [`ss_clipboard::ClipCoord`] handle bridging to the clipboard coordinator. Returns when the
/// control stream closes or a data-plane channel drops.
#[allow(clippy::too_many_arguments)]
pub(super) async fn run(
    mut ctrl_send: quinn::SendStream,
    ctrl_recv: quinn::RecvStream,
    initial_mode: slipstream_core::Mode,
    codec: crate::encode::Codec,
    live_reconfig_ok: bool,
    adaptive_fec: bool,
    session_bitrate_kbps: u32,
    // Encoder-truth bridge (data plane → here, §ABR overdrive): the encoder's live applied rate,
    // its discovered codec-level ceiling (0 = unknown), and the "encode can't hold cadence"
    // flag. Read at `SetBitrate`-resolve time so the ack — the base the client's controller
    // climbs from — never promises a rate the encoder won't run at.
    live_bitrate: Arc<AtomicU32>,
    encoder_ceiling_kbps: Arc<AtomicU32>,
    cadence_degraded: Arc<AtomicBool>,
    fec_target_ctl: Arc<AtomicU8>,
    // Phase-locked capture bridge: client PhaseReports land here latest-wins; the encode loop's
    // controller drains at its own ~1 Hz cadence (design/phase-locked-capture.md).
    phase_ctl: Arc<super::stream::PhaseCtl>,
    // Phase 6: the measured transport-state machine + its published policy. The control task
    // feeds one window per LossReport (RTT from the QUIC connection, loss from the report,
    // capacity from the probe bursts) and publishes the policy on every state change.
    conn: quinn::Connection,
    transport_state: Arc<std::sync::Mutex<crate::transport_state::TransportStateMachine>>,
    transport_policy: Arc<crate::transport_state::TransportPolicyShared>,
    reconfig_tx: std::sync::mpsc::Sender<slipstream_core::Mode>,
    keyframe_tx: std::sync::mpsc::Sender<()>,
    rfi_tx: std::sync::mpsc::Sender<(u32, u32)>,
    bitrate_tx: std::sync::mpsc::Sender<u32>,
    probe_tx: std::sync::mpsc::Sender<ProbeRequest>,
    mut probe_result_rx: tokio::sync::mpsc::UnboundedReceiver<ProbeResult>,
    mut reconfig_result_rx: tokio::sync::mpsc::UnboundedReceiver<Reconfigured>,
    mut cursor_shape_rx: tokio::sync::mpsc::UnboundedReceiver<slipstream_core::quic::CursorShape>,
    cursor_client_draws: Arc<AtomicBool>,
    clip_enabled: Arc<AtomicBool>,
    clip: ss_clipboard::ClipCoord,
) {
    let ss_clipboard::ClipCoord {
        available: clip_available,
        cmd_tx: clip_cmd_tx,
        offer_rx: mut clip_offer_rx,
    } = clip;
    // Set once `clip_offer_rx` closes (coordinator gone / inert handle) so its `select!` branch
    // stops firing on a perpetually-ready `None`.
    let mut clip_offer_closed = false;
    // Phase 6 measurement state: the last window feed, the probe-measured capacity, and the
    // smoothed RTT + variation the machine classifies on.
    let mut last_transport_feed = std::time::Instant::now();
    let mut last_capacity_bps: u64 = 0;
    let mut rtt_ewma_ms: f64 = 0.0;
    let mut rtt_var_ms: f64 = 0.0;
    let mut active = initial_mode;
    // Host-side switch rate limit (a backstop against a hostile/broken client spamming
    // Reconfigure into pipeline-rebuild churn — the drain-to-newest in the data plane already
    // coalesces a well-behaved resize drag; compliant clients self-limit to ≥ 1 s).
    const MIN_SWITCH_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);
    let mut last_accepted_switch: Option<std::time::Instant> = None;
    // Resumable framing: this read is one arm of a `select!` whose siblings fire on every probe
    // result / reconfigure / clip offer, so the read future is dropped routinely. `io::read_msg`
    // would lose the partial frame and misalign the stream for the rest of the session.
    let mut ctrl_reader = io::MsgReader::new(ctrl_recv);
    loop {
        tokio::select! {
            msg = ctrl_reader.read_msg() => {
                let Ok(msg) = msg else { break }; // stream closed
                if let Ok(req) = Reconfigure::decode(&msg) {
                    let now = std::time::Instant::now();
                    let valid = req.mode.refresh_hz > 0
                        && crate::encode::validate_dimensions(
                            codec,
                            req.mode.width,
                            req.mode.height,
                        )
                        .is_ok();
                    let too_soon = last_accepted_switch
                        .is_some_and(|t| now.duration_since(t) < MIN_SWITCH_INTERVAL);
                    let ok = if !live_reconfig_ok {
                        // Backend can't live-reconfigure (gamescope / synthetic /
                        // per-client-mode identity — see the gate above): honest downgrade,
                        // the client keeps scaling client-side.
                        tracing::info!(mode = ?req.mode,
                            "mode switch rejected (backend cannot live-reconfigure)");
                        false
                    } else if !valid {
                        tracing::warn!(mode = ?req.mode, "mode switch rejected (invalid dimensions)");
                        false
                    } else if too_soon {
                        tracing::warn!(mode = ?req.mode, "mode switch rejected (rate-limited)");
                        false
                    } else {
                        true
                    };
                    if ok {
                        active = req.mode;
                        last_accepted_switch = Some(now);
                        tracing::info!(mode = ?req.mode, "mode switch accepted");
                    }
                    let ack = Reconfigured { accepted: ok, mode: active };
                    if io::write_msg(&mut ctrl_send, &ack.encode()).await.is_err() {
                        break;
                    }
                    if ok && reconfig_tx.send(req.mode).is_err() {
                        break; // data plane gone
                    }
                } else if RequestKeyframe::decode(&msg).is_ok() {
                    // Client recovery: its decoder wedged — force the next encoded frame to
                    // be an IDR. Coalesced in the encode loop (a wedge fires several before
                    // the IDR lands); a send error just means the data plane is gone.
                    tracing::debug!("client requested keyframe (decode recovery)");
                    if keyframe_tx.send(()).is_err() {
                        break; // data plane gone
                    }
                } else if let Ok(req) = RfiRequest::decode(&msg) {
                    // Client LTR-RFI recovery: it lost the frame range `[first, last]` and asks
                    // the encoder to re-reference a known-good older frame instead of paying for
                    // a full IDR. The encode loop attempts `invalidate_ref_frames`, falling back
                    // to a coalesced keyframe when the encoder can't (range too old / no RFI).
                    tracing::debug!(
                        first = req.first_frame,
                        last = req.last_frame,
                        "client requested reference-frame invalidation (loss recovery)"
                    );
                    if rfi_tx.send((req.first_frame, req.last_frame)).is_err() {
                        break; // data plane gone
                    }
                } else if let Ok(rep) = LossReport::decode(&msg) {
                    // Adaptive FEC: size recovery to the loss the client is seeing. The data-plane
                    // send loop reads `fec_target_ctl` and applies it per frame. Ignored when FEC
                    // is pinned via SLIPSTREAM_FEC_PCT.
                    // Phase 6: feed the transport-state machine one measurement window
                    // (RTT from the QUIC control connection, loss from this report, capacity
                    // from the latest probe burst). Publishes the state's policy on change.
                    feed_transport(
                        &transport_state,
                        &transport_policy,
                        &conn,
                        u64::from(rep.loss_ppm),
                        last_capacity_bps,
                        live_bitrate.load(Ordering::Relaxed) as u64 * 1000,
                        &mut last_transport_feed,
                        &mut rtt_ewma_ms,
                        &mut rtt_var_ms,
                    );
                    if adaptive_fec {
                        // Fast attack, slow decay: jump straight to what the reported loss
                        // needs, but come DOWN only one point per clean report (~750 ms). The
                        // memoryless controller ping-ponged on periodic burst loss (Wi-Fi
                        // scans / BT coexistence, a burst every few seconds): a single clean
                        // window dropped FEC back to the floor, so every next burst hit an
                        // unprotected stream — an unrecoverable frame, a freeze, and a
                        // recovery-IDR burst, once per cycle. Decaying over ~10 windows keeps
                        // the stream covered across the gap while still converging to FEC_MIN
                        // on a genuinely clean link. The transport state's policy is the floor
                        // (a WAN state never drops below its FEC protection).
                        let prev = fec_target_ctl.load(Ordering::Relaxed);
                        let target = adapt_fec(rep.loss_ppm)
                            .max(prev.saturating_sub(1))
                            .max(transport_policy.fec_floor());
                        fec_target_ctl.store(target, Ordering::Relaxed);
                        if prev != target {
                            tracing::debug!(
                                loss_ppm = rep.loss_ppm,
                                fec_pct = target,
                                prev_fec_pct = prev,
                                "adaptive FEC adjusted"
                            );
                        }
                    }
                } else if let Ok(req) = SetBitrate::decode(&msg) {
                    // Mid-stream bitrate renegotiation (adaptive bitrate): clamp exactly like
                    // the Hello request, ack the resolved value, then hand it to the data-plane
                    // thread, which rebuilds the encoder in place at the same mode — the fresh
                    // encoder's first frame is an IDR with in-band parameter sets, so the
                    // client's decoder follows without a reconnect.
                    // PyroWave: the rate is PINNED (§4.6 — quality collapses under rate
                    // descent; recovery pressure is answered by codec fallback, not AIMD).
                    // Our client controller is off for this codec; this guards older or
                    // foreign clients by acking the unchanged session rate.
                    let resolved = if codec == crate::encode::Codec::PyroWave {
                        tracing::info!(
                            requested_kbps = req.bitrate_kbps,
                            pinned_kbps = session_bitrate_kbps,
                            "PyroWave session: mid-stream bitrate retarget refused (pinned)"
                        );
                        session_bitrate_kbps
                    } else {
                        let mut r = resolve_bitrate_kbps(req.bitrate_kbps);
                        // Encoder truth (§ABR overdrive): the ack below is the base the
                        // client's controller climbs from, so it must not promise past the
                        // encoder's discovered codec-level ceiling — the pre-fix path acked
                        // 1.01 Gbps while the ASIC ran 794 Mbps, and the controller climbed
                        // from the phantom number forever (a ~0.6 s rebuild + IDR per step).
                        let ceiling = encoder_ceiling_kbps.load(Ordering::Relaxed);
                        if ceiling != 0 && r > ceiling {
                            r = ceiling;
                        }
                        // Climb refusal while encode can't hold cadence: on a fat LAN no
                        // network signal ever stops the climb, and past the compute knee more
                        // bits only deepen the miss. Resolve a CLIMB to the current applied
                        // rate (descents pass — they're the cure); the short ack teaches the
                        // client controller its ceiling.
                        let live = live_bitrate.load(Ordering::Relaxed);
                        if cadence_degraded.load(Ordering::Relaxed) && live != 0 && r > live {
                            tracing::info!(
                                requested_kbps = req.bitrate_kbps,
                                held_kbps = live,
                                "bitrate climb refused — encode is behind cadence"
                            );
                            r = live;
                        }
                        r
                    };
                    tracing::debug!(
                        requested_kbps = req.bitrate_kbps,
                        resolved_kbps = resolved,
                        "mid-stream bitrate change requested"
                    );
                    let ack = BitrateChanged {
                        bitrate_kbps: resolved,
                    };
                    if io::write_msg(&mut ctrl_send, &ack.encode()).await.is_err() {
                        break;
                    }
                    if bitrate_tx.send(resolved).is_err() {
                        break; // data plane gone
                    }
                } else if let Ok(req) = ProbeRequest::decode(&msg) {
                    tracing::info!(
                        target_kbps = req.target_kbps,
                        duration_ms = req.duration_ms,
                        "speed-test probe requested"
                    );
                    if probe_tx.send(req).is_err() {
                        break; // data plane gone
                    }
                } else if let Ok(probe) = ClockProbe::decode(&msg) {
                    // Wall-clock skew handshake: echo the client's t1 with our receive (t2) and
                    // send (t3) stamps, both in the host clock the AU pts_ns uses. Answered
                    // inline on the control stream — cheap, no data-plane involvement.
                    let t2_ns = now_ns();
                    let echo = ClockEcho {
                        t1_ns: probe.t1_ns,
                        t2_ns,
                        t3_ns: now_ns(),
                    };
                    if io::write_msg(&mut ctrl_send, &echo.encode()).await.is_err() {
                        break;
                    }
                } else if let Ok(pr) = slipstream_core::quic::PhaseReport::decode(&msg) {
                    // Phase-locked capture: latest-wins into the bridge — no data-plane hop, the
                    // encode loop polls on its own cadence. Only vsync-aware presenters send
                    // these (CLIENT_CAP_PHASE_LOCK), and SLIPSTREAM_PHASE_LOCK=0 host-side leaves
                    // the stored report undrained/inert.
                    phase_ctl.store(pr);
                } else if let Ok(m) = slipstream_core::quic::CursorRenderMode::decode(&msg) {
                    // Who renders the pointer (design/remote-desktop-sweep.md §8): the client's
                    // mouse-model flip. Latest-wins into the shared flag; the data-plane loop
                    // edge-detects it per tick (forward+exclude vs composite). Inert for
                    // sessions that never negotiated the cursor cap.
                    cursor_client_draws.store(m.client_draws, Ordering::Relaxed);
                    tracing::info!(
                        client_draws = m.client_draws,
                        "cursor render mode set by client"
                    );
                } else if let Ok(ctl) = ClipControl::decode(&msg) {
                    // Shared clipboard enable/disable (design/clipboard-and-file-transfer.md
                    // §3.1). Reply with the resolved state; the operator policy is authoritative
                    // over the client's request. When the policy allows it but no backend bound
                    // (gamescope / older GNOME), enable is refused with BACKEND_UNAVAILABLE so the
                    // client can say *why*. The resolved `enabled` gates the coordinator.
                    let policy = ss_clipboard::policy();
                    let (enabled, resolved_policy, reason) = match policy {
                        None => (false, 0, slipstream_core::quic::CLIP_REASON_POLICY_DISABLED),
                        Some(p) if ctl.enabled && !clip_available => {
                            (false, p, slipstream_core::quic::CLIP_REASON_BACKEND_UNAVAILABLE)
                        }
                        Some(p) => {
                            let files_ok = p & slipstream_core::quic::CLIP_POLICY_FILES != 0;
                            let wants_files =
                                ctl.flags & slipstream_core::quic::CLIP_FLAG_FILES != 0;
                            let reason = if wants_files && !files_ok {
                                slipstream_core::quic::CLIP_REASON_NO_FILES
                            } else {
                                slipstream_core::quic::CLIP_REASON_OK
                            };
                            (ctl.enabled, p, reason)
                        }
                    };
                    clip_enabled.store(enabled, Ordering::SeqCst);
                    // Drive the coordinator: enable re-announces the current host clipboard,
                    // disable drops any selection we own. A dropped send (inert handle) is fine.
                    let _ = clip_cmd_tx.send(ClipCoordCmd::SetEnabled(enabled));
                    tracing::info!(
                        enabled,
                        files = enabled
                            && resolved_policy & slipstream_core::quic::CLIP_POLICY_FILES != 0,
                        "clipboard control"
                    );
                    let state = ClipState {
                        enabled,
                        policy: resolved_policy,
                        reason,
                    };
                    if io::write_msg(&mut ctrl_send, &state.encode()).await.is_err() {
                        break;
                    }
                } else if let Ok(offer) = ClipOffer::decode(&msg) {
                    // The client copied: hand its lazy format list to the coordinator, which
                    // installs a host-side source that fetches from the client on host paste.
                    tracing::debug!(
                        seq = offer.seq,
                        kinds = offer.kinds.len(),
                        "clipboard offer from client"
                    );
                    let mimes = offer.kinds.iter().map(|k| k.mime.clone()).collect();
                    let _ = clip_cmd_tx.send(ClipCoordCmd::RemoteOffer {
                        seq: offer.seq,
                        mimes,
                    });
                } else {
                    tracing::warn!("unknown control message — ignoring");
                }
            }
            result = probe_result_rx.recv() => {
                let Some(result) = result else { break }; // data plane gone
                // Phase 6: the probe burst MEASURES link capacity — record it for the
                // transport-state machine's next window.
                if result.duration_ms > 0 {
                    last_capacity_bps =
                        result.bytes_sent.saturating_mul(8) * 1000 / result.duration_ms as u64;
                }
                if io::write_msg(&mut ctrl_send, &result.encode()).await.is_err() {
                    break;
                }
            }
            shape = cursor_shape_rx.recv() => {
                // Cursor-forward bridge (M2): the encode loop diffed a new pointer bitmap.
                // Rare (shape changes are human-paced); ≤ ~58 KiB fits the u16 frame by
                // construction (cursor_fwd downscales).
                let Some(shape) = shape else { break }; // data plane gone
                if io::write_msg(&mut ctrl_send, &shape.encode()).await.is_err() {
                    break;
                }
            }
            offer = clip_offer_rx.recv(), if !clip_offer_closed => {
                // Host copied → the coordinator minted a `ClipOffer`; forward it to the client
                // (only while sync is on — a race with a just-received disable would otherwise
                // leak a stale offer). `None` = coordinator gone; disable this branch.
                match offer {
                    Some(offer) => {
                        if clip_enabled.load(Ordering::SeqCst)
                            && io::write_msg(&mut ctrl_send, &offer.encode()).await.is_err()
                        {
                            break;
                        }
                    }
                    None => clip_offer_closed = true,
                }
            }
            correction = reconfig_result_rx.recv() => {
                // H2 rollback/correction ack: the data plane reports the mode ACTUALLY live
                // after a rebuild that failed (stayed at the old mode) or that the backend
                // honored at a different refresh. Track it so a later rejection's
                // `mode: active` echo is truthful too.
                let Some(ack) = correction else { break }; // data plane gone
                active = ack.mode;
                if io::write_msg(&mut ctrl_send, &ack.encode()).await.is_err() {
                    break;
                }
            }
        }
    }
}

/// Feed the transport-state machine one measurement window (~750 ms cadence, driven by the
/// client's LossReports). RTT comes from the QUIC control connection (same path as the data
/// plane, measured — never assumed); variation is an EWMA of the RTT deltas; capacity comes
/// from the latest speed-test probe burst (0 = not yet measured). On a state change, the new
/// state's policy is published to the send path.
// Measurement-window state: three shared refs (state machine, policy, QUIC connection), three
// sample scalars, and three `&mut` EWMA/feed accumulators. Bundling them would hide which are
// mutated in place versus read.
#[allow(clippy::too_many_arguments)]
fn feed_transport(
    state: &Arc<std::sync::Mutex<crate::transport_state::TransportStateMachine>>,
    policy: &Arc<crate::transport_state::TransportPolicyShared>,
    conn: &quinn::Connection,
    loss_ppm: u64,
    capacity_bps: u64,
    target_bps: u64,
    last_feed: &mut std::time::Instant,
    rtt_ewma_ms: &mut f64,
    rtt_var_ms: &mut f64,
) {
    use crate::transport_state::{TransportPolicy, TransportSample};
    let now = std::time::Instant::now();
    let elapsed = now.duration_since(*last_feed);
    *last_feed = now;
    // Minimum spacing — don't classify on a burst of reports inside one window.
    if elapsed < std::time::Duration::from_millis(500) {
        return;
    }
    let rtt_ms = conn.rtt().as_secs_f64() * 1000.0;
    if *rtt_ewma_ms == 0.0 {
        *rtt_ewma_ms = rtt_ms;
        *rtt_var_ms = 0.0;
    } else {
        let d = (rtt_ms - *rtt_ewma_ms).abs();
        *rtt_var_ms = 0.8 * *rtt_var_ms + 0.2 * d;
        *rtt_ewma_ms = 0.8 * *rtt_ewma_ms + 0.2 * rtt_ms;
    }
    let mut machine = state.lock().unwrap_or_else(|e| e.into_inner());
    let before = machine.state();
    let after = machine.feed(
        TransportSample {
            rtt_ms: *rtt_ewma_ms,
            rtt_var_ms: *rtt_var_ms,
            loss_ppm,
            capacity_bps,
            target_bps,
        },
        elapsed,
    );
    if after != before {
        tracing::info!(
            from = before.label(),
            to = after.label(),
            rtt_ms = format!("{:.1}", *rtt_ewma_ms),
            var_ms = format!("{:.1}", *rtt_var_ms),
            loss_ppm,
            capacity_mbps = capacity_bps / 1_000_000,
            "transport state changed (measured)"
        );
        policy.apply(TransportPolicy::for_state(after));
    }
}
