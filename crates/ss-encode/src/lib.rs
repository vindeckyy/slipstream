//! Linux hardware and software video encode. Binds FFmpeg; never rewrites codecs. Low-latency
//! preset, B-frames off. The backend is per-GPU: NVENC on NVIDIA (`*_nvenc`, accepts `bgr0` and
//! does RGB→YUV on the GPU, so no host-side CSC) and VAAPI on AMD/Intel (`*_vaapi`; the CPU-input
//! fallback swscales RGB→NV12, the zero-copy path imports the capture dmabuf straight into a VA
//! surface). Vulkan Video and PyroWave provide Linux-specific zero-copy paths. One [`Encoder`]
//! trait, selected in [`open_video`]. The crate depends on the shared frame vocabulary (`ss-frame`)
//! and zero-copy plumbing (`ss-zerocopy`), never on capture.
#![cfg(target_os = "linux")]
// NOTE: no crate-wide `#![allow(dead_code)]`. It was inherited from the pre-extraction host crate
// root as scaffolding for backend paths defined ahead of the build that used them, but a census
// across every feature combination found it was hiding exactly two items, so it
// bought nothing and blinded the crate to future rot. Genuinely test-only helpers carry
// `#[cfg(test)]` instead.
// Every unsafe block in this module tree carries a `// SAFETY:` proof; enforce it (unsafe-proof
// program). As a parent module this also covers the child Linux backends.
#![deny(clippy::undocumented_unsafe_blocks)]

use anyhow::Result;
use ss_frame::{CapturedFrame, PixelFormat};

#[path = "backend/codec.rs"]
mod codec;
pub use codec::*;

/// The unified latency policy applied by every encoder backend (Phase 4).
pub mod profile;
pub use profile::{LatencyProfile, ProfileConfig};

impl Codec {
    /// The `quic` codec bitfield the host can currently **emit** on the slipstream/1 native path,
    /// given the resolved encode backend — the same GPU-aware advertisement GameStream builds for
    /// Moonlight (the host `gamestream::serverinfo`), in `quic::CODEC_*` bits. The GPU-less software
    /// encoder (openh264) produces H.264 only; the probed Linux backends advertise exactly what the
    /// GPU encodes ([`vaapi_codec_support`] / [`nvenc_codec_support`] where available; AV1 encode is
    /// narrow, an old iGPU might lack HEVC); NVENC keeps the Moonlight-validated
    /// static superset. An empty probe means the GPU wasn't usable at probe time (GPU-less CI,
    /// wrong-vendor pref), not that it encodes nothing — fall back to the superset so `resolve_codec`
    /// still lands on HEVC for an auto client, exactly the pre-probe behaviour. Fed to
    /// [`slipstream_core::quic::resolve_codec`] against the client's advertised codecs.
    pub fn host_wire_caps() -> u8 {
        // PyroWave rides on top of whatever H.26x set resolves below: feature-gated and inert in
        // negotiation unless the client explicitly prefers it (resolve_codec ignores the bit in its ladder). Advertised
        // whenever the backend could open: AMD/Intel capture hands raw dmabufs it imports
        // directly, and an NVIDIA-auto host's PyroWave sessions flip capture to CPU RGB
        // per-session instead (the host `session_plan::SessionPlan::output_format`) — the EGL→CUDA
        // frames the `auto` GPU path would deliver are NVENC-only. Only a software/GPU-less pref
        // keeps the bit off (no Vulkan device to open).
        // Resolved ONCE for every gate below (pyro bit, software pin, vulkan ceiling, the
        // zero-copy plane): this fn backs the codec advertisement on polled paths, and the
        // resolver's auto arm samples live GPU-preference state — four samples per call was the
        // probe-amplification the WP7.6 diff critic caught.
        #[cfg(target_os = "linux")]
        let backend = linux_resolved_backend();
        #[cfg(all(target_os = "linux", feature = "pyrowave"))]
        let pyro = if backend != LinuxBackend::Software {
            slipstream_core::quic::CODEC_PYROWAVE
        } else {
            0u8
        };
        #[cfg(not(feature = "pyrowave"))]
        let pyro = 0u8;
        let base = (|| {
            /// The static GPU superset (H.264 | HEVC | AV1) — mirrors the GameStream
            /// `SERVER_CODEC_MODE_SUPPORT` advertisement for the unprobed backends.
            const GPU_SUPERSET: u8 = slipstream_core::quic::CODEC_H264
                | slipstream_core::quic::CODEC_HEVC
                | slipstream_core::quic::CODEC_AV1;
            {
                if backend == LinuxBackend::Software {
                    return slipstream_core::quic::CODEC_H264;
                }
                // A pref that FORCES the raw Vulkan Video backend can only ever serve what that
                // backend encodes: `open_video_backend`'s `vulkan` arm bails outright for anything
                // that is not HEVC/AV1 ("the Vulkan Video encoder supports HEVC + AV1; the session
                // negotiated {codec:?}"). Advertising H.264 there let a client negotiate it and die
                // at encoder open. Without the `vulkan-encode` feature that arm cannot open at all,
                // so the pref advertises nothing.
                //
                // This is a CEILING intersected with the device probe below, never a replacement
                // for it: pinning a static HEVC|AV1 would ADD AV1 on the AMD/Intel hosts whose probe
                // currently withholds it (pre-RDNA3, pre-Arc), i.e. it would re-create this very bug
                // for a different codec. Intersecting can only narrow.
                let pref_ceiling: u8 = match backend {
                    // Feature-blindness rider (WP7.6 critics): the resolver knows the PREF is
                    // vulkan; only this cfg! knows the BUILD can open it. Dropping the inner
                    // branch would advertise HEVC|AV1 on a featureless build — the
                    // advertise-then-die-at-open bug this ceiling exists to prevent.
                    LinuxBackend::Vulkan => {
                        if cfg!(feature = "vulkan-encode") {
                            slipstream_core::quic::CODEC_HEVC | slipstream_core::quic::CODEC_AV1
                        } else {
                            0
                        }
                    }
                    _ => GPU_SUPERSET,
                };
                if linux_zero_copy_is_vaapi_for(backend) {
                    if let Some(m) = vaapi_codec_support().wire_mask() {
                        return m & pref_ceiling;
                    }
                }
                // NVENC: ask the DRIVER which codecs this chip's encoder exposes, the same way the
                // VAAPI arm above does — a static superset advertised HEVC on a 1st-gen Maxwell
                // (HEVC needs 2nd-gen Maxwell+, AV1 needs Ada+), and a client that believed it got
                // ~15 s of blank video and a disconnect instead of a stream. Fails OPEN: a probe
                // that can't answer (no direct-SDK build, no CUDA, an old driver) yields `None` and
                // leaves the historical superset standing, so this can only ever narrow the
                // advertisement to something the GPU really encodes.
                #[cfg(feature = "nvenc")]
                if backend == LinuxBackend::Nvenc {
                    if let Some(m) = nvenc_codec_support().wire_mask() {
                        return m & pref_ceiling;
                    }
                }
                // NVENC without the probe (no `nvenc` feature / probe declined) — or an empty VAAPI
                // probe (see above): the static superset, like GameStream.
                GPU_SUPERSET & pref_ceiling
            }
        })();
        base | pyro
    }
}

/// Open a video encoder for frames of the given `format` and mode, selecting the Linux GPU
/// backend for this host: **NVENC** on NVIDIA or **VAAPI** on AMD/Intel.
/// When `cuda` is true the encoder takes GPU frames (`AV_PIX_FMT_CUDA`) from the NVIDIA zero-copy
/// path; otherwise it takes packed RGB/BGR CPU frames (and, on VAAPI, a future dmabuf payload).
/// `format`/`bitrate_bps`/`codec`/mode come from session negotiation; the caller derives `cuda`
/// from the first captured frame's payload. The Linux backend is auto-detected (override:
/// `SLIPSTREAM_ENCODER=auto|nvenc|vaapi`).
#[allow(clippy::too_many_arguments)]
pub fn open_video(
    codec: Codec,
    format: PixelFormat,
    width: u32,
    height: u32,
    fps: u32,
    bitrate_bps: u64,
    cuda: bool,
    bit_depth: u8,
    chroma: ChromaFormat,
    // The session may hand this encoder cursor bitmaps to composite (cursor-as-metadata
    // captures). Backends whose fast path can't blend (Vulkan EFC RGB-direct) key off it.
    cursor_blend: bool,
    // Ceiling on the per-frame slice count this session's CLIENT decoder accepts: 1 =
    // single-slice only (the safe default toward decoders that never asked — Amlogic TV SoCs
    // wedge on multi-slice AUs), a Moonlight `videoEncoderSlicesPerFrame` request, or 32 (= no
    // client-side limit) for a `VIDEO_CAP_MULTI_SLICE` slipstream/1 client. Backends that split
    // frames (Linux direct-NVENC, §7 LN1) clamp their default against it;
    // `SLIPSTREAM_NVENC_SLICES` stays the explicit operator override in both directions.
    max_slices: u32,
) -> Result<Box<dyn Encoder>> {
    let (inner, backend) = open_video_backend(
        codec,
        format,
        width,
        height,
        fps,
        bitrate_bps,
        cuda,
        bit_depth,
        chroma,
        cursor_blend,
        max_slices,
    )?;
    // Record what this session encodes on (the mgmt API's "currently used GPU"): the backend label
    // is reported by `open_video_backend` from the branch that ACTUALLY opened — not re-derived by
    // mirroring its dispatch, which went stale the moment a backend gained an internal fallback
    // (the default-on Vulkan Video path falls back to VAAPI on a failed open, and a dispatch
    // mirror would report "vaapi" for every Vulkan session or vice versa). The GPU identity is the
    // same selection the capturer was created on ([`ss_gpu::selected_gpu`]). Dropping the
    // returned encoder ends the record, so the live count is correct by construction.
    let gpu = if backend == "software" {
        ss_gpu::ActiveGpu {
            id: String::new(),
            name: "CPU (openh264)".into(),
            vendor_id: 0,
            backend,
        }
    } else {
        match ss_gpu::selected_gpu() {
            Some(sel) => ss_gpu::ActiveGpu {
                id: sel.info.id,
                name: sel.info.name,
                vendor_id: sel.info.vendor_id,
                backend,
            },
            None => ss_gpu::ActiveGpu {
                id: String::new(),
                name: "GPU".into(),
                vendor_id: 0,
                backend,
            },
        }
    };
    // The session asked for a composited pointer; say so loudly if the backend that actually opened
    // cannot deliver one. Since the negotiation became caps-aware ([`cursor_blend_capable`] gates
    // the cursor channel, and the session plan keeps cursor sessions off the native-NV12/RGB-direct
    // shapes), no PLANNED path reaches this: capture negotiates embedded-cursor mode wherever the
    // resolved backend can't blend. What remains reachable is the open-time divergence the plan
    // cannot see — a Vulkan Video open failing back to VAAPI mid-`open_amd_intel`, and the
    // gamescope residual (gamescope has no embedded mode, so a never-blending backend there —
    // H.264→VAAPI, software — still streams cursorless). This is the backstop that keeps those
    // honest in the logs; `open_video` cannot re-plan capture, so a warning is deliberately all
    // it does.
    if cursor_blend && !inner.caps().blends_cursor {
        tracing::warn!(
            backend,
            "session negotiated a composited cursor but this encode backend does not blend \
             CapturedFrame::cursor — the pointer will be MISSING from the stream unless the \
             capturer composites it"
        );
    }
    Ok(Box::new(TrackedEncoder {
        inner,
        _session: ss_gpu::session_begin(gpu),
    }))
}

