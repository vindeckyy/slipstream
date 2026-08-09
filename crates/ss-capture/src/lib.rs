//! Frame capture: the Linux xdg-ScreenCast and PipeWire portal capturer plus synthetic test
//! sources behind the `Capturer` trait. The crate uses `ss-frame` and `ss-zerocopy`; encode-backend
//! facts arrive pre-resolved in [`ZeroCopyPolicy`].

// Every unsafe block in this crate carries a `// SAFETY:` proof; enforce it (unsafe-proof program).
#![deny(clippy::undocumented_unsafe_blocks)]
// …and that program only covers a whole `unsafe fn` body once the body needs its own block: in
// edition 2021 `unsafe_op_in_unsafe_fn` is allow-by-default, which exempted the crate's hardest FFI
// (the ring/slot construction, the channel broker, every D3D converter ctor) from the deny above.
#![deny(unsafe_op_in_unsafe_fn)]

use anyhow::Result;
use ss_frame::{CapturedFrame, FramePayload, PixelFormat};

/// Host-local cursor hide flag (stream sessions): capture keeps publishing overlays for the
/// client while the OS cursor is suppressed on the host display.
pub mod host_cursor_flag;
// The Linux capturer reaches `DmabufFrame` through `super::`; `CursorOverlay` it names directly as
// `ss_frame::CursorOverlay`, so only `DmabufFrame` needs to sit in this crate root's scope.
#[cfg(target_os = "linux")]
use ss_frame::DmabufFrame;

/// Cheap, bounded capture-side telemetry. The hot path updates counters atomically and the host
/// samples this snapshot at its existing statistics boundary, so enabling the management capture
/// does not add a lock or allocation to frame delivery.
#[derive(Clone, Copy, Debug, Default)]
pub struct CaptureTelemetry {
    /// Wall-clock nanoseconds stamped immediately before the frame is published to the handoff
    /// slot. This is the capture-arrival anchor used by the host's existing frame-age calculation.
    pub last_frame_ns: u64,
    /// Frames published by the source since the capturer was opened.
    pub frames_published: u64,
    /// Published frames that replaced a frame the consumer had not taken yet.
    pub frames_overwritten: u64,
    /// Extra PipeWire buffers discarded while selecting the newest buffer in a callback.
    pub buffers_drained: u64,
    /// Negotiated frame width and height. Zero means negotiation has not completed.
    pub width: u32,
    pub height: u32,
    /// Negotiated modifier, or zero for linear/unknown.
    pub modifier: u64,
    /// Frames dropped because the implicit fence did not signal within the deadline budget
    /// (Phase 3 — the producer was still rendering; the frame was never read).
    pub fence_timeouts: u64,
    /// Whether this capture delivers GPU frames (zero-copy) and why not when it doesn't —
    /// the Phase-3 zero-copy diagnostic (never silently presenting a copy path as equivalent).
    pub zerocopy: bool,
    pub zerocopy_reason: &'static str,
}

/// Wall-clock nanoseconds used for the capture timestamp carried through the encoder and wire
/// timing probes. Capturers that cannot expose a producer timestamp still get a consistent host
/// arrival anchor at the moment their frame is materialized.
pub(crate) fn capture_now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0)
}

/// Produces frames from a captured output. Lives on its own thread, handing frames over without
/// ever blocking the compositor — the Linux portal publishes into a one-deep OVERWRITING slot
/// (drop-oldest), so a stalled consumer costs the intermediate frames and is still handed the
/// freshest one.
pub trait Capturer: Send {
    // ---- Frames -----------------------------------------------------------------------------
    // `next_frame` blocks for one; `try_latest` is the steady-state non-blocking read;
    // `wait_arrival` + `supports_arrival_wait` are the frame-driven trigger that replaces a
    // free-running tick.

    fn next_frame(&mut self) -> Result<CapturedFrame>;

    /// [`next_frame`](Self::next_frame) with a caller-chosen first-frame budget instead of the
    /// backend's default. The pipeline retry loop shortens its FIRST attempt's wait: a PipeWire
    /// stream connected while gamescope re-inits its headless takeover can negotiate a format,
    /// reach `Streaming`, and still never receive a buffer — a fresh connect then delivers within
    /// ~0.5 s, so waiting out the full default budget on a doomed stream just delays the retry
    /// that fixes it. Backends without an internal wait budget ignore it (the default delegates).
    fn next_frame_within(&mut self, _budget: std::time::Duration) -> Result<CapturedFrame> {
        self.next_frame()
    }

