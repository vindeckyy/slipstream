//! `slipstream-windows` — the Windows session client.
//!
//! Today this binary is the **transport + admission + present-seam** half of the client: it
//! discovers `_slipstream._udp` hosts, honours the `pair=required` advert rule, runs the same PIN
//! pairing ceremony as every sibling client, connects over QUIC, negotiates a mode/codec, receives
//! every access unit of the video plane, and can record them to a playable elementary stream.
//!
//! The window and the hardware decode path attach at the two traits in [`present`] — see PR.md for
//! the exact anchors and why they are not in this change (no FFmpeg development SDK on the build
//! box, and `ss-presenter` is Vulkan/Linux-only). Everything else here is real: no stubs in the
//! connect path.
//!
//! Decode is Media Foundation (the OS HEVC/H.264 decoder MFTs) and presentation is a Win32 window
//! with a GDI blit: [`mf`] and [`window`]. Those two modules hold the only `unsafe` in the crate.
#![deny(unsafe_code)]

mod args;
mod connect;
mod discovery;
mod ff;
mod identity;
mod input;
mod mf;
mod present;
mod session;
mod window;

use anyhow::{anyhow, Context, Result};
use args::{hex, Action, Args, Target};
use present::Presenter;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use session::{run_stream, SessionConfig, SessionOutcome};

/// Pipeline counters, printed once a second: AUs received → decoded → presented, plus the interval
/// between consecutive presented frames. The interval distribution is the point: "steady low fps"
/// and "mostly fast with periodic 200-400 ms hitches" are different bugs with different fixes.
#[derive(Default)]
struct PipeStats {
    aus: u64,
    decoded: u64,
    presented: u64,
    /// Presented-frame intervals (µs) within the current one-second window.
    intervals: Vec<u32>,
    last_present: Option<Instant>,
    window: Option<Instant>,
    /// Highest frame-queue depth the decoder reported (flow control, bounded).
    queue_max: usize,
    /// Milliseconds blocked writing an AU because the queue was full.
    blocked_ms: u64,
    /// Diagnostic stage totals for the current window (nanoseconds) and the number of frames
    /// they were accumulated over.
    poll_ns: u64,
    spin_ns: u64,
    spins: u64,
    decode_ns: u64,
    present_ns: u64,
    convert_ns: u64,
    blit_ns: u64,
    /// Last cumulative values of the presenter's own counters (they are cumulative).
    seen_convert_ns: u64,
    seen_blit_ns: u64,
    timed: u64,
}

impl PipeStats {
    fn note_au(&mut self) {
        self.aus += 1;
    }

    fn note_decoded(&mut self) {
        self.decoded += 1;
    }

    /// Fold one loop iteration's stage costs in. `presenter_stage` is the presenter's cumulative
    /// `(convert_ns, blit_ns)`.
    fn note_costs(
        &mut self,
        poll_ns: u64,
        spin_ns: u64,
        decode_ns: u64,
        present_ns: u64,
        presenter_stage: (u64, u64),
    ) {
        self.poll_ns += poll_ns;
        self.spin_ns += spin_ns;
        self.decode_ns += decode_ns;
        self.present_ns += present_ns;
        self.convert_ns += presenter_stage.0.saturating_sub(self.seen_convert_ns);
        self.blit_ns += presenter_stage.1.saturating_sub(self.seen_blit_ns);
        self.seen_convert_ns = presenter_stage.0;
        self.seen_blit_ns = presenter_stage.1;
    }

    fn note_presented(&mut self) {
        self.presented += 1;
        let now = Instant::now();
        if let Some(prev) = self.last_present {
            self.intervals
                .push(now.duration_since(prev).as_micros().min(u32::MAX as u128) as u32);
        }
        self.last_present = Some(now);
    }

