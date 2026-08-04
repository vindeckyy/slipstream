//! The video data plane: on RTSP PLAY, learn the client's UDP endpoint (it pings the video
//! port), then run capture → NVENC encode → [`VideoPacketizer`] → UDP send. The source is
//! either real portal desktop capture (`SLIPSTREAM_VIDEO_SOURCE=portal`, the portal PipeWire path) or
//! a synthetic test pattern (default). Runs on its own native thread.

// Every `unsafe` block in this file carries a `// SAFETY:` proof; enforce it.
#![deny(clippy::undocumented_unsafe_blocks)]

use super::VIDEO_PORT;
use crate::capture::{self, Capturer, FastSyntheticCapturer};
use crate::encode::{self, Codec};
use anyhow::{Context, Result};
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Negotiated video parameters from the RTSP ANNOUNCE.
#[derive(Clone, Copy, Debug)]
pub struct StreamConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub packet_size: usize,
    pub bitrate_kbps: u32,
    pub codec: Codec,
    /// Client's `x-nv-vqos[0].fec.minRequiredFecPackets` — parity floor per FEC block.
    pub min_fec: u8,
    /// Client requested HDR (`dynamicRangeMode != 0`) AND the host can deliver it ([`host_hdr_capable`]).
    /// Drives the capturer's proactive advanced-color enable; the encoder picks Main10 from the captured
    /// (P010) frame format. Always `false` on a non-HDR host, so the SDR path is unchanged.
    pub hdr: bool,
    /// Client's `x-nv-video[0].videoEncoderSlicesPerFrame` — the per-frame slice count its
    /// decoder wants (moonlight-common-c requests 1 for every HARDWARE decoder and 4 only for
    /// software slice-threading). Honored as the encoder's slicing ceiling: hardware TV decoders
    /// (Amlogic — Chromecast with Google TV) wedge the whole device on multi-slice AUs they
    /// never asked for (the 0.17.0 4-slice-default field regression). Absent ⇒ 1.
    pub slices: u32,
}

/// A pooled capturer plus the three properties reuse must match on — its HDR-ness, its
/// metadata-cursor mode (both fixed at PipeWire-negotiation time) and **which screen it is
/// actually capturing**: the `capture_monitor` pin, or `None` for the portal's own pick. A
/// mismatch on any of them needs a fresh screencast session (see `AppState::video_cap`).
///
/// The pin belongs in the key because it is a *live* setting — the console can re-aim the host
/// between two GameStream connects (`design/per-monitor-portal-capture.md` §7.3). Without it the
/// second connect would silently keep streaming the previous screen, which is the exact failure
/// the pin exists to prevent.
pub type PooledCapturer = (Box<dyn Capturer>, bool, bool, Option<String>);

/// Slot for the persistent screen capturer, shared with the control plane and reused across
/// streams so a reconnect doesn't open a second (conflicting) screencast session.
pub type CapturerSlot = Arc<std::sync::Mutex<Option<PooledCapturer>>>;

/// A pending client reference-frame-invalidation range (lost `firstFrame..=lastFrame`), set by the
/// control plane and drained by the video thread (see [`AppState::rfi_range`](super::AppState)).
pub type RfiSlot = Arc<std::sync::Mutex<Option<(i64, i64)>>>;

/// What the stream thread needs to give the launched game a lifetime
/// (design/session-game-lifetime.md). Bundled because the three only exist together — the control
/// plane builds them from the live `AppState` at RTSP PLAY, and the stream thread is where they are
/// spent.
pub struct GameLifetime {
    /// The session's deliberate-quit flag ([`super::AppState::quit`]), read when the stream ends: a
    /// decision may end the game, a drop gets a reconnect window first.
    pub quit: Arc<AtomicBool>,
    /// Hex cert fingerprint of the paired client that owns the launch, so only it can reclaim its own
    /// game. `None` when the peer cert couldn't be read.
    pub fingerprint: Option<String>,
    /// Ends the whole session, deliberately — the action for "the launched game exited".
    pub on_game_exit: super::OnSessionLost,
}