    /// Non-blocking: the freshest frame available since the last call, or `None` if none has
    /// arrived (the caller reuses its last frame to hold a steady output rate). The default
    /// just produces a frame each call — fine for instant synthetic sources; the portal
    /// overrides it to drain its channel without blocking.
    fn try_latest(&mut self) -> Result<Option<CapturedFrame>> {
        self.next_frame().map(Some)
    }

    /// Whether this backend can block until a frame ARRIVES ([`wait_arrival`]
    /// (Self::wait_arrival)) — the frame-driven encode trigger (latency plan T1.1). `false`
    /// (the default) keeps the encode loop on its legacy fixed-cadence tick for this backend.
    fn supports_arrival_wait(&self) -> bool {
        false
    }

    /// Block until a FRESH frame is available via [`try_latest`](Self::try_latest) or
    /// `deadline` passes — the encode loop's frame-driven wait (latency plan T1.1): waking on
    /// the compositor's publish instead of sampling at a free-running tick deletes the
    /// sample-and-hold (~half a frame interval on average). Must NOT consume the frame — the
    /// loop's `try_latest` call does that — so a backend implements this by waiting on a wakeup
    /// and then PEEKING its hand-off slot. Only called when
    /// [`supports_arrival_wait`](Self::supports_arrival_wait) is `true`; errors surface at the
    /// following `try_latest`.
    fn wait_arrival(&mut self, _deadline: std::time::Instant) {}

    // ---- Lifecycle --------------------------------------------------------------------------
    // Whether the capturer is being used right now, and whether it can still be used at all.

    /// Gate expensive per-frame work so the capturer can be kept alive (reused) between
    /// streams without burning CPU. The portal capturer skips the de-pad copy while inactive and
    /// flushes its frame mailbox on `false`; the default is a no-op (synthetic sources are produced
    /// on demand). Set `true` for the duration of a stream, `false` when it ends.
    ///
    /// `&mut self`: it mutates capturer state, and every caller owns the capturer. It took `&self`
    /// only because the flag happened to be an `Arc<AtomicBool>` — an implementation detail leaking
    /// into the contract, and one the mailbox flush this now also does would not have shared.
    fn set_active(&mut self, _active: bool) {}

    /// Whether this capturer can still produce frames — the gate a caller that POOLS capturers
    /// across streams must consult before reusing one.
    ///
    /// Some backends have TERMINAL states that are only observable by trying to consume a frame:
    /// the Linux portal capturer's zero-copy poison flag, a dead PipeWire thread, and a source that
    /// never returns to `Streaming` are all sticky, and each makes every subsequent
    /// [`next_frame`](Self::next_frame) / [`try_latest`](Self::try_latest) fail — for that backend
    /// an `Err` from either is terminal, never transient. A pool that re-admits such a capturer
    /// wedges the next session permanently (it re-fails at the same point, every reconnect), which
    /// is why this predicate exists rather than leaving callers to infer liveness from an error they
    /// have often already discarded.
    ///
    /// `true` (the default) for backends with no terminal state, including synthetic sources.
    fn is_alive(&self) -> bool {
        true
    }

