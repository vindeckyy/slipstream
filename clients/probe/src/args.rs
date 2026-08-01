//! CLI argument parsing and small helpers for slipstream-probe.

use anyhow::{anyhow, Context, Result};
use slipstream_core::config::GamepadPref;
use slipstream_core::quic::endpoint;
use slipstream_core::{CompositorPref, Mode};
use std::io::Write;

pub(crate) struct Args {
    pub(crate) connect: String,
    pub(crate) mode: Mode,
    pub(crate) out: Option<String>,
    pub(crate) input_test: bool,
    /// `--mic-test` — stream a synthetic 440 Hz tone as the mic uplink (proves the mic path).
    pub(crate) mic_test: bool,
    /// `--mic-burst` — pace the mic-test like a real client's input tap (2× 20 ms per 40 ms),
    /// the arrival shape that exercises host-side jitter buffering.
    pub(crate) mic_burst: bool,
    /// `--touch-test` — drag a synthetic finger in a circle (proves the touch path).
    pub(crate) touch_test: bool,
    /// `--rich-input-test` — drive the DualSense touchpad + motion over 0xCC (host needs
    /// `SLIPSTREAM_GAMEPAD=dualsense`); also logs the 0xCD HID-output feedback that comes back.
    pub(crate) rich_input_test: bool,
    /// `--quit` — close the connection with the deliberate-quit code (`QUIT_CLOSE_CODE`) at end of
    /// stream, so the host tears its virtual display down immediately (skips keep-alive linger). A
    /// bare exit closes with code 0 → the host lingers for a reconnect. Tests the #2 quit path.
    pub(crate) quit: bool,
    /// `--seconds N` — cap the receive loop at N seconds, then end the session gracefully (reach the
    /// `conn.close`). Without it the loop runs to the 120s cap. Lets a test bound a live-host stream so
    /// the client-initiated close (with/without `--quit`) fires promptly.
    pub(crate) seconds: Option<u64>,
    pub(crate) pin: Option<[u8; 32]>,
    /// `--remode WxHxFPS:SECS` — request this mode SECS seconds into the stream.
    pub(crate) remode: Option<(Mode, u32)>,
    /// `--rebitrate KBPS:SECS` — send a mid-stream [`SetBitrate`] (the adaptive-bitrate control
    /// message) SECS seconds into the stream: the headless validator for the host's in-place
    /// encoder rate retarget (Phase 3.2) / rebuild fallback. Wiggles the cursor around the switch
    /// so a damage-driven idle desktop actually publishes frames through it.
    pub(crate) rebitrate: Option<(u32, u32)>,
    /// `--pair PIN` — run the pairing ceremony instead of a session.
    pub(crate) pair: Option<String>,
    /// `--name LABEL` — how the host labels this client when pairing.
    pub(crate) name: String,
    /// `--compositor NAME` — request a host compositor backend (auto|kwin|wlroots|mutter|gamescope).
    pub(crate) compositor: CompositorPref,
    /// `--gamepad NAME` — request a host virtual-pad backend (auto|xbox360|dualsense).
    pub(crate) gamepad: GamepadPref,
    /// `--bitrate KBPS` — request this encoder bitrate (kilobits/s); 0 = host default.
    pub(crate) bitrate_kbps: u32,
    /// `--audio-channels N` — request stereo (2), 5.1 (6) or 7.1 (8) audio; default 2. The probe
    /// multistream-decodes the host's frames and asserts the per-channel sample count, so it's the
    /// headless validator for the surround encode path.
    pub(crate) audio_channels: u8,
    /// `--codec h264|hevc|av1|auto` — the preferred video codec (soft; the host honors it when it can
    /// emit it, else falls back). The probe always advertises it can decode all three; this just sets
    /// the preference byte. `auto` (default) = no preference (host decides). `0` = auto.
    pub(crate) preferred_codec: u8,
    /// `--launch ID` — ask the host to launch a library title in this session (a store-qualified
    /// id from the host's `GET /api/v1/library`, e.g. `steam:570`). Host resolves it; `None` = none.
    pub(crate) launch: Option<String>,
    /// `--speed-test KBPS:MS` — after the stream starts, ask the host for a `MS`-millisecond
    /// bandwidth probe burst at `KBPS`, then report measured throughput + loss.
    pub(crate) speed_test: Option<(u32, u32)>,
    /// `--cursor-capture` — negotiate the cursor channel (`CLIENT_CAP_CURSOR`) and immediately
    /// flip it to the capture model (`CursorRenderMode { client_draws: false }`), then wiggle the
    /// pointer with RELATIVE motion for the whole stream: the headless reproduction of a
    /// pointer-lock client expecting the HOST to composite the cursor into the video. Decode the
    /// dump and look for the pointer — a cursorless dump is the bug this flag was built to catch.
    pub(crate) cursor_capture: bool,
    /// `--cursor-nochannel` — the same relative wiggle WITHOUT the cursor channel: no
    /// `CLIENT_CAP_CURSOR`, no render-mode flip. The headless reproduction of a client LATCHED
    /// in capture mode at connect (`console.rs` `latched_mouse` — it never advertises the
    /// channel), the shape of the 2026-07 "no cursor in Mutter capture mode" field report. The
    /// host must composite the metadata cursor on its own; decode the dump and look for the
    /// pointer.
    pub(crate) cursor_nochannel: bool,
    /// `--discover [SECS]` — browse the LAN for native (`_slipstream._udp`) hosts for `SECS`
    /// seconds (default 4), print what's found, and exit. No connection is made.
    pub(crate) discover: Option<u64>,
    /// `--clock-resync` — after the connect-time skew handshake, immediately run a SECOND
    /// handshake on the same control stream and assert both estimates are sane and consistent:
    /// the headless validator for the host answering `ClockProbe` at any time (what the native
    /// clients' mid-stream re-sync relies on). Aborts the session when the re-probe fails.
    pub(crate) clock_resync: bool,
}