/// Ties the `ss_gpu` live-session record to the encoder's lifetime; pure delegation
/// otherwise.
struct TrackedEncoder {
    inner: Box<dyn Encoder>,
    _session: ss_gpu::ActiveSession,
}

impl Encoder for TrackedEncoder {
    fn submit(&mut self, frame: &CapturedFrame) -> Result<()> {
        self.inner.submit(frame)
    }
    fn submit_indexed(&mut self, frame: &CapturedFrame, wire_index: u32) -> Result<()> {
        self.inner.submit_indexed(frame, wire_index)
    }
    fn caps(&self) -> EncoderCaps {
        self.inner.caps()
    }
    fn request_keyframe(&mut self) {
        self.inner.request_keyframe()
    }
    fn set_hdr_meta(&mut self, meta: Option<slipstream_core::quic::HdrMeta>) {
        self.inner.set_hdr_meta(meta)
    }
    fn invalidate_ref_frames(&mut self, first_frame: i64, last_frame: i64) -> bool {
        self.inner.invalidate_ref_frames(first_frame, last_frame)
    }
    // Forwarded for the same reason as `set_wire_chunking` below — the unforwarded default
    // (`false` = "backend can't pipeline, stop asking") silently killed the §7 LN3 contention
    // escalation for every session, since the host loop only ever holds the wrapped box.
    fn set_pipelined(&mut self, on: bool) -> bool {
        self.inner.set_pipelined(on)
    }
    // The classic TrackedEncoder trap: a defaulted trait method that isn't forwarded
    // silently no-ops through the wrapper (bit the direct-NVENC work, then THIS — the
    // §4.4 chunking probe run hit the default while the plan said Some(1408)).
    fn set_wire_chunking(&mut self, shard_payload: usize) {
        self.inner.set_wire_chunking(shard_payload)
    }
    // Forwarded for the same reason as `set_wire_chunking` above — an unforwarded default here
    // would silently leave the in-place backends pipelining past the capturer's ring.
    fn set_input_ring_depth(&mut self, depth: usize) {
        // Phase 4: `LowLatency` caps the encoder's input ring at one — no hidden frame queue
        // behind the encoder to absorb a slow backend (a slow backend must instead be handled
        // by ABR, never by deepening the pipeline).
        self.inner
            .set_input_ring_depth(depth.min(LatencyProfile::current().config().max_input_depth))
    }
    fn poll(&mut self) -> Result<Option<EncodedFrame>> {
        self.inner.poll()
    }
    // Both chunked-poll methods forwarded (the same trap class): the defaults would report
    // "not chunked" and wrap whole AUs, silently discarding the sub-frame overlap.
    fn supports_chunked_poll(&self) -> bool {
        self.inner.supports_chunked_poll()
    }
    fn poll_chunk(&mut self) -> Result<Option<AuChunk>> {
        self.inner.poll_chunk()
    }
    fn reset(&mut self) -> bool {
        self.inner.reset()
    }
    fn reconfigure_bitrate(&mut self, bps: u64) -> bool {
        self.inner.reconfigure_bitrate(bps)
    }
    // Forwarded (the same trap class as `set_wire_chunking`): the unforwarded default `None`
    // would tell the session loop "no encoder truth here" and it would keep pacing/acking the
    // requested rate even where NVENC clamped to the codec-level ceiling.
    fn applied_bitrate_bps(&self) -> Option<u64> {
        self.inner.applied_bitrate_bps()
    }
    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}

/// Ceiling applied to the negotiated bitrate before it reaches openh264: software H.264 realistically
/// caps far below the rates a hardware session negotiates, and handing it the full figure just
/// misconfigures its rate control. Module-scope so BOTH software arms share one value — the Linux
/// arm was missing the clamp the hardware path applied.
const SW_BITRATE_CEIL: u64 = 100_000_000;