    /// Stable backend label used by pipeline diagnostics and stats. It is intentionally a static
    /// string so sampling it never allocates.
    fn backend_name(&self) -> &'static str {
        "unknown"
    }

    /// Return a point-in-time capture telemetry snapshot. Backends without instrumentation keep
    /// the default zero-valued snapshot.
    fn telemetry(&self) -> CaptureTelemetry {
        CaptureTelemetry::default()
    }

    // ---- Cursor -----------------------------------------------------------------------------
    // The out-of-band pointer: where it is, who draws it, and (Linux/gamescope) where to read it.

    /// The capture source's LIVE cursor state, when it arrives out-of-band from the frames
    /// Default `None`: the Linux portal path attaches its cursor to frames instead.
    fn cursor(&mut self) -> Option<ss_frame::CursorOverlay> {
        None
    }

    /// LIVE cursor-render flip for a cursor-forward session (design/remote-desktop-sweep.md §8):
    /// `on = true` asks the client to draw the pointer outside the video; `on = false` keeps it in
    /// the video. The Linux portal keeps the cursor metadata separate and the encode loop blends
    /// it when needed.
    fn set_cursor_forward(&mut self, _on: bool) {}

    /// Attach a gamescope cursor source (remote-desktop-sweep Phase C). gamescope paints no
    /// `SPA_META_Cursor`, so [`cursor`](Self::cursor)'s slot stays empty — this hands the Linux
    /// portal capturer a way to reach gamescope's nested Xwaylands (it may run several — one per
    /// `--xwayland-count`) so it reads the pointer shape/position over X11 (XFixes +
    /// QueryPointer), following whichever display is focused, and publishes it into that same slot.
    /// Called once, after the capturer is built, only for gamescope sessions. Default no-op: every
    /// non-gamescope capturer already has a cursor source.
    #[cfg(target_os = "linux")]
    fn attach_gamescope_cursor(&mut self, _targets: GamescopeCursorTargets) {}

    // ---- Stream properties ------------------------------------------------------------------

    /// The source's static HDR mastering metadata (SMPTE ST.2086 + content light level), when the
    /// capturer can read it from the output, or a generic HDR10 block once an HDR stream is
    /// negotiated (Linux, where neither the portal nor gamescope exposes a
    /// real mastering volume). `None` = unknown / SDR / a backend that doesn't expose it.
    /// The stream loop forwards this to the encoder (in-band SEI) and the client (`0xCE` datagram),
    /// so the two stay a single source of truth. May change mid-session if the source is regraded.
    fn hdr_meta(&self) -> Option<slipstream_core::quic::HdrMeta> {
        None
    }

    /// How many frames the encode loop may keep in flight (submitted but not yet polled) before it
    /// blocks. `1` (the default) is the synchronous loop: capture → submit → poll-blocks, so the
    /// per-frame wall time is `capture+convert + encode`. A capturer that hands a fresh output texture
    /// per frame (so the encode of N reads a different texture than the convert of N+1 writes) can return
    /// `>1` to PIPELINE: the loop submits N+1 before polling N, overlapping the convert/copy on the 3D
    /// engine with the NVENC-ASIC encode of the prior frame, dropping per-frame wall toward `max(...)`.
    fn pipeline_depth(&self) -> usize {
        1
    }

    // ---- Host-initiated resize --------------------------------------------------------------
    // These two are ONE operation split in half and must be implemented together: a backend that
    // returns `Some` from `capture_target_id` is promising `resize_output` works, and one that
    // implements `resize_output` without the identity leaves the caller no way to check that the
    // display it just reconfigured is still this capturer's. Both defaults decline.

    /// The OS display-target id this capturer is bound to. `None` means the backend has no such
    /// identity.
    ///
    /// PAIRED with [`resize_output`](Self::resize_output) — see the cluster note above.
    fn capture_target_id(&self) -> Option<u32> {
        None
    }

    /// HOST-INITIATED output resize (latency plan P2.3): the session's resize handler has ALREADY
    /// committed the display's new mode (the manager's in-place mode set), so a capable capturer
    /// re-sizes its capture surface NOW — no descriptor-poll debounce (that machinery stays, for
    /// EXTERNAL changes only) and no teardown: the capture pipeline and its send thread survive;
    /// only the encoder is swapped by the caller once the first new-size frame arrives. Returns
    /// `true` when handled; `false` (the default) routes the caller to the full-rebuild path.
    ///
    /// PAIRED with [`capture_target_id`](Self::capture_target_id) — see the cluster note above.
    fn resize_output(&mut self, _width: u32, _height: u32) -> bool {
        false
    }

    /// Recreate the delivery ring at the CURRENT mode and re-run the driver attach handshake —
    /// the recovery half of a swap-chain bounce the descriptor poller cannot see: an
    /// exclusive-topology eviction (the vdisplay re-assert watchdog) is a real topology change,
    /// so the OS drives COMMIT_MODES on the live virtual display too and the driver's swap-chain
    /// is recreated while this capturer keeps waiting on the old ring attachment — frames stop
    /// with an unchanged descriptor (same mode, same HDR), so the two-strike debounce never
    /// trips. Arms the same recover-or-drop window as a real resize, so a driver that cannot
    /// re-attach still fails the session cleanly. Returns `true` when handled; `false` (the
    /// default) means the backend has no in-place ring recovery and the caller should treat the
    /// pipeline as unrecoverable in place.
    fn recreate_ring_in_place(&mut self) -> bool {
        false
    }
}