/// Spawn the video stream thread (idempotent via `running`). Stops when `running` clears.
/// `force_idr` is set by the control stream on a client recovery request; `video_cap` holds
/// the persistent capturer the thread borrows for the stream's duration.
#[allow(clippy::too_many_arguments)]
pub fn start(
    cfg: StreamConfig,
    app: Option<super::apps::AppEntry>,
    running: Arc<AtomicBool>,
    force_idr: Arc<AtomicBool>,
    rfi_range: RfiSlot,
    video_cap: CapturerSlot,
    stats: Arc<crate::stats_recorder::StatsRecorder>,
    on_lost: super::OnSessionLost,
    life: GameLifetime,
) {
    let _ = std::thread::Builder::new()
        .name("slipstream-video".into())
        .spawn(move || {
            // Same scheduling posture as the native path's capture/encode thread (Linux nice -10 /
            // Windows HIGHEST + session tuning) — GameStream previously ran unboosted on Linux.
            crate::native::boost_thread_priority(true);
            // A GameStream viewer may be video-only too — hold the suspend/idle inhibitor for
            // this stream's lifetime (plane parity with the native LiveSessionGuard).
            let _sleep = crate::sleep_inhibit::hold();
            let _host_cursor = crate::host_cursor::hold();
            tracing::info!(?cfg, "video stream starting");
            // Lifecycle events + the script-facing marker file, plane parity with the native loop
            // (RFC §4): `announce` emits `stream.started`/`stream.stopped` and holds the marker for
            // the span between. It runs BEFORE `run` because `run` is what launches the app — the
            // marker has to exist by the time the title's own wrapper script executes, or the
            // wrapper takes its "not streaming" branch mid-stream. The RTSP layer carries no client
            // device name, so `client` is empty here — the `plane` field is what hooks key on.
            // `client.connected` fires alongside `stream.started` because a Moonlight client has no
            // persistent connection to anchor it to.
            let stream_marker = crate::stream_marker::announce(crate::stream_marker::StreamInfo {
                width: cfg.width,
                height: cfg.height,
                refresh_hz: cfg.fps,
                hdr: cfg.hdr,
                client: String::new(),
                launch: app.as_ref().map(|a| a.title.clone()),
                plane: crate::events::Plane::Gamestream,
            });
            let event_client = crate::events::ClientRef {
                name: String::new(),
                fingerprint: None,
                plane: crate::events::Plane::Gamestream,
            };
            crate::events::emit(crate::events::EventKind::ClientConnected {
                client: event_client.clone(),
            });
            // GPU clock pin (Linux, opt-in `SLIPSTREAM_PIN_CLOCKS`): hold the box-wide vendor clock
            // floor while this compat-plane stream runs, refcounted with every other live session
            // across both planes. Released when the closure exits (stream stopped) — so idle clocks
            // aren't pinned between Moonlight sessions. No-op off Linux / when the flag is unset.
            #[cfg(target_os = "linux")]
            let _clock_pin = crate::gpuclocks::session_pin();
            let result = run(
                cfg,
                app.as_ref(),
                &running,
                &force_idr,
                &rfi_range,
                &video_cap,
                &stats,
                &on_lost,
                &life,
            );
            // A clean return is a stop (RTSP teardown / cancel / client unreachable) → `quit`;
            // an error return is `error`. The compat plane can't tell a user stop from an idle
            // vanish the way the native plane's typed close code can.
            let reason = match &result {
                Ok(()) => crate::events::DisconnectReason::Quit,
                Err(_) => crate::events::DisconnectReason::Error,
            };
            if let Err(e) = result {
                tracing::error!(error = %format!("{e:#}"), "video stream failed");
            }
            running.store(false, Ordering::SeqCst);
            // Retract the marker and fire `stream.stopped` — explicitly here, before
            // `client.disconnected`, so the compat plane keeps the native loop's event order.
            drop(stream_marker);
            crate::events::emit(crate::events::EventKind::ClientDisconnected {
                client: event_client,
                reason,
            });
            tracing::info!("video stream stopped");
        });
}

