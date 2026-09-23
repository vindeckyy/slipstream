//! WASAPI loopback capture: the default render endpoint's post-mix loopback at the
//! session's channel count, resampled to 48 kHz `f32`, into the same bounded-chunk
//! channel the PipeWire capturer feeds — so the Opus framing above is untouched.
//!
//! Format handling is explicit and honest: the mix format must be 32-bit float
//! (plain `WAVE_FORMAT_IEEE_FLOAT` or an extensible float subformat — what the engine
//! mixes in); integer mix formats fail loudly instead of misdecoding. Any mix rate is
//! resampled to [`SAMPLE_RATE`] with a stateful linear resampler (good enough for a
//! game stream; a higher-quality SRC is a follow-up, not a blocker). Channel mapping:
//! identical counts pass through untouched (Windows surround order FL FR FC LFE RL RR
//! SL SR is exactly the GameStream order, so no remap); surround→stereo downmixes with
//! ITU coefficients; narrower mixes zero-fill missing positions (the Linux zero-upmix
//! rule); mono duplicates to stereo.
//!
//! A default-device change (unplug, HDMI modeset) surfaces as
//! `AUDCLNT_E_DEVICE_INVALIDATED` and terminally fails the capturer — the caller
//! reopens against the new default, exactly like the PipeWire thread-death path.

use super::super::{AudioCapturer, AudioTelemetry, SAMPLE_RATE};
use anyhow::{Context, Result};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender},
    Arc,
};
use std::thread;
use std::time::Duration;
use windows::Win32::{
    Media::Audio::{
        eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDevice, IMMDeviceEnumerator,
        MMDeviceEnumerator, AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_E_DEVICE_INVALIDATED,
        AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, WAVEFORMATEX, WAVEFORMATEXTENSIBLE,
    },
    Media::Multimedia::WAVE_FORMAT_IEEE_FLOAT,
    System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED,
    },
};

/// `WAVE_FORMAT_EXTENSIBLE` (0xFFFE) — not projected by windows-rs, spelled literally.
const WAVE_FORMAT_EXTENSIBLE: u32 = 0xFFFE;

/// `KSDATAFORMAT_SUBTYPE_IEEE_FLOAT` (`{00000003-0000-0010-8000-00AA00389B71}`) —
/// not projected by windows-rs, so spelled literally for the extensible-subformat check.
const SUBTYPE_IEEE_FLOAT: windows::core::GUID =
    windows::core::GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71);

/// Loopback buffer duration: 1 s in 100 ns units.
const BUFFER_HNS: i64 = 10_000_000;

/// Mix-format facts the engine hands us, plus its stream latency for telemetry.
struct Ready {
    channels: u16,
    rate: u32,
    engine_latency_ms: f64,
}

/// WASAPI loopback capturer. Owns the worker thread; dropping stops it.
pub struct WasapiLoopback {
    chunks: Receiver<Vec<f32>>,
    channels: u32,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
    ring_samples: Arc<AtomicUsize>,
    ring_capacity: Arc<AtomicUsize>,
    underruns: Arc<AtomicU64>,
    overflow_dropped: Arc<AtomicU64>,
    engine_latency_ms: f64,
}