/// A deterministic moving test pattern (BGRx). Lets the spike exercise the encode → file →
/// `slipstream_core` path with no live capture session, and produces obviously non-static
/// content (a sweeping bar + animated gradient) so the encoded output is verifiable.
pub struct SyntheticCapturer {
    width: u32,
    height: u32,
    fps: u32,
    frame_idx: u64,
    buf: Vec<u8>,
}

impl SyntheticCapturer {
    const BPP: usize = 4; // emits BGRx

    pub fn new(width: u32, height: u32, fps: u32) -> Self {
        assert!(width > 0 && height > 0 && fps > 0);
        let buf = vec![0u8; width as usize * height as usize * Self::BPP];
        SyntheticCapturer {
            width,
            height,
            fps,
            frame_idx: 0,
            buf,
        }
    }
}

impl Capturer for SyntheticCapturer {
    fn backend_name(&self) -> &'static str {
        "synthetic"
    }

    fn next_frame(&mut self) -> Result<CapturedFrame> {
        let w = self.width as usize;
        let h = self.height as usize;
        let bpp = Self::BPP;
        let t = self.frame_idx;
        // A vertical bar sweeps left→right once every ~2s; the background is a gradient
        // whose phase advances each frame, so every pixel changes frame-to-frame.
        let bar_x = ((t * w as u64) / (self.fps as u64 * 2)) % w as u64;
        let phase = (t % 256) as usize;
        for y in 0..h {
            let row = y * w * bpp;
            for x in 0..w {
                let i = row + x * bpp;
                let on_bar = (x as u64).abs_diff(bar_x) < 8;
                // BGRx byte order: [B, G, R, x]
                self.buf[i] = if on_bar {
                    255
                } else {
                    ((x + phase) & 0xff) as u8
                };
                self.buf[i + 1] = if on_bar {
                    255
                } else {
                    ((y + phase) & 0xff) as u8
                };
                self.buf[i + 2] = if on_bar { 255 } else { ((x + y) & 0xff) as u8 };
                self.buf[i + 3] = 0;
            }
        }
        let pts_ns = self.frame_idx * 1_000_000_000 / self.fps as u64;
        self.frame_idx += 1;
        Ok(CapturedFrame {
            width: self.width,
            height: self.height,
            pts_ns,
            format: PixelFormat::Bgrx,
            payload: FramePayload::Cpu(self.buf.clone()),
            cursor: None,
            stage_ns: ss_frame::CaptureStageTimes::default(),
        })
    }
}

/// A cheap moving test pattern (BGRx) for the streaming path: a pulsing field + a white band
/// sweeping down, generated with whole-buffer `fill`s so it stays real-time even at 5K.
pub struct FastSyntheticCapturer {
    width: u32,
    height: u32,
    frame_idx: u64,
    buf: Vec<u8>,
    /// SLIPSTREAM_SYNTH_NOISE: every frame is fresh high-entropy noise NVENC can't compress or
    /// predict, so the encoder hits its (CBR) bitrate target — a throughput test of the real
    /// encode→FEC→send→recv path. The default flat/band content compresses to ~nothing, so it
    /// can't generate real Mbps (the encoder is content-driven). xorshift over u64 chunks.
    noise: bool,
    rng: u64,
}

impl FastSyntheticCapturer {
    pub fn new(width: u32, height: u32) -> Self {
        assert!(width > 0 && height > 0);
        FastSyntheticCapturer {
            width,
            height,
            frame_idx: 0,
            buf: vec![0u8; width as usize * height as usize * 4],
            noise: std::env::var_os("SLIPSTREAM_SYNTH_NOISE").is_some(),
            rng: 0x9e3779b97f4a7c15,
        }
    }
}