/// The Linux half of [`open_video_backend`], with the pref INJECTED — the seam exists so the
/// dispatch is testable without mutating process env (`set_var` races `getenv` in parallel
/// tests) or fighting the once-latched `ss_host_config::config()`. Production takes the
/// config pref via the caller; shared validation (dims/fps/chroma degrade) stays there too.
#[cfg(target_os = "linux")]
#[allow(clippy::too_many_arguments)]
fn open_video_backend_linux(
    pref: &str,
    codec: Codec,
    format: PixelFormat,
    width: u32,
    height: u32,
    fps: u32,
    bitrate_bps: u64,
    cuda: bool,
    bit_depth: u8,
    chroma: ChromaFormat,
    cursor_blend: bool,
    max_slices: u32,
) -> Result<(Box<dyn Encoder>, &'static str)> {
    // A NEGOTIATED PyroWave session (client advertised + preferred it, plan §3) routes
    // straight to that backend — the SLIPSTREAM_ENCODER pref below stays a lab override.
    if codec == Codec::PyroWave {
        #[cfg(feature = "pyrowave")]
        {
            return pyrowave::PyroWaveEncoder::open(width, height, fps, bitrate_bps, chroma)
                .map(|e| (Box::new(e) as Box<dyn Encoder>, "pyrowave"));
        }
        #[cfg(not(feature = "pyrowave"))]
        anyhow::bail!(
            "session negotiated PyroWave but this host was built without --features \
             slipstream-host/pyrowave (the advertisement bit should not have been set)"
        );
    }
    // Pick the GPU encode backend. NVIDIA → NVENC/CUDA (the original path, unchanged);
    // AMD/Intel → VAAPI (one libavcodec backend for both). Auto-detect by default so a single
    // Linux binary serves any GPU; `SLIPSTREAM_ENCODER` forces a specific backend (and surfaces
    // its errors crisply instead of silently trying the other).
    // AMD/Intel opener. Default = libav VAAPI. With `--features vulkan-encode` +
    // SLIPSTREAM_VULKAN_ENCODE, an HEVC/AV1 session instead opens the raw Vulkan Video backend
    // (real RFI loss recovery the VAAPI path can't express); a failed open falls back to VAAPI so
    // the stream never dies over the new path. `format`/`bit_depth`/`chroma` only matter to VAAPI
    // — the Vulkan backend imports the dmabuf and does its own CSC.
    let open_amd_intel = || -> Result<(Box<dyn Encoder>, &'static str)> {
        // HDR (10-bit + a PQ/BT.2020 capture format) keeps this backend for EITHER codec, as
        // long as the device says it can: the compute CSC has a BT.2020 10-bit variant, the EFC
        // has a BT.2020 conversion, and the session opens Main10 / AV1-at-10. That keeps the two
        // things an HDR session would otherwise lose — real RFI recovery, and the compute CSC's
        // cursor blend, which is the only way a gamescope pointer reaches the stream (gamescope
        // has no embedded-cursor mode to fall back to).
        //
        // `vulkan_encode_available_at` is the device's own answer to the same profile query the
        // open will make, so this is a prediction that cannot disagree with reality — and a `no`
        // routes to libav VAAPI here rather than burning a failed session open first.
        #[cfg(feature = "vulkan-encode")]
        let ten_bit_session = bit_depth == 10 && format.is_hdr_rgb10();
        #[cfg(feature = "vulkan-encode")]
        if matches!(codec, Codec::H265 | Codec::Av1)
            && vulkan_encode_enabled()
            && vulkan_encode_available_at(codec, ten_bit_session)
        {
            match vulkan_video::VulkanVideoEncoder::open(
                codec,
                format,
                width,
                height,
                fps,
                bitrate_bps,
                cursor_blend,
            ) {
                Ok(e) => {
                    tracing::info!(
                        codec = ?codec,
                        "Linux Vulkan Video encode (real RFI via DPB reference slots) — \
                         set SLIPSTREAM_VULKAN_ENCODE=0 for libav VAAPI"
                    );
                    return Ok((Box::new(e) as Box<dyn Encoder>, "vulkan"));
                }
                // Native NV12 (SLIPSTREAM_PIPEWIRE_NV12 capture) has no VAAPI fallback:
                // libav's dmabuf lane would import the two-plane buffer as packed RGB
                // (silent garbage) and its CPU lane bails per frame — die crisply instead.
                Err(e) if format == PixelFormat::Nv12 => {
                    return Err(e.context(
                        "Vulkan Video open failed on a native-NV12 capture \
                         — no VAAPI fallback exists; set SLIPSTREAM_PIPEWIRE_NV12=0 to \
                         restore the packed-RGB negotiation",
                    ));
                }
                Err(e) => tracing::warn!(
                    error = %format!("{e:#}"),
                    "Vulkan Video encode open failed — falling back to libav VAAPI"
                ),
            }
        }
        // Same rule when the Vulkan backend was never eligible (H264 session,
        // SLIPSTREAM_VULKAN_ENCODE=0, or a build without the feature).
        if format == PixelFormat::Nv12 {
            anyhow::bail!(
                "native NV12 capture requires the Vulkan Video encoder (HEVC/AV1 \
                 session, --features vulkan-encode, SLIPSTREAM_VULKAN_ENCODE not 0) — this \
                 session resolved to libav VAAPI; set SLIPSTREAM_PIPEWIRE_NV12=0 to restore \
                 the packed-RGB negotiation"
            );
        }
        vaapi::VaapiEncoder::open(
            codec,
            format,
            width,
            height,
            fps,
            bitrate_bps,
            bit_depth,
            chroma,
        )
        .map(|e| (Box::new(e) as Box<dyn Encoder>, "vaapi"))
    };
    let open_nvidia = || -> Result<(Box<dyn Encoder>, &'static str)> {
        open_nvenc_probed(
            codec,
            format,
            width,
            height,
            fps,
            bitrate_bps,
            cuda,
            bit_depth,
            chroma,
            cursor_blend,
            max_slices,
        )
        .map(|e| (e, "nvenc"))
    };
    // The dispatch consumes the same resolver the capability mirrors consult, so the alias table
    // exists once. Arm bodies and their open-site labels stay in sync.
    match resolve_linux_backend(pref, linux_auto_is_vaapi, cuda) {
        Some(LinuxBackend::Nvenc) => open_nvidia(),
        Some(LinuxBackend::AmdIntel) => open_amd_intel(),
        // Force the raw Vulkan Video HEVC backend (real RFI). Needs `--features vulkan-encode`.
        Some(LinuxBackend::Vulkan) => {
            #[cfg(feature = "vulkan-encode")]
            {
                if !matches!(codec, Codec::H265 | Codec::Av1) {
                    anyhow::bail!(
                        "the Vulkan Video encoder supports HEVC + AV1; the session negotiated {codec:?}"
                    );
                }
                vulkan_video::VulkanVideoEncoder::open(
                    codec,
                    format,
                    width,
                    height,
                    fps,
                    bitrate_bps,
                    cursor_blend,
                )
                .map(|e| (Box::new(e) as Box<dyn Encoder>, "vulkan"))
            }
            #[cfg(not(feature = "vulkan-encode"))]
            {
                let _ = (format, bit_depth, chroma);
                anyhow::bail!(
                    "SLIPSTREAM_ENCODER=vulkan requires a build with --features vulkan-encode"
                )
            }
        }
        // PyroWave — the opt-in wired-LAN intra-only wavelet codec. Explicit-only, and
        // EXPERIMENTAL until CODEC_PYROWAVE negotiation lands (plan Phase 2): no shipping
        // client can decode the stream yet, so this arm exists for host-side bring-up and
        // latency work only. Vendor-agnostic (any Vulkan 1.3 GPU); ignores the negotiated
        // codec — every AU is an independently-decodable wavelet frame.
        Some(LinuxBackend::Pyrowave) => {
            #[cfg(feature = "pyrowave")]
            {
                tracing::warn!(
                    ?codec,
                    "SLIPSTREAM_ENCODER=pyrowave forces the all-intra wavelet stream \
                     regardless of the negotiated codec — only a pyrowave-feature client \
                     that ALSO preferred CODEC_PYROWAVE can display it (lab override; \
                     normal sessions negotiate it instead)"
                );
                // The lab override forces the wavelet stream onto a session negotiated for
                // another codec — that session's chroma may be HEVC-4:4:4, which the
                // pyrowave encoder doesn't do yet, so pin the override to 4:2:0.
                pyrowave::PyroWaveEncoder::open(
                    width,
                    height,
                    fps,
                    bitrate_bps,
                    ChromaFormat::Yuv420,
                )
                .map(|e| (Box::new(e) as Box<dyn Encoder>, "pyrowave"))
            }
            #[cfg(not(feature = "pyrowave"))]
            {
                anyhow::bail!(
                    "SLIPSTREAM_ENCODER=pyrowave requires a build with --features slipstream-host/pyrowave"
                )
            }
        }
        // GPU-less software H.264 (openh264) — for a headless / GPU-lost box. Explicit-only:
        // `auto` never picks it (a box with `/dev/nvidiactl` present but a dead driver would
        // otherwise wrongly resolve to NVENC). Needs H.264 (openh264 emits only that) and a CPU
        // RGB frame, which the capturer delivers because the software backend resolves `gpu=false`.
        Some(LinuxBackend::Software) => {
            if codec != Codec::H264 {
                anyhow::bail!(
                    "the software encoder emits H.264 only; the session negotiated {codec:?} \
                     (a client must advertise CODEC_H264 to reach a software host)"
                );
            }
            let _ = (cuda, bit_depth); // software path is CPU + 8-bit only
            sw::OpenH264Encoder::open(
                format,
                width,
                height,
                fps,
                bitrate_bps.min(SW_BITRATE_CEIL),
            )
            .map(|e| (Box::new(e) as Box<dyn Encoder>, "software"))
        }
        // The auto arm lives in the resolver now (a CUDA frame can only be consumed by
        // NVENC; else the shared manual-preference/NVIDIA-presence decision).
        None => anyhow::bail!(
            "unknown SLIPSTREAM_ENCODER={pref:?} — use auto (default), nvenc, vaapi, vulkan, pyrowave, or software"
        ),
    }
}

/// Open the Linux encoder backend. Returns the encoder together with the display label of the
/// branch that actually opened (`nvenc`/`vaapi`/`vulkan`/`software`) — the label feeds
/// the mgmt API's live-session record, and only the open site knows which internal fallback won
/// (e.g. Vulkan Video falling back to VAAPI).
#[allow(clippy::too_many_arguments)]
fn open_video_backend(
    codec: Codec,
    format: PixelFormat,
    width: u32,
    height: u32,
    fps: u32,
    bitrate_bps: u64,
    cuda: bool,
    bit_depth: u8,
    chroma: ChromaFormat,
    cursor_blend: bool,
    max_slices: u32,
) -> Result<(Box<dyn Encoder>, &'static str)> {
    validate_dimensions(codec, width, height)?;
    // Refresh/fps must be positive and sane: fps feeds the encoder time_base (`Rational(1, fps)`)
    // and the pts→ns conversion (`pts * 1e9 / fps`), so 0 builds a 1/0 rational / divides by zero.
    // The mid-stream Reconfigure path already guards `refresh_hz > 0`; enforcing it at this single
    // open chokepoint makes EVERY path (initial Hello, GameStream ANNOUNCE, Reconfigure) safe
    // regardless of which backend opens (security-review 2026-06-28 S5).
    if fps == 0 || fps > 1000 {
        anyhow::bail!("invalid refresh/fps {fps}: must be 1..=1000 Hz");
    }
    // 4:4:4 is HEVC- and PyroWave-only. The negotiator should never pass `Yuv444` for another
    // codec (it gates on the codec + `can_encode_444`), but defend the contract here so a future
    // caller can't silently emit a stream no decoder expects: an unsupported 4:4:4 request
    // degrades to 4:2:0 with a warning.
    let chroma = if chroma.is_444() && codec != Codec::H265 && codec != Codec::PyroWave {
        tracing::warn!(
            ?codec,
            "4:4:4 requested for a non-HEVC codec — encoding 4:2:0"
        );
        ChromaFormat::Yuv420
    } else {
        chroma
    };
    open_video_backend_linux(
        ss_host_config::config().encoder_pref.as_str(),
        codec,
        format,
        width,
        height,
        fps,
        bitrate_bps,
        cuda,
        bit_depth,
        chroma,
        cursor_blend,
        max_slices,
    )
}