impl WasapiLoopback {
    pub fn open(channels: u32) -> Result<Self> {
        anyhow::ensure!(
            matches!(channels, 1 | 2 | 6 | 8),
            "unsupported audio channel count {channels} (want 1, 2, 6 or 8)"
        );
        let (tx, rx) = sync_channel::<Vec<f32>>(64);
        let (ready_tx, ready_rx) = sync_channel::<Result<Ready>>(1);
        let stop = Arc::new(AtomicBool::new(false));
        let ring_samples = Arc::new(AtomicUsize::new(0));
        let ring_capacity = Arc::new(AtomicUsize::new(0));
        let underruns = Arc::new(AtomicU64::new(0));
        let overflow_dropped = Arc::new(AtomicU64::new(0));
        let worker = thread::Builder::new()
            .name("slipstream-wasapi".into())
            .spawn({
                let (stop, ring_samples, ring_capacity, overflow_dropped) = (
                    stop.clone(),
                    ring_samples.clone(),
                    ring_capacity.clone(),
                    overflow_dropped.clone(),
                );
                move || {
                    if let Err(e) = wasapi_thread(
                        channels,
                        tx,
                        ready_tx,
                        stop,
                        ring_samples,
                        ring_capacity,
                        overflow_dropped,
                    ) {
                        tracing::error!(error = %format!("{e:#}"), "wasapi loopback thread failed");
                    }
                }
            })
            .context("spawn wasapi thread")?;
        // Bring-up handshake (mirrors the PipeWire capturer): an unusable engine surfaces
        // as an open ERROR — engaging the callers' reopen backoff — and the mix format is
        // validated before the first session depends on it.
        let ready = match ready_rx.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(ready)) => ready,
            Ok(Err(e)) => return Err(e),
            Err(_) => anyhow::bail!("wasapi loopback init timed out"),
        };
        Ok(WasapiLoopback {
            chunks: rx,
            channels,
            stop,
            worker: Some(worker),
            ring_samples,
            ring_capacity,
            underruns,
            overflow_dropped,
            engine_latency_ms: ready.engine_latency_ms,
        })
    }
}

impl Drop for WasapiLoopback {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

/// Decrement the shared standing-occupancy counter by the drained chunk's frames.
fn retire_samples(ring_samples: &Arc<AtomicUsize>, chunk_len: usize, channels: u32) {
    let per_ch = chunk_len / channels.max(1) as usize;
    let _ = ring_samples.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |cur| {
        Some(cur.saturating_sub(per_ch))
    });
}

impl AudioCapturer for WasapiLoopback {
    fn next_chunk(&mut self) -> Result<Vec<f32>> {
        match self.chunks.recv_timeout(Duration::from_secs(5)) {
            Ok(c) => {
                retire_samples(&self.ring_samples, c.len(), self.channels);
                Ok(c)
            }
            // A quiet engine (silence produces no packets) is NOT a failure — return an
            // empty chunk so the caller keeps the capturer alive. Only a dead worker
            // thread is an Err (→ caller reopens).
            Err(RecvTimeoutError::Timeout) => {
                self.underruns.fetch_add(1, Ordering::Relaxed);
                Ok(Vec::new())
            }
            Err(RecvTimeoutError::Disconnected) => Err(anyhow::anyhow!("wasapi thread ended")),
        }
    }

    fn channels(&self) -> u32 {
        self.channels
    }

    fn drain(&mut self) {
        while let Ok(c) = self.chunks.try_recv() {
            retire_samples(&self.ring_samples, c.len(), self.channels);
        }
    }

    fn telemetry(&self) -> AudioTelemetry {
        let samples = self.ring_samples.load(Ordering::Relaxed);
        AudioTelemetry {
            quantum_ms: self.engine_latency_ms,
            ring_samples: samples,
            ring_capacity: self.ring_capacity.load(Ordering::Relaxed),
            underruns: self.underruns.load(Ordering::Relaxed),
            overflow_dropped: self.overflow_dropped.load(Ordering::Relaxed),
            playout_age_ms: if samples > 0 {
                samples as f64 / SAMPLE_RATE as f64 * 1000.0
            } else {
                0.0
            },
        }
    }
}