impl Capturer for FastSyntheticCapturer {
    fn backend_name(&self) -> &'static str {
        "synthetic-fast"
    }

    fn next_frame(&mut self) -> Result<CapturedFrame> {
        if self.noise {
            // Fresh, every-frame-decorrelated noise: reseed from the frame index so consecutive
            // frames share no structure (forces large P-frames too, not just the keyframe).
            let mut s = self
                .rng
                .wrapping_add(self.frame_idx.wrapping_mul(0x2545F491_4F6CDD1D))
                | 1;
            for c in self.buf.chunks_exact_mut(8) {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                c.copy_from_slice(&s.to_le_bytes());
            }
            self.rng = s;
        } else {
            let (w, h) = (self.width as usize, self.height as usize);
            let row = w * 4;
            let shade = (self.frame_idx % 256) as u8;
            self.buf.fill(shade);
            let band_h = (h / 20).max(1);
            let band_y = (self.frame_idx as usize * 6) % h;
            for y in band_y..(band_y + band_h).min(h) {
                self.buf[y * row..(y + 1) * row].fill(0xff);
            }
        }
        self.frame_idx += 1;
        Ok(CapturedFrame {
            width: self.width,
            height: self.height,
            pts_ns: 0,
            format: PixelFormat::Bgrx,
            payload: FramePayload::Cpu(self.buf.clone()),
            cursor: None,
            stage_ns: ss_frame::CaptureStageTimes::default(),
        })
    }
}

/// The encode-backend facts the Linux zero-copy negotiation needs, resolved **once** here (the host
/// facade, which may reach the host `encode`) and passed **into** the capturer — so the capturer never
/// calls back into `encode`, keeping the capture→encode dependency one-way (plan §2.4 / §W6). The
/// three facts were formerly re-derived inside the PipeWire thread via
/// `encode::{linux_zero_copy_is_vaapi, resolved_backend_is_gpu, pyrowave_capture_modifiers}`.
#[cfg(target_os = "linux")]
#[derive(Clone, Default)]
pub struct ZeroCopyPolicy {
    /// The GPU encode backend resolves to VAAPI (AMD/Intel) — the capturer hands raw dmabufs
    /// straight through instead of the EGL→CUDA import (the host `encode::linux_zero_copy_is_vaapi`).
    pub backend_is_vaapi: bool,
    /// The resolved backend produces GPU-resident frames (everything but the software encoder) —
    /// used only to phrase the CPU-fallback warning (the host `encode::resolved_backend_is_gpu`).
    pub backend_is_gpu: bool,
    /// THIS session encodes PyroWave: the frames' consumer is the wavelet encoder's own Vulkan
    /// device, which imports raw dmabufs on ANY vendor — so the capturer takes the raw-dmabuf
    /// passthrough (like the VAAPI backend) instead of the EGL→CUDA import whose payloads only
    /// NVENC can consume. Per-session (the codec is negotiated), unlike `backend_is_vaapi`.
    pub pyrowave_session: bool,
    /// THIS session's encoder can ingest a producer-native NV12 capture (the Linux raw Vulkan
    /// Video backend on an H265/AV1 session — resolved by the host facade via
    /// `ss_encode::linux_native_nv12_ok`). Gates whether the negotiation PREFERS gamescope's
    /// producer-side NV12 pod: libav VAAPI (H264's backend) would misread the two-plane buffer,
    /// so H264/GameStream/PyroWave sessions must never see NV12 frames.
    pub native_nv12_session: bool,
    /// The PyroWave encoder's Vulkan-importable dmabuf modifiers for the capture's packed-RGB fourcc,
    /// resolved when the session encodes PyroWave (the passthrough advertises them so Mutter+NVIDIA,
    /// which allocates tiled-only, still negotiates zero-copy). Empty otherwise.
    pub pyrowave_modifiers: Vec<u64>,
    /// The resolved encoder can ingest a packed 10-bit PQ CUDA payload (`ss_encode::linux_hdr_cuda_ok`
    /// — direct-SDK NVENC only). An HDR capture builds the GPU importer ONLY when this holds:
    /// libav's HDR route wants a P010 hardware frame it swscales into, so a packed-2:10:10:10 CUDA
    /// buffer would land in a P010 surface as garbage. `false` ⇒ HDR takes the CPU path, exactly as
    /// it did before the direct backend learned 10-bit.
    pub hdr_cuda_ok: bool,
}

/// Discovers gamescope's nested Xwayland cursor targets — `(DISPLAY, XAUTHORITY)`, one per
/// `--xwayland-count` — for [`Capturer::attach_gamescope_cursor`].
///
/// A CLOSURE, not the `Vec` it used to be, and re-run on a slow cadence by the cursor worker. The
/// snapshot was taken once, before the game launched: gamescope creates a second Xwayland for the
/// game but only advertises the FIRST in any child's environ, so the game's display was invisible to
/// discovery — and when the connected (Big Picture) display then reported "gamescope is not drawing
/// the pointer here", the source blanked the cursor for the whole game session, which is the exact
/// regression the module doc says it fixed. A provider also lets the worker retry a display that
/// died, and lets a stream that starts BEFORE the game converge instead of staying cursorless.
///
/// Built by the host facade (it wraps `ss_vdisplay::gamescope_xwayland_cursor_targets`), exactly
/// like [`FrameChannelSender`] — so the capture→host edge stays one-way.
#[cfg(target_os = "linux")]
pub type GamescopeCursorTargets =
    std::sync::Arc<dyn Fn() -> Vec<(String, Option<String>)> + Send + Sync>;

