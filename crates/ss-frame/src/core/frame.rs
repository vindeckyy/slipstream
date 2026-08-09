/// Packed pixel layout of a [`CapturedFrame`]. The ScreenCast portal negotiates the
/// format; on wlroots it is commonly packed `RGB` (3 bytes/pixel). The encoder maps these
/// to an NVENC-accepted input format (`rgb0`/`bgr0`/`rgba`/`bgra`), expanding 3→4 bytes
/// where needed — no host-side colour conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelFormat {
    /// `[B,G,R,x]`, 4 bpp.
    Bgrx,
    /// `[R,G,B,x]`, 4 bpp.
    Rgbx,
    /// `[B,G,R,A]`, 4 bpp.
    Bgra,
    /// `[R,G,B,A]`, 4 bpp.
    Rgba,
    /// `[R,G,B]`, 3 bpp.
    Rgb,
    /// `[B,G,R]`, 3 bpp.
    Bgr,
    /// 10-bit RGB packed as `R10G10B10A2`, 4 bpp. The HDR capture path
    /// produces this: scRGB FP16 desktop pixels are converted to BT.2020 PQ and written here, then
    /// handed to NVENC as `ABGR10` for an HEVC Main10 / HDR10 encode.
    Rgb10a2,
    /// `NV12`: 8-bit BT.709 limited-range YUV 4:2:0, handed to NVENC as `NV12`.
    Nv12,
    /// `P010`: 10-bit BT.2020 PQ limited-range YUV 4:2:0, handed to NVENC as `YUV420_10BIT`.
    P010,
    /// Planar 8-bit YUV **4:4:4** (BT.709; range per `SLIPSTREAM_444_FULLRANGE`). Produced by the
    /// Linux zero-copy worker's GPU convert for a 4:4:4 session ([`FramePayload::Cuda`] with
    /// `DeviceBuffer::yuv444` — three full-res planes stacked in one allocation); NVENC encodes
    /// it natively under the Range-Extensions profile. Never a CPU payload.
    Yuv444,
    /// 10-bit RGB packed `x:R:G:B 2:10:10:10` little-endian (SPA `xRGB_210LE`, DRM `XRGB2101010` /
    /// `XR30`, ffmpeg `x2rgb10le`, NVENC `ARGB10`) — as an LE u32: B in bits 0-9, G 10-19, R 20-29.
    /// The Linux GNOME 50+ HDR screencast source format: Mutter advertises it (with BT.2020
    /// primaries + SMPTE ST.2084 PQ transfer) for a monitor in HDR mode, so the samples are
    /// PQ-encoded BT.2020 RGB.
    X2Rgb10,
    /// 10-bit RGB packed `x:B:G:R 2:10:10:10` little-endian (SPA `xBGR_210LE`, DRM `XBGR2101010` /
    /// `XB30`, ffmpeg `x2bgr10le`, NVENC `ABGR10`) — as an LE u32: R in bits 0-9, G 10-19, B 20-29;
    /// the same component order as [`Rgb10a2`]. The second GNOME 50+ HDR screencast format has the
    /// same PQ/BT.2020 colorimetry as [`X2Rgb10`](Self::X2Rgb10).
    X2Bgr10,
}

impl PixelFormat {
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            PixelFormat::Rgb | PixelFormat::Bgr => 3,
            // Three full-res 1-byte planes (GPU-resident only; no CPU payload carries this).
            PixelFormat::Yuv444 => 3,
            _ => 4,
        }
    }

    /// True for the packed 10-bit RGB layouts a Linux HDR (BT.2020 PQ) capture negotiates —
    /// the formats that make a session's encode bit depth 10 (HEVC Main10 / 10-bit AV1).
    pub fn is_hdr_rgb10(self) -> bool {
        matches!(self, PixelFormat::X2Rgb10 | PixelFormat::X2Bgr10)
    }
}