/// The WASAPI worker: COM init, default-device loopback, packet → resample/remap → chunk
/// channel. Any engine failure is terminal and reported through `ready_tx` (bring-up) or
/// by ending the chunk channel (mid-session).
#[allow(clippy::too_many_arguments)]
fn wasapi_thread(
    want_channels: u32,
    tx: SyncSender<Vec<f32>>,
    ready_tx: SyncSender<Result<Ready>>,
    stop: Arc<AtomicBool>,
    ring_samples: Arc<AtomicUsize>,
    ring_capacity: Arc<AtomicUsize>,
    overflow_dropped: Arc<AtomicU64>,
) -> Result<()> {
    // SAFETY: MTA init for this thread only; every COM object below is created and used
    // here and never leaves the thread.
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)
            .ok()
            .context("wasapi: CoInitializeEx")?;
    }
    // SAFETY: all four objects are created here and reference-counted; the mix facts are
    // copied out before use. Runs on the COM-initialized worker.
    let (_device, client, capture, mix) = unsafe { open_default_loopback() }?;
    if !conversion_supported(mix.channels as u32, want_channels) {
        let e = anyhow::anyhow!(
            "wasapi: mix has {} channels, session wants {want_channels} (unsupported conversion)",
            mix.channels
        );
        let _ = ready_tx.send(Err(anyhow::anyhow!("{e:#}")));
        return Err(e);
    }
    // SAFETY: `client` is live; latency is a plain value read synchronously.
    let engine_latency_ms = unsafe { client.GetStreamLatency().unwrap_or(0) } as f64 / 10_000.0;
    let _ = ready_tx.send(Ok(Ready {
        channels: mix.channels,
        rate: mix.rate,
        engine_latency_ms,
    }));
    ring_capacity.store(480 * 4, Ordering::Relaxed);
    let mut resampler = LinearResampler::new(mix.rate, SAMPLE_RATE);
    // SAFETY: starting a live stream on objects created above, on this thread.
    unsafe {
        client.Start().context("wasapi: Start")?;
    }
    let result = packet_loop(
        &capture,
        &mix,
        want_channels,
        &mut resampler,
        &tx,
        &stop,
        &ring_samples,
        &overflow_dropped,
    );
    // SAFETY: balances `Start` on the same live client.
    unsafe {
        let _ = client.Stop();
    }
    result
}

/// Supported (mix → session) channel conversions. Identical counts pass through;
/// surround→stereo downmixes; narrower mixes zero-fill (the Linux zero-upmix rule);
/// mono duplicates to stereo. Anything else fails at open instead of misrouting.
fn conversion_supported(mix_channels: u32, want_channels: u32) -> bool {
    matches!(
        (mix_channels, want_channels),
        (1, 1) | (2, 2) | (6, 6) | (8, 8) | (1, 2) | (6, 2) | (8, 2) | (2, 6) | (2, 8)
    )
}

/// Mix-format facts read off the engine's `WAVEFORMATEX`.
struct MixFormat {
    channels: u16,
    rate: u32,
}

/// Open the default render endpoint's loopback stream and read its mix format.
///
/// Returns the device (kept alive for the stream's lifetime), client, capture client,
/// and mix facts. All four are created here and reference-counted.
unsafe fn open_default_loopback(
) -> Result<(IMMDevice, IAudioClient, IAudioCaptureClient, MixFormat)> {
    // SAFETY: every object is created here and reference-counted; the mix block is
    // copied out and freed before return. All calls run on the COM-initialized worker.
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .context("wasapi: device enumerator")?;
        let device = enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .context("wasapi: default render endpoint")?;
        let client: IAudioClient = device
            .Activate(CLSCTX_ALL, None)
            .context("wasapi: IAudioClient")?;
        let pformat = client.GetMixFormat().context("wasapi: mix format")?;
        // SAFETY: `GetMixFormat` hands us a CoTaskMem block we must free; the facts are
        // copied out first, the block is freed before `Initialize` runs, and the client
        // holds its own copy of the format from `Initialize` on.
        let mix = read_mix_format(&*pformat).context("wasapi: mix format unsupported")?;
        client
            .Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK,
                BUFFER_HNS,
                0,
                pformat,
                None,
            )
            .context("wasapi: Initialize loopback")?;
        CoTaskMemFree(Some(pformat as *const core::ffi::c_void));
        let capture: IAudioCaptureClient =
            client.GetService().context("wasapi: IAudioCaptureClient")?;
        Ok((device, client, capture, mix))
    }
}