#[cfg(target_os = "linux")]
pub fn capturer_supports_444(_encoder_ingests_rgb_444: bool) -> bool {
    true
}

/// Whether the **native-plane** capturer (a compositor virtual output) can deliver an HDR (10-bit
/// PQ/BT.2020) source **on this platform alone**, without knowing which compositor will be
/// driven — the platform half of the gate the slipstream/1 handshake consults before negotiating
/// 10-bit (mirroring [`capturer_supports_444`]).
///
/// Linux: `false`, and this is NOT the whole Linux answer any more. It says only that no Linux
/// virtual output is HDR-capable *by platform*: Mutter's `RecordVirtual` virtual-monitor streams
/// advertise 8-bit BGRx/BGRA exclusively (still true on the GNOME 51 dev branch) and report no
/// BT2020/PQ colour capabilities, and KWin/wlroots virtual outputs are the same. The one Linux
/// virtual output that CAN be 10-bit — gamescope's PipeWire node, with our carried
/// `pipewire-hdr` patch (`packaging/gamescope`) — depends on the resolved compositor **and** the
/// resolved gamescope binary, neither of which this crate knows. The host resolves it in
/// `capture::capturer_supports_hdr_for(compositor)`, which consults this for the platform floor;
/// the other Linux HDR path (the GNOME 50+ portal **monitor mirror**, `open_portal_monitor` with
/// `want_hdr`) is gated separately by the GameStream plane (`host_hdr_capable` + the live monitor
/// colour-mode probe).
#[cfg(target_os = "linux")]
pub fn capturer_supports_hdr() -> bool {
    false
}

/// Which HDR capture source a `want_hdr` negotiation failure belongs to.
///
/// The failure latch below is **per source**, because the two Linux HDR sources fail for
/// completely unrelated reasons and share nothing but the word "HDR": the portal monitor mirror
/// fails when the mirrored monitor leaves HDR mode (a live, box-state fact), a gamescope virtual
/// output fails when the spawned binary has no 10-bit formats (a static, binary-identity fact).
/// A single process-wide latch let either one disable the other until the host restarted.
#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HdrSource {
    /// The GNOME 50+ portal **monitor mirror** (`open_portal_monitor` with `want_hdr`) — the
    /// GameStream plane's HDR path.
    PortalMonitor,
    /// A compositor **virtual output** (`open_virtual_output` with `want_hdr`) — today only
    /// gamescope's PipeWire node, with the carried `pipewire-hdr` patch.
    VirtualOutput,
}

/// Per-source latch: a `want_hdr` capture failed to negotiate the HDR (10-bit PQ) offer — the
/// producer never accepted it (monitor left HDR mode between the probe and the negotiation,
/// NVIDIA EGL not listing LINEAR for XR30, an unpatched gamescope…). Later sessions **on that
/// same source** consult [`hdr_capture_failed`] and fall back to the SDR offer instead of
/// re-running the same doomed 10-second negotiation timeout on every reconnect. Sticky until host
/// restart (matching the zero-copy downgrade latches); the log line at latch time says so.
/// Indexed by [`HdrSource`] — see its doc for why one shared latch was wrong.
#[cfg(target_os = "linux")]
static HDR_CAPTURE_FAILED: [std::sync::atomic::AtomicBool; 2] = [
    std::sync::atomic::AtomicBool::new(false),
    std::sync::atomic::AtomicBool::new(false),
];

#[cfg(target_os = "linux")]
impl HdrSource {
    fn slot(self) -> usize {
        match self {
            HdrSource::PortalMonitor => 0,
            HdrSource::VirtualOutput => 1,
        }
    }
}

#[cfg(target_os = "linux")]
pub fn hdr_capture_failed(source: HdrSource) -> bool {
    HDR_CAPTURE_FAILED[source.slot()].load(std::sync::atomic::Ordering::Relaxed)
}