#[allow(clippy::too_many_arguments)]
fn run(
    cfg: StreamConfig,
    app: Option<&super::apps::AppEntry>,
    running: &Arc<AtomicBool>,
    force_idr: &AtomicBool,
    rfi_range: &std::sync::Mutex<Option<(i64, i64)>>,
    video_cap: &std::sync::Mutex<Option<PooledCapturer>>,
    // Shared stats recorder for the web-console capture/graph. Threaded into `stream_body` (the
    // encode loop); per-frame sample emission is wired by a later pass.
    stats: &Arc<crate::stats_recorder::StatsRecorder>,
    // Whole-session teardown for the send thread's client-unreachable detection.
    on_lost: &super::OnSessionLost,
    // The launched game's lifetime wiring (quit flag, launch owner, game-exit teardown).
    life: &GameLifetime,
) -> Result<()> {
    // GameStream capture/encode thread: apply Windows session tuning (no-op off Windows).
    ss_frame::session_tuning::on_hot_thread();
    // Reject an out-of-range client mode before allocating capture/encode buffers.
    encode::validate_dimensions(cfg.codec, cfg.width, cfg.height)
        .context("client-requested video mode")?;
    let sock = UdpSocket::bind(("0.0.0.0", VIDEO_PORT)).context("bind video UDP")?;
    // Grow SO_SNDBUF/RCVBUF (avoid host-side ENOBUFS at high bitrate) like the native plane.
    // The opt-in DSCP/QoS tag happens after connect below (Windows qWAVE derives the flow from
    // the connected 5-tuple).
    slipstream_core::transport::grow_socket_buffers(&sock);
    // The client pings the video port so we learn where to send; it re-pings until video
    // flows, so a missed early ping is fine.
    sock.set_read_timeout(Some(Duration::from_secs(10)))?;
    tracing::info!(
        port = VIDEO_PORT,
        "video: awaiting client ping to learn endpoint"
    );
    let mut probe = [0u8; 256];
    let (_, client) = sock
        .recv_from(&mut probe)
        .context("video: no client ping within 10s")?;
    sock.connect(client)
        .context("connect client video endpoint")?;
    // Opt-in DSCP/QoS-tag this as the video class (SLIPSTREAM_DSCP=1); the guard keeps the
    // Windows qWAVE flow alive for the whole stream (this function's scope IS the stream).
    let _qos_flow = slipstream_core::transport::set_media_qos(
        &sock,
        slipstream_core::transport::MediaClass::Video,
    );
    tracing::info!(%client, "video: client endpoint learned");
    // Short label for web-console stats captures: the client's peer IP.
    let client_label = client.ip().to_string();

    // Native client-resolution source: create a compositor virtual output sized to the client's
    // request and capture it (no scaling). Self-contained — deliberately NOT pooled in
    // `video_cap`, since a reconnect at a different resolution needs a freshly-sized output; the
    // output is released when this capturer drops at stream end (RAII via its keepalive).
    if ss_host_config::config().video_source.as_deref() == Some("virtual") {
        // Reference point for adopting the launched game's processes: anything the host will call
        // "this session's game" has to have started after this instant. Taken HERE — before the prep
        // steps, before the source (a bare-spawn gamescope nests the game inside it), before the
        // launch — because a reading taken later would reject the very process it is meant to find.
        // Erring early can only ever include more of our own launch, never a copy from before it.
        let launch_stamp = crate::gamelease::launch_clock();
        // Everything the host knows about the title being launched, resolved in ONE library scan:
        // what to run, what to call it, and how to recognize it once it is up.
        let target = resolve_gs_app(app);
        // Moonlight has no session resume, so a client coming back for a game it left behind does it
        // by launching the title again. Reprieve that game before anything starts, so the copy the
        // player is about to be handed isn't killed out from under them when the old window closes.
        if let Some(t) = target.as_ref() {
            let reprieved =
                crate::gamelease::readopt(life.fingerprint.as_deref(), t.game.id.as_deref());
            if reprieved > 0 {
                tracing::info!(
                    reprieved,
                    title = %t.game.title,
                    "gamestream: this client came back for its game — keeping it"
                );
            }
        }
        // Per-app prep steps (RFC §6): the entry's own `prep` plus a custom library title's,
        // run synchronously BEFORE the virtual output opens or anything launches (an HDR
        // toggle / sink switch must land first — and gamescope's nested launch happens inside
        // `open_gs_virtual_source`). The guard's drop runs the undos at stream end — reverse
        // order, best-effort, on every exit path including a panic-unwind.
        let mut prep_cmds = app.map(|a| a.prep.clone()).unwrap_or_default();
        if let Some(lib_id) = app.and_then(|a| a.library_id.as_deref()) {
            prep_cmds.extend(crate::library::prep_for(lib_id));
        }
        let prep_env = [(
            "PF_APP_TITLE".to_string(),
            app.map(|a| a.title.clone()).unwrap_or_default(),
        )];
        let _prep = (!prep_cmds.is_empty()).then(|| crate::hooks::run_prep(&prep_cmds, &prep_env));
        // Open the virtual-display source: pick the live compositor, normalize the session env
        // (apply_session_env/apply_input_env — gamescope ATTACH/resize + KWin/Mutter retargeting,
        // exactly like the native plane), create a virtual output at the client mode, and capture it.
        // Re-runnable: the encode loop calls it again on a mid-stream capture loss to FOLLOW a
        // Desktop<->Game switch.
        let (mut capturer, compositor, gamescope_route) =
            open_gs_virtual_source(cfg, app, target.as_ref(), &life.quit)?;
        // Only the Linux `launch_is_nested` reads it; gamescope does not exist on Windows.
        #[cfg(not(target_os = "linux"))]
        let _ = &gamescope_route;
        // Register this session in the admission table. GameStream acquires a REAL display but
        // never registered, so `admit`'s Windows budgets — `max_displays` and the NVENC session
        // headroom — could not see it: both gate on `!live.is_empty()`, so a Moonlight-held display
        // was invisible to them and a native connect could be admitted past a budget the box had
        // already spent. Identity is `None` (the compat plane has no cert fingerprint), which is the
        // same "anonymous" the conflict policy already handles. Dropped at the end of `run`.
        let _admission_guard = crate::vdisplay::admission::register(
            None,
            (cfg.width, cfg.height, cfg.fps),
            life.quit.clone(),
            "gamestream".to_string(),
        );
        tracing::info!(
            ?compositor,
            app = ?app.map(|a| &a.title),
            w = cfg.width,
            h = cfg.height,
            "video source: virtual display (native client resolution)"
        );
        // Launch the app's command now that capture is live, for the backends that DON'T nest it via
        // set_launch_command above: Windows (no gamescope) and, on Linux, everything but gamescope's
        // bare-spawn sub-mode (kwin/mutter/wlroots stream the existing desktop; a managed/attached
        // gamescope is a running session to launch INTO — `launch_session_command` routes both).
        // A library title (Steam/Epic/GOG/Xbox/custom, surfaced in /applist) carries its
        // store-qualified id — resolved against the host's OWN library (the client can only pick an
        // existing title, never inject a command). An apps.json entry instead carries an
        // operator-typed `cmd`. Library id wins when both are set.
        #[cfg(windows)]
        if let Some(t) = target.as_ref() {
            // A library title launches by its store-qualified id (the interactive-session spawner
            // resolves the store's own recipe); an operator-typed command runs as itself.
            let launched = match (t.game.id.as_deref(), t.command.as_deref()) {
                (Some(id), _) => crate::library::launch_gamestream_library(id),
                (None, Some(cmd)) => crate::library::launch_gamestream_command(cmd),
                (None, None) => Ok(()),
            };
            if let Err(e) = launched {
                tracing::warn!(title = %t.game.title, error = %e, "gamestream: could not launch app");
            }
        }
        // Linux keeps the spawned child rather than dropping it: it is the primary liveness signal
        // for a title whose store told us nothing else, and the handle the termination ladder
        // signals. A gamescope bare spawn already nested the command (`set_launch_command` in the
        // source open), so launching again would start it twice.
        #[cfg(target_os = "linux")]
        let spawned_launch = match target.as_ref().and_then(|t| t.command.as_deref()) {
            Some(_) if crate::vdisplay::launch_is_nested(compositor, gamescope_route.as_ref()) => {
                None
            }
            Some(cmd) => match crate::library::launch_session_command(compositor, cmd) {
                Ok(spawned) => Some(spawned),
                Err(e) => {
                    tracing::warn!(command = %cmd, error = %e, "gamestream: could not launch app");
                    None
                }
            },
            None => None,
        };

        // The launched game's lifetime, in both directions (design/session-game-lifetime.md) — the
        // compat plane's half of what the native plane already does:
        //
        // * **its exit ends the session**, so Moonlight returns to its app list instead of leaving
        //   the player on a bare desktop or a hidden launcher.
        // * **this session ending can end it** — never by default; only when the operator asked, and
        //   for a mere drop only after a reconnect window (the guard's drop). Moonlight can't resume
        //   a session, but the window still protects unsaved progress on a network blip, and a
        //   relaunch of the same title reclaims the game (above).
        let _game_life = target.as_ref().map(|t| {
            #[cfg(target_os = "linux")]
            let nested = crate::vdisplay::launch_is_nested(compositor, gamescope_route.as_ref());
            #[cfg(not(target_os = "linux"))]
            let nested = false;
            #[cfg(target_os = "linux")]
            let child = spawned_launch.map(|s| (s.child, s.group_leader));
            #[cfg(not(target_os = "linux"))]
            let child = None;

            let on_exit: crate::gamelease::OnExit = {
                let on_game_exit = life.on_game_exit.clone();
                Box::new(move || {
                    // Read the setting at fire time, so flipping it mid-session takes effect. The
                    // lease itself keeps running either way — the status surface still reports the
                    // game.
                    if !crate::session_settings::get().session_on_game_exit {
                        tracing::info!(
                            "the launched game exited, but ending the session on game exit is off — \
                             leaving the stream up"
                        );
                        return;
                    }
                    tracing::info!("the launched game exited — ending the session");
                    // Deliberate: the player finished. The display skips its keep-alive linger and
                    // the launch state is cleared, so Moonlight's next `/launch` starts cleanly.
                    on_game_exit();
                })
            };
            let lease = crate::gamelease::open(
                crate::gamelease::LeaseRequest {
                    game: t.game.clone(),
                    // RTSP carries no client device name, so the peer IP is the best label there is
                    // (the same one the stats capture uses).
                    client: client_label.clone(),
                    plane: crate::events::Plane::Gamestream,
                    spec: t.detect.clone(),
                    nested,
                    child,
                    launch_stamp,
                },
                on_exit,
            );
            // Declared first so it drops first: the console loses the live row before the policy
            // below can replace it with a `grace` one, rather than briefly showing both.
            let published = crate::session_status::publish_gamestream_game(lease.shared());
            (
                published,
                crate::gamelease::SessionGuard::new(
                    lease,
                    life.quit.clone(),
                    life.fingerprint.clone(),
                ),
            )
        });
        // Rebuild closure: re-open the source on a mid-stream capture loss, RE-DETECTING the live
        // compositor — so a Desktop<->Game switch (at the client's fixed mode) is FOLLOWED in place
        // without a Moonlight reconnect. (A resolution change can't be followed mid-stream on
        // GameStream — WxH is locked at ANNOUNCE — but a session toggle keeps the negotiated mode.)
        let rebuild =
            || open_gs_virtual_source(cfg, app, target.as_ref(), &life.quit).map(|(c, _, _)| c);
        return stream_body(
            &mut capturer,
            Some(&rebuild),
            // Mirrors the source's own `set_hw_cursor` request (`open_gs_virtual_source`):
            // cursor-as-metadata + host blend wherever the backend composites — the
            // compositor-EMBEDS fallback never paints on a Mutter virtual stream (the
            // native plane's no-channel rule, `session_plan::cursor_blend_for`). gamescope
            // remains the pointerless residual — its capture carries no cursor either way
            // (the native plane's XFixes source is not wired on this plane).
            compositor != crate::vdisplay::Compositor::Gamescope
                && blend_capable_metadata_cursor(&cfg),
            &sock,
            cfg,
            running,
            force_idr,
            rfi_range,
            stats,
            &client_label,
            on_lost,
        );
    }

    // Reuse the persistent capturer (one screencast session → clean reconnect); create it on
    // the first stream. Borrow it for this stream and return it on exit. Reuse is gated on the
    // pooled capturer's HDR-ness matching this stream's negotiated `cfg.hdr` — the depth is a
    // PipeWire-negotiation-time property of the screencast session, so an HDR↔SDR change needs a
    // fresh session (same pattern as the audio capturer's channel-count gate).
    // Cursor-as-metadata only where the encode backend this session resolves to composites
    // `frame.cursor` (the caps-aware negotiation — mirror of the native plane's); otherwise ask
    // the portal to EMBED the pointer so no backend × cursor-mode combination streams
    // cursorless. Synthetic frames carry no pointer either way.
    let metadata_cursor = blend_capable_metadata_cursor(&cfg);
    // Which screen this stream must show. The host-wide pin (§5.3) applies to the compat plane too:
    // the portal chooser cannot name a head, so a pinned host MIRRORS it here the same way the
    // virtual source does via `vdisplay::open`. Without this a Moonlight client on a pinned host
    // would silently get whichever monitor the portal handed back — "showing the wrong monitor is
    // worse than showing none" is the rule the whole feature is built on.
    #[cfg(target_os = "linux")]
    let pinned = crate::vdisplay::capture_monitor();
    #[cfg(not(target_os = "linux"))]
    let pinned: Option<String> = None;
    let pooled = match video_cap.lock().unwrap().take() {
        Some((c, was_hdr, was_meta, ref was_pin))
            if was_hdr == cfg.hdr && was_meta == metadata_cursor && *was_pin == pinned =>
        {
            Some(c)
        }
        Some((c, was_hdr, was_meta, was_pin)) => {
            tracing::info!(
                was_hdr,
                want_hdr = cfg.hdr,
                was_metadata_cursor = was_meta,
                want_metadata_cursor = metadata_cursor,
                was_monitor = was_pin.as_deref().unwrap_or("<portal's pick>"),
                want_monitor = pinned.as_deref().unwrap_or("<portal's pick>"),
                "video source: pooled capturer depth/cursor-mode/monitor mismatch — opening a \
                 fresh screencast session"
            );
            drop(c);
            None
        }
        None => None,
    };
    let mut capturer: Box<dyn Capturer> = match pooled {
        Some(c) => {
            tracing::info!("video source: reusing capturer");
            c
        }
        #[cfg(target_os = "linux")]
        None if ss_host_config::config().video_source.as_deref() == Some("portal")
            && pinned.is_some() =>
        {
            let connector = pinned.as_deref().expect("guarded by the match arm");
            tracing::info!(
                hdr = cfg.hdr,
                metadata_cursor,
                monitor = connector,
                "video source: mirroring the pinned monitor (portal source, host pin)"
            );
            open_gs_mirror_source(connector, cfg, metadata_cursor)
                .with_context(|| format!("mirror the pinned monitor {connector:?}"))?
        }
        None if ss_host_config::config().video_source.as_deref() == Some("portal") => {
            tracing::info!(
                hdr = cfg.hdr,
                metadata_cursor,
                capture_method = ss_host_config::config()
                    .capture_method
                    .as_deref()
                    .unwrap_or("auto"),
                "video source: desktop capture"
            );
            capture::open_desktop_capture(cfg.hdr, metadata_cursor)
                .context("open desktop capturer")?
        }
        None => {
            tracing::info!("video source: synthetic test pattern");
            Box::new(FastSyntheticCapturer::new(cfg.width, cfg.height))
        }
    };
    capturer.set_active(true);
    // Portal/synthetic source: no compositor virtual output to re-detect, so no rebuild closure.
    let result = stream_body(
        &mut capturer,
        None,
        metadata_cursor,
        &sock,
        cfg,
        running,
        force_idr,
        rfi_range,
        stats,
        &client_label,
        on_lost,
    );
    capturer.set_active(false);
    // Re-pool ONLY a capturer that can still produce frames. Every terminal state of the portal
    // backend is sticky (`Capturer::is_alive`): a dead zerocopy-import worker, an exited PipeWire
    // thread, or a compositor that went away all make the NEXT stream fail at exactly the same
    // point — and this path has no rebuild closure (unlike the virtual-output path above), so a
    // re-admitted dead capturer wedged GameStream portal video permanently, at 10 s per reconnect
    // attempt. Dropping it instead costs one fresh screencast session on the next connect. Note
    // `result` may already be `Err` here, which is itself that signal. (`metadata_cursor` and the
    // monitor pin ride along as the other two reuse keys, beside HDR-ness — see `PooledCapturer`.)
    if result.is_ok() && capturer.is_alive() {
        *video_cap.lock().unwrap() = Some((capturer, cfg.hdr, metadata_cursor, pinned));
    } else {
        tracing::info!(
            stream_failed = result.is_err(),
            capturer_alive = capturer.is_alive(),
            "video source: retiring the pooled capturer — the next stream opens a fresh screencast \
             session"
        );
    }
    result
}