/// Validate the mix format (32-bit float) and extract channels + rate.
///
/// `pformat` borrows the engine's `WAVEFORMATEX` for the call — the caller frees the block.
unsafe fn read_mix_format(pformat: *const WAVEFORMATEX) -> Result<MixFormat> {
    // SAFETY: `pformat` points at the live `GetMixFormat` block for this call. The
    // projection declares the struct byte-packed, so it is copied whole (one aligned
    // read — never a field reference into the packed layout) and every read below hits
    // the aligned local; the subformat GUID likewise goes through `read_unaligned`.
    // Nothing is written or retained.
    unsafe {
        // The projection declares the struct byte-packed: fields are copied out by
        // value first (allowed) and only the locals are referenced below — notably by
        // the `bail!` formatters, which borrow their arguments.
        let f: WAVEFORMATEX = *pformat;
        let format_tag = f.wFormatTag;
        let bits_per_sample = f.wBitsPerSample;
        let n_channels = f.nChannels;
        let n_samples_per_sec = f.nSamplesPerSec;
        let cb_size = f.cbSize;
        let is_float = format_tag as u32 == WAVE_FORMAT_IEEE_FLOAT && bits_per_sample == 32;
        let extensible_extra = std::mem::size_of::<WAVEFORMATEXTENSIBLE>()
            .saturating_sub(std::mem::size_of::<WAVEFORMATEX>());
        let subformat = std::ptr::addr_of!((*(pformat as *const WAVEFORMATEXTENSIBLE)).SubFormat)
            .read_unaligned();
        let is_ext_float = format_tag as u32 == WAVE_FORMAT_EXTENSIBLE
            && cb_size as usize >= extensible_extra
            && subformat == SUBTYPE_IEEE_FLOAT;
        if !(is_float || is_ext_float) {
            anyhow::bail!(
                "mix format is tag {} {}-bit (need 32-bit float; integer mixes unsupported)",
                format_tag,
                bits_per_sample
            );
        }
        if !matches!(n_channels, 1 | 2 | 6 | 8) {
            anyhow::bail!("mix has {} channels (want 1, 2, 6 or 8)", n_channels);
        }
        Ok(MixFormat {
            channels: n_channels,
            rate: n_samples_per_sec,
        })
    }
}

/// Stateful linear resampler for interleaved `f32`: any engine rate → 48 kHz.
struct LinearResampler {
    /// Input frames per output frame.
    ratio: f64,
    /// Fractional position in the endless input stream.
    pos: f64,
}

impl LinearResampler {
    fn new(in_rate: u32, out_rate: u32) -> Self {
        LinearResampler {
            ratio: in_rate.max(1) as f64 / out_rate.max(1) as f64,
            pos: 0.0,
        }
    }

    /// Resample `input` (interleaved, `channels` per frame), appending output frames to
    /// `out`. Keeps fractional state across calls so chunks join seamlessly.
    fn feed(&mut self, input: &[f32], channels: usize, out: &mut Vec<f32>) {
        if input.is_empty() || channels == 0 {
            return;
        }
        let in_frames = input.len() / channels;
        if (self.ratio - 1.0).abs() < f64::EPSILON {
            out.extend_from_slice(&input[..in_frames * channels]);
            return;
        }
        while self.pos < in_frames as f64 {
            let i0 = self.pos.floor() as usize;
            let frac = (self.pos - i0 as f64) as f32;
            let i1 = (i0 + 1).min(in_frames - 1);
            for c in 0..channels {
                let a = input[i0 * channels + c];
                let b = input[i1 * channels + c];
                out.push(a + (b - a) * frac);
            }
            self.pos += self.ratio;
        }
        self.pos -= in_frames as f64;
    }
}