#[cfg(target_os = "linux")]
pub(crate) fn note_hdr_capture_failed(source: HdrSource) {
    if !HDR_CAPTURE_FAILED[source.slot()].swap(true, std::sync::atomic::Ordering::Relaxed) {
        match source {
            HdrSource::PortalMonitor => tracing::warn!(
                "HDR capture negotiation failed on the monitor mirror — this host will offer SDR \
                 for that source for the rest of the process lifetime (restart the host after \
                 fixing the monitor's HDR mode to retry)"
            ),
            HdrSource::VirtualOutput => tracing::warn!(
                "HDR capture negotiation failed on the virtual output — this host will offer SDR \
                 for that source for the rest of the process lifetime (is the spawned gamescope \
                 the slipstream build? see packaging/gamescope)"
            ),
        }
    }
}
// The Linux platform backend lives under `platform/linux/`; crate-root shims keep the public
// capture paths stable.
#[cfg(target_os = "linux")]
mod platform;

// One-time PipeWire library init, shared by the video (portal) and audio capture threads.
#[cfg(target_os = "linux")]
pub use platform::linux::pwinit;

// The GNOME BT.2100 colour-mode probe — the host's capture-side gate for offering HDR on the
// portal monitor path (see `open_portal_monitor`'s `want_hdr`).
#[cfg(target_os = "linux")]
pub use platform::linux::gnome_hdr_monitor_active;

/// Open the Linux xdg-ScreenCast portal capturer for a client-sized monitor. `anchored` drives
/// ScreenCast off a RemoteDesktop session (KWin/GNOME) so it inherits that grant headlessly.
/// `want_hdr` offers the GNOME 50+ HDR formats (10-bit PQ/BT.2020 dmabufs) instead of the SDR
/// set — pass it only when the mirrored monitor is actually in HDR mode (the host probes
/// DisplayConfig) or the negotiation runs into its 10 s timeout and latches the SDR downgrade.
/// `want_metadata_cursor` asks for cursor-as-metadata (`SPA_META_Cursor`) — pass it only when
/// the session's encode path composites `CapturedFrame::cursor` (the host consults
/// `ss-encode`'s `cursor_blend_capable`); otherwise the portal EMBEDS the pointer so it is
/// never silently lost. The [`ZeroCopyPolicy`] carries the pre-resolved encode-backend facts
/// (the one-way edge).
#[cfg(target_os = "linux")]
pub fn open_portal_monitor(
    anchored: bool,
    want_hdr: bool,
    want_metadata_cursor: bool,
    policy: ZeroCopyPolicy,
) -> Result<Box<dyn Capturer>> {
    platform::linux::PortalCapturer::open(
        anchored,
        want_hdr && !hdr_capture_failed(HdrSource::PortalMonitor),
        want_metadata_cursor,
        policy,
    )
    .map(|c| Box::new(c) as Box<dyn Capturer>)
}

/// Open the Linux portal capturer bound to an already-created virtual output's PipeWire node. The
/// caller (host facade) explodes its `VirtualOutput` into these primitives + owns nothing after —
/// the capturer takes `keepalive`, so dropping it releases the output. `allow_zerocopy` mirrors
/// `OutputFormat::gpu`; `want_444` selects the planar-YUV444 GPU convert. `want_hdr` offers the
/// 10-bit PQ/BT.2020 formats instead of the SDR set — pass it only when the output was actually
/// brought up HDR (a gamescope spawned with `--hdr-enabled` off our `pipewire-hdr` build); the
/// host resolves that in `capture::capturer_supports_hdr_for` **before** the Welcome, because a
/// session that negotiated PQ cannot fall back to SDR afterwards.
#[cfg(target_os = "linux")]
#[allow(clippy::too_many_arguments)]
pub fn open_virtual_output(
    remote_fd: Option<std::os::fd::OwnedFd>,
    node_id: u32,
    preferred_mode: Option<(u32, u32, u32)>,
    keepalive: Box<dyn Send>,
    allow_zerocopy: bool,
    want_444: bool,
    want_hdr: bool,
    policy: ZeroCopyPolicy,
    expect_exact_dims: bool,
) -> Result<Box<dyn Capturer>> {
    platform::linux::PortalCapturer::from_virtual_output(
        remote_fd,
        node_id,
        preferred_mode,
        keepalive,
        allow_zerocopy,
        want_444,
        want_hdr && !hdr_capture_failed(HdrSource::VirtualOutput),
        policy,
        expect_exact_dims,
    )
    .map(|c| Box::new(c) as Box<dyn Capturer>)
}