/// DRM FourCC for a packed 32-bit format name (little-endian, e.g. `b"XR24"`).
#[cfg(target_os = "linux")]
const fn drm_fourcc_code(c: &[u8; 4]) -> u32 {
    (c[0] as u32) | ((c[1] as u32) << 8) | ((c[2] as u32) << 16) | ((c[3] as u32) << 24)
}

/// Map a SPA/our [`PixelFormat`] to the DRM FourCC EGL expects for import. SPA byte order `BGRx`
/// ⇒ DRM `XRGB8888` (memory B,G,R,X), etc. Lives with the frame vocabulary (not in
/// `ss-zerocopy`) because it consumes [`PixelFormat`], which sits above that crate.
#[cfg(target_os = "linux")]
pub fn drm_fourcc(format: PixelFormat) -> Option<u32> {
    use PixelFormat::*;
    Some(match format {
        Bgrx => drm_fourcc_code(b"XR24"), // DRM_FORMAT_XRGB8888
        Bgra => drm_fourcc_code(b"AR24"), // DRM_FORMAT_ARGB8888
        Rgbx => drm_fourcc_code(b"XB24"), // DRM_FORMAT_XBGR8888
        Rgba => drm_fourcc_code(b"AB24"), // DRM_FORMAT_ABGR8888
        // Linux native NV12 capture (gamescope PipeWire): one LINEAR dmabuf with contiguous Y then
        // interleaved UV, exposed under DRM_FORMAT_NV12.
        Nv12 => drm_fourcc_code(b"NV12"),
        // The GNOME 50+ HDR screencast formats (packed 2:10:10:10, PQ/BT.2020).
        X2Rgb10 => drm_fourcc_code(b"XR30"), // DRM_FORMAT_XRGB2101010
        X2Bgr10 => drm_fourcc_code(b"XB30"), // DRM_FORMAT_XBGR2101010
        // 24-bit packed RGB/BGR have no straightforward dmabuf import here; use the CPU path.
        // Rgb10a2/P010 are not direct dmabuf inputs here; Yuv444 is convert output, never a capture
        // source.
        Rgb | Bgr | Rgb10a2 | P010 | Yuv444 => return None,
    })
}

/// Output format resolved once per session and passed into `capture_virtual_output`. The capture
/// path does not re-derive the encode backend, so both sides use the same residency and format
/// decision.
#[derive(Clone, Copy, Debug)]
pub struct OutputFormat {
    /// Produce GPU-resident frames for a GPU encoder rather than CPU staging. `false` for the
    /// GPU-less software encoder.
    pub gpu: bool,
    /// HDR capture uses a 10-bit format. `false` means 8-bit SDR.
    pub hdr: bool,
    /// Full-chroma 4:4:4 session. `false` on every 4:2:0 session.
    pub chroma_444: bool,
    /// Whether this session uses the PyroWave wavelet codec. `false` on every other session.
    pub pyrowave: bool,
    /// THIS session's encoder can ingest a producer-native NV12 capture (Linux raw Vulkan Video
    /// backend on an H265/AV1 session. The Linux capture negotiation offers gamescope the NV12 pod
    /// only when this is set because the VAAPI fallback expects packed RGB.
    pub nv12_native: bool,
    /// The session negotiated the cursor-forward channel. Linux portal metadata keeps the pointer
    /// separate from the captured frame; the session plan's `cursor_blend` gate handles the rest.
    pub hw_cursor: bool,
}