/// Open a capturer on the **pinned physical monitor** for the compat plane's portal source
/// (`design/per-monitor-portal-capture.md` §5.3). The pin is host-wide, so it has to be honored on
/// every plane that captures a screen — and the portal source is the one that otherwise takes
/// "whichever head the portal hands back".
///
/// Deliberately *not* the `open_gs_virtual_source` path: this source launches nothing and creates no
/// virtual output, so it needs neither the game-lifetime machinery nor the registry (a mirror is
/// [`DisplayOwnership::External`](crate::vdisplay::DisplayOwnership) and would pass straight through
/// it anyway). A missing monitor fails the stream loudly rather than falling back to another screen.
#[cfg(target_os = "linux")]
fn open_gs_mirror_source(
    connector: &str,
    cfg: StreamConfig,
    metadata_cursor: bool,
) -> Result<Box<dyn Capturer>> {
    // Follow the live session first, exactly as the virtual source does — a mirror host that
    // switched Desktop↔Game since startup must be enumerated against the compositor that is up now.
    let active = crate::vdisplay::detect_active_session();
    crate::vdisplay::observe_session_instance(&active);
    crate::vdisplay::apply_session_env(&active);
    let compositor = crate::vdisplay::compositor_for_kind(active.kind)
        .map(Ok)
        .unwrap_or_else(crate::vdisplay::detect)
        .context("detect compositor")?;
    // A mirror streams an existing head — no gamescope sub-mode applies, so the resolved route is
    // deliberately dropped here rather than carried.
    let _ = crate::vdisplay::apply_input_env(compositor, false);
    let mut vd = crate::vdisplay::open_mirror(compositor, connector)?;
    // Cursor mode is the session's negotiated one: metadata where this encode path composites
    // `frame.cursor`, otherwise let the compositor embed it (§7.5 — one resolver, per-backend
    // expression).
    vd.set_hw_cursor(metadata_cursor);
    // The mirror backend ignores the requested mode by design (§7.3 — a panel runs at the mode its
    // owner set, and the client scales); pass the client's anyway so the argument stays honest.
    let vout = vd
        .create(slipstream_core::Mode {
            width: cfg.width,
            height: cfg.height,
            refresh_hz: cfg.fps,
        })
        .context("start mirroring the pinned monitor")?;
    crate::capture::capture_virtual_output(
        vout,
        ss_frame::OutputFormat::resolve(cfg.hdr, crate::zerocopy::enabled()),
        crate::session_plan::CaptureBackend::resolve(),
    )
    .context("attach a capturer to the mirrored monitor")
}