/// Open the plain-X11 desktop capturer: `GetImage` on the root window, cropped to the primary
/// RandR output's CRTC when the server reports one. CPU frames only (packed BGRx, or BGRA on a
/// depth-32 root) — no portal, no PipeWire, no zero-copy.
///
/// This is the LAST-RESORT source, for a bare Xorg session with neither an xdg-ScreenCast portal
/// nor a compositor virtual output. Every frame copies the whole framebuffer through the X socket
/// (~8 MB at 1080p, ~33 MB at 4K), so prefer [`open_portal_monitor`] / [`open_virtual_output`]
/// wherever they work. Fails at open (with the layout it found) on a server whose root is not the
/// ordinary LSB-first 32-bpp TrueColor case.
#[cfg(target_os = "linux")]
pub fn open_x11_desktop() -> Result<Box<dyn Capturer>> {
    platform::linux::X11Capturer::open().map(|c| Box::new(c) as Box<dyn Capturer>)
}

/// Open a DRM/KMS primary-plane desktop capturer. The source exports the active packed-RGB
/// framebuffer as a dma-buf for the Linux encoder import path, so this path has no host readback.
#[cfg(target_os = "linux")]
pub fn open_kms_desktop() -> Result<Box<dyn Capturer>> {
    platform::linux::kms::open_kms_desktop()
}

/// Open KMS while honoring the host's effective physical-monitor pin.
#[cfg(target_os = "linux")]
pub fn open_kms_desktop_for_monitor(monitor: Option<&str>) -> Result<Box<dyn Capturer>> {
    platform::linux::kms::open_kms_desktop_for_monitor(monitor)
}

/// Open an NVIDIA NvFBC desktop capturer. NvFBC and CUDA are loaded at runtime, so an unavailable
/// NVIDIA driver returns an ordinary backend error and the compositor-aware pipeline can retry its
/// next candidate.
#[cfg(target_os = "linux")]
pub fn open_nvfbc_desktop() -> Result<Box<dyn Capturer>> {
    platform::linux::nvfbc::open_nvfbc_desktop()
}

/// Open NvFBC while honoring the host's effective physical-monitor pin.
#[cfg(target_os = "linux")]
pub fn open_nvfbc_desktop_for_monitor(monitor: Option<&str>) -> Result<Box<dyn Capturer>> {
    platform::linux::nvfbc::open_nvfbc_desktop_for_monitor(monitor)
}

/// True when an accessible DRM card exposes an active universal primary plane.
#[cfg(target_os = "linux")]
pub fn probe_kms() -> bool {
    platform::linux::kms::probe_kms()
}

/// True when KMS can open and Prime-export the selected monitor's active primary plane.
#[cfg(target_os = "linux")]
pub fn probe_kms_for_monitor(monitor: Option<&str>) -> bool {
    platform::linux::kms::probe_kms_for_monitor(monitor)
}

/// True when the NVIDIA NvFBC library, CUDA path, X display, and driver status all allow capture.
#[cfg(target_os = "linux")]
pub fn probe_nvfbc() -> bool {
    platform::linux::nvfbc::probe_nvfbc()
}

/// True when NvFBC and CUDA can capture the selected physical monitor.
#[cfg(target_os = "linux")]
pub fn probe_nvfbc_for_monitor(monitor: Option<&str>) -> bool {
    platform::linux::nvfbc::probe_nvfbc_for_monitor(monitor)
}

/// Open the wlroots `zwlr_screencopy_manager_v1` desktop capturer (SHM path). CPU frames only
/// (packed BGRx / BGRA from `wl_shm` Xrgb8888 / Argb8888) — no portal, no PipeWire, no dmabuf.
///
/// Needs `WAYLAND_DISPLAY` and a compositor that advertises screencopy (Sway, River, Hyprland).
/// Fails clearly when either is missing. Captures the first `wl_output` with a current mode and
/// composites the cursor into each frame.
#[cfg(target_os = "linux")]
pub fn open_wlr_desktop() -> Result<Box<dyn Capturer>> {
    platform::linux::WlrCapturer::open().map(|c| Box::new(c) as Box<dyn Capturer>)
}