impl OutputFormat {
    /// Resolve the output format for an entry point that doesn't build a full [`SessionPlan`]
    /// (`crate::session_plan`) — the GameStream + spike paths. `gpu` is the encoder's GPU-residency,
    /// resolved by the caller via `ss_encode::resolved_backend_is_gpu` and passed **in** (capture
    /// never re-derives the backend — the one-way capture→encode edge, plan §2.4 / §W4); `hdr` as given.
    /// The native slipstream/1 path uses `SessionPlan::output_format()` instead (it already resolved the
    /// encoder), so neither path makes a capturer re-derive it.
    pub fn resolve(hdr: bool, gpu: bool) -> Self {
        OutputFormat {
            gpu,
            hdr,
            // The GameStream + spike paths are always 4:2:0 (4:4:4 is slipstream/1-native only).
            chroma_444: false,
            // GameStream never negotiates PyroWave (native slipstream/1 only).
            pyrowave: false,
            // GameStream/spike sessions never negotiate the cursor channel.
            hw_cursor: false,
            // Conservative: the GameStream + spike paths don't resolve the codec here, and a
            // Moonlight client may negotiate H264 (whose VAAPI backend can't ingest NV12) — so
            // they never prefer the producer-native NV12 pod. The slipstream/1 plane opts in via
            // `SessionPlan::output_format()`, which knows the codec.
            nv12_native: false,
        }
    }
}

/// A mouse-cursor overlay to composite onto a frame at encode time (cursor-as-metadata). Rides on
/// [`CapturedFrame::cursor`] for the GPU zero-copy payloads (Cuda/Dmabuf), whose pixels never touch
/// the CPU — the encoder blends this small bitmap into its owned surface (Vulkan CSC image / CUDA
/// devbuf / VA surface). The CPU de-pad path composites the cursor inline instead, so it leaves
/// this `None`. `rgba` is `Arc` so attaching the (unchanged) bitmap to every frame is a refcount
/// bump, not a copy; `serial` bumps only when the bitmap image changes, so the encoder re-uploads
/// its small GPU texture on change and just moves a push-constant otherwise.
#[derive(Clone)]
pub struct CursorOverlay {
    /// Top-left in frame pixels where the bitmap is drawn (already = reported position − hotspot).
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    /// Straight-alpha RGBA pixels, `w*h*4` (bytes R,G,B,A).
    pub rgba: std::sync::Arc<Vec<u8>>,
    /// Bumps whenever `rgba`/`w`/`h` change; stable across position-only moves.
    pub serial: u64,
    /// Hotspot (the pixel that IS the pointer position) within `w`×`h`. The blend paths ignore
    /// it (`x`/`y` are already hotspot-adjusted); the cursor-forward channel ships it to the
    /// client so a locally-drawn OS cursor points with the right pixel.
    pub hot_x: u32,
    pub hot_y: u32,
    /// Compositor-reported pointer visibility. `false` = an app on the host grabbed/hid the
    /// pointer — the cursor-forward channel turns that into the client's relative-mode hint
    /// (remote-desktop-sweep M3). The encode loop STRIPS invisible overlays before the frame
    /// reaches any blend path, so encoders may keep treating `Some` as "draw it".
    pub visible: bool,
}

/// A captured frame. [`format`](Self::format)/dimensions describe the pixels regardless of
/// where they live — [`payload`](Self::payload) is either a CPU buffer (the spike/fallback path)
/// or a GPU buffer already on the device (the zero-copy path, plan §9).
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub pts_ns: u64,
    /// Pixel layout of the payload.
    pub format: PixelFormat,
    pub payload: FramePayload,
    /// Cursor overlay to blend at encode time (GPU zero-copy payloads only); `None` when there's no
    /// visible cursor or the pixels were already composited on the CPU de-pad path. See
    /// [`CursorOverlay`].
    pub cursor: Option<CursorOverlay>,
    /// Per-stage capture timings (Phase 3): wall-clock ns per capture-pipeline stage, filled by
    /// the PipeWire backend; all zero on backends without stage instrumentation. Copied into the
    /// host's per-frame latency artifact record by the encode loop.
    pub stage_ns: CaptureStageTimes,
}