/// What the compat plane resolved about the app a client launched: identity for the lease, the status
/// surface and the `game.*` events; the signals that recognize the running game; and the command to
/// run it.
struct GsApp {
    game: crate::gamelease::GameRef,
    detect: crate::library::DetectSpec,
    /// The resolved shell command. `Some` on Linux, which runs it itself; `None` for a Windows
    /// library title, which launches by id through the interactive-session spawner instead.
    command: Option<String>,
}

/// Resolve a `/launch`ed catalog entry against the host's **own** library — the client sends only an
/// appid, and everything the session does with the title afterwards comes from what the host knows
/// about it.
///
/// A library pick carries its store's detect signals. An operator-typed `apps.json` command has no
/// library entry behind it, so its title is the whole identity and its own first token — when that is
/// an absolute executable — the only signal there is; the host spawns it directly anyway, so the child
/// is the primary tracking either way. `None` = nothing to launch (Desktop, or an unresolvable entry).
fn resolve_gs_app(app: Option<&super::apps::AppEntry>) -> Option<GsApp> {
    let app = app?;
    if let Some(id) = app.library_id.as_deref() {
        match crate::library::resolve_launch(id) {
            Some(t) => {
                return Some(GsApp {
                    game: t.game,
                    detect: t.detect,
                    command: t.command,
                })
            }
            // Same fallback (and same warning) the plain command resolution has always had, so a
            // client picking a stale title sees why nothing started.
            None => tracing::warn!(
                launch_id = id,
                "requested launch id not in this host's library (or no launch recipe) — ignoring"
            ),
        }
    }
    let cmd = app
        .cmd
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty())?;
    Some(GsApp {
        game: crate::gamelease::GameRef {
            id: None,
            store: None,
            title: if app.title.trim().is_empty() {
                cmd.to_string()
            } else {
                app.title.clone()
            },
        },
        detect: crate::library::spec_from_command(cmd),
        command: Some(cmd.to_string()),
    })
}