pub(crate) fn parse_mode(m: &str) -> Option<Mode> {
    let mut it = m.split('x');
    Some(Mode {
        width: it.next()?.parse().ok()?,
        height: it.next()?.parse().ok()?,
        refresh_hz: it.next()?.parse().ok()?,
    })
}

pub(crate) fn parse_hex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(out)
}

pub(crate) fn hex(fp: &[u8; 32]) -> String {
    fp.iter().map(|b| format!("{b:02x}")).collect()
}

/// This client's persistent identity (`~/.config/slipstream/client-{cert,key}.pem`),
/// generated on first use — presented on every connect so hosts can recognize it once
/// paired.
pub(crate) fn load_or_create_identity() -> Result<(String, String)> {
    let home = std::env::var("HOME").context("HOME unset")?;
    let dir = std::path::PathBuf::from(home).join(".config/slipstream");
    let (cp, kp) = (dir.join("client-cert.pem"), dir.join("client-key.pem"));
    if let (Ok(c), Ok(k)) = (std::fs::read_to_string(&cp), std::fs::read_to_string(&kp)) {
        // Re-lock a store an older build left world-readable (this key is shared with the other
        // clients' `~/.config/slipstream/client-key.pem`); best-effort.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
            let _ = std::fs::set_permissions(&kp, std::fs::Permissions::from_mode(0o600));
        }
        return Ok((c, k));
    }
    let (c, k) = endpoint::generate_identity().map_err(|e| anyhow!("generate identity: {e}"))?;
    std::fs::create_dir_all(&dir)?;
    // The certificate is public; the key is the mTLS credential a paired host authorizes for full
    // remote control, so it must not be world-readable — create it 0600 (a plain `fs::write`
    // honors the umask → typically 0644).
    std::fs::write(&cp, &c)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&kp)?;
        f.write_all(k.as_bytes())?;
    }
    #[cfg(not(unix))]
    std::fs::write(&kp, &k)?;
    tracing::info!(cert = %cp.display(), "generated client identity");
    Ok((c, k))
}