/// Open NVENC, probing this GPU's real max bitrate. NVENC rejects `avcodec_open2` with EINVAL
/// when the bitrate exceeds what any codec level can express, and that ceiling is
/// GPU/driver-specific (an RTX 4090 caps HEVC at ~800 Mbps; an RTX 5070 Ti accepts >1 Gbps). So
/// open at the requested rate first and step down ONLY if this GPU refuses it — each GPU then
/// runs at its own actual maximum, and a capable card is never clamped to a conservative guess.
/// The codec's theoretical level ceiling is just the first step-down candidate, not a blind cap.
#[cfg(target_os = "linux")]
#[allow(clippy::too_many_arguments)]
fn open_nvenc_probed(
    codec: Codec,
    format: PixelFormat,
    width: u32,
    height: u32,
    fps: u32,
    bitrate_bps: u64,
    cuda: bool,
    bit_depth: u8,
    chroma: ChromaFormat,
    cursor_blend: bool,
    max_slices: u32,
) -> Result<Box<dyn Encoder>> {
    // Consumed by the direct-SDK arm below.
    #[cfg(not(feature = "nvenc"))]
    let _ = (cursor_blend, max_slices);
    // Direct-SDK NVENC (design/linux-direct-nvenc.md): the DEFAULT on NVIDIA, and only for a CUDA
    // capture payload (it registers CUDADEVICEPTR inputs — a CPU/dmabuf frame can't feed it, so those
    // keep the libav path; and `cuda` is false on AMD/Intel, so they stay on VAAPI). Set
    // SLIPSTREAM_NVENC_DIRECT=0 to fall back to libav. It self-clamps the bitrate internally (its own
    // level-ceiling binary search at session open), so it skips the probe-loop stepping below.
    #[cfg(feature = "nvenc")]
    if cuda && nvenc_direct_enabled() {
        tracing::info!(
            codec = codec.nvenc_name(),
            "Linux direct-SDK NVENC (real RFI + recovery anchor) — set SLIPSTREAM_NVENC_DIRECT=0 for libav"
        );
        return Ok(Box::new(nvenc_cuda::NvencCudaEncoder::open(
            codec,
            format,
            width,
            height,
            fps,
            bitrate_bps,
            cuda,
            bit_depth,
            chroma,
            cursor_blend,
            max_slices,
        )?) as Box<dyn Encoder>);
    }
    // The silent-degrade trap: a build without `--features nvenc` compiles the direct-SDK
    // path OUT, and a CUDA session quietly loses real RFI + the no-IDR bitrate reconfigure
    // with nothing in the logs. This bit the Linux packagers once (fixed e89b2f60) and an
    // ad-hoc host deploy again on 2026-07-14 — say it loudly instead. (Skipped when the
    // operator explicitly chose libav via SLIPSTREAM_NVENC_DIRECT=0.)
    #[cfg(not(feature = "nvenc"))]
    if cuda
        && !std::env::var("SLIPSTREAM_NVENC_DIRECT")
            .map(|v| matches!(v.trim(), "0" | "false" | "no" | "off"))
            .unwrap_or(false)
    {
        // Once per process — featureless builds rebuild the encoder on every bitrate step,
        // and one line is enough to diagnose the build.
        static WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            tracing::warn!(
                "direct-SDK NVENC is NOT compiled into this build (`--features slipstream-host/nvenc`) \
                 — CUDA frames take the libav path: no RFI loss recovery, and every adaptive-bitrate \
                 step costs an encoder rebuild + IDR"
            );
        }
    }
    const MIN_PROBE_BPS: u64 = 50_000_000;
    let mut candidates = vec![bitrate_bps];
    let cap = codec.max_bitrate_bps();
    if cap < bitrate_bps {
        candidates.push(cap);
    }
    let mut b = bitrate_bps.min(cap);
    while b > MIN_PROBE_BPS {
        b = b * 3 / 4;
        candidates.push(b);
    }
    let mut last: Option<anyhow::Error> = None;
    for (i, &b) in candidates.iter().enumerate() {
        match linux::NvencEncoder::open(
            codec, format, width, height, fps, b, cuda, bit_depth, chroma,
        ) {
            Ok(enc) => {
                if i > 0 {
                    tracing::warn!(
                        requested_mbps = bitrate_bps / 1_000_000,
                        opened_mbps = b / 1_000_000,
                        codec = codec.nvenc_name(),
                        "this GPU's NVENC refused the requested bitrate (EINVAL) — opened at the \
                         highest rate it accepts; request AV1 or a lower bitrate for more"
                    );
                }
                return Ok(Box::new(enc) as Box<dyn Encoder>);
            }
            // EINVAL = above this GPU's level ceiling → step down. Any other failure (no GPU,
            // bad mode, OOM) is real — surface it rather than masking it with bitrate retries.
            Err(e) if nvenc_open_einval(&e) => last = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(last.unwrap_or_else(|| anyhow::anyhow!("encoder open failed at every probed bitrate")))
}

/// Whether a libav NVENC open failed with EINVAL — the "bitrate above this GPU's level ceiling"
/// signal [`open_nvenc_probed`]'s ladder steps down on. Typed: the root `ffmpeg::Error` survives
/// the `anyhow` context chain, so match it there instead of substring-matching the English
/// strerror rendering of the whole chain — which also fired on any OTHER wrapped EINVAL (e.g. a
/// CUDA-context errno) and steered the ladder on failures that have nothing to do with bitrate.
#[cfg(target_os = "linux")]
fn nvenc_open_einval(e: &anyhow::Error) -> bool {
    use ffmpeg_next as ffmpeg;
    matches!(
        e.downcast_ref::<ffmpeg::Error>(),
        Some(ffmpeg::Error::Other {
            errno: ffmpeg::util::error::EINVAL
        })
    )
}

/// Whether the direct-SDK NVENC path is active. **Default ON** — on-glass validated 2026-07-12:
/// real RFI landed 73/73 as clean P-frame recovery anchors (never IDR) on an RTX host with a real
/// Steam Deck client (design/linux-direct-nvenc.md §9). `SLIPSTREAM_NVENC_DIRECT=0` (also `false`/
/// `no`/`off`) is the libav escape hatch. Only consulted for a CUDA capture payload on an NVIDIA
/// host — the `cuda` gate in `open_nvenc_probed` keeps AMD/Intel on VAAPI regardless — and only
/// with `--features nvenc`.
#[cfg(all(target_os = "linux", feature = "nvenc"))]
fn nvenc_direct_enabled() -> bool {
    std::env::var("SLIPSTREAM_NVENC_DIRECT")
        .map(|v| !matches!(v.trim(), "0" | "false" | "no" | "off"))
        .unwrap_or(true)
}

/// Whether the raw Vulkan Video HEVC encode backend is active for AMD/Intel. **Default ON** —
/// on-glass validated 2026-07-12 on an AMD RADV 780M with a real Deck-class client: the pipelined
/// encoder ran a rock-solid 1080p@240 HEVC session and healed loss with clean P-frame recovery
/// anchors (never IDR) via explicit DPB reference slots — real reference-frame invalidation the
/// libavcodec VAAPI path can't express (design/linux-vulkan-video-encode.md). `SLIPSTREAM_VULKAN_ENCODE=0`
/// (also `false`/`no`/`off`) is the libav-VAAPI escape hatch. Only consulted with
/// `--features vulkan-encode`, and a failed open falls back to VAAPI, so an unsupported device
/// (e.g. a Mesa without h265 encode, or an untested Intel/ANV where the path misbehaves at open)
/// degrades gracefully to the old backend rather than breaking the stream.
#[cfg(all(target_os = "linux", feature = "vulkan-encode"))]
fn vulkan_encode_enabled() -> bool {
    std::env::var("SLIPSTREAM_VULKAN_ENCODE")
        .map(|v| !matches!(v.trim(), "0" | "false" | "no" | "off"))
        .unwrap_or(true)
}

/// Whether THIS session's encoder can ingest a producer-native NV12 capture: only the raw
/// Vulkan Video backend does (libav VAAPI would misread the two-plane buffer as packed RGB —
/// [`open_video`] refuses the combination), so the session's codec must be one it encodes and
/// the backend must be eligible to open. The host facade threads the verdict into the capture
/// negotiation (`OutputFormat::nv12_native` → `ZeroCopyPolicy::native_nv12_session`), which
/// then PREFERS gamescope's producer-side NV12 pod (default-on; `SLIPSTREAM_PIPEWIRE_NV12=0`
/// escapes at the capture gate).
///
/// This verdict is load-bearing in a way the other capability helpers are not: once the producer
/// has been asked for two-plane NV12 there is **no fallback**. [`open_video`] deliberately makes a
/// failed Vulkan open FATAL for an NV12 capture rather than degrading to libav VAAPI, because VAAPI
/// would import that buffer as packed RGB and stream silent garbage. So a wrong `true` here does
/// not cost quality — it kills the session at its first frame. Hence the real device probe below,
/// and hence the conjuncts are ordered cheapest-first so it only runs when everything else already
/// said yes.
#[cfg(target_os = "linux")]
pub fn linux_native_nv12_ok(codec: Codec) -> bool {
    #[cfg(feature = "vulkan-encode")]
    {
        matches!(codec, Codec::H265 | Codec::Av1)
            && vulkan_encode_enabled()
            // Which backend this host actually resolves to. This used to be a denylist of the
            // EXPLICIT prefs that skip Vulkan Video ("nvenc"|"nvidia"|"cuda"|"pyrowave"), which
            // silently missed the one that matters: the DEFAULT `encoder_pref` is `""`, and `""`
            // resolves to `auto`, which on an NVIDIA box opens NVENC. So a stock NVIDIA host passed
            // this gate. `linux_zero_copy_is_vaapi` layers the pref on top of the same auto decision
            // `open_video` makes, which is exactly what the note on `linux_auto_is_vaapi` says a
            // capability probe must consult — and it is what the downstream consumer of this very
            // verdict already uses to pick its zero-copy path.
            && linux_zero_copy_is_vaapi()
            // …and only then ask the GPU. Ordered last on purpose: it opens a Vulkan instance.
            && vulkan_encode_available(codec)
    }
    #[cfg(not(feature = "vulkan-encode"))]
    {
        let _ = codec;
        false
    }
}

/// Can this host's encode path ingest a **packed 10-bit PQ/BT.2020 CUDA payload** — i.e. may an
/// HDR capture stay zero-copy on NVIDIA?
///
/// Only the direct-SDK NVENC backend can: it registers the buffer as an `ARGB10`/`ABGR10` input
/// surface and does the BT.2020 CSC in the encoder itself. The libav fallback cannot — its HDR
/// route builds a **P010** hardware frames context and swscales the RGB into it, so handing it a
/// packed-10-bit CUDA buffer would copy 2:10:10:10 words into a P010 surface and stream garbage.
/// So when the direct path is compiled out or vetoed (`SLIPSTREAM_NVENC_DIRECT=0`), the capturer
/// must NOT build the importer for an HDR session and the frames take the CPU path instead — the
/// same route HDR took before the direct path learned 10-bit.
///
/// Resolved by the host facade into [`ss_capture::ZeroCopyPolicy`], like every other
/// encode-backend fact capture is allowed to know (the one-way capture→encode edge).
#[cfg(target_os = "linux")]
pub fn linux_hdr_cuda_ok() -> bool {
    #[cfg(feature = "nvenc")]
    {
        // Same two terms `open_nvenc_probed` uses to take the direct arm — minus `cuda`, which is
        // the very thing the caller is deciding.
        nvenc_direct_enabled() && !linux_zero_copy_is_vaapi()
    }
    #[cfg(not(feature = "nvenc"))]
    {
        false
    }
}

/// Whether the encode backend this session will resolve to composites [`CapturedFrame::cursor`]
/// ([`EncoderCaps::blends_cursor`]) — answered BEFORE capture opens, so the host plans cursor
/// delivery honestly instead of discovering a cursorless stream after the fact (the
/// `blends_cursor` audit finding): a blend-capable backend takes cursor-as-metadata capture
/// (pointer-free frames + host composite on demand — the cursor channel's contract); for
/// anything else the host must have the compositor EMBED the pointer. The sibling verdict of
/// [`linux_native_nv12_ok`], threaded into the same negotiation.
///
/// `cuda_planned` is the caller's prediction of a CUDA capture payload (NVIDIA + zero-copy — the
/// prediction `SessionPlan` already makes); `ten_bit` the negotiated depth. Both shift the
/// dispatch: a CPU payload keeps NVIDIA on libav NVENC (no blend), and a 10-bit session keeps
/// Vulkan Video (which blends) only where the device advertises a 10-bit profile for that codec —
/// `vulkan_encode_available_at`, the same query the open makes.
#[cfg(target_os = "linux")]
pub fn cursor_blend_capable(codec: Codec, cuda_planned: bool, ten_bit: bool) -> bool {
    // A negotiated PyroWave session routes to that backend before the pref is consulted
    // (`open_video_backend_linux`), and its wavelet CSC composites the metadata cursor.
    if codec == Codec::PyroWave {
        return true;
    }
    let direct_nvenc = {
        #[cfg(feature = "nvenc")]
        {
            nvenc_direct_enabled()
        }
        #[cfg(not(feature = "nvenc"))]
        {
            false
        }
    };
    let vulkan_csc = {
        // The compute-CSC arm — the one that blends. Eligibility mirrors `open_amd_intel`;
        // the device probe runs last (it opens a Vulkan instance, cached per GPU+codec).
        #[cfg(feature = "vulkan-encode")]
        {
            // Mirrors `open_amd_intel` exactly, depth included — the device answers whether a
            // 10-bit profile for this codec exists, so the prediction and the open agree.
            matches!(codec, Codec::H265 | Codec::Av1)
                && vulkan_encode_enabled()
                && vulkan_encode_available_at(codec, ten_bit)
        }
        #[cfg(not(feature = "vulkan-encode"))]
        {
            let _ = ten_bit; // the depth only ever narrows the Vulkan arm
            false
        }
    };
    let backend = resolve_linux_backend(
        ss_host_config::config().encoder_pref.as_str(),
        linux_auto_is_vaapi,
        cuda_planned,
    );
    cursor_blend_capable_for(backend, cuda_planned, direct_nvenc, vulkan_csc)
}

/// The dispatch-mirroring core of [`cursor_blend_capable`], device-free for the unit tests.
/// `direct_nvenc` = the direct-SDK NVENC path is compiled in and enabled; `vulkan_csc` = the
/// Vulkan Video compute-CSC arm (the one that blends) is compiled in, enabled, and
/// device-supported for the session's codec.
#[cfg(target_os = "linux")]
fn cursor_blend_capable_for(
    backend: Option<LinuxBackend>,
    cuda_planned: bool,
    direct_nvenc: bool,
    vulkan_csc: bool,
) -> bool {
    match backend {
        // The wavelet CSC composites the metadata cursor (`linux/pyrowave.rs`).
        Some(LinuxBackend::Pyrowave) => true,
        // Only the direct-SDK arm blends (VkSlotBlend), and it only takes CUDA payloads —
        // a CPU-payload session stays on libav NVENC, which cannot blend.
        Some(LinuxBackend::Nvenc) => cuda_planned && direct_nvenc,
        // The Vulkan Video compute-CSC path blends, at either depth. The session plan keeps a
        // cursor-blend session off the native-NV12 and RGB-direct shapes
        // (`SessionPlan::output_format` / `VulkanVideoEncoder::open`), so CSC eligibility IS the
        // answer — including the depth term, which `vulkan_csc` already carries.
        Some(LinuxBackend::AmdIntel) | Some(LinuxBackend::Vulkan) => vulkan_csc,
        // CPU frames: the capturer composites the metadata cursor inline before the encoder
        // runs, but the ENCODER blends nothing — the cursor channel's on-demand composite
        // contract can't be honored. Report the encoder's truth.
        Some(LinuxBackend::Software) | None => false,
    }
}

/// Can this GPU + driver actually open a Vulkan Video **encode** session for `codec`? Cached per
/// (selected GPU, codec) — the [`can_encode_10bit`] idiom, with the probe run outside the lock.
///
/// Only [`linux_native_nv12_ok`] consults this, and only for the no-fallback decision described
/// there. It is deliberately NOT wired into [`open_video`]'s dispatch: that path already degrades
/// to VAAPI on a failed open for every non-NV12 capture, so making it pay for a probe would buy
/// nothing. Prediction and truth stay separate — the probe answers "would it open", the open itself
/// reports what actually did.
#[cfg(all(target_os = "linux", feature = "vulkan-encode"))]
fn vulkan_encode_available(codec: Codec) -> bool {
    vulkan_encode_caps(codec).supported
}

/// Can the Vulkan Video backend encode `codec` at this depth on the selected GPU? The gate that
/// decides whether an HDR session gets the good path (real RFI + the compute CSC's cursor blend)
/// or falls back to libav VAAPI — asked per codec, because a device can advertise HEVC Main10 and
/// still decline 10-bit AV1.
#[cfg(all(target_os = "linux", feature = "vulkan-encode"))]
fn vulkan_encode_available_at(codec: Codec, ten_bit: bool) -> bool {
    let caps = vulkan_encode_caps(codec);
    caps.supported
        && if ten_bit {
            caps.ten_bit
        } else {
            caps.eight_bit
        }
}

/// The device's Vulkan Video encode capabilities for `codec`, cached per (selected GPU, codec) —
/// a web-console GPU change re-probes on the new adapter before the next Welcome, mirroring
/// `can_encode_444`/`can_encode_10bit`. The probe creates and destroys its own Vulkan instance,
/// so it is worth caching but safe to call from anywhere.
///
/// The `vulkan-encode` half of the cfg is not decoration: the return type comes from the
/// `vulkan_video` module, which only exists under that feature. Every caller is already gated;
/// leaving these two definitions on bare `target_os = "linux"` is what broke a featureless Linux
/// build (caught on Fedora 44, 2026-07-28).
#[cfg(all(target_os = "linux", feature = "vulkan-encode"))]
fn vulkan_encode_caps(codec: Codec) -> vulkan_video::VulkanEncodeCaps {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    #[allow(clippy::type_complexity)]
    static CACHE: OnceLock<Mutex<HashMap<(String, &'static str), vulkan_video::VulkanEncodeCaps>>> =
        OnceLock::new();
    let key = (ss_gpu::selection_key(), codec.label());
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(v) = cache.lock().unwrap().get(&key) {
        return *v;
    }
    let caps = vulkan_video::probe_encode_caps(codec);
    if caps.supported {
        tracing::info!(
            ?codec,
            eight_bit = caps.eight_bit,
            ten_bit = caps.ten_bit,
            "Vulkan Video encode probed — producer-native NV12 capture is eligible, and a 10-bit \
             session keeps this backend (real RFI + the compute CSC's cursor blend) where the \
             device accepts the profile"
        );
    } else {
        tracing::info!(
            ?codec,
            "Vulkan Video encode unavailable (no encode queue for this codec) — keeping the \
             packed-RGB capture negotiation (the native-NV12 path has no VAAPI fallback)"
        );
    }
    cache.lock().unwrap().insert(key, caps);
    caps
}

/// Cheap, side-effect-free NVIDIA-presence probe for the `auto` backend selector: the NVIDIA
/// kernel driver exposes these device nodes, AMD/Intel boxes have neither. Deliberately does NOT
/// create a CUDA context (that would allocate GPU state on every host that merely *might* be
/// NVIDIA). `SLIPSTREAM_ENCODER` overrides this entirely.
#[cfg(target_os = "linux")]
fn nvidia_present() -> bool {
    std::path::Path::new("/dev/nvidiactl").exists() || std::path::Path::new("/dev/nvidia0").exists()
}

/// The `auto` Linux backend decision, shared by [`open_video`] and [`linux_zero_copy_is_vaapi`]:
/// a manual web-console GPU preference (when that GPU is present — [`ss_gpu::manual_selection`])
/// picks its vendor's backend — AMD/Intel → VAAPI on that GPU's render node, NVIDIA → NVENC (still
/// requiring the proprietary driver's device nodes; a nouveau NVIDIA GPU can't NVENC) — otherwise
/// today's NVIDIA-presence probe, unchanged.
///
/// ⚠ This resolves the **`auto` case only** — it deliberately ignores `encoder_pref`. It is NOT a
/// mirror of [`open_video`]'s dispatch and must not be used to decide which backend a capability
/// probe should ask: use [`linux_zero_copy_is_vaapi`], which layers `encoder_pref` on top of this.
/// (`can_encode_10bit` used this directly and answered for the wrong backend whenever a host
/// forced one.)
#[cfg(target_os = "linux")]
fn linux_auto_is_vaapi() -> bool {
    if let Some(g) = ss_gpu::manual_selection() {
        if g.vendor_id == ss_gpu::VENDOR_NVIDIA {
            return !nvidia_present();
        }
        return true;
    }
    !nvidia_present()
}

/// The resolved Linux encode backend. ONE alias table is consumed by [`open_video_backend`]'s
/// dispatch and every capability/advertisement
/// mirror below, so the two can never drift again (the audit counted five partial hand-copies of
/// this decision). Labels still come from the open sites (the mgmt-record convention) — this
/// resolves which ARM runs, never what actually opened.
#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LinuxBackend {
    Nvenc,
    AmdIntel,
    Vulkan,
    Pyrowave,
    Software,
}

/// The pure core. `None` = the pref names no backend — [`open_video_backend`] BAILS on that,
/// while the capability mirrors historically resolved it as auto (via
/// [`linux_resolved_backend`]); that divergence is recorded, not accidental. `auto_is_vaapi` is
/// deliberately LAZY: explicit prefs must stay zero-probe — `/serverinfo` polls through the
/// mirrors, and `gamestream/serverinfo.rs` already litigated exactly that probe cost.
#[cfg(target_os = "linux")]
fn resolve_linux_backend(
    pref: &str,
    auto_is_vaapi: impl FnOnce() -> bool,
    cuda: bool,
) -> Option<LinuxBackend> {
    Some(match pref {
        "nvenc" | "nvidia" | "cuda" => LinuxBackend::Nvenc,
        "vaapi" | "amd" | "intel" => LinuxBackend::AmdIntel,
        "vulkan" | "vulkan-video" => LinuxBackend::Vulkan,
        "pyrowave" => LinuxBackend::Pyrowave,
        "software" | "sw" | "openh264" => LinuxBackend::Software,
        // A CUDA frame can ONLY be consumed by NVENC. Otherwise the shared auto decision
        // (manual web-console GPU preference, else the NVIDIA-presence probe) picks the backend —
        // see `linux_auto_is_vaapi`.
        "auto" | "" => {
            if cuda || !auto_is_vaapi() {
                LinuxBackend::Nvenc
            } else {
                LinuxBackend::AmdIntel
            }
        }
        _ => return None,
    })
}

/// Config-reading wrapper for the capability/advertisement mirrors: an unknown pref resolves as
/// auto — exactly what every mirror's old `_` arm did. `cuda = false`: the mirrors answer
/// pre-session questions (a CUDA payload exists only inside a live NVIDIA session).
#[cfg(target_os = "linux")]
fn linux_resolved_backend() -> LinuxBackend {
    let pref = ss_host_config::config().encoder_pref.as_str();
    resolve_linux_backend(pref, linux_auto_is_vaapi, false).unwrap_or_else(|| {
        if linux_auto_is_vaapi() {
            LinuxBackend::AmdIntel
        } else {
            LinuxBackend::Nvenc
        }
    })
}

/// The dmabuf modifiers the PyroWave encoder's Vulkan device imports for the capture's
/// packed-RGB fourcc — advertised by the capture when the pyrowave passthrough is active
/// (the VAAPI LINEAR-only policy starves it on Mutter+NVIDIA, which allocates tiled only).
#[cfg(all(target_os = "linux", feature = "pyrowave"))]
pub fn pyrowave_capture_modifiers(fourcc: u32) -> Vec<u64> {
    pyrowave::capture_modifiers(fourcc)
}

/// True if the Linux GPU encode backend resolves to VAAPI (AMD/Intel) rather than NVENC — mirrors
/// [`open_video`]'s dispatch so the capturer can choose the matching zero-copy path (raw dmabuf
/// passthrough for VAAPI vs the EGL→CUDA import for NVENC).
#[cfg(target_os = "linux")]
pub fn linux_zero_copy_is_vaapi() -> bool {
    linux_zero_copy_is_vaapi_for(linux_resolved_backend())
}
/// The zero-copy-plane decision for an ALREADY-resolved backend — so a caller that has just
/// resolved (e.g. `host_wire_caps`, which consults several gates per call on a POLLED endpoint)
/// pays for the resolution once instead of once per gate (the `serverinfo.rs` probe-cost class,
/// caught by the WP7.6 diff critic).
#[cfg(target_os = "linux")]
fn linux_zero_copy_is_vaapi_for(backend: LinuxBackend) -> bool {
    match backend {
        LinuxBackend::Nvenc => false,
        LinuxBackend::AmdIntel => true,
        // PyroWave ingests the raw capture dmabuf itself (Vulkan import + compute CSC) on ANY
        // vendor — it must get the passthrough payload, never the EGL→CUDA import.
        LinuxBackend::Pyrowave => true,
        // EXACTLY the old `_` fallthrough for these two prefs — preserved, not endorsed. The
        // vulkan-on-NVIDIA half is FILED (EGL→CUDA capture negotiated for a dmabuf-importing
        // backend; reachable only via the explicit lab pref); the software half is latent (a
        // software host pins H.264, so the GPU-backend probes this steers never negotiate).
        LinuxBackend::Vulkan | LinuxBackend::Software => linux_auto_is_vaapi(),
    }
}

/// Which codecs the active GPU can actually ENCODE. Used to build the GameStream codec
/// advertisement so a client never negotiates a codec the GPU can't do (AV1 encode is narrow —
/// Intel Arc/Xe2+, AMD RDNA3+/RDNA4 — so it must be probed, not assumed).
#[derive(Clone, Copy, Debug)]
pub struct CodecSupport {
    pub h264: bool,
    pub h265: bool,
    pub av1: bool,
}

impl CodecSupport {
    /// The probed codecs as a `quic::CODEC_*` bitfield, or `None` when the probe found nothing —
    /// meaning the GPU wasn't usable at probe time (GPU-less CI, a wrong-vendor pref), NOT that it
    /// encodes zero codecs; the caller then falls back to the static superset (the native-path
    /// analogue of `gamestream::serverinfo::probed_mask`).
    pub fn wire_mask(self) -> Option<u8> {
        let mut m = 0u8;
        if self.h264 {
            m |= slipstream_core::quic::CODEC_H264;
        }
        if self.h265 {
            m |= slipstream_core::quic::CODEC_HEVC;
        }
        if self.av1 {
            m |= slipstream_core::quic::CODEC_AV1;
        }
        (m != 0).then_some(m)
    }
}

/// Probe the active NVIDIA GPU for its encodable codecs (cached once per process). Asks the driver
/// for this chip's encode-GUID list over the direct SDK — see [`nvenc_cuda::probe_support`] for
/// why it is not shaped like the VAAPI probe below, and for the fail-open contract. Process-wide
/// (not per selected GPU) on purpose: the direct backend opens on the shared `cuda::context()`,
/// i.e. CUDA device 0, so that is the chip whose answer applies.
#[cfg(all(target_os = "linux", feature = "nvenc"))]
pub fn nvenc_codec_support() -> CodecSupport {
    use std::sync::OnceLock;
    static LOGGED: OnceLock<()> = OnceLock::new();
    let probed = nvenc_cuda::probe_support();
    LOGGED.get_or_init(|| {
        tracing::info!(
            h264 = probed.codecs.h264,
            h265 = probed.codecs.h265,
            av1 = probed.codecs.av1,
            hevc_444 = probed.hevc_444,
            "NVENC encode capabilities probed"
        );
    });
    probed.codecs
}

/// Probe the active Linux GPU backend for its encodable codecs (cached; opens a tiny encoder per
/// codec, once). The AMD/Intel backend — NVIDIA has [`nvenc_codec_support`] (callers gate on
/// [`linux_zero_copy_is_vaapi`]).
#[cfg(target_os = "linux")]
pub fn vaapi_codec_support() -> CodecSupport {
    use std::sync::OnceLock;
    static CACHE: OnceLock<CodecSupport> = OnceLock::new();
    *CACHE.get_or_init(|| {
        let caps = CodecSupport {
            h264: vaapi::probe_can_encode(Codec::H264),
            h265: vaapi::probe_can_encode(Codec::H265),
            av1: vaapi::probe_can_encode(Codec::Av1),
        };
        tracing::info!(
            h264 = caps.h264,
            h265 = caps.h265,
            av1 = caps.av1,
            "VAAPI encode capabilities probed"
        );
        caps
    })
}

/// Whether the active GPU encode backend can actually produce a full-chroma **4:4:4** HEVC stream.
/// Resolved (and cached, once) *before* the Welcome so the host advertises the chroma it will really
/// encode — the honest-downgrade channel. 4:4:4 is HEVC-only; the probe opens a tiny encoder on the
/// active backend (NVENC FREXT is broad on NVIDIA, but VAAPI 4:4:4 is hardware-specific, so it
/// must be probed, never assumed). Non-HEVC codecs are always `false`.
pub fn can_encode_444(codec: Codec) -> bool {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    if codec == Codec::PyroWave {
        // PyroWave does its own RGB→YCbCr CSC (capture always hands it a full-chroma source),
        // so 4:4:4 needs no GPU encode probe — only the full-res-chroma CSC variant:
        // `rgb2yuv444.comp` supplies the full-resolution chroma conversion.
        return true;
    }
    if codec != Codec::H265 {
        return false;
    }
    // Cached per selected GPU (was a process-lifetime OnceLock): a web-console preference change
    // re-probes on the newly selected adapter before the next Welcome.
    static CACHE: OnceLock<Mutex<HashMap<String, bool>>> = OnceLock::new();
    let key = ss_gpu::selection_key();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(v) = cache.lock().unwrap().get(&key) {
        return *v;
    }
    let supported = if linux_zero_copy_is_vaapi() {
        vaapi::probe_can_encode_444(codec)
    } else {
        // NVIDIA. On a direct-SDK host the answer comes from the driver's caps bit over the
        // direct SDK ([`nvenc_cuda::probe_support`]) — the same
        // `NV_ENC_CAPS_SUPPORT_YUV444_ENCODE` the live session re-checks at open
        // (`query_caps` → `yuv444_supported`). It must not fall back to the libav open-probe
        // when direct NVENC is active, because that can poison later direct sessions.
        #[cfg(feature = "nvenc")]
        {
            if nvenc_direct_enabled() {
                nvenc_cuda::probe_support().hevc_444
            } else {
                linux::probe_can_encode_444(codec)
            }
        }
        #[cfg(not(feature = "nvenc"))]
        {
            linux::probe_can_encode_444(codec)
        }
    };
    tracing::info!(supported, "HEVC 4:4:4 encode capability probed");
    cache.lock().unwrap().insert(key, supported);
    supported
}

/// Whether the active GPU encode backend can actually produce a **10-bit** stream for `codec`
/// (HEVC Main10 / AV1 10-bit). Resolved (and cached per selected GPU) *before* the Welcome so the
/// negotiated bit depth — and the HDR/SDR colour label derived from it — matches what the encoder
/// will really emit: the honest-downgrade channel, exactly like [`can_encode_444`]. Without this
/// gate a default-on `SLIPSTREAM_10BIT` would negotiate 10-bit on a GPU/backend that then silently
/// falls back to 8-bit post-Welcome (label HDR / stream SDR).
///
/// Backend truth: Linux probes a tiny real Main10 open on the resolved backend, using libav NVENC
/// or VAAPI for the HDR X2RGB10 to P010 path. The direct-SDK CUDA path and Vulkan Video stay 8-bit
/// and a 10-bit session routes around them.
pub fn can_encode_10bit(codec: Codec) -> bool {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    if !codec.supports_10bit() {
        return false;
    }
    if codec == Codec::PyroWave {
        // PyroWave is depth-agnostic, but the Linux capture path does not expose HDR input for it.
        return false;
    }
    // Cached per (selected GPU, codec) — a web-console preference change re-probes on the newly
    // selected adapter before the next Welcome, mirroring `can_encode_444`.
    static CACHE: OnceLock<Mutex<HashMap<(String, &'static str), bool>>> = OnceLock::new();
    let key = (ss_gpu::selection_key(), codec.label());
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(v) = cache.lock().unwrap().get(&key) {
        return *v;
    }
    let supported = {
        // NVENC (libav, the HDR P010 swscale path) or, on AMD/Intel, VAAPI or Vulkan Video can
        // serve the session. Both AMD/Intel paths are checked because the Vulkan opener falls back
        // to VAAPI when needed. Capture-side HDR support is resolved separately by the host.
        if linux_zero_copy_is_vaapi() {
            let vulkan10 = {
                #[cfg(feature = "vulkan-encode")]
                {
                    vulkan_encode_enabled() && vulkan_encode_available_at(codec, true)
                }
                #[cfg(not(feature = "vulkan-encode"))]
                {
                    false
                }
            };
            vulkan10 || vaapi::probe_can_encode_10bit(codec)
        } else {
            linux::probe_can_encode_10bit(codec)
        }
    };
    tracing::info!(codec = ?codec, supported, "10-bit encode capability probed");
    cache.lock().unwrap().insert(key, supported);
    supported
}

/// Marker in an encoder error's `anyhow` context chain: the failure is a deterministic
/// consequence of the session's configuration, so an in-place rebuild retry can never succeed
/// (e.g. the backend binding to a wrong-vendor adapter — the same wall on every attempt). The
/// stream loop's reset ladder downcasts for this and ends the session immediately with the
/// carried cause instead of burning its rebuild budget first. Attach with
/// `anyhow::Error::new(TerminalEncoderError).context("the actual cause")`.
#[derive(Clone, Copy, Debug)]
pub struct TerminalEncoderError;

impl std::fmt::Display for TerminalEncoderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("deterministic configuration error — an encoder rebuild cannot fix this")
    }
}