/// Remap one engine packet (`in_ch` interleaved frames) into session order (`want_ch`).
/// Same counts pass through; surround→stereo downmixes (ITU); narrower mixes zero-fill
/// (the Linux zero-upmix rule); mono duplicates to stereo.
fn remap_channels(packet: &[f32], in_ch: usize, want_ch: u32, out: &mut Vec<f32>) {
    let want = want_ch as usize;
    if in_ch == want {
        out.extend_from_slice(packet);
        return;
    }
    let frames = packet.len() / in_ch.max(1);
    match (in_ch, want) {
        // Mono → stereo: duplicate.
        (1, 2) => {
            for sample in packet.iter().take(frames) {
                out.push(*sample);
                out.push(*sample);
            }
        }
        // Surround → stereo: FL + 0.707(C + SL) / FR + 0.707(C + SR), LFE dropped
        // (the standard Lo/Ro fold — keeps dialogue and effects balanced).
        (6, 2) | (8, 2) => {
            for f in 0..frames {
                let base = f * in_ch;
                let (fl, fr, fc, _lfe, rl, rr) = (
                    packet[base],
                    packet[base + 1],
                    packet[base + 2],
                    packet[base + 3],
                    packet[base + 4],
                    packet[base + 5],
                );
                out.push(fl + std::f32::consts::FRAC_1_SQRT_2 * (fc + rl));
                out.push(fr + std::f32::consts::FRAC_1_SQRT_2 * (fc + rr));
            }
        }
        // Narrower → wider: zero-fill the missing positions.
        _ => {
            for f in 0..frames {
                for c in 0..want {
                    out.push(if c < in_ch {
                        packet[f * in_ch + c]
                    } else {
                        0.0
                    });
                }
            }
        }
    }
}