pub(crate) fn parse_args() -> Args {
    let argv: Vec<String> = std::env::args().collect();
    let get = |flag: &str| {
        argv.iter()
            .skip_while(|a| *a != flag)
            .nth(1)
            .map(String::as_str)
    };
    let mode = get("--mode").and_then(parse_mode).unwrap_or(Mode {
        width: 1280,
        height: 720,
        refresh_hz: 60,
    });
    let remode = get("--remode").and_then(|s| {
        let (m, secs) = s.split_once(':')?;
        Some((parse_mode(m)?, secs.parse().ok()?))
    });
    let rebitrate = get("--rebitrate").and_then(|s| {
        let (kbps, secs) = s.split_once(':')?;
        Some((kbps.parse().ok()?, secs.parse().ok()?))
    });
    // A present-but-malformed --pin must abort, not silently downgrade to trust-on-first-use
    // (the user asked for verification; fail closed).
    let pin = match get("--pin") {
        None => None,
        Some(s) => {
            match parse_hex32(s) {
                Some(p) => Some(p),
                None => {
                    eprintln!("--pin must be exactly 64 hex chars (the host logs its fingerprint at startup)");
                    std::process::exit(2);
                }
            }
        }
    };
    // A present-but-unrecognized --compositor must abort rather than silently auto-detect.
    let compositor = match get("--compositor") {
        None => CompositorPref::Auto,
        Some(s) => match CompositorPref::from_name(s) {
            Some(c) => c,
            None => {
                eprintln!("--compositor must be one of: auto, kwin, wlroots, mutter, gamescope");
                std::process::exit(2);
            }
        },
    };
    // Same fail-closed discipline for --gamepad.
    let gamepad = match get("--gamepad") {
        None => GamepadPref::Auto,
        Some(s) => match GamepadPref::from_name(s) {
            Some(g) => g,
            None => {
                eprintln!(
                    "--gamepad must be one of: auto, xbox360, dualsense, xboxone, dualshock4"
                );
                std::process::exit(2);
            }
        },
    };
    Args {
        connect: get("--connect").unwrap_or("127.0.0.1:9777").to_string(),
        mode,
        out: get("--out").map(String::from),
        input_test: argv.iter().any(|a| a == "--input-test"),
        mic_test: argv.iter().any(|a| a == "--mic-test"),
        mic_burst: argv.iter().any(|a| a == "--mic-burst"),
        touch_test: argv.iter().any(|a| a == "--touch-test"),
        rich_input_test: argv.iter().any(|a| a == "--rich-input-test"),
        quit: argv.iter().any(|a| a == "--quit"),
        seconds: get("--seconds").and_then(|s| s.parse().ok()),
        pin,
        remode,
        rebitrate,
        pair: get("--pair").map(String::from),
        name: get("--name").unwrap_or("slipstream-probe").to_string(),
        compositor,
        gamepad,
        bitrate_kbps: get("--bitrate").and_then(|s| s.parse().ok()).unwrap_or(0),
        audio_channels: slipstream_core::audio::normalize_channels(
            get("--audio-channels")
                .and_then(|s| s.parse().ok())
                .unwrap_or(2),
        ),
        preferred_codec: match get("--codec").unwrap_or("auto") {
            "h264" | "avc" => slipstream_core::quic::CODEC_H264,
            "hevc" | "h265" => slipstream_core::quic::CODEC_HEVC,
            "av1" => slipstream_core::quic::CODEC_AV1,
            "pyrowave" => slipstream_core::quic::CODEC_PYROWAVE,
            _ => 0, // auto — no preference
        },
        launch: get("--launch").map(str::to_string),
        speed_test: get("--speed-test").and_then(|s| {
            let (kbps, ms) = s.split_once(':')?;
            Some((kbps.parse().ok()?, ms.parse().ok()?))
        }),
        // `--discover` may be a bare flag or carry a seconds value (`--discover 8`); only treat
        // the following token as a count when it parses as a number (else it's the next flag).
        discover: argv
            .iter()
            .any(|a| a == "--discover")
            .then(|| get("--discover").and_then(|s| s.parse().ok()).unwrap_or(4)),
        clock_resync: argv.iter().any(|a| a == "--clock-resync"),
        cursor_capture: argv.iter().any(|a| a == "--cursor-capture"),
        cursor_nochannel: argv.iter().any(|a| a == "--cursor-nochannel"),
    }
}

pub(crate) fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Human name for the negotiated `Welcome::codec` (also the natural `--out` file extension). The
/// bitstream is dumped verbatim, so an H.264 software-host session should be saved as `.h264`.
pub(crate) fn codec_ext(codec: u8) -> &'static str {
    match codec {
        slipstream_core::quic::CODEC_H264 => "h264",
        slipstream_core::quic::CODEC_AV1 => "av1",
        slipstream_core::quic::CODEC_PYROWAVE => "pyrowave",
        _ => "h265",
    }
}