/// Wall-clock nanosecond stamps for the capture pipeline stages (latency Phase 3). All fields are
/// ns since the UNIX epoch; `0` = the stage did not run or is not instrumented. Filled on the
/// capture thread (PipeWire callback); consumed by the host encode loop, which copies them into
/// the per-frame artifact record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CaptureStageTimes {
    /// 1. Capture callback entry (`.process`/buffer arrival).
    pub callback_entry_ns: u64,
    /// 2. Newest-buffer selection complete (the frame this record describes is known).
    pub newest_selection_ns: u64,
    /// 3a. Implicit-fence wait start.
    pub fence_wait_start_ns: u64,
    /// 3b. Implicit-fence wait end (signaled or budget expired).
    pub fence_wait_end_ns: u64,
    /// 4. DMA-BUF / EGL / CUDA import completion (the encoder-facing surface is ready).
    pub import_end_ns: u64,
    /// 5. CUDA or Vulkan handoff completion (raw passthrough publish on the GPU path).
    pub handoff_end_ns: u64,
    /// 6a. CPU row-copy (de-pad) completion.
    pub depad_end_ns: u64,
    /// 6b. CPU colour conversion completion (same stage as 6a when the copy IS the conversion).
    pub convert_end_ns: u64,
    /// 7. Cursor composition completion.
    pub cursor_end_ns: u64,
    /// 8. Publish to the newest-frame slot (== the frame's `pts_ns` anchor).
    pub publish_ns: u64,
    /// SPA_META_Header flags when the compositor supplied the meta (0 = none).
    pub source_meta_flags: u32,
    /// SPA_META_Header pts when supplied (producer's clock domain; 0 = none).
    pub source_meta_pts_ns: u64,
}

/// A captured frame still living in a DMA-BUF. Packed RGB uses one plane. Native Linux NV12
/// (gamescope PipeWire) travels in ONE fd: Y starts at `offset`, and the interleaved UV plane
/// lives at `plane1`'s offset/stride when the producer reported them — else at the contiguous
/// fallback `offset + stride * frame_height` with the shared `stride`.
///
/// Owns a *dup* of the PipeWire buffer's fd, so the frame can travel to the encode thread and be
/// imported there without the compositor's buffer being closed underneath it. Content stability
/// across the brief import window relies on the compositor's buffer pool depth, like any zero-copy
/// capture.
#[cfg(target_os = "linux")]
pub struct DmabufFrame {
    pub fd: std::os::fd::OwnedFd,
    /// DRM FourCC (`XR24` for BGRx, `NV12` for native 4:2:0).
    pub fourcc: u32,
    /// DRM format modifier the compositor allocated (0 = LINEAR).
    pub modifier: u64,
    /// Second-plane `(offset, stride)` within the SAME fd, when the producer reported one (the
    /// PipeWire buffer's plane-1 chunk — NV12's interleaved UV). `None` falls back to the
    /// contiguous-plane contract above. Always `None` for single-plane packed RGB.
    pub plane1: Option<(u32, u32)>,
    pub offset: u32,
    pub stride: u32,
}

/// Where a captured frame's pixels live.
pub enum FramePayload {
    /// Tightly-packed CPU pixels in `format`, `width*height*bytes_per_pixel` (no row padding).
    Cpu(Vec<u8>),
    /// A pitched GPU buffer (BGRA-order, on the shared CUDA context) — the NVIDIA zero-copy path.
    /// The dmabuf has already been imported + copied into this owned device buffer.
    #[cfg(target_os = "linux")]
    Cuda(ss_zerocopy::DeviceBuffer),
    /// A raw DMA-BUF: packed RGB for the existing GPU CSC paths, or native NV12 from a producer
    /// such as gamescope. The encoder imports it without a host copy.
    #[cfg(target_os = "linux")]
    Dmabuf(DmabufFrame),
}

impl CapturedFrame {
    /// True if the frame's pixels are a GPU/CUDA buffer (the NVIDIA zero-copy path).
    pub fn is_cuda(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            matches!(self.payload, FramePayload::Cuda(_))
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    /// True if the frame is a raw dmabuf (the VAAPI zero-copy path).
    pub fn is_dmabuf(&self) -> bool {
        #[cfg(target_os = "linux")]
        {
            matches!(self.payload, FramePayload::Dmabuf(_))
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }
}
