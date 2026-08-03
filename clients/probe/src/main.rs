//! `slipstream-probe` — the reference client for `slipstream/1`: QUIC control plane, UDP data
//! plane, input over QUIC datagrams. Two modes, decided by the host's Welcome:
//!
//! * **verification** (`frames > 0`, synthetic host): byte-checks deterministic test frames;
//! * **stream** (`frames == 0`, virtual host): receives real encoded AUs, writes a playable
//!   elementary stream (the dump extension follows the negotiated codec — `.h265`/`.h264`/`.av1`;
//!   the probe advertises all three), and reports per-frame **capture→received latency**
//!   percentiles (the host stamps each frame with its capture wall clock; same-host runs share
//!   that clock).
//!
//! `--input-test` exercises the input plane: scripted mouse/keyboard datagrams during the
//! stream (watch them land in the host session, e.g. xev inside gamescope). `--mic-test`
//! exercises the mic uplink: a synthetic 440 Hz tone streamed as Opus (0xCB) → the host's
//! virtual microphone source (record it host-side to hear the tone). `--touch-test` drags a
//! synthetic finger in a circle → host libei `ei_touchscreen` injection. `--rich-input-test`
//! drives a virtual DualSense touchpad + motion over the 0xCC plane (host on
//! `SLIPSTREAM_GAMEPAD=dualsense`) and logs the 0xCD HID-output feedback (lightbar / adaptive
//! triggers) that comes back.
//!
//! `--pin <64-hex>` pins the host's certificate fingerprint (the host logs it at startup);
//! without it the client trusts on first use and prints the observed fingerprint to pin.
//! `--pair <PIN>` runs the SPAKE2 pairing ceremony: read the PIN the host prints when it
//! arms pairing (`--allow-pairing`/`--require-pairing`), pass it here; on success the
//! client prints the verified host fingerprint to `--pin` from then on.
//! Host→client datagrams (Opus audio, rumble) are counted and reported with the stream
//! stats — decode/playback is the platform clients' job.
//!
//! `--compositor NAME` requests a host compositor backend (`auto`|`kwin`|`wlroots`|`mutter`|
//! `gamescope`); the host honors it if available, else auto-detects and reports the resolved
//! choice in its Welcome (logged as `session offer … compositor=…`).
//!
//! `--gamepad NAME` requests a host virtual-pad backend
//! (`auto`|`xbox360`|`dualsense`|`xboxone`|`dualshock4`); the host honors it where available (the
//! UHID pads — DualSense, DualShock 4 — need Linux), else falls back to X-Box 360, and reports the
//! resolved choice in its Welcome (logged as `session offer … gamepad=…`).
//!
//! `--discover [SECS]` browses the LAN for native (`_slipstream._udp`) hosts the host advertises
//! over mDNS, prints each (name, addr:port, pairing requirement, cert fingerprint to pin), and
//! exits without connecting.
//!
//! Usage: `slipstream-probe [--connect HOST:PORT] [--mode WxHxFPS] [--remode WxHxFPS:SECS]
//!         [--rebitrate KBPS:SECS]
//!         [--out FILE] [--bitrate KBPS] [--codec auto|h264|hevc|av1] [--audio-channels 2|6|8]
//!         [--launch APP] [--name NAME] [--speed-test KBPS:MS]
//!         [--input-test | --mic-test [--mic-burst] | --touch-test | --rich-input-test]
//!         [--pin HEX | --pair PIN] [--compositor NAME] [--gamepad NAME] | --discover [SECS]`
//! Env: `SLIPSTREAM_CLIENT_10BIT=1` / `SLIPSTREAM_CLIENT_444=1` advertise the 10-bit / 4:4:4 caps.
#![forbid(unsafe_code)]

mod args;
mod artifact;
mod discover;
mod session;

use anyhow::{anyhow, Context, Result};
use args::{hex, load_or_create_identity, parse_args, Args};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = parse_args();
    if let Err(e) = run(args) {
        tracing::error!("{e:#}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    // Discovery mode: browse the LAN for native hosts, print them, and exit (no connection).
    if let Some(secs) = args.discover {
        return discover::discover(secs);
    }
    // Pairing mode: run the PIN ceremony and print the fingerprint to pin, then exit.
    if let Some(pin) = &args.pair {
        let (host, port) = args
            .connect
            .rsplit_once(':')
            .context("--connect host:port")?;
        let identity = load_or_create_identity()?;
        let fp = slipstream_core::client::NativeClient::pair(
            host,
            port.parse().context("port")?,
            (&identity.0, &identity.1),
            pin,
            &args.name,
            std::time::Duration::from_secs(90),
        )
        .map_err(|e| anyhow!("pairing failed: {e:?} (wrong PIN?)"))?;
        tracing::info!(
            fingerprint = %hex(&fp),
            "PAIRED — connect with --pin {} from now on",
            hex(&fp)
        );
        return Ok(());
    }
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    rt.block_on(session::session(args))
}
