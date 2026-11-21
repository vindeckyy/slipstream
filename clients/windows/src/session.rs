//! Runtime for one Windows client streaming session.
//!
//! This module owns the handshake, admission and receive/decode/present loop. It never exits the
//! process: the CLI boundary in `main` decides what an outcome means for the process exit code.

use anyhow::{anyhow, Context, Result};
use std::io::Write;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use slipstream_core::config::Role;
use slipstream_core::input::InputEvent;
use slipstream_core::packet::FLAG_PROBE;
use slipstream_core::quic::{endpoint, io, Hello, Start, Welcome};
use slipstream_core::transport::UdpTransport;
use slipstream_core::{Session, SlipstreamError};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::args::{hex, Args};
use crate::connect::{self, Machine};
use crate::identity;
use crate::input::{InputSender, InputSenderGuard};
use crate::present::{ConsolePresenter, Decoder, PassthroughDecoder, Presenter};
use crate::{ff, mf, window, PipeStats};

/// Inputs needed to run one client session. The CLI has already parsed and resolved these.
#[derive(Debug)]
pub struct SessionConfig {
    pub remote: SocketAddr,
    pub args: Args,
    /// Input route created by the CLI boundary. `None` keeps headless/record-only runs quiet.
    pub input: Option<InputSender>,
    /// Receiver paired with `input`; handed to the QUIC datagram pump once admitted.
    pub input_rx: Option<UnboundedReceiver<InputEvent>>,
}

/// How a session ended. Every normal termination has a variant; unexpected I/O and protocol errors
/// remain `Err` so the CLI still reports them as failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionOutcome {
    /// The peer ended a session that delivered at least one video AU.
    Ended,
    /// QUIC handshake, hello/welcome exchange or admission failed.
    ConnectFailed,
    /// The session was admitted but published no video AUs.
    ZeroFrames,
    /// The user closed the presentation window.
    WindowClosed,
    /// No frames arrived for the receive timeout.
    TimedOut,
}

impl SessionOutcome {
    /// Classify the result of a completed stream attempt.
    pub fn from_result(connected: bool, stop: SessionStop, frames: u64) -> Self {
        if !connected {
            return Self::ConnectFailed;
        }
        if frames > 0 {
            return match stop {
                SessionStop::Ended => Self::Ended,
                SessionStop::WindowClosed => Self::WindowClosed,
                SessionStop::TimedOut => Self::TimedOut,
            };
        }
        match stop {
            SessionStop::Ended => Self::ZeroFrames,
            SessionStop::WindowClosed => Self::WindowClosed,
            SessionStop::TimedOut => Self::TimedOut,
        }
    }
}

/// Why the receive loop stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStop {
    Ended,
    WindowClosed,
    TimedOut,
}

