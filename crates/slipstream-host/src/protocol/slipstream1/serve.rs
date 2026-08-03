//! One client session loop for the native `slipstream/1` host (plan §W1 — carved out of the
//! [`super`] module): pairing gate / delegated approval, handshake hand-off, input/audio planes,
//! data-plane spawn, and teardown. [`serve_session`] is the per-connection task `serve` spawns.

use super::*;

/// How a served connection ended. A peer that completes the QUIC handshake and closes cleanly
/// (code 0) without ever opening the control stream is a reachability probe (the clients'
/// hosts-page "online" pips / `--reachable`) or an abandoned connect — routine, and logged
/// quietly: as a WARN it buried the real failures in a wake-on-LAN triage log.
pub(super) enum Served {
    Session,
    ProbeClose,
}

/// One client session: handshake → input/audio planes → data plane until done/disconnect.
/// Everything torn down on return (RAII: virtual output, encoder, threads via channel close).
/// A connection whose first message is a PairRequest runs the pairing ceremony instead.
// Each argument is a distinct host-lifetime handle threaded from `serve` (config, the audio +
// injector services, the trust store, pairing state) — bundling them into a context struct would
// obscure more than it'd save.
#[allow(clippy::too_many_arguments)]
pub(super) async fn serve_session(
    conn: quinn::Connection,
    opts: &Slipstream1Options,
    audio_cap: &AudioCapSlot,
    inj_tx: std::sync::mpsc::Sender<InputEvent>,
    mic_tx: std::sync::mpsc::SyncSender<crate::audio::MicFrame>,
    host_fp: &[u8; 32],
    np: &NativePairing,
    last_pairing: &std::sync::Mutex<Option<std::time::Instant>>,
    stats: Arc<StatsRecorder>,
    // The session slot. Owned here (not just held by the spawning task) because an unpaired knock
    // RELEASES it while parked for delegated approval, then RE-ACQUIRES one on approval — so a
    // parked knock can't hold a streaming slot. `sem` is the pool it re-acquires from.
    mut permit: tokio::sync::OwnedSemaphorePermit,
    sem: Arc<tokio::sync::Semaphore>,
) -> Result<Served> {
    let peer = conn.remote_address();

    // First message decides what this connection is: a pairing ceremony or a session.
    let (mut send, mut recv) = match tokio::time::timeout(HANDSHAKE_TIMEOUT, conn.accept_bi())
        .await
        .map_err(|_| anyhow!("control stream timeout"))?
    {
        // A clean close before any control stream: a reachability probe / abandoned connect,
        // not a failed session (see [`Served::ProbeClose`]).
        Err(quinn::ConnectionError::ApplicationClosed(ref ac))
            if ac.error_code == quinn::VarInt::from_u32(0) =>
        {
            return Ok(Served::ProbeClose);
        }
        r => r.context("accept control stream")?,
    };
    let first = tokio::time::timeout(HANDSHAKE_TIMEOUT, io::read_msg(&mut recv))
        .await
        .map_err(|_| anyhow!("first message timeout"))??;
    if let Ok(req) = PairRequest::decode(&first) {
        // The client fingerprint (cert possession is proven by the QUIC handshake) is needed to honor
        // a fingerprint-bound PIN window (#9): a window the operator armed for a SPECIFIC device must
        // not be consumable — or burnable — by any other fingerprint.
        let Some(client_fp) = endpoint::peer_fingerprint(&conn) else {
            close_rejected(
                &conn,
                slipstream_core::reject::RejectReason::IdentityRequired,
            );
            anyhow::bail!("pairing requires the client to present a certificate");
        };
        let client_fp_hex = fingerprint_hex(&client_fp);
        // Resolve the live arming PIN per attempt (so a lapsed window no longer pairs), honoring any
        // fingerprint binding.
        let pin = match np.pin_for_attempt(&client_fp_hex) {
            crate::native_pairing::PinAttempt::Pin(pin) => pin,
            crate::native_pairing::PinAttempt::Disarmed => {
                close_rejected(
                    &conn,
                    slipstream_core::reject::RejectReason::PairingNotArmed,
                );
                anyhow::bail!(
                    "pairing not armed (arm it in the console, or start with --allow-pairing)"
                )
            }
            // Armed for a DIFFERENT device — reject without running the ceremony, so this attempt does
            // NOT consume (burn) the operator's window for the device they actually selected (#9).
            crate::native_pairing::PinAttempt::BoundToOther => {
                close_rejected(
                    &conn,
                    slipstream_core::reject::RejectReason::PairingBoundToOtherDevice,
                );
                anyhow::bail!(
                    "pairing is armed for a different device — this attempt does not consume the window"
                )
            }
        };
        {
            let mut last = last_pairing.lock().unwrap();
            if let Some(t) = *last {
                if t.elapsed() < PAIRING_COOLDOWN {
                    close_rejected(
                        &conn,
                        slipstream_core::reject::RejectReason::PairingRateLimited,
                    );
                    anyhow::bail!("pairing rate-limited — retry shortly");
                }
            }
            *last = Some(std::time::Instant::now());
        }
        return pair_ceremony(&conn, send, recv, req, host_fp, np, &pin)
            .await
            .map(|()| Served::Session);
    }

    // Pairing gate for a session Hello (a PairRequest was handled above). Lifted OUT of the
    // `handshake` future below for two reasons: (1) the approval wait must not be bound by the
    // short HANDSHAKE_TIMEOUT — a human reads the console and clicks Approve; (2) the NVENC session
    // permit is released while parked, so a knock awaiting approval can't hold a streaming slot.
    // On approval the device is now paired, so the handshake proceeds and the session starts with
    // NO client reconnect (delegated approval, roadmap §8b-1).
    if opts.require_pairing {
        // Decode just enough to gate (the Hello carries the device name for the pending label);
        // the `handshake` future re-decodes for the real session — a few dozen bytes, negligible.
        let gate_hello = Hello::decode(&first).map_err(|e| anyhow!("Hello decode: {e:?}"))?;
        if gate_hello.abi_version != slipstream_core::WIRE_VERSION {
            close_rejected(
                &conn,
                slipstream_core::reject::RejectReason::WireVersionMismatch,
            );
            anyhow::bail!(
                "wire version mismatch: client {} host {}",
                gate_hello.abi_version,
                slipstream_core::WIRE_VERSION
            );
        }
        let fp = endpoint::peer_fingerprint(&conn);
        let known = fp
            .as_ref()
            .map(|fp| np.is_paired(&fingerprint_hex(fp)))
            .unwrap_or(false);
        if !known {
            // An anonymous client (no certificate) has no identity to approve — reject outright
            // (the PIN ceremony is its way in). Mirrors the prior behavior for anonymous knocks.
            let Some(fp) = fp else {
                close_rejected(
                    &conn,
                    slipstream_core::reject::RejectReason::IdentityRequired,
                );
                anyhow::bail!(
                    "unpaired anonymous client rejected (this host requires pairing — present a \
                     client identity and approve it in the console, or run the PIN ceremony)"
                );
            };
            let fp_hex = fingerprint_hex(&fp);
            // Sanitize the wire-supplied name before it reaches the log / console (untrusted: an
            // unpaired device could embed terminal escapes / bidi overrides); note_pending stores
            // the same sanitized form and derives a fingerprint label when empty.
            let label = crate::native_pairing::sanitize_device_name(
                gate_hello.name.as_deref().unwrap_or(""),
                &fp_hex,
            );
            tracing::info!(name = %label, fingerprint = %fp_hex,
                "unpaired device knocked — parking connection for delegated approval in the console");
            // Record the QUIC-validated source IP so the pending queue's per-source cap can stop one
            // host from flooding/evicting genuine knocks (#13). The returned knock generation makes
            // this connection the ONE an approval admits — a retrying client parks a fresh
            // connection per knock, and admitting every parked sibling on a single Approve spun up
            // three concurrent Mutter virtual monitors and segfaulted gnome-shell (2026-07-10).
            let knock_seq = np.note_pending(&label, &fp_hex, Some(peer.ip()));
            // Free the session slot while a human decides — a parked knock must not hold an NVENC
            // permit (a handful of parked knocks would otherwise block every real session).
            drop(permit);
            let decision = tokio::select! {
                d = np.wait_for_decision(&fp_hex, knock_seq, PENDING_APPROVAL_WAIT) => d,
                // The client gave up (closed the connection) before a decision — stop waiting.
                _ = conn.closed() => anyhow::bail!("client disconnected before pairing approval"),
            };
            match decision {
                PairingDecision::Approved => {
                    tracing::info!(name = %label, fingerprint = %fp_hex,
                        "device approved in console — admitting session (no reconnect)");
                }
                PairingDecision::Denied => {
                    close_rejected(&conn, slipstream_core::reject::RejectReason::Denied);
                    anyhow::bail!("pairing request denied in the console")
                }
                PairingDecision::TimedOut => {
                    close_rejected(
                        &conn,
                        slipstream_core::reject::RejectReason::ApprovalTimeout,
                    );
                    anyhow::bail!(
                        "pairing request not approved within {PENDING_APPROVAL_WAIT:?} \
                         — the device can knock again"
                    )
                }
                PairingDecision::Superseded => {
                    close_rejected(&conn, slipstream_core::reject::RejectReason::Superseded);
                    anyhow::bail!(
                        "parked knock superseded by a newer connection from the same device — \
                         only the newest is admitted on approval"
                    )
                }
            }
            // Re-acquire a session slot for the now-approved session (waits if all slots are busy,
            // exactly like any freshly accepted client).
            permit = sem
                .clone()
                .acquire_owned()
                .await
                .expect("session semaphore is never closed");
        }
    }
    // Held for the rest of the session (RAII frees the slot on return). For an already-paired
    // client this is the original permit; for a just-approved knock it's the re-acquired one.
    let _permit = permit;

    let source = opts.source;
    let frames = opts.frames;
    let data_port = opts.data_port;
    // Session-transition trace (latency plan P0.1): zeroed here — the Hello is in hand, pairing
    // gates are behind us — and finished by the send thread when the FIRST video packet leaves.
    // The completed totals surface per session in `session_status` (→ mgmt `/status`).
    let bringup = crate::bringup::Trace::start("bringup", Arc::new(AtomicU32::new(0)));
    // The mid-stream resize counterpart: each accepted Reconfigure runs its own trace into this
    // shared slot (latest wins), registered alongside the bring-up total.
    let resize_ms: Arc<AtomicU32> = Arc::new(AtomicU32::new(0));

    // Stop signal: stream duration elapsed or the client went away. Created (with its watcher)
    // BEFORE the handshake so the Welcome-time display prep can already observe a client that
    // vanished mid-handshake (its build-retry loop aborts on `stop`).
    let stop = Arc::new(AtomicBool::new(false));
    // Deliberate-quit signal: set (before `stop`, so the display lease reads it on teardown) when
    // the client closed the connection with `QUIT_CODE` — a user "stop", which skips the
    // keep-alive linger. A bare disconnect / idle timeout leaves it false → the display lingers
    // for a reconnect.
    let quit = Arc::new(AtomicBool::new(false));
    {
        let stop = stop.clone();
        let quit = quit.clone();
        let conn = conn.clone();
        tokio::spawn(async move {
            let reason = conn.closed().await;
            if matches!(&reason, quinn::ConnectionError::ApplicationClosed(ac)
                if ac.error_code == quinn::VarInt::from_u32(QUIT_CODE))
            {
                quit.store(true, Ordering::SeqCst);
            }
            stop.store(true, Ordering::SeqCst);
        });
    }

    let (hello, welcome, udp_port, data_sock, direct, start, compositor, gamescope_route, prep) =
        tokio::time::timeout(
            HANDSHAKE_TIMEOUT,
            handshake::negotiate(
                &conn,
                &mut send,
                &mut recv,
                &first,
                source,
                frames,
                data_port,
                &bringup,
                quit.clone(),
                stop.clone(),
            ),
        )
        .await
        .map_err(|_| anyhow!("handshake timed out after {HANDSHAKE_TIMEOUT:?}"))??;
    let (ctrl_send, ctrl_recv) = (send, recv);
    // Can this session's backend live-reconfigure (mid-stream Reconfigure)? Gated OFF for:
    //   * gamescope (all sub-modes): a spawn respawn restarts the game, managed restarts the box's
    //     game-mode session, attach doesn't own the display — a resize must never relaunch the title
    //     (design/midstream-resolution-resize.md H1/D3). The client keeps scaling client-side.
    //   * an `identity: per-client-mode` policy: the mode is part of the display-identity slot key,
    //     so a resize would resolve a DIFFERENT slot — on Windows a fresh monitor ADD instead of the
    //     in-place reconfigure, on KWin a differently-named output — defeating the policy's
    //     per-resolution identity. Honest downgrade: reject, client scales (H5).
    //   * a monitor MIRROR (a `capture_monitor` pin): a physical head runs at the mode its owner set
    //     and the mirror backend ignores the requested one, so a resize would restart the identical
    //     cast at the identical size (design/per-monitor-portal-capture.md §7.3).
    // The SYNTHETIC source stays reconfigurable on purpose (nothing to rebuild — the ack round-trip
    // is the whole effect): it is the compositor-free protocol test source, and the C-ABI roundtrip
    // test + client harnesses exercise the Reconfigure/Reconfigured plumbing through it.
    // Captured once at session setup; the control task answers `accepted: false` when gated.
    let live_reconfig_ok = {
        let per_client_mode_identity = crate::vdisplay::policy::prefs()
            .configured_effective()
            .is_some_and(|e| e.identity == crate::vdisplay::policy::Identity::PerClientMode);
        // Read once here, like the identity above: this session opened its display under whatever
        // the pin said at bring-up, so a console change mid-session must not retroactively change
        // what THIS session answers a Reconfigure with. Linux-only because `vdisplay::open` only
        // routes to the mirror there — a pin left in a Windows host's settings streams nothing
        // different, and must not silently disable resize as a side effect.
        #[cfg(target_os = "linux")]
        let mirrored =
            source == Slipstream1Source::Virtual && crate::vdisplay::capture_monitor().is_some();
        #[cfg(not(target_os = "linux"))]
        let mirrored = false;
        reconfig_allowed(compositor, per_client_mode_identity, mirrored)
    };
    // Negotiated codec (HEVC / H.264 / AV1), derived from the Welcome. `Copy`, so the control task's
    // `async move` captures a copy and it stays usable for the data-plane SessionContext below.
    let codec = crate::encode::Codec::from_wire(welcome.codec);
    let client_udp = std::net::SocketAddr::new(peer.ip(), start.client_udp_port);
    tracing::info!(
        %client_udp,
        udp_port,
        mode = ?hello.mode,
        compositor = compositor.map(|c| c.id()).unwrap_or("none"),
        gamepad = welcome.gamepad.as_str(),
        "handshake complete — streaming"
    );

    // Control task: the handshake stream stays open for mid-stream renegotiation and speed
    // tests. A validated Reconfigure is acked, then handed to the data-plane thread, which
    // rebuilds capture/encoder/virtual output at the new mode (the data plane itself is
    // untouched). A ProbeRequest is handed to the data plane, which bursts FLAG_PROBE filler and
    // hands back a ProbeResult that this task writes to the client. The two control directions
    // (inbound requests, outbound probe results) are multiplexed with `select!`.
    let (reconfig_tx, reconfig_rx) = std::sync::mpsc::channel::<slipstream_core::Mode>();
    let (keyframe_tx, keyframe_rx) = std::sync::mpsc::channel::<()>();
    // Client LTR-RFI recovery: the control task forwards each `RfiRequest`'s lost-frame range here;
    // the encode loop prefers `Encoder::invalidate_ref_frames` (a clean re-anchor P-frame) over a
    // full IDR when the encoder supports it (native-AMF LTR / Windows NVENC).
    let (rfi_tx, rfi_rx) = std::sync::mpsc::channel::<(u32, u32)>();
    let (bitrate_tx, bitrate_rx) = std::sync::mpsc::channel::<u32>();
    // Encoder-truth bridge, data plane → control task (§ABR overdrive). The encode loop publishes
    // here; the control task reads at `SetBitrate`-resolve time, so the ack the client's
    // controller climbs from tracks what the encoder ACTUALLY does, not what was asked:
    // - `live_bitrate`: the encoder's applied rate (kbps) — also the send pacer's/console's view.
    // - `encoder_ceiling_kbps`: the discovered codec-level ceiling (0 = none discovered yet);
    //   resolves land at min(policy clamp, ceiling), so overshoots stop costing rebuilds.
    // - `cadence_degraded`: encode can't hold the frame cadence — a climb is refused (acked at
    //   the current rate); the network isn't the bottleneck, more bits are anti-medicine.
    // Plain atomics, not a channel: only the freshest value matters, and only at resolve time.
    let live_bitrate = Arc::new(AtomicU32::new(welcome.bitrate_kbps));
    let encoder_ceiling_kbps = Arc::new(AtomicU32::new(0));
    let cadence_degraded = Arc::new(AtomicBool::new(false));
    let (probe_tx, probe_rx) = std::sync::mpsc::channel::<ProbeRequest>();
    let (probe_result_tx, probe_result_rx) = tokio::sync::mpsc::unbounded_channel::<ProbeResult>();
    // Mode-switch outcome, data plane → control task (same pattern as `probe_result_tx`): the accept
    // ack is written BEFORE the rebuild, so a failed rebuild (host stays at the old mode) or a
    // backend that honored a different refresh must CORRECT the client's mode slot with a second
    // `Reconfigured { accepted: true, mode: <actually live> }` — the client handler treats any
    // accepted ack as "the active mode is now X" and fixes itself; old clients just log it.
    let (reconfig_result_tx, reconfig_result_rx) =
        tokio::sync::mpsc::unbounded_channel::<Reconfigured>();
    // Cursor-forward bridge (M2): the encode loop diffs each frame's cursor serial and hands
    // changed SHAPES here; the control task (the control stream's sole writer) sends them.
    // Same shape as `probe_result_tx`. Wired even when the channel wasn't negotiated — it
    // just never fires then.
    let (cursor_shape_tx, cursor_shape_rx) =
        tokio::sync::mpsc::unbounded_channel::<slipstream_core::quic::CursorShape>();
    // Negotiated cursor forwarding: the HOST_CAP_CURSOR bit the Welcome advertised, read back
    // rather than recomputed (`handshake::cursor_forward` computed it once, with the encoder
    // blend-capability gate — re-running it here could drift, and would re-probe).
    let cursor_forward = welcome.host_caps & slipstream_core::quic::HOST_CAP_CURSOR != 0;
    // Who renders the pointer RIGHT NOW (client `CursorRenderMode`, flipped live by the mouse-
    // model chord): `true` = client draws (exclude + forward), `false` = host composites (the
    // capture model). Starts true — the pre-message behavior for cap sessions. Control task
    // writes, data-plane loop edge-detects.
    let cursor_client_draws = Arc::new(AtomicBool::new(true));
    let cursor_client_draws_dp = cursor_client_draws.clone();
    // Adaptive FEC: the control task maps each client LossReport to a recovery percent and publishes
    // it here; the data-plane send loop reads + applies it per frame. Disabled (pinned) when
    // SLIPSTREAM_FEC_PCT is set. Seeded with the session's starting FEC so it's a no-op until a report.
    let adaptive_fec = fec_static_override().is_none();
    let fec_target = Arc::new(AtomicU8::new(welcome.fec.fec_percent));
    let fec_target_ctl = fec_target.clone();
    // Phase-locked capture bridge (design/phase-locked-capture.md): the control task stores the
    // client's PhaseReports here; the encode loop's controller drains them. Inert until a
    // vsync-aware client actually reports.
    let phase_ctl = Arc::new(stream::PhaseCtl::new());
    let phase_ctl_control = phase_ctl.clone();
    // The session's negotiated rate — the pin PyroWave retarget-refusals ack (§4.6).
    let session_bitrate_kbps = welcome.bitrate_kbps;
    // Shared-clipboard enable state (client `ClipControl` → host). The coordinator reads it to
    // decide whether to forward host copies; the control task flips it on each `ClipControl`.
    let clip_enabled = Arc::new(AtomicBool::new(false));
    // Start the host clipboard coordinator. On success it watches the session clipboard, forwards
    // host copies as `ClipOffer`s (`clip.offer_rx` → control task → client), installs client
    // offers as a lazy source, and owns the fetch-stream accept loop. `available` is false when
    // there's no backend (gamescope / older GNOME / an unsupported platform) — the control task
    // then answers `ClipControl` with `BACKEND_UNAVAILABLE` and the decline loop below handles
    // stray fetch streams.
    let clip = ss_clipboard::start(conn.clone(), clip_enabled.clone(), compositor.is_some()).await;
    let clip_available = clip.available;
    // Phase 6: the measured transport-state machine (control task feeds it, send path reads
    // its policy). One per session — the classification is link-private.
    let transport_state =
        Arc::new(std::sync::Mutex::new(crate::transport_state::TransportStateMachine::default()));
    let transport_policy = Arc::new(crate::transport_state::TransportPolicyShared::from_env());
    tokio::spawn(control::run(
        ctrl_send,
        ctrl_recv,
        hello.mode,
        codec,
        live_reconfig_ok,
        adaptive_fec,
        session_bitrate_kbps,
        live_bitrate.clone(),
        encoder_ceiling_kbps.clone(),
        cadence_degraded.clone(),
        fec_target_ctl,
        phase_ctl_control,
        conn.clone(),
        transport_state.clone(),
        transport_policy.clone(),
        reconfig_tx,
        keyframe_tx,
        rfi_tx,
        bitrate_tx,
        probe_tx,
        probe_result_rx,
        reconfig_result_rx,
        cursor_shape_rx,
        cursor_client_draws,
        clip_enabled,
        clip,
    ));
    // Fetch streams with no backend behind them are answered `CLIP_FETCH_UNAVAILABLE` instead of
    // hanging (the coordinator owns `accept_bi` when a backend is live — exactly one consumer).
    if !clip_available && ss_clipboard::enabled() {
        ss_clipboard::spawn_decline_loop(conn.clone());
    }

    // Input plane: QUIC datagrams → channel → a native per-session thread. Pointer/keyboard
    // events are forwarded to the host-lifetime [`InjectorService`] (`inj_tx`) so the portal
    // grant persists across sessions; this thread owns the session's virtual gamepads (uinput,
    // per-session) and sends force feedback back over `conn`. It exits when the channel closes
    // (datagram task ends on disconnect) — fresh gamepad state per session.
    //
    // ONE channel for both event kinds deliberately: rich input (gyro at the pad's report
    // rate) used to ride a second channel that the thread only drained after the main
    // channel's 4 ms recv timeout — every motion sample of a pure-gyro aim (no button
    // traffic) ate up to 4 ms of added latency/jitter. A single channel wakes the thread on
    // whichever arrives.
    let (input_tx, input_rx) = std::sync::mpsc::channel::<ClientInput>();
    let rich_tx = input_tx.clone();
    // The stream loop's handle into the same pipeline: it parks the seat pointer on the
    // streamed surface (stream.rs `park_pointer`) through exactly the path client input takes.
    #[cfg(target_os = "linux")]
    let input_tx_stream = input_tx.clone();
    let input_handle = {
        let conn = conn.clone();
        let gamepad = welcome.gamepad;
        std::thread::Builder::new()
            .name("slipstream1-input".into())
            .spawn(move || input_thread(input_rx, conn, inj_tx, gamepad))
            .context("spawn input thread")?
    };
    // One reader for ALL client→host datagrams, demuxed by magic byte (two read_datagram loops
    // would race for datagrams): 0xCB → mic uplink (Opus, forwarded to the host-lifetime mic
    // service), 0xCC → rich input (DualSense touchpad / motion, to the per-session input thread),
    // 0xC8 → input (also the input thread). The magics are disjoint, so decode order doesn't
    // matter. Unknown tags are ignored.
    let input_conn = conn.clone();
    tokio::spawn(async move {
        let (mut input_count, mut mic_count, mut rich_count) = (0u64, 0u64, 0u64);
        while let Ok(d) = input_conn.read_datagram().await {
            if let Some((seq, pts, opus)) = slipstream_core::quic::decode_mic_datagram(&d) {
                mic_count += 1;
                // Host-lifetime mic service (bounded queue): `try_send` drops the frame when the
                // service is full or gone, never blocking this datagram loop (security-review S6).
                // seq + pts ride along — the pump's de-jitter reorders, conceals losses and
                // tracks cadence with them (they used to be decoded here and thrown away).
                let _ = mic_tx.try_send(crate::audio::MicFrame {
                    seq,
                    pts_ns: pts,
                    opus: opus.to_vec(),
                });
            } else if let Some(rich) = slipstream_core::quic::RichInput::decode(&d) {
                rich_count += 1;
                if rich_tx.send(ClientInput::Rich(rich)).is_err() {
                    break;
                }
            } else if let Some(pen) = slipstream_core::quic::PenBatch::decode(&d) {
                // 0xCC kind 0x05 — the stylus plane (RichInput::decode returns None for it by
                // design; see slipstream_core::quic::pen). Routed to the same input thread,
                // which owns the per-session tracker + virtual tablet.
                rich_count += 1;
                if rich_tx.send(ClientInput::Pen(pen)).is_err() {
                    break;
                }
            } else if let Some(mut ev) = InputEvent::decode(&d) {
                input_count += 1;
                // Wire hygiene: KEY_FLAG_SEMANTIC_VK is an in-process tag (GameStream ingest
                // only) — strip it from network events so a client can't flip the host's
                // key-decoding convention. Other kinds keep flags verbatim (MouseMoveAbs packs
                // its reference extent there).
                if matches!(
                    ev.kind,
                    slipstream_core::input::InputKind::KeyDown
                        | slipstream_core::input::InputKind::KeyUp
                ) {
                    ev.flags &= !crate::inject::KEY_FLAG_SEMANTIC_VK;
                }
                if input_tx.send(ClientInput::Event(ev)).is_err() {
                    break;
                }
            }
        }
        tracing::info!(
            input = input_count,
            mic = mic_count,
            rich = rich_count,
            "client datagram stream ended"
        );
    });

    // (The stop/quit flags + their disconnect watcher are created above, before the handshake, so
    // the Welcome-time display prep can observe a mid-handshake disconnect.)
    // Lifecycle events (RFC §4): this point — handshake complete, pairing/admission passed — is
    // where the client counts as CONNECTED; the close watcher below pairs it with the
    // disconnect + its decoded reason. A client rejected earlier never emits either.
    let event_client = crate::events::ClientRef {
        name: hello.name.clone().unwrap_or_default(),
        fingerprint: endpoint::peer_fingerprint(&conn).map(|fp| fingerprint_hex(&fp)),
        plane: crate::events::Plane::Native,
    };
    crate::events::emit(crate::events::EventKind::ClientConnected {
        client: event_client.clone(),
    });
    {
        let conn = conn.clone();
        tokio::spawn(async move {
            let reason = conn.closed().await;
            let why = match &reason {
                quinn::ConnectionError::ApplicationClosed(ac)
                    if ac.error_code == quinn::VarInt::from_u32(QUIT_CODE) =>
                {
                    crate::events::DisconnectReason::Quit
                }
                quinn::ConnectionError::TimedOut => crate::events::DisconnectReason::Timeout,
                _ => crate::events::DisconnectReason::Error,
            };
            crate::events::emit(crate::events::EventKind::ClientDisconnected {
                client: event_client,
                reason: why,
            });
        });
    }

    // Register this now-live session for mode-conflict admission (Stage 4): carry its identity, the
    // negotiated mode, and its stop flag so a LATER connecting client's admission can see it and
    // (under `steal`) signal it. The guard removes the entry when this session ends.
    let _live_guard = {
        let id = endpoint::peer_fingerprint(&conn);
        let label = id
            .map(|fp| {
                fp.iter()
                    .take(4)
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            })
            .unwrap_or_else(|| "client".to_string());
        crate::vdisplay::admission::register(
            id,
            (
                welcome.mode.width,
                welcome.mode.height,
                welcome.mode.refresh_hz,
            ),
            stop.clone(),
            label,
        )
    };

    // Audio plane (virtual source only — synthetic runs are protocol tests): desktop Opus
    // → host→client QUIC datagrams, on its own native thread. Best-effort on every failure
    // (no PipeWire audio, spawn error): the session continues without audio — and a spawn
    // error must NOT early-return here, the threads above are already running.
    let audio_handle = if opts.source == Slipstream1Source::Virtual {
        let conn = conn.clone();
        let stop = stop.clone();
        let cap = audio_cap.clone();
        let channels = welcome.audio_channels;
        let transport_policy_audio = transport_policy.clone();
        std::thread::Builder::new()
            .name("slipstream1-audio".into())
            .spawn(move || audio_thread(conn, stop, cap, channels, transport_policy_audio))
            .map_err(|e| tracing::warn!(error = %e, "audio thread spawn failed — session continues without audio"))
            .ok()
    } else {
        None
    };

    // HDR static metadata (ST.2086 mastering + CEA-861.3 content light level), host → client, sent
    // once at session start when an HDR session was negotiated, as a generic HDR10 baseline. The
    // virtual-source stream loop then sends the source display's REAL mastering metadata (Windows
    // GetDesc1) as soon as capture starts and re-sends it on keyframes; the client applies the
    // latest it receives. This baseline covers the synthetic source and the pre-capture gap.
    if welcome.color.is_hdr() {
        // Prefer the CLIENT's own display volume (Hello::display_hdr): the virtual display's EDID
        // now advertises it, so host apps tone-map to exactly that volume — echoing it here keeps
        // the mastering metadata honest end-to-end. Generic HDR10 only for older clients.
        let meta = hello
            .display_hdr
            .unwrap_or_else(ss_frame::hdr::generic_hdr10);
        let _ = conn.send_datagram(slipstream_core::quic::encode_hdr_meta_datagram(&meta).into());
        tracing::info!(
            client_volume = hello.display_hdr.is_some(),
            "sent HDR10 static metadata (0xCE baseline)"
        );
    }

    // Test hook (synthetic source only): a scripted feedback burst on the host→client
    // planes — rumble (0xCA) + DualSense HID-output (0xCD) — so loopback tests can assert
    // the client's feedback path without a real game writing output reports to a real pad.
    if opts.source == Slipstream1Source::Synthetic
        && std::env::var("SLIPSTREAM_TEST_FEEDBACK").as_deref() == Ok("1")
    {
        use slipstream_core::quic::HidOutput;
        // v2 envelope (seq 0, 400 ms TTL) so the loopback/probe assertion covers the self-
        // terminating tail, not just the level.
        let d = slipstream_core::quic::encode_rumble_datagram_v2(0, 0x4000, 0x8000, 0, 400);
        let _ = conn.send_datagram(d.to_vec().into());
        for h in [
            HidOutput::Led {
                pad: 0,
                r: 10,
                g: 20,
                b: 30,
            },
            HidOutput::PlayerLeds {
                pad: 0,
                bits: 0b00100,
            },
            HidOutput::Trigger {
                pad: 0,
                which: 1,
                effect: vec![0x21, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            },
        ] {
            let _ = conn.send_datagram(h.encode().into());
        }
        tracing::info!("SLIPSTREAM_TEST_FEEDBACK: scripted rumble + hidout burst sent");
    }

    // Data plane on a native thread (no async on the hot path — design invariant).
    let cfg = welcome.session_config(Role::Host);
    let source = opts.source;
    let (seconds, frames) = (opts.seconds, opts.frames);
    let mode = hello.mode;
    // Script-facing runtime marker: `$XDG_RUNTIME_DIR/slipstream/stream` exists (with this session's
    // negotiated mode) for exactly as long as this session streams. Held by RAII to session end, so
    // every exit path — clean disconnect, error, panic-unwind — retracts it. Lets a launch wrapper
    // branch "streaming → run the game as-is; not → my local multi-head gamescope" (see the module).
    let _stream_marker = crate::stream_marker::announce(crate::stream_marker::StreamInfo {
        width: mode.width,
        height: mode.height,
        refresh_hz: mode.refresh_hz,
        hdr: welcome.color.is_hdr(),
        client: hello.name.clone().unwrap_or_default(),
        launch: hello.launch.clone(),
        plane: crate::events::Plane::Native,
    });
    // GPU clock pin (Linux, opt-in `SLIPSTREAM_PIN_CLOCKS`): hold the box-wide vendor clock floor for
    // as long as THIS session streams, refcounted with every other live session across both planes.
    // RAII like the marker above — armed on the first live client, released when the last one
    // disconnects, so idle clocks aren't pinned while nobody is connected. No-op off Linux / when
    // the flag is unset.
    #[cfg(target_os = "linux")]
    let _clock_pin = crate::gpuclocks::session_pin();
    // The session's launch, threaded into the data plane. Windows carries the store-qualified id
    // (spawned into the interactive user session once capture is live); other hosts resolve the id
    // to its shell command HERE against the host's own library — a client can only ever pick an
    // existing title, never send a command — and the data plane runs it per-backend (nested into a
    // bare-spawn gamescope, or spawned into the live session once capture is up).
    // ONE library lookup for the whole session: enumerating the installed stores touches every
    // launcher's on-disk metadata, and the data plane needs three things out of it — what to run, what
    // to call the title, and how to recognize its process once a launcher has handed off
    // (design/session-game-lifetime.md §4).
    let launch_target =
        hello
            .launch
            .as_deref()
            .and_then(|id| match crate::library::resolve_launch(id) {
                Some(t) => {
                    tracing::info!(
                        launch_id = id,
                        title = %t.game.title,
                        command = t.command.as_deref().unwrap_or("-"),
                        "resolved library launch for this session"
                    );
                    Some(t)
                }
                None => {
                    tracing::warn!(
                        launch_id = id,
                        "client requested a launch id not in this host's library — ignoring"
                    );
                    None
                }
            });
    #[cfg(target_os = "windows")]
    let launch_for_dp = launch_target.as_ref().and(hello.launch.clone());
    #[cfg(not(target_os = "windows"))]
    let launch_for_dp = launch_target.as_ref().and_then(|t| t.command.clone());
    // A client reconnecting inside its game's reconnect window takes the game back: nothing is ended,
    // and this session adopts it. Matched on (this client, this title) so it can only ever reclaim its
    // own game.
    if let Some(target) = launch_target.as_ref() {
        let fp = slipstream_core::quic::endpoint::peer_fingerprint(&conn).map(hex::encode);
        crate::gamelease::readopt(fp.as_deref(), target.game.id.as_deref());
    }
    // Per-title prep steps (RFC §6) for a launched CUSTOM library title: run synchronously
    // before the data plane starts (so before the display opens and the title spawns); the
    // guard's drop — any serve_session exit — runs the undos in reverse, best-effort.
    // `block_in_place`: prep is blocking operator code and this is a multi-thread runtime;
    // the closure only runs when the title actually has prep steps.
    let _prep = hello.launch.as_deref().and_then(|id| {
        let cmds = crate::library::prep_for(id);
        let env = [("PF_APP_ID".to_string(), id.to_string())];
        (!cmds.is_empty())
            .then(|| tokio::task::block_in_place(|| crate::hooks::run_prep(&cmds, &env)))
    });
    let bitrate_kbps = welcome.bitrate_kbps; // resolved encoder bitrate (Hello clamped, or default)
                                             // "Automatic" request: the resolved rate is a host default — for PyroWave a per-mode
                                             // bpp pin the data plane re-resolves on a mid-stream mode switch.
    let bitrate_auto = hello.bitrate_kbps == 0;
    let bit_depth = welcome.bit_depth; // resolved encode bit depth (8, or 10 when negotiated)
                                       // Resolved chroma — derive the typed value back from the wire byte the Welcome carried (so the
                                       // session uses exactly what the client was told). `Yuv444` only when the handshake gate passed.
    let chroma = if welcome.chroma_format == slipstream_core::quic::CHROMA_IDC_444 {
        crate::encode::ChromaFormat::Yuv444
    } else {
        crate::encode::ChromaFormat::Yuv420
    };
    let stop_stream = stop.clone();
    let quit_stream = quit.clone();
    // The client display's HDR volume (Hello): the virtual display's EDID advertises it (host apps
    // tone-map to the client's real panel) and the 0xCE mastering metadata echoes it. `None` =
    // older client / no HDR display → the built-in defaults everywhere.
    let client_hdr = hello.display_hdr;
    let fec_target_dp = fec_target.clone(); // data-plane handle to the adaptive-FEC target
    let conn_stream = conn.clone(); // for sending the source's real HDR metadata (0xCE) mid-stream
                                    // Per-AU host-timing emission (0xCF): only when the client advertised the cap bit. All
                                    // first-party clients do (the core connector ORs it in); an older client leaves it clear
                                    // and gets no extra datagrams.
    let timing_conn = (hello.video_caps & slipstream_core::quic::VIDEO_CAP_HOST_TIMING != 0)
        .then(|| conn.clone());
    // Probe-sequence capability: the client reassembles speed-test filler in its own index window,
    // so mid-session bursts don't consume video frame indexes. An older client (bit clear) gets
    // mid-session probes declined instead — see `run_probe_burst`.
    let probe_seq = hello.video_caps & slipstream_core::quic::VIDEO_CAP_PROBE_SEQ != 0;
    // Streamed-AU capability: the client's reassembler accepts sentinel-headed streamed blocks,
    // so a chunked encoder session may ship an AU's early FEC blocks while its tail encodes.
    let streamed_au = hello.video_caps & slipstream_core::quic::VIDEO_CAP_STREAMED_AU != 0;
    // Multi-slice capability: the client's DECODER accepts AUs carrying several slice NALs, so
    // the encoder may keep its multi-slice default (§7 LN1). Absent ⇒ single-slice frames —
    // TV-SoC decoders (Amlogic: Chromecast with Google TV) wedge the device on multi-slice AUs.
    let multi_slice = hello.video_caps & slipstream_core::quic::VIDEO_CAP_MULTI_SLICE != 0;
    let stats_dp = stats; // data-plane handle to the shared stats recorder
                          // Short label for web-console stats captures: the client's cert-fingerprint prefix, else its
                          // peer IP (no fingerprint = anonymous TOFU/--open client).
    let client_label = endpoint::peer_fingerprint(&conn)
        .map(|fp| fingerprint_hex(&fp)[..12].to_string())
        .unwrap_or_else(|| conn.remote_address().ip().to_string());
    // The client's DISPLAY name for the status surface (local summary → the tray's connect
    // toast): the trust store's operator-curated name for this fingerprint first (a rename at
    // approval time wins over whatever the device calls itself), else the sanitized Hello name.
    // `None` (nameless knock from an old client / Android) keeps the summary name-free.
    let client_name = endpoint::peer_fingerprint(&conn)
        .map(|fp| fingerprint_hex(&fp))
        .and_then(|fp_hex| {
            np.list()
                .into_iter()
                .find(|c| c.fingerprint == fp_hex)
                .map(|c| c.name)
                .or_else(|| {
                    let raw = hello.name.as_deref().unwrap_or("").trim();
                    (!raw.is_empty())
                        .then(|| crate::native_pairing::sanitize_device_name(raw, &fp_hex))
                })
        });
    // Transition-trace handles for the data plane (P0.1): the punch stamp + the virtual-stream
    // stages ride the same per-session trace; resizes write their totals into the shared slot.
    let bringup_dp = bringup.clone();
    let resize_ms_dp = resize_ms.clone();
    let result: Result<()> = async {
        let stream_thread = tokio::task::spawn_blocking(move || -> Result<()> {
            // Bring up the (already-bound) data-plane socket. Default: hole-punch — wait briefly
            // for the client's punch, then stream to its OBSERVED source, so video traverses a
            // NAT / stateful inter-VLAN firewall (control + side planes ride the client-initiated
            // QUIC, but the raw video UDP needs the client to open the path first); falls back to
            // the reported address for clients that don't punch (flat-LAN, unchanged). With a fixed
            // `--data-port` (`direct`), skip the punch-wait and stream straight to the reported
            // address — the operator declared a reachable, firewall-opened port, so there's no
            // punch-timeout to pay. (Direct trusts the reported port: it can't cross a client-side
            // NAT that remaps it.)
            let bound = if direct {
                UdpTransport::from_socket(data_sock, &client_udp.to_string()).map(|t| (t, false))
            } else {
                UdpTransport::from_socket_punch(
                    data_sock,
                    &client_udp.to_string(),
                    // Only honour a punch from the peer QUIC already authenticated: the punch is
                    // there to discover the NAT-remapped *port*, and `client_udp`'s IP is the
                    // host-observed QUIC remote (only its port is client-reported).
                    client_udp.ip(),
                    std::time::Duration::from_millis(2500),
                )
            };
            let (transport, punched) = match bound {
                Ok(v) => v,
                Err(e) => {
                    // Surface the failure here directly: a data-plane bind error would otherwise be
                    // reported only after teardown (and a teardown stall could swallow it entirely).
                    tracing::error!(error = %e, %client_udp, udp_port, "data-plane socket setup failed");
                    return Err(anyhow::Error::new(e)).context("bind data plane");
                }
            };
            bringup_dp.mark("punch_done");
            tracing::info!(
                %client_udp,
                udp_port,
                direct,
                punched,
                "data plane bound (direct=true → fixed --data-port, streaming to the reported \
                 address with no hole-punch; else punched=true → the client's observed source, \
                 false → no punch seen, the reported address)"
            );
            let mut session = Session::new(cfg, Box::new(transport))
                .map_err(|e| anyhow!("host session: {e:?}"))?;
            match source {
                Slipstream1Source::Synthetic => synthetic_stream(
                    &mut session,
                    frames,
                    &stop_stream,
                    &probe_rx,
                    &probe_result_tx,
                    &fec_target_dp,
                    timing_conn.as_ref(),
                    probe_seq,
                ),
                Slipstream1Source::Virtual => {
                    let compositor = compositor
                        .expect("the Virtual source resolves a compositor during the handshake");
                    let ctx = SessionContext {
                        session,
                        mode,
                        seconds,
                        stop: stop_stream,
                        quit: quit_stream,
                        reconfig: reconfig_rx,
                        keyframe: keyframe_rx,
                        rfi: rfi_rx,
                        bitrate_rx,
                        compositor,
                        transport_state: transport_state.clone(),
                        transport_policy: transport_policy.clone(),
                        gamescope_route,
                        bitrate_kbps,
                        live_bitrate,
                        encoder_ceiling_kbps,
                        cadence_degraded,
                        bitrate_auto,
                        bit_depth,
                        chroma,
                        codec,
                        probe_rx,
                        probe_result_tx,
                        reconfig_result_tx,
                        fec_target: fec_target_dp,
                        phase: phase_ctl,
                        conn: conn_stream,
                        timing_conn,
                        cursor_forward,
                        cursor_shape_tx,
                        cursor_client_draws: cursor_client_draws_dp,
                        probe_seq,
                        streamed_au,
                        multi_slice,
                        stats: stats_dp,
                        client_label,
                        client_name,
                        launch: launch_for_dp,
                        launch_target,
                        client_hdr,
                        bringup: bringup_dp,
                        resize_ms: resize_ms_dp,
                        #[cfg(target_os = "linux")]
                        input_tx: input_tx_stream,
                    };
                    match prep {
                        // P1.1: the display prep started at Welcome on its own thread — hand it
                        // the post-punch context and adopt its result as the stream result (that
                        // thread runs `virtual_stream` on the pipeline it already built).
                        Some((ctx_tx, prep_thread)) => match ctx_tx.send(ctx) {
                            Ok(()) => match prep_thread.join() {
                                Ok(r) => r,
                                Err(_) => Err(anyhow!("prepared stream thread panicked")),
                            },
                            // The prep thread died before the hand-off (panicked during prep —
                            // its guard/lease unwound): run the stream inline instead.
                            Err(std::sync::mpsc::SendError(ctx)) => {
                                tracing::warn!(
                                    "display-prep thread gone before hand-off — building inline"
                                );
                                virtual_stream(ctx, None)
                            }
                        },
                        None => virtual_stream(ctx, None),
                    }
                }
            }
        });
        // `stop` is only ADVISORY: the stream thread observes it between iterations, so a call that
        // blocks without a bound INSIDE one (a compositor CLI that never returns, a D-Bus round-trip
        // on a stuck bus, a driver wait on a hung GPU) never reaches the check — and nothing else
        // can end the session, because every teardown below runs only once this await resolves. That
        // made one stuck syscall a permanent zombie: it kept its semaphore slot (four of them and the
        // host stops accepting entirely), its admission entry (a later client gets "host busy"
        // forever) and its stream marker, and even the console's Stop button — which just sets this
        // same flag — could not clear it.
        //
        // So bound the wait: once the session HAS been told to stop, give the thread
        // `STREAM_STOP_GRACE` to return, then stop waiting for it and let teardown run. The thread is
        // detached, not killed (a blocking thread can't be cancelled in Rust) — it keeps its capturer
        // and encoder until the stuck call returns, and its own guards unwind if it ever does. That
        // is a leak, but a bounded one: the session's slot and admission entry come back, so the rest
        // of the host keeps serving.
        tokio::select! {
            joined = stream_thread => joined.context("stream thread")??,
            () = stop_overdue(&stop) => {
                tracing::error!(
                    grace_s = STREAM_STOP_GRACE.as_secs(),
                    "stream thread has not returned since the session was stopped — abandoning it so \
                     the session slot is freed. Its capture/encoder stay held until the stuck call \
                     returns; this is a HOST WEDGE — please report it with the log above"
                );
                anyhow::bail!("stream thread wedged after stop");
            }
        }
        // Give the client a moment to drain before the close.
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        Ok(())
    }
    .await;

    // Teardown on EVERY path (a failed data plane must not leave the connection open with
    // audio still streaming): stop the audio thread, close, then join both side-plane
    // threads so the next session starts fresh (closing the connection ends the datagram
    // task, which drops the input channel, which exits the input thread + its gamepads).
    stop.store(true, Ordering::SeqCst);
    conn.close(
        if result.is_ok() { 0u32 } else { 1u32 }.into(),
        if result.is_ok() { b"done" } else { b"error" },
    );
    // Bounded, for the same reason the stream-thread wait is: the input thread exits only when the
    // datagram task drops its channel, which the `conn.close()` above forces — but a join is the
    // last unbounded await in teardown, and one stuck side thread must not hold the session's
    // permit/admission entry (released when this fn returns) hostage.
    let side_threads = tokio::task::spawn_blocking(move || {
        if let Some(h) = audio_handle {
            let _ = h.join();
        }
        let _ = input_handle.join();
    });
    if tokio::time::timeout(SIDE_THREAD_JOIN_GRACE, side_threads)
        .await
        .is_err()
    {
        tracing::warn!(
            grace_s = SIDE_THREAD_JOIN_GRACE.as_secs(),
            "audio/input threads did not exit after the connection closed — detaching them"
        );
    }
    // The capture (and our gamescope session's VirtualOutput) are gone by here. If this was the
    // host-managed gamescope path on a box that autologs into gaming mode (Bazzite default), put the
    // TV's gaming session back so it's the default when no one is streaming.
    crate::vdisplay::restore_managed_session();
    result.map(|()| Served::Session)
}