    /// Print and reset the one-second window. Returns nothing; the numbers are the record.
    fn tick(&mut self) {
        let now = Instant::now();
        let start = *self.window.get_or_insert(now);
        if now.duration_since(start) < Duration::from_secs(1) {
            return;
        }
        let mut iv = std::mem::take(&mut self.intervals);
        iv.sort_unstable();
        let pick = |q: f64| -> f64 {
            if iv.is_empty() {
                return 0.0;
            }
            let i = ((iv.len() - 1) as f64 * q) as usize;
            iv[i] as f64 / 1000.0
        };
        tracing::info!(
            received = self.aus,
            decoded = self.decoded,
            presented = self.presented,
            ivl_p50_ms = format!("{:.1}", pick(0.50)),
            ivl_p95_ms = format!("{:.1}", pick(0.95)),
            ivl_p99_ms = format!("{:.1}", pick(0.99)),
            ivl_max_ms = format!("{:.1}", pick(1.0)),
            n_ivl = iv.len(),
            queue_max = self.queue_max,
            blocked_ms = self.blocked_ms,
            "pipeline received -> decoded -> presented"
        );
        // Stage split for the same window, in milliseconds per frame. `poll` is the whole
        // `Session::poll_frame` call (including its NoFrame sleep, also shown as `spin`),
        // `decode` is the decoder call (ffmpeg stdin write + queue pop), `present` the whole
        // presenter call, and `convert`/`blit` the presenter's own two stages.
        let n = self.timed.max(1) as f64; // per-frame denominator
        tracing::info!(
            frames = self.timed,
            poll_ms = format!("{:.2}", self.poll_ns as f64 / 1e6 / n),
            spin_ms = format!("{:.2}", self.spin_ns as f64 / 1e6 / n),
            spins = self.spins,
            decode_ms = format!("{:.2}", self.decode_ns as f64 / 1e6 / n),
            present_ms = format!("{:.2}", self.present_ns as f64 / 1e6 / n),
            convert_ms = format!("{:.2}", self.convert_ns as f64 / 1e6 / n),
            blit_ms = format!("{:.2}", self.blit_ns as f64 / 1e6 / n),
            "stage cost per frame"
        );
        self.poll_ns = 0;
        self.spin_ns = 0;
        self.spins = 0;
        self.decode_ns = 0;
        self.present_ns = 0;
        self.convert_ns = 0;
        self.blit_ns = 0;
        self.timed = 0;
        self.window = Some(now);
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = match args::parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    if let Err(e) = run(args) {
        tracing::error!("{e:#}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<()> {
    match args.action() {
        None => {
            println!("{}", args::usage());
            Ok(())
        }
        Some(Action::Discover(secs)) => discovery::run(secs),
        Some(Action::Pair) => pair(&args),
        Some(Action::Play) => play(&args),
        Some(Action::Connect) => stream(args),
    }
}

/// `--play FILE`: decode a recorded Annex-B stream into the window with no network.
///
/// This is the decode+present stack in isolation, which is exactly what a live-session run cannot
/// separate from host/network state.
fn play(args: &Args) -> Result<()> {
    let path = args.play.as_deref().context("--play needs a file")?;
    let data = std::fs::read(path).with_context(|| format!("read {path}"))?;
    let codec = if args.preferred_codec == 0 {
        slipstream_core::quic::CODEC_HEVC
    } else {
        args.preferred_codec
    };
    let mut decoder = ff::FfmpegDecoder::new(args.mode.width, args.mode.height, codec)?;
    let mut presenter = window::GdiPresenter::new(
        &format!("slipstream — {path}"),
        args.mode.width,
        args.mode.height,
        None,
    )
    .context("create window")?;
    let aus = split_aus(&data);
    if aus.is_empty() {
        return Err(anyhow!("{path}: no Annex-B access units found"));
    }
    let started = Instant::now();
    let mut presented = 0u64;
    let mut stats = PipeStats::default();
    // Paced to the nominal refresh so the run is watchable; `SLIPSTREAM_PLAY_UNPACED=1` runs the
    // decode+present stack as fast as it can, which is how its cost (not the pacer's) is measured.
    let unpaced = std::env::var_os("SLIPSTREAM_PLAY_UNPACED").is_some();
    let frame_gap = if unpaced {
        Duration::ZERO
    } else {
        Duration::from_micros(1_000_000 / args.mode.refresh_hz.max(1) as u64)
    };
    for au in &aus {
        if presenter.closed() {
            break;
        }
        if let Some(s) = args.seconds {
            if started.elapsed() > Duration::from_secs(s) {
                break;
            }
        }
        stats.note_au();
        let t_dec = Instant::now();
        let decoded = decoder.decode_one(au)?;
        let decode_ns = t_dec.elapsed().as_nanos() as u64;
        let t_pres = Instant::now();
        stats.timed += 1;
        match decoded {
            Some(f) => {
                stats.note_decoded();
                presented += 1;
                stats.note_presented();
                presenter.present(Some(&f));
            }
            None => presenter.present(None),
        }
        let present_ns = t_pres.elapsed().as_nanos() as u64;
        stats.note_costs(0, 0, decode_ns, present_ns, presenter.stage_ns());
        stats.tick();
        if !frame_gap.is_zero() {
            std::thread::sleep(frame_gap);
        }
    }
    // Drain what the decoder still holds once its input closes.
    for f in decoder.finish() {
        if presenter.closed() {
            break;
        }
        stats.note_decoded();
        stats.note_presented();
        presented += 1;
        presenter.present(Some(&f));
    }
    let secs = started.elapsed().as_secs_f64();
    println!(
        "played {} AUs from {path}: presented {presented} frames in {secs:.1}s ({:.1} fps presented)",
        aus.len(),
        presented as f64 / secs.max(0.001)
    );
    Ok(())
}

/// Split an Annex-B elementary stream into access units at AUD NALs (HEVC type 35, H.264 type 9).
/// A stream without AUDs falls back to one NAL per unit, which is what the encoder block emits.
fn split_aus(data: &[u8]) -> Vec<Vec<u8>> {
    // Start-code offsets.
    let mut starts = Vec::new();
    let mut i = 0usize;
    while i + 3 <= data.len() {
        if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
            let pos = if i > 0 && data[i - 1] == 0 { i - 1 } else { i };
            starts.push((pos, i + 3));
            i += 3;
        } else {
            i += 1;
        }
    }
    let nal_type = |body: usize| -> u8 { data.get(body).map(|b| (b >> 1) & 0x3F).unwrap_or(0) };
    let auds: Vec<usize> = (0..starts.len())
        .filter(|&k| nal_type(starts[k].1) == 35)
        .collect();
    let boundaries: Vec<usize> = if auds.len() > 1 {
        auds.iter().map(|&k| starts[k].0).collect()
    } else {
        starts.iter().map(|&(pos, _)| pos).collect()
    };
    boundaries
        .iter()
        .enumerate()
        .filter_map(|(k, &start)| {
            let end = boundaries.get(k + 1).copied().unwrap_or(data.len());
            (end > start).then(|| data[start..end].to_vec())
        })
        .collect()
}

fn target_of(args: &Args) -> Result<Target> {
    let raw = args
        .connect
        .as_deref()
        .context("no host given (pass HOST[:PORT] or --discover)")?;
    Target::parse(raw).map_err(|e| anyhow!(e))
}

fn resolve(t: &Target) -> Result<SocketAddr> {
    use std::net::ToSocketAddrs;
    let addrs: Vec<SocketAddr> = (t.host.as_str(), t.port)
        .to_socket_addrs()
        .with_context(|| format!("resolve {}:{}", t.host, t.port))?
        .collect();
    // Prefer IPv4: the host's data plane is UDP over v4 today.
    addrs
        .iter()
        .find(|a| a.is_ipv4())
        .or_else(|| addrs.first())
        .copied()
        .ok_or_else(|| anyhow!("{}:{} resolved to no addresses", t.host, t.port))
}

/// `--pair PIN`: the SPAKE2 ceremony. On success the host trusts this client's certificate, so
/// later connects are admitted without a PIN — only the fingerprint pin matters.
fn pair(args: &Args) -> Result<()> {
    let addr = resolve(&target_of(args)?)?;
    let pin = args.pair.as_deref().context("--pair needs a PIN")?;
    let (cert, key) = identity::load_or_create()?;
    let fp = slipstream_core::client::NativeClient::pair(
        &addr.ip().to_string(),
        addr.port(),
        (&cert, &key),
        pin,
        &args.name,
        Duration::from_secs(90),
    )
    .map_err(|e| anyhow!("pairing failed: {e:?} (wrong PIN? is pairing armed on the host?)"))?;
    println!("PAIRED — host fingerprint {}", hex(&fp));
    println!("connect with: slipstream-windows {addr} --pin {}", hex(&fp));
    Ok(())
}

fn stream(args: Args) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    rt.block_on(stream_async(args))
}

async fn stream_async(args: Args) -> Result<()> {
    let remote = resolve(&target_of(&args)?)?;
    let headless = std::env::var_os("SLIPSTREAM_WINDOWS_HEADLESS").is_some();
    let (input, input_rx) = if headless {
        (None, None)
    } else {
        let (sender, rx) = input::InputSender::channel();
        (Some(sender), Some(rx))
    };
    match run_stream(SessionConfig {
        remote,
        args,
        input,
        input_rx,
    })
    .await?
    {
        SessionOutcome::ZeroFrames => {
            tracing::warn!(
                "zero frames: the host accepted the session but published none \
                 (a wedged host capture thread does this — it is a host-side bug, not a client one)"
            );
            std::process::exit(3);
        }
        SessionOutcome::Ended
        | SessionOutcome::ConnectFailed
        | SessionOutcome::WindowClosed
        | SessionOutcome::TimedOut => Ok(()),
    }
}