/// Show the host fingerprint and require an explicit `yes` before anything is streamed.
///
/// Fails closed. A non-interactive stdin is an error rather than an automatic accept, and a read
/// error, EOF, a bare newline or any answer other than `y`/`yes` is a refusal. There is no
/// "press enter to continue": a stray newline must never be enough to hand a host live keyboard
/// and mouse input.
fn confirm_fingerprint(host: &str, fp_hex: &str) -> Result<bool> {
    use std::io::{IsTerminal, Write};
    let stdin = std::io::stdin();
    if !stdin.is_terminal() {
        return Err(anyhow!(
            "cannot confirm {host}'s fingerprint: stdin is not a terminal. Verify {fp_hex} out of \
             band and re-run with --trust {fp_hex}."
        ));
    }
    eprintln!("\n{host} presents certificate fingerprint:\n  {fp_hex}\n");
    eprint!("Trust this host and send it your keyboard and mouse? [y/N] ");
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    if stdin.read_line(&mut line).is_err() {
        return Ok(false);
    }
    Ok(matches!(
        line.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

/// Run one streaming session to completion.
pub async fn run_stream(cfg: SessionConfig) -> Result<SessionOutcome> {
    let SessionConfig {
        remote,
        args,
        input,
        input_rx,
    } = cfg;
    let (cert, key) = identity::load_or_create()?;
    let (ep, observed) =
        endpoint::client_pinned_with_identity(args.pin, Some((cert.as_str(), key.as_str())));
    let ep = ep.map_err(|e| anyhow!("QUIC client endpoint: {e}"))?;
    let conn = ep
        .connect(remote, "slipstream")
        .context("connect")?
        .await
        .context("QUIC handshake (a pin mismatch fails here)")?;
    match (args.pin, *observed.lock().unwrap()) {
        (Some(_), _) => tracing::info!(%remote, "connected — host fingerprint pinned"),
        (None, Some(fp)) => tracing::info!(
            %remote,
            fingerprint = %hex(&fp),
            "connected (trust-on-first-use) — pass --pin to verify this host from now on"
        ),
        (None, None) => tracing::info!(%remote, "connected"),
    }

    let (mut send, recv) = conn.open_bi().await.context("open control stream")?;
    let mut recv = io::MsgReader::new(recv);
    io::write_msg(
        &mut send,
        &Hello {
            abi_version: slipstream_core::WIRE_VERSION,
            mode: args.mode,
            compositor: slipstream_core::CompositorPref::Auto,
            gamepad: slipstream_core::config::GamepadPref::Auto,
            bitrate_kbps: args.bitrate_kbps,
            name: Some(args.name.clone()),
            launch: None,
            // No decode backend yet: advertise no codec capability bits rather than claiming
            // 10-bit/4:4:4 we cannot render.
            video_caps: 0,
            audio_channels: args.audio_channels,
            video_codecs: slipstream_core::quic::CODEC_H264
                | slipstream_core::quic::CODEC_HEVC
                | slipstream_core::quic::CODEC_AV1,
            preferred_codec: args.preferred_codec,
            display_hdr: slipstream_core::client::display_hdr_env_override(),
            client_caps: 0,
        }
        .encode(),
    )
    .await?;
    // A host that completes the QUIC handshake but never sends `Welcome` (wedged encoder/parked
    // session) must fail, not hang the client forever.
    let welcome_msg = tokio::time::timeout(Duration::from_secs(15), recv.read_msg())
        .await
        .map_err(|_| {
            anyhow!(
                "host {remote} accepted the QUIC handshake but sent no session offer within 15 s \
                 (the host's capture/encode pipeline is stalled, or the session is parked for \
                 approval) — this is a host-side condition, not a client one"
            )
        })??;
    let welcome = Welcome::decode(&welcome_msg).map_err(|e| anyhow!("Welcome decode: {e:?}"))?;
    tracing::info!(
        mode = ?welcome.mode,
        frames = welcome.frames,
        codec = welcome.codec,
        bit_depth = welcome.bit_depth,
        encrypt = welcome.encrypt,
        udp_port = welcome.udp_port,
        "session offer"
    );

    // Admission, before a single byte of video is consumed. `connect::decide` is the only authority
    // for this: this CLI takes a manual `ip:port`, which has no advert, so the certificate the host
    // presents is the sole anchor and it must pass the confirmation card below.
    let mut machine = Machine::new();
    machine
        .connect(&remote.to_string())
        .map_err(|e| anyhow!(e))?;
    let observed_fp = *observed.lock().unwrap();
    let admission = connect::decide(args.pin.is_some(), None, true);
    let host_label = remote.to_string();
    match machine.admitted(admission) {
        connect::Transition::Stay(_, msg) => tracing::info!(%msg, "admission"),
        connect::Transition::DropAndIdle(msg) => {
            conn.close(0u32.into(), b"refused");
            return Err(anyhow!(msg));
        }
    }

    // The trust gate. With no pin, core accepts *any* host certificate by design, so this is the
    // only place the decision is enforced: nothing streams, and `can_send_input()` stays false,
    // until the operator confirms the fingerprint they were shown.
    if machine.awaiting_trust() {
        let fp = observed_fp.ok_or_else(|| {
            anyhow!("cannot confirm trust: the host presented no certificate fingerprint")
        })?;
        let fp_hex = hex(&fp);
        let confirmed = match args.trust {
            Some(want) => {
                if want != fp {
                    tracing::warn!(
                        %remote,
                        expected = %hex(&want),
                        presented = %fp_hex,
                        "--trust fingerprint does not match the host"
                    );
                }
                want == fp
            }
            None => confirm_fingerprint(&host_label, &fp_hex)?,
        };
        if !confirmed {
            conn.close(0u32.into(), b"trust declined");
            return Err(anyhow!(
                "{host_label} presents fingerprint {fp_hex}, which was not confirmed - refusing to \
                 stream. Verify it out of band, then re-run with --trust {fp_hex} (or --pin {fp_hex} \
                 to pin it permanently)."
            ));
        }
        machine.trusted().map_err(|e| anyhow!(e))?;
        tracing::info!(%remote, fingerprint = %fp_hex, "host fingerprint confirmed");
    }

    // Admission is the first gate: input events before this point are never queued. The second
    // gate is window focus, maintained by WM_ACTIVATE/WM_SETFOCUS/WM_KILLFOCUS in `window.rs`.
    let _input_guard = input.as_ref().map(|s| {
        s.set_enabled(true);
        InputSenderGuard::new(s.clone())
    });
    if let (Some(sender), Some(rx)) = (&input, input_rx) {
        let pump_conn = conn.clone();
        let pump_sender = sender.clone();
        tokio::spawn(async move { pump_sender.pump(rx, pump_conn).await });
    }

    // Reserve our data-plane port, then let the host start.
    let probe = std::net::UdpSocket::bind("0.0.0.0:0")?;
    let udp_port = probe.local_addr()?.port();
    drop(probe);
    io::write_msg(
        &mut send,
        &Start {
            client_udp_port: udp_port,
        }
        .encode(),
    )
    .await?;

    let host_udp = SocketAddr::new(remote.ip(), welcome.udp_port);
    let session_cfg = welcome.session_config(Role::Client);
    let expected = welcome.frames;
    let codec = welcome.codec;
    let mode = welcome.mode;
    let record = args.record.clone();
    let seconds = args.seconds;

    // The host may refuse the mode we asked for and send `Reconfigured` before the first AU.
    // Keep reading the control stream so that correction is not lost.
    let delivered: std::sync::Arc<std::sync::Mutex<Option<slipstream_core::config::Mode>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    {
        let delivered = delivered.clone();
        let mut ctl = recv;
        tokio::spawn(async move {
            while let Ok(msg) = ctl.read_msg().await {
                match slipstream_core::quic::Reconfigured::decode(&msg) {
                    Ok(rc) => {
                        tracing::info!(
                            accepted = rc.accepted,
                            width = rc.mode.width,
                            height = rc.mode.height,
                            refresh_hz = rc.mode.refresh_hz,
                            "host corrected the session mode"
                        );
                        *delivered.lock().unwrap() = Some(rc.mode);
                    }
                    Err(_) => tracing::debug!(len = msg.len(), "control message (not a mode ack)"),
                }
            }
        });
    }

    let started = Instant::now();
    let (frames, bytes, presented, stop) =
        tokio::task::spawn_blocking(move || -> Result<(u64, u64, u64, SessionStop)> {
        let transport = UdpTransport::connect(&format!("0.0.0.0:{udp_port}"), &host_udp.to_string())
            .context("bind data plane")?;
        let mut session =
            Session::new(session_cfg, Box::new(transport)).map_err(|e| anyhow!("client session: {e:?}"))?;
        let mut sink = match &record {
            Some(p) => Some(std::io::BufWriter::new(
                std::fs::File::create(p).with_context(|| format!("create {p}"))?,
            )),
            None => None,
        };

        // Decode + present. `SLIPSTREAM_WINDOWS_HEADLESS=1` keeps the old counting path (CI, and
        // `--record`-only runs). Otherwise: Media Foundation → Win32 window. A window that cannot
        // be created is reported and the session still runs headless — never a silent black frame.
        let headless = std::env::var_os("SLIPSTREAM_WINDOWS_HEADLESS").is_some();
        let (mut decoder, mut presenter): (Box<dyn Decoder>, Box<dyn Presenter>) = if headless {
            (
                Box::new(PassthroughDecoder::default()),
                Box::new(ConsolePresenter::default()),
            )
        } else {
            // Default decode is the `ffmpeg.exe` child (see [`ff`]); the Media Foundation MFT is
            // opt-in because the Microsoft HEVC decoder is an async DXVA MFT that needs a D3D11
            // device manager before it emits anything.
            let use_mf = std::env::var_os("SLIPSTREAM_WINDOWS_DECODER_MF").is_some();
            let dec: Box<dyn Decoder> = if use_mf {
                Box::new(mf::MfDecoder::new(mode.width, mode.height, codec)?)
            } else {
                Box::new(ff::FfmpegDecoder::new(mode.width, mode.height, codec)?)
            };
            let pres: Box<dyn Presenter> = match window::GdiPresenter::new(
                &format!("slipstream — {host_label}"),
                mode.width,
                mode.height,
                input.clone(),
            ) {
                Ok(p) => Box::new(p),
                Err(e) => {
                    tracing::warn!(error = %format!("{e:#}"), "no window — presenting headless");
                    Box::new(ConsolePresenter::default())
                }
            };
            (dec, pres)
        };
        presenter.status(&format!("receiving from {host_label}"));
        let (mut frames, mut bytes) = (0u64, 0u64);
        let mut presented = 0u64;
        let mut stats = PipeStats::default();
        let mut last_rx = Instant::now();
        // The geometry we are currently sized for; starts at what the Welcome promised and moves
        // to whatever the host says it is actually delivering.
        let mut sized = (mode.width, mode.height);
        let stop;
        loop {
            if presenter.closed() {
                tracing::info!("window closed — ending the session");
                stop = SessionStop::WindowClosed;
                break;
            }
            if let Some(m) = delivered.lock().unwrap().take() {
                if (m.width, m.height) != sized {
                    tracing::info!(
                        from_w = sized.0,
                        from_h = sized.1,
                        to_w = m.width,
                        to_h = m.height,
                        refresh_hz = m.refresh_hz,
                        "adopting the delivered mode (requested mode was refused)"
                    );
                    sized = (m.width, m.height);
                    decoder.resize(m.width, m.height);
                    presenter.resize(m.width, m.height);
                }
            }
            if let Some(secs) = seconds {
                if started.elapsed() > Duration::from_secs(secs) {
                    stop = SessionStop::Ended;
                    break;
                }
            } else if last_rx.elapsed() > Duration::from_secs(45) {
                tracing::warn!("no frames for 45 s — ending the session");
                stop = SessionStop::TimedOut;
                break;
            }
            if expected > 0 && frames >= expected as u64 {
                stop = SessionStop::Ended;
                break;
            }
            let t_poll = Instant::now();
            let polled = session.poll_frame();
            let poll_ns = t_poll.elapsed().as_nanos() as u64;
            let mut spin_ns = 0u64;
            match polled {
                Ok(frame) => {
                    last_rx = Instant::now();
                    if frame.flags & FLAG_PROBE as u32 != 0 {
                        continue; // speed-test filler, not video
                    }
                    frames += 1;
                    stats.note_au();
                    bytes += frame.data.len() as u64;
                    if let Some(s) = sink.as_mut() {
                        s.write_all(&frame.data).context("write AU")?;
                    }
                    let t_dec = Instant::now();
                    let decoded = decoder.decode(&frame.data, codec);
                    let decode_ns = t_dec.elapsed().as_nanos() as u64;
                    if decoded.is_some() {
                        stats.note_decoded();
                        presented += 1;
                        stats.note_presented();
                    }
                    let t_pres = Instant::now();
                    presenter.present(decoded.as_ref());
                    let present_ns = t_pres.elapsed().as_nanos() as u64;
                    let (qm, bm) = decoder.flow_stats();
                    stats.queue_max = qm;
                    stats.blocked_ms = bm;
                    stats.timed += 1;
                    stats.note_costs(
                        poll_ns,
                        spin_ns,
                        decode_ns,
                        present_ns,
                        presenter.stage_ns(),
                    );
                    stats.tick();
                }
                Err(SlipstreamError::NoFrame) => {
                    let t_spin = Instant::now();
                    std::thread::sleep(Duration::from_micros(300));
                    spin_ns = t_spin.elapsed().as_nanos() as u64;
                    stats.spins += 1;
                    // Poll+spin without a frame still costs the same wall time; attribute it to
                    // the next frame's window so the split stays per-frame.
                    stats.poll_ns += poll_ns;
                    stats.spin_ns += spin_ns;
                }
                Err(e) => return Err(anyhow!("poll_frame: {e:?}")),
            }
        }
        if let Some(mut s) = sink {
            s.flush().ok();
        }
        Ok((frames, bytes, presented, stop))
    })
    .await??;

    conn.close(0u32.into(), b"quit");
    println!(
        "session ended: received {frames} video AUs, presented {presented} frames, {bytes} bytes in {:.1}s ({:.1} fps presented)",
        started.elapsed().as_secs_f64(),
        presented as f64 / started.elapsed().as_secs_f64().max(0.001),
    );
    Ok(SessionOutcome::from_result(true, stop, frames))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_frames_maps_to_zero_frames() {
        assert_eq!(
            SessionOutcome::from_result(true, SessionStop::Ended, 0),
            SessionOutcome::ZeroFrames
        );
    }

    #[test]
    fn clean_end_with_frames_maps_to_ended() {
        assert_eq!(
            SessionOutcome::from_result(true, SessionStop::Ended, 1),
            SessionOutcome::Ended
        );
    }

    #[test]
    fn external_stops_keep_their_cause() {
        assert_eq!(
            SessionOutcome::from_result(true, SessionStop::WindowClosed, 0),
            SessionOutcome::WindowClosed
        );
        assert_eq!(
            SessionOutcome::from_result(true, SessionStop::TimedOut, 10),
            SessionOutcome::TimedOut
        );
        assert_eq!(
            SessionOutcome::from_result(true, SessionStop::WindowClosed, 10),
            SessionOutcome::WindowClosed
        );
    }

    #[test]
    fn connect_failed_maps_to_connect_failed() {
        assert_eq!(
            SessionOutcome::from_result(false, SessionStop::Ended, 0),
            SessionOutcome::ConnectFailed
        );
    }
}