impl std::error::Error for TerminalEncoderError {}

/// True if the resolved Linux encode backend produces GPU-resident frames.
/// The software encoder is the only path that requires CPU staging.
#[cfg(target_os = "linux")]
pub fn resolved_backend_is_gpu() -> bool {
    linux_resolved_backend() != LinuxBackend::Software
}

/// Linux capture does not expose a packed RGB source for an encoder-side 4:4:4 CSC.
#[cfg(target_os = "linux")]
pub fn resolved_backend_ingests_rgb_444() -> bool {
    false
}

// Linux backends. `#[path]` keeps the `crate::*` module names flat.
#[cfg(target_os = "linux")]
#[path = "backend/linux/mod.rs"]
mod linux;
// Direct-SDK NVENC on Linux (CUDA input; design/linux-direct-nvenc.md) — real RFI + recovery anchor
// + reset() lever the libavcodec `linux::NvencEncoder` can't express. Opt-in behind
// `SLIPSTREAM_NVENC_DIRECT`; the `.so` resolves at runtime, so `--features nvenc` stays safe on a
// driver-less or AMD Linux box.
#[cfg(all(target_os = "linux", feature = "nvenc"))]
#[path = "backend/linux/nvenc_cuda.rs"]
mod nvenc_cuda;
// Actionable `NVENCSTATUS` → cause mapping, so a failed session open names its real cause instead
// of the old misleading "(no NVIDIA GPU?)".
#[cfg(all(target_os = "linux", feature = "nvenc"))]
#[path = "backend/nvenc_status.rs"]
mod nvenc_status;
// Direct-SDK NVENC glue (`NvStatusExt`/`nv_ok`, `codec_guid`) shared by the Linux backend.
#[cfg(all(target_os = "linux", feature = "nvenc"))]
#[path = "backend/nvenc_core.rs"]
mod nvenc_core;
// Slot-family RFI policy used by the Linux Vulkan Video backend.
#[cfg(all(target_os = "linux", feature = "vulkan-encode"))]
#[path = "backend/rfi.rs"]
mod rfi;
// Shared libavcodec glue (`pixel_to_av`, swscale consts) for Linux NVENC and VAAPI.
#[cfg(target_os = "linux")]
#[path = "backend/libav.rs"]
mod libav;
// Software (openh264) H.264 encoder for headless and GPU-less Linux hosts.
#[cfg(target_os = "linux")]
#[path = "backend/sw.rs"]
mod sw;
#[cfg(target_os = "linux")]
#[path = "backend/linux/vaapi.rs"]
mod vaapi;
// Raw Vulkan Video HEVC encode on Linux (AMD/Intel; design/linux-vulkan-video-encode.md) — real RFI
// via explicit DPB reference slots (the app owns the DPB), the open-stack twin of the direct-NVENC
// path. Does an on-GPU RGB→NV12 compute CSC since capture delivers packed-RGB dmabufs. Opt-in behind
// `SLIPSTREAM_VULKAN_ENCODE` until on-glass validated; needs `--features vulkan-encode`.
#[cfg(all(target_os = "linux", feature = "vulkan-encode"))]
#[path = "backend/linux/vulkan_video.rs"]
mod vulkan_video;
// Vendored `VK_KHR_video_encode_av1` bindings (host-only) — the AV1 encode structs our pinned
// `ash 0.38.0+1.3.281` predates (finalized Vulkan 1.3.290). Copied verbatim from ash-master's
// generated code rather than bumping `ash` (which breaks the SDL/Vulkan client). Consumed by
// `vulkan_video.rs` via `super::vk_av1_encode`.
#[cfg(all(target_os = "linux", feature = "vulkan-encode"))]
#[path = "backend/linux/vk_av1_encode.rs"]
mod vk_av1_encode;
// Vendored `VK_VALVE_video_encode_rgb_conversion` bindings (host-only) — RGB encode source with
// the VCN EFC front-end doing the CSC (design/vulkan-rgb-direct-encode.md). ash 0.38 predates
// the extension; same vendoring rationale as `vk_av1_encode`.
#[cfg(all(target_os = "linux", feature = "vulkan-encode"))]
#[path = "backend/linux/vk_valve_rgb.rs"]
mod vk_valve_rgb;
// Small ash leaf helpers shared by the Linux Vulkan encode backends (dmabuf import, image/memory
// utilities) — extracted from `vulkan_video.rs` when the PyroWave backend arrived.
#[cfg(all(
    target_os = "linux",
    any(feature = "vulkan-encode", feature = "pyrowave")
))]
#[path = "backend/linux/vk_util.rs"]
mod vk_util;
// PyroWave — the opt-in wired-LAN intra-only wavelet codec (design/pyrowave-codec-plan.md §4.3):
// pure Vulkan compute via the vendored `pyrowave-sys`, sub-ms encode, every frame a keyframe.
// Explicit-only behind SLIPSTREAM_ENCODER=pyrowave; EXPERIMENTAL until CODEC_PYROWAVE lands.
#[cfg(all(target_os = "linux", feature = "pyrowave"))]
#[path = "backend/linux/pyrowave.rs"]
mod pyrowave;
// Shared PyroWave AU wire-framing (§4.4) for the Linux backend.
#[cfg(all(target_os = "linux", feature = "pyrowave"))]
#[path = "backend/pyrowave_wire.rs"]
mod pyrowave_wire;