/// Open the virtual-display video source for a GameStream session: pick the LIVE compositor + normalize
/// the session env (apply_session_env/apply_input_env — gamescope ATTACH/resize, KWin/Mutter
/// retargeting) exactly like the native plane (native.rs resolve_compositor), create a virtual
/// output at the client's mode, and capture it. Returns the capturer (it owns the output's keepalive;
/// the stateless VirtualDisplay factory is dropped here) plus the resolved compositor. An apps.json
/// entry can PIN a compositor (skips the live detect/retarget). Re-run on a mid-stream capture loss to
/// FOLLOW a Desktop<->Game switch: it re-detects the now-live compositor and re-targets at it. Does NOT
/// launch the app (that happens once at stream start; a rebuild must not re-spawn it).
/// Cursor-as-metadata for this plane (GameStream has no cursor channel): only where the encode
/// backend this session resolves to composites `frame.cursor` — the same CUDA-payload
/// prediction `SessionPlan`/`handshake::cursor_forward` make (the NVIDIA resolution plus the
/// zero-copy master switch). Shared by the monitor-mirror and virtual-output sources so their
/// `set_hw_cursor` request and `stream_body`'s blend flag cannot drift.
fn blend_capable_metadata_cursor(cfg: &StreamConfig) -> bool {
    #[cfg(target_os = "linux")]
    {
        let cuda_planned = !crate::encode::linux_zero_copy_is_vaapi() && crate::zerocopy::enabled();
        crate::encode::cursor_blend_capable(cfg.codec, cuda_planned, cfg.hdr)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = cfg;
        false
    }
}