/// The packet loop: acquire → remap/resample → chunk channel. `DEVICE_INVALIDATED`
/// (unplug/HDMI modeset moved the default) is terminal — the caller reopens.
#[allow(clippy::too_many_arguments)]
fn packet_loop(
    capture: &IAudioCaptureClient,
    mix: &MixFormat,
    want_channels: u32,
    resampler: &mut LinearResampler,
    tx: &SyncSender<Vec<f32>>,
    stop: &Arc<AtomicBool>,
    ring_samples: &Arc<AtomicUsize>,
    overflow_dropped: &Arc<AtomicU64>,
) -> Result<()> {
    let in_ch = mix.channels as usize;
    // ~10 ms output chunks at 48 kHz (what the Opus reframer consumes).
    const CHUNK_FRAMES: usize = 480;
    let chunk_samples = CHUNK_FRAMES * want_channels as usize;
    let mut pending: Vec<f32> = Vec::with_capacity(chunk_samples * 2);
    // Move whole chunks into the bounded channel; on a full channel the oldest data is
    // already gone (audio is lossy/real-time — a stale chunk is worse than a dropped one).
    let send_pending = |pending: &mut Vec<f32>| {
        while pending.len() >= chunk_samples {
            let chunk: Vec<f32> = pending.drain(..chunk_samples).collect();
            ring_samples.fetch_add(CHUNK_FRAMES, Ordering::Relaxed);
            if tx.try_send(chunk).is_err() {
                overflow_dropped.fetch_add(1, Ordering::Relaxed);
                let _ = ring_samples.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |cur| {
                    Some(cur.saturating_sub(CHUNK_FRAMES))
                });
            }
        }
    };
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        // SAFETY: packet-size query on the live capture client; no pointers.
        let packets = unsafe { capture.GetNextPacketSize() }.context("wasapi: packets")?;
        if packets == 0 {
            std::thread::sleep(Duration::from_millis(5));
            continue;
        }
        // SAFETY: `data`/`frames`/`flags` are live locals filled synchronously; the
        // buffer is valid until the matching `ReleaseBuffer` below, on this thread.
        let acquisition = unsafe {
            let mut data: *mut u8 = std::ptr::null_mut();
            let mut frames = 0u32;
            let mut flags = 0u32;
            match capture.GetBuffer(&mut data, &mut frames, &mut flags, None, None) {
                Ok(()) => Ok((data, frames, flags)),
                Err(e) => Err(e),
            }
        };
        let (data, frames, flags) = match acquisition {
            Ok(v) => v,
            Err(e) if e.code() == AUDCLNT_E_DEVICE_INVALIDATED => {
                anyhow::bail!("wasapi: default device invalidated — reopen to recover");
            }
            Err(e) => anyhow::bail!("wasapi: GetBuffer: {e}"),
        };
        if frames > 0 {
            // The engine mix is float (validated at open); a silent buffer carries no
            // valid samples — and zeros map to zeros under every conversion, so the
            // silent path skips the remap entirely.
            let silent = flags as i32 & AUDCLNT_BUFFERFLAGS_SILENT.0 != 0;
            let mut mapped = Vec::with_capacity(frames as usize * want_channels as usize);
            if silent {
                mapped.resize(frames as usize * want_channels as usize, 0.0);
            } else {
                // SAFETY: `data` points at `frames * in_ch` live `f32`s until
                // `ReleaseBuffer` below (float mix validated at open).
                unsafe {
                    let samples =
                        std::slice::from_raw_parts(data as *const f32, frames as usize * in_ch);
                    remap_channels(samples, in_ch, want_channels, &mut mapped);
                }
            }
            let mut resampled: Vec<f32> =
                Vec::with_capacity(frames as usize * want_channels as usize + 64);
            resampler.feed(&mapped, want_channels as usize, &mut resampled);
            pending.extend_from_slice(&resampled);
            send_pending(&mut pending);
        }
        // SAFETY: balances the `GetBuffer` above on the same live client.
        unsafe {
            capture
                .ReleaseBuffer(frames)
                .context("wasapi: ReleaseBuffer")?;
        }
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn resampler_passes_through_at_unity() {
        let mut r = LinearResampler::new(48000, 48000);
        let mut out = Vec::new();
        r.feed(&[1.0, 2.0, 3.0, 4.0], 2, &mut out);
        assert_eq!(out, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn resampler_halves_96k_to_48k() {
        let mut r = LinearResampler::new(96000, 48000);
        let mut out = Vec::new();
        // 4 input frames of a ramp; expect ~2 output frames interpolating midpoints.
        r.feed(&[0.0, 2.0, 4.0, 6.0], 1, &mut out);
        assert_eq!(out.len(), 2);
        assert!((out[0] - 0.0).abs() < 1e-6);
        assert!((out[1] - 4.0).abs() < 1e-6);
    }

    #[test]
    fn resampler_keeps_state_across_chunks() {
        let mut r = LinearResampler::new(44100, 48000);
        let mut out = Vec::new();
        r.feed(&[0.5; 44], 1, &mut out);
        let n1 = out.len();
        r.feed(&[0.5; 44], 1, &mut out);
        // 88 in-frames at 44.1k ≈ 96 out-frames total; split across two feeds.
        assert!((out.len() as i32 - 96).abs() <= 2, "got {}", out.len());
        assert!(n1 > 0 && n1 < out.len());
    }

    #[test]
    fn downmix_folds_center_and_sides() {
        // FL FR FC LFE RL RR → stereo Lo/Ro.
        let packet = [1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        let mut out = Vec::new();
        remap_channels(&packet, 6, 2, &mut out);
        assert_eq!(out.len(), 2);
        assert!((out[0] - (1.0 + std::f32::consts::FRAC_1_SQRT_2 * 2.0)).abs() < 1e-4);
        assert!((out[1] - (0.0 + std::f32::consts::FRAC_1_SQRT_2 * 1.0)).abs() < 1e-4);
    }

    #[test]
    fn narrow_mixes_zero_fill_and_mono_duplicates() {
        let mut out = Vec::new();
        remap_channels(&[0.5, 0.25], 2, 6, &mut out);
        assert_eq!(out, vec![0.5, 0.25, 0.0, 0.0, 0.0, 0.0]);
        out.clear();
        remap_channels(&[0.5], 1, 2, &mut out);
        assert_eq!(out, vec![0.5, 0.5]);
    }

    #[test]
    fn conversions_cover_the_negotiated_matrix() {
        for (mix, want) in [
            (1, 1),
            (2, 2),
            (6, 6),
            (8, 8),
            (1, 2),
            (6, 2),
            (8, 2),
            (2, 6),
            (2, 8),
        ] {
            assert!(conversion_supported(mix, want), "{mix}->{want}");
        }
        assert!(!conversion_supported(1, 6));
        assert!(!conversion_supported(8, 1));
    }
}