/// Whether a PyroWave mode fits the vendored rate controller's packed 16-bit block index
/// (`patches/0002-rdo-saving-clamp.patch` note): false ≈ 8K-class 4:4:4. The negotiator
/// downgrades such a session to 4:2:0 before the Welcome; the encoders also refuse outright.
#[cfg(all(target_os = "linux", feature = "pyrowave"))]
pub fn pyrowave_mode_fits_rdo(width: u32, height: u32, chroma444: bool) -> bool {
    pyrowave_wire::block_count_32x32(width, height, chroma444) <= u16::MAX as u32
}
#[cfg(not(all(target_os = "linux", feature = "pyrowave")))]
pub fn pyrowave_mode_fits_rdo(_width: u32, _height: u32, _chroma444: bool) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The probed-capability → wire-bitfield mapping the native codec advertisement is built from.
    #[test]
    fn codec_support_wire_mask() {
        use slipstream_core::quic::{CODEC_AV1, CODEC_H264, CODEC_HEVC};
        let all = CodecSupport {
            h264: true,
            h265: true,
            av1: true,
        };
        assert_eq!(all.wire_mask(), Some(CODEC_H264 | CODEC_HEVC | CODEC_AV1));
        let hevc_only = CodecSupport {
            h264: false,
            h265: true,
            av1: false,
        };
        assert_eq!(hevc_only.wire_mask(), Some(CODEC_HEVC));
        // An all-false probe means "GPU unusable at probe time", not "zero codecs" — `None` tells
        // the caller to advertise the static superset instead of refusing every handshake.
        let none = CodecSupport {
            h264: false,
            h265: false,
            av1: false,
        };
        assert_eq!(none.wire_mask(), None);
    }

    /// The cursor-blend capability mirror, arm by arm — the table the caps-aware negotiation
    /// (cursor channel grant, metadata-vs-embedded capture) stands on. Each row names the arm's
    /// blending stage or the reason there is none.
    #[cfg(target_os = "linux")]
    #[test]
    fn cursor_blend_capability_mirrors_the_dispatch() {
        use LinuxBackend::*;
        // PyroWave: the wavelet CSC composites, always.
        assert!(cursor_blend_capable_for(
            Some(Pyrowave),
            false,
            false,
            false
        ));
        // NVIDIA: only the direct-SDK arm blends (VkSlotBlend), and only for CUDA payloads.
        assert!(cursor_blend_capable_for(Some(Nvenc), true, true, false));
        assert!(
            !cursor_blend_capable_for(Some(Nvenc), false, true, false),
            "a CPU payload stays on libav NVENC, which cannot blend"
        );
        assert!(
            !cursor_blend_capable_for(Some(Nvenc), true, false, false),
            "SLIPSTREAM_NVENC_DIRECT=0 (or a build without the feature) is the libav path"
        );
        // AMD/Intel: the Vulkan Video compute-CSC arm blends — at 8 AND 10 bits, since its CSC
        // has a BT.2020 Main10 variant. VAAPI never does. The depth term lives in `vulkan_csc`
        // (10-bit AV1 is the one combination that still falls through to VAAPI), so this arm is
        // now exactly "did an eligible CSC arm resolve".
        assert!(cursor_blend_capable_for(Some(AmdIntel), false, false, true));
        assert!(
            !cursor_blend_capable_for(Some(AmdIntel), false, false, false),
            "no eligible Vulkan CSC arm (H.264, SLIPSTREAM_VULKAN_ENCODE=0, unsupported \
             device) resolves to libav VAAPI, which cannot blend"
        );
        assert!(cursor_blend_capable_for(Some(Vulkan), false, false, true));
        // Software / unknown pref: CPU frames; the encoder blends nothing.
        assert!(!cursor_blend_capable_for(Some(Software), false, true, true));
        assert!(!cursor_blend_capable_for(None, false, true, true));
    }

    /// WP7.7 guard (the cheap half): every `Encoder` trait method must be explicitly forwarded by
    /// `TrackedEncoder`. A defaulted trait method that isn't forwarded silently no-ops through the
    /// wrapper — the trap has bitten three times (`set_wire_chunking`'s §4.4 chunking probe,
    /// `set_pipelined`'s LN3 escalation, `applied_bitrate_bps`'s ABR truth), and every Phase 7
    /// consolidation that adds a trait method re-arms it. Source-text parse, not reflection: both
    /// blocks are top-level rustfmt items, so each ends at the first column-0 `}` and every method
    /// name sits on a line starting `fn ` (a wrapped signature still carries the name there).
    #[test]
    fn tracked_encoder_forwards_every_trait_method() {
        fn item_block<'a>(src: &'a str, marker: &str) -> &'a str {
            let start = src
                .find(marker)
                .unwrap_or_else(|| panic!("marker {marker:?} not found — update this guard"));
            let body = &src[start..];
            let end = body
                .find("\n}")
                .unwrap_or_else(|| panic!("no column-0 close brace after {marker:?}"));
            &body[..end]
        }
        fn fn_names(block: &str) -> std::collections::BTreeSet<&str> {
            block
                .lines()
                .map(str::trim_start)
                .filter(|l| !l.starts_with("//"))
                .filter_map(|l| l.strip_prefix("fn "))
                .map(|rest| {
                    rest.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .next()
                        .expect("split yields at least one item")
                })
                .collect()
        }
        // `find` takes the FIRST occurrence: the real impl precedes this test's own copy of the
        // marker string in the included source.
        let trait_fns = fn_names(item_block(
            include_str!("backend/codec.rs"),
            "pub trait Encoder: Send {",
        ));
        let impl_fns = fn_names(item_block(
            include_str!("lib.rs"),
            "impl Encoder for TrackedEncoder {",
        ));
        assert!(
            trait_fns.len() >= 12,
            "only {} trait methods parsed — the extraction markers have rotted, fix the parse \
             before trusting this guard",
            trait_fns.len()
        );
        let missing: Vec<_> = trait_fns.difference(&impl_fns).collect();
        assert!(
            missing.is_empty(),
            "Encoder methods NOT forwarded by TrackedEncoder: {missing:?} — the host loop only \
             ever holds the wrapped box, so an unforwarded default silently disables the feature \
             for every session. Forward each one in `impl Encoder for TrackedEncoder`."
        );
        // The reverse direction (impl fn absent from the trait) is a compile error, so equality
        // here is pure belt-and-braces against a parse regression.
        assert_eq!(trait_fns, impl_fns);
    }

    /// The typed-EINVAL classifier the bitrate ladder keys on (Phase 8): the `ffmpeg::Error`
    /// must survive `with_context` layers as a downcastable source — pinned here because the
    /// entire ladder's step-down behavior rests on it, and an eager `format!` anywhere between
    /// `open_with` and the ladder would silently break it (the ladder would stop stepping and
    /// 4K sessions would surface errors instead of degrading).
    #[cfg(target_os = "linux")]
    #[test]
    fn nvenc_open_einval_survives_context_layers() {
        use ffmpeg_next as ffmpeg;
        let e = anyhow::Error::from(ffmpeg::Error::Other {
            errno: ffmpeg::util::error::EINVAL,
        })
        .context("open hevc_nvenc (3840x2160@120, 400000000 bps)")
        .context("outer");
        assert!(nvenc_open_einval(&e));
        // ENOSYS (or any other errno) must not step the ladder.
        let e = anyhow::Error::from(ffmpeg::Error::Other {
            errno: ffmpeg::util::error::ENOSYS,
        })
        .context("open");
        assert!(!nvenc_open_einval(&e));
    }

    /// The phrase WITHOUT the type no longer classifies — the fragility that was removed: the
    /// old string match trusted any error whose rendering contained "Invalid argument".
    #[cfg(target_os = "linux")]
    #[test]
    fn nvenc_open_einval_ignores_untyped_text() {
        let e = anyhow::anyhow!("driver said: Invalid argument (not a typed libav errno)");
        assert!(!nvenc_open_einval(&e));
    }

    /// WP7.6: the resolver's full alias table, pinned. The panicking closure is the laziness
    /// contract — an explicit pref must resolve WITHOUT running the auto probe (`/serverinfo`
    /// polls through the mirrors; `gamestream/serverinfo.rs` litigated exactly that cost).
    #[cfg(target_os = "linux")]
    #[test]
    fn linux_backend_resolver_table() {
        use LinuxBackend::*;
        let no_probe = || -> bool { panic!("explicit prefs must not run the auto probe") };
        for pref in ["nvenc", "nvidia", "cuda"] {
            assert_eq!(resolve_linux_backend(pref, no_probe, false), Some(Nvenc));
        }
        for pref in ["vaapi", "amd", "intel"] {
            assert_eq!(resolve_linux_backend(pref, no_probe, false), Some(AmdIntel));
        }
        for pref in ["vulkan", "vulkan-video"] {
            assert_eq!(resolve_linux_backend(pref, no_probe, false), Some(Vulkan));
        }
        assert_eq!(
            resolve_linux_backend("pyrowave", no_probe, false),
            Some(Pyrowave)
        );
        for pref in ["software", "sw", "openh264"] {
            assert_eq!(resolve_linux_backend(pref, no_probe, false), Some(Software));
        }
        // auto (and the default empty pref): the probe decides…
        assert_eq!(resolve_linux_backend("", || true, false), Some(AmdIntel));
        assert_eq!(resolve_linux_backend("auto", || false, false), Some(Nvenc));
        // …except a CUDA frame, which only NVENC can consume — and that short-circuits the
        // probe too (`||` order).
        assert_eq!(resolve_linux_backend("auto", no_probe, true), Some(Nvenc));
        // Unknown prefs name no backend: the dispatch bails; the mirrors' wrapper maps this to
        // auto (the historical `_`-arm behavior, recorded in the design doc).
        assert_eq!(resolve_linux_backend("banana", no_probe, false), None);
        // An explicit pref is never overridden by `cuda` (a CUDA payload cannot appear in a
        // session whose pref forced a non-NVENC backend; the table must not mask such a bug).
        assert_eq!(
            resolve_linux_backend("vaapi", no_probe, true),
            Some(AmdIntel)
        );
    }

    /// WP7.6: the REAL dispatch through the resolver, GPU-free via the software arm (openh264 is
    /// vendored + CPU-only; its own tests already run on every leg). This is the only in-CI
    /// execution of the Linux dispatch — before this test it had zero test call sites. The pref
    /// is INJECTED through `open_video_backend_linux` — the first draft mutated `SLIPSTREAM_ENCODER`
    /// via `set_var`, the binary's first non-ignored env mutation, racing `getenv` in the sw/codec/
    /// nvenc_core tests (diff-critic catch); the seam removes the race and the config-latch
    /// coupling outright.
    #[cfg(target_os = "linux")]
    #[test]
    fn open_video_backend_dispatches_software() {
        let (enc, label) = open_video_backend_linux(
            "software",
            Codec::H264,
            PixelFormat::Bgrx,
            64,
            64,
            30,
            1_000_000,
            false,
            8,
            ChromaFormat::Yuv420,
            false,
            4,
        )
        .expect("software arm must open GPU-free");
        assert_eq!(label, "software");
        drop(enc);
        // The software arm is codec-gated — a non-H.264 session must take the bail, not fall
        // through to another backend.
        let err = match open_video_backend_linux(
            "software",
            Codec::H265,
            PixelFormat::Bgrx,
            64,
            64,
            30,
            1_000_000,
            false,
            8,
            ChromaFormat::Yuv420,
            false,
            4,
        ) {
            // `expect_err` needs `Ok: Debug` and `Box<dyn Encoder>` isn't — match instead.
            Ok(_) => panic!("software emits H.264 only; an H.265 session must be refused"),
            Err(e) => e,
        };
        assert!(err.to_string().contains("H.264"), "{err:#}");
    }
}