fn open_gs_virtual_source(
    cfg: StreamConfig,
    app: Option<&super::apps::AppEntry>,
    // The resolved title (see [`resolve_gs_app`]) — its command is what a bare-spawn gamescope nests
    // and what decides whether this session is a dedicated game session at all. Resolved once by the
    // caller, so a mid-stream rebuild can't re-resolve to something different.
    launch: Option<&GsApp>,
    // The session's deliberate-quit flag, handed to the display's keep-alive lease.
    quit: &Arc<AtomicBool>,
) -> Result<(
    Box<dyn Capturer>,
    crate::vdisplay::Compositor,
    Option<crate::vdisplay::GamescopeRoute>,
)> {
    let (compositor, gamescope_route) = if let Some(c) = app.and_then(|a| a.compositor) {
        // An app-pinned compositor still needs a route resolved, or `create` falls through to a
        // bare spawn on a box pinned to the managed session (mirrors `native::resolve_compositor`).
        let r = crate::vdisplay::resolve_gamescope_route(c, false);
        (c, r)
    } else {
        // Windows has a single virtual-display backend (ss-vdisplay); `vdisplay::open` ignores the
        // compositor arg there, so short-circuit the Linux session-detection state machine with a
        // placeholder — mirrors `native::resolve_compositor`. Without this, the Linux `detect()`
        // below bails on Windows ("could not detect compositor … XDG_CURRENT_DESKTOP=''"), which
        // killed the GameStream video thread → black screen (the native plane was already guarded).
        #[cfg(target_os = "windows")]
        {
            (crate::vdisplay::Compositor::Kwin, None)
        }
        #[cfg(not(target_os = "windows"))]
        {
            // A client is (re)connecting → cancel any pending TV-session restore (review #3).
            crate::vdisplay::cancel_pending_tv_restore();
            let active = crate::vdisplay::detect_active_session();
            // A4: fold any compositor-instance change (idle-time Game↔Desktop switch) into the epoch
            // before acquiring, so a GameStream reconnect never reuses a dead-instance node.
            crate::vdisplay::observe_session_instance(&active);
            crate::vdisplay::apply_session_env(&active);
            // Dedicated game session (B0): a GameStream app whose launch RESOLVES to a command (library
            // id / apps.json command), under `game_session=dedicated` with gamescope available, gets its
            // own headless gamescope spawn at the client mode — same routing as the native plane. Gate on
            // the resolved command so an unresolvable entry falls back to auto routing (review #9).
            let has_launch = launch.and_then(|t| t.command.as_deref()).is_some();
            if crate::vdisplay::wants_dedicated_game_session(has_launch) {
                let r =
                    crate::vdisplay::apply_input_env(crate::vdisplay::Compositor::Gamescope, true);
                (crate::vdisplay::Compositor::Gamescope, r)
            } else {
                let c = crate::vdisplay::compositor_for_kind(active.kind)
                    .map(Ok)
                    .unwrap_or_else(crate::vdisplay::detect)
                    .context("detect compositor")?;
                let r = crate::vdisplay::apply_input_env(c, false);
                (c, r)
            }
        }
    };
    let mut vd = crate::vdisplay::open(compositor).context("open virtual display")?;
    // Out-of-band cursor for the virtual source (the native plane's no-channel rule, mirrored):
    // GameStream has no cursor channel, and the compositor-EMBEDS fallback never paints on a
    // Mutter virtual stream (stage-global overlay suppression since Mutter 48 — see
    // ss-vdisplay `mutter.rs`), so ask for cursor-as-metadata wherever the resolved backend
    // composites `frame.cursor`; `stream_body`'s blend flag mirrors this request. gamescope
    // stays off: its capture carries no metadata either way, and the request would cost the
    // native-NV12 shape for nothing.
    vd.set_hw_cursor(
        compositor != crate::vdisplay::Compositor::Gamescope && blend_capable_metadata_cursor(&cfg),
    );
    // Carry the resolved launch command on the backend instance (per-session) rather than a
    // process-global env var, so concurrent sessions can't stomp each other's launch target. It is
    // the RESOLVED command, so gamescope's bare spawn nests a library title exactly like an
    // apps.json command (it previously nested only `cmd`, silently dropping library picks). Off
    // Linux this is a no-op backend-side, and a library title resolves to no command at all — the
    // interactive-session spawner launches it by id instead.
    vd.set_launch_command(launch.and_then(|t| t.command.clone()));
    // This plane's resolved gamescope sub-mode, on the instance for the same reason as the launch
    // command above — the GameStream and native planes both call `apply_input_env`, so publishing
    // through the process env let either retarget the other's `create`.
    vd.set_gamescope_route(gamescope_route.clone());
    // Serialize with the slipstream/1 plane's IDD-push setup dance (Goal-1 §2.5). A GameStream
    // connect used to skip it entirely, so it could ADD/reconfigure the shared monitor while a
    // native session was mid-build (and vice versa), and its sealed-channel delivery would replace
    // the native session's ring (newest-wins) — each plane could freeze the other. GameStream has
    // no cooperative stop-flag plumbing, so it registers a flag nobody reads: a LATER session that
    // preempts this one signals it, waits the 3 s release grace, then force-preempts the monitor —
    // this session then fails on capture and tears down cleanly (the intended handover). GameStream
    // is anonymous (no client cert), so it holds the ANONYMOUS slot (0) — GS stays single-display,
    // and only a later slot-0 session (another GS/anonymous connect) preempts it.
    #[cfg(target_os = "windows")]
    let _idd_setup_guard = matches!(
        crate::session_plan::CaptureBackend::resolve(),
        crate::session_plan::CaptureBackend::IddPush
    )
    .then(|| {
        crate::vdisplay::manager::vdm().begin_idd_setup(
            0,
            std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        )
    });
    let vout = crate::vdisplay::registry::acquire(
        &mut vd,
        slipstream_core::Mode {
            width: cfg.width,
            height: cfg.height,
            refresh_hz: cfg.fps,
        },
        // GameStream's deliberate quit is the Moonlight "Quit App" (nvhttp `h_cancel`), the
        // management stop, or the launched game exiting — not a QUIC close code. All three set the
        // session's quit flag ([`super::AppState::quit`]), so the display skips its keep-alive linger
        // for a stop the way it does on the native plane, and only a real drop lingers.
        quit.clone(),
        None, // fresh session — no display superseded
    )
    .context("create virtual output at client resolution")?;
    // HDR: pass the negotiated `cfg.hdr` (client asked for HDR AND the host can deliver it). On the
    // Windows IDD-push path this proactively enables advanced color on the virtual display so a Main10
    // PQ stream flows even from an SDR desktop; an already-HDR desktop streams PQ regardless (the
    // capturer follows the display). No-op on Linux: virtual-output capture is SDR-only upstream
    // (Mutter RecordVirtual), and `host_hdr_capable` therefore keeps `cfg.hdr` false for this
    // source — the Linux HDR path is the portal monitor mirror (`video_source=portal`).
    let mut capturer = capture::capture_virtual_output(
        vout,
        capture::OutputFormat::resolve(cfg.hdr, crate::encode::resolved_backend_is_gpu()),
        crate::session_plan::CaptureBackend::resolve(),
    )
    .context("capture virtual output")?;
    capturer.set_active(true);
    Ok((capturer, compositor, gamescope_route))
}

/// The encoder bit depth implied by the captured frame's pixel format: a 10-bit (HDR) source — the
/// Windows IDD-push capturer's `P010`/`Rgb10a2` when the desktop is HDR — opens NVENC as HEVC Main10
/// (BT.2020 PQ); everything else is 8-bit. The encoder backends already key the real profile off the
/// `format`, so this just keeps the `bit_depth` argument honest (the old hard-coded `8` mislabeled an
/// HDR stream that the format had already promoted to 10-bit).
pub(super) fn gs_bit_depth(format: crate::capture::PixelFormat) -> u8 {
    use crate::capture::PixelFormat;
    match format {
        // Windows IDD-push HDR formats, and the Linux GNOME 50+ portal HDR formats.
        PixelFormat::P010 | PixelFormat::Rgb10a2 | PixelFormat::X2Rgb10 | PixelFormat::X2Bgr10 => {
            10
        }
        _ => 8,
    }
}

/// Packetize + paced-send + encode loop (plan §W1); [`run`] calls [`stream_data::stream_body`].
mod stream_data;
use stream_data::stream_body;

#[cfg(test)]
mod tests {
    use super::*;
    use stream_data::{spawn_sender, PacketBatch, WireBatch};
    use slipstream_core::latency::{FrameTimings, LatencyArtifact};

    fn entry(title: &str, cmd: Option<&str>) -> super::super::apps::AppEntry {
        super::super::apps::AppEntry {
            id: 1,
            title: title.to_string(),
            compositor: None,
            cmd: cmd.map(str::to_string),
            library_id: None,
            prep: Vec::new(),
        }
    }

    /// What the compat plane decides to track. The negative cases are the load-bearing ones: a
    /// Desktop entry launches nothing, so it must take no lease at all — the feature has to stay
    /// completely inert for a plain desktop stream.
    #[test]
    fn only_an_entry_that_launches_something_gets_tracked() {
        // Nothing selected (no `/launch` app) → nothing to track.
        assert!(resolve_gs_app(None).is_none());
        // Desktop: a title with no command and no library id.
        assert!(resolve_gs_app(Some(&entry("Desktop", None))).is_none());
        // A blank command is the same as none.
        assert!(resolve_gs_app(Some(&entry("Blank", Some("   ")))).is_none());

        // An operator-typed apps.json command: no library entry behind it, so the title is the whole
        // identity and there is no store-qualified id for the console to match box art on.
        let t = resolve_gs_app(Some(&entry(
            "Steam Big Picture",
            Some("  steam -gamepadui  "),
        )))
        .expect("a command entry is tracked");
        assert_eq!(t.command.as_deref(), Some("steam -gamepadui"));
        assert_eq!(t.game.id, None);
        assert_eq!(t.game.store, None);
        assert_eq!(t.game.title, "Steam Big Picture");
        // `steam` is a PATH lookup, not an absolute executable, so nothing is asserted about the
        // process — the host's own child is what tracks it (see `library::spec_from_command`).
        assert!(t.detect.is_empty());

        // A titleless entry still shows up as something a human can read.
        let t = resolve_gs_app(Some(&entry("", Some("/opt/game/run")))).expect("tracked");
        assert_eq!(t.game.title, "/opt/game/run");
    }

    /// End-to-end check of the send thread: batches pushed on the channel arrive, complete and
    /// byte-identical, at a peer socket via the paced sendmmsg path.
    #[test]
    fn sender_delivers_batches() {
        let rx_sock = UdpSocket::bind("127.0.0.1:0").unwrap();
        // Generous: on a CI host saturated by parallel release builds, this thread can be
        // starved for whole seconds between recv() wakeups.
        rx_sock
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let tx_sock = UdpSocket::bind("127.0.0.1:0").unwrap();
        tx_sock.connect(rx_sock.local_addr().unwrap()).unwrap();

        let running = Arc::new(AtomicBool::new(true));
        let (tx, rx) = std::sync::mpsc::sync_channel::<WireBatch>(2);
        spawn_sender(
            tx_sock,
            rx,
            Duration::from_millis(8), // ~120fps frame interval
            running.clone(),
            Arc::new(|| {}),
            Arc::new(std::sync::Mutex::new(None::<LatencyArtifact>)),
        )
        .unwrap();

        // 3 frames of 20 packets, content-tagged for verification. The TOTAL burst must fit
        // the receive socket's DEFAULT buffer even if this thread never drains concurrently
        // (a starved CI runner): a 1200 B datagram costs ~2.5 KB kernel truesize, and the
        // default rmem (~212 KB) holds only ~80 — a bigger burst gets silently dropped by
        // the kernel and the test can never complete (the old 3×100 flaked exactly there).
        const PER_FRAME: usize = 20;
        let mut sent = Vec::new();
        for f in 0..3u8 {
            let batch: PacketBatch = (0..PER_FRAME as u8)
                .map(|i| {
                    let mut p = vec![0u8; 1200];
                    p[0] = f;
                    p[1] = i;
                    p
                })
                .collect();
            sent.extend(batch.iter().cloned());
            tx.send(WireBatch {
                pkts: batch,
                timings: FrameTimings::new("synthetic"),
            })
            .unwrap();
        }
        drop(tx); // sender drains then exits

        let mut got = 0usize;
        let mut buf = [0u8; 2048];
        while got < sent.len() {
            let n = rx_sock.recv(&mut buf).expect("packet within timeout");
            assert_eq!(n, 1200);
            let (f, i) = (buf[0] as usize, buf[1] as usize);
            assert_eq!(&buf[..n], &sent[f * PER_FRAME + i][..], "payload intact");
            got += 1;
        }
        assert_eq!(got, 3 * PER_FRAME);
        assert!(running.load(Ordering::SeqCst), "no spurious client-gone");
    }
}
