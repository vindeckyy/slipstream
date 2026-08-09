//! Frame capture facade (plan §7 / §W6). The capturers themselves live in the `ss-capture`
//! subsystem crate; this host module is the thin BRIDGE that (a) re-exports the shared frame
//! vocabulary + the capturer types so every `crate::capture::*` path is unchanged, and (b) keeps
//! the orchestration entry points — [`open_portal_monitor`] / [`open_desktop_capture`] /
//! [`capture_virtual_output`] — which know about `crate::{vdisplay, session_plan, inject, encode}`
//! and hand ss-capture the pre-resolved facts it needs, so the capturer never reaches back into
//! the orchestrator.

use anyhow::{bail, Context, Result};

#[cfg(target_os = "linux")]
use crate::session_plan::CaptureBackend;

// The shared frame vocabulary lives in `ss-frame`; re-export the pieces host modules still name via
// `crate::capture::*` (the capture mechanics that used the rest moved into ss-capture).
pub use ss_frame::{CapturedFrame, OutputFormat, PixelFormat};
// The capturer types + trait + synthetics live in `ss-capture`; re-export them at the old paths.
// `capturer_supports_hdr` is deliberately NOT re-exported: on Linux it is only the platform floor,
// and a caller reaching for it by that name would silently miss the gamescope arm. The host's
// answer is [`capturer_supports_hdr_for`] below.
pub use ss_capture::{capturer_supports_444, Capturer, FastSyntheticCapturer, SyntheticCapturer};
/// Resolve the [`ss_capture::ZeroCopyPolicy`] for a Linux capture session from the encode backend —
/// the one reach into `crate::encode` the capturer must NOT make itself (it would recreate the
/// capture→encode cycle). Resolved here (the host facade) and threaded in, so the edge stays one-way
/// (plan §2.4 / §W6).
#[cfg(target_os = "linux")]
fn zero_copy_policy(
    pyrowave_session: bool,
    native_nv12_session: bool,
) -> ss_capture::ZeroCopyPolicy {
    let backend_is_vaapi = crate::encode::linux_zero_copy_is_vaapi();
    // The raw-dmabuf passthrough serves a PyroWave session on ANY vendor (the wavelet encoder's
    // own Vulkan device imports the dmabuf) — per-session from the negotiated codec, plus the
    // global `SLIPSTREAM_ENCODER=pyrowave` lab lever (which also flips `backend_is_vaapi`).
    #[cfg(feature = "pyrowave")]
    let pyrowave_session =
        pyrowave_session || ss_host_config::config().encoder_pref.as_str() == "pyrowave";
    #[cfg(not(feature = "pyrowave"))]
    let pyrowave_session = {
        let _ = pyrowave_session;
        false
    };
    #[cfg(feature = "pyrowave")]
    let pyrowave_modifiers = if pyrowave_session {
        // BGRx is the capture path's canonical packed-RGB format (the modifier advertisement keys
        // on it). `drm_fourcc(Bgrx)` is always `Some`.
        ss_frame::drm_fourcc(PixelFormat::Bgrx)
            .map(crate::encode::pyrowave_capture_modifiers)
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    #[cfg(not(feature = "pyrowave"))]
    let pyrowave_modifiers = Vec::new();
    ss_capture::ZeroCopyPolicy {
        backend_is_vaapi,
        backend_is_gpu: crate::encode::resolved_backend_is_gpu(),
        pyrowave_session,
        pyrowave_modifiers,
        native_nv12_session,
        // Only the direct-SDK NVENC backend takes a packed 10-bit PQ CUDA payload; without it an
        // HDR capture must stay on the CPU path (libav's HDR route swscales into a P010 hardware
        // frame). Resolved here, in the facade, like every other encode fact capture is told.
        hdr_cuda_ok: ss_encode::linux_hdr_cuda_ok(),
    }
}

/// Open a live capturer for a client-sized monitor via the xdg ScreenCast portal. `want_hdr`
/// offers the GNOME 50+ 10-bit PQ/BT.2020 formats (pass it only when the session negotiated HDR
/// AND the mirrored monitor is in HDR mode — see [`ss_capture::gnome_hdr_monitor_active`]).
/// `want_metadata_cursor` asks for cursor-as-metadata — pass it only when the session's encode
/// backend composites `CapturedFrame::cursor` (`encode::cursor_blend_capable`); otherwise the
/// portal embeds the pointer, so no backend × cursor-mode combination streams cursorless.
#[cfg(target_os = "linux")]
pub fn open_portal_monitor(
    want_hdr: bool,
    want_metadata_cursor: bool,
) -> Result<Box<dyn Capturer>> {
    // On RemoteDesktop-capable desktops (KWin/GNOME) anchor ScreenCast to a RemoteDesktop
    // session so it inherits that grant headlessly; wlroots/Sway has no RemoteDesktop portal,
    // so use a plain ScreenCast session there.
    let anchored = crate::inject::default_backend() == crate::inject::Backend::Libei;
    // Monitor mirrors never carry the native PyroWave plane (GameStream protocol) — per-session
    // passthrough is virtual-output-only; the global encoder-pref lever still applies inside.
    // Native NV12 stays off too: the mirror path doesn't resolve the codec here, and the desktop
    // compositors it mirrors (GNOME/KWin) don't produce NV12 anyway.
    ss_capture::open_portal_monitor(
        anchored,
        want_hdr,
        want_metadata_cursor,
        zero_copy_policy(false, false),
    )
}

/// Open a capturer for an *existing* desktop (GameStream mirror / `video_source=portal` path).
///
/// Honours `SLIPSTREAM_CAPTURE_METHOD` / [`CaptureBackend::resolve`]:
/// - `auto` — try the compositor-aware [`CaptureBackend::desktop_auto_order_for`] list, skipping
///   failures with `tracing::debug`
/// - `portal` / `kwin` — [`open_portal_monitor`] (KWin Phase 1 still goes through the portal)
/// - `x11` — [`ss_capture::open_x11_desktop`]
/// - `wlr` — [`ss_capture::open_wlr_desktop`]
/// - `kms` — DRM primary-plane dma-buf capture
/// - `nvfbc` — NVIDIA NvFBC shared-CUDA capture
#[cfg(target_os = "linux")]
pub fn open_desktop_capture(
    want_hdr: bool,
    want_metadata_cursor: bool,
) -> Result<Box<dyn Capturer>> {
    let compositor = crate::vdisplay::detect().ok();
    let pipeline = crate::session_plan::LinuxDisplayPipeline::for_desktop(compositor);
    tracing::info!(pipeline = %pipeline.label(), "resolved Linux desktop display pipeline");

    let mut errors = Vec::new();
    for backend in pipeline.candidates.iter().copied() {
        match open_desktop_backend(backend, want_hdr, want_metadata_cursor) {
            Ok(c) => {
                tracing::info!(
                    compositor = pipeline
                        .compositor
                        .map(|value| value.id())
                        .unwrap_or("unknown"),
                    backend = backend.as_str(),
                    "desktop capture pipeline selected"
                );
                return Ok(c);
            }
            Err(e) => {
                tracing::debug!(
                    compositor = pipeline.compositor.map(|value| value.id()).unwrap_or("unknown"),
                    backend = backend.as_str(),
                    error = %format!("{e:#}"),
                    "desktop capture pipeline candidate rejected"
                );
                errors.push(format!("{}: {e:#}", backend.as_str()));
                if pipeline.requested_capture.is_some() {
                    break;
                }
            }
        }
    }
    let preference = pipeline
        .requested_capture
        .map(|value| value.as_str())
        .unwrap_or("auto");
    bail!(
        "desktop capture {preference}: every compatible pipeline candidate failed ({})",
        errors.join("; ")
    )
}

/// Open one concrete desktop-mirror backend. Used by [`open_desktop_capture`] (and auto-order).
#[cfg(target_os = "linux")]
fn open_desktop_backend(
    backend: CaptureBackend,
    want_hdr: bool,
    want_metadata_cursor: bool,
) -> Result<Box<dyn Capturer>> {
    match backend {
        CaptureBackend::Portal | CaptureBackend::Kwin => {
            open_portal_monitor(want_hdr, want_metadata_cursor)
        }
        CaptureBackend::X11 => ss_capture::open_x11_desktop().context("open X11 desktop capturer"),
        CaptureBackend::Wlr => ss_capture::open_wlr_desktop().context("open wlr desktop capturer"),
        CaptureBackend::Kms => {
            ss_capture::open_kms_desktop_for_monitor(crate::vdisplay::capture_monitor().as_deref())
                .context("open KMS desktop capturer")
        }
        CaptureBackend::NvFbc => ss_capture::open_nvfbc_desktop_for_monitor(
            crate::vdisplay::capture_monitor().as_deref(),
        )
        .context("open NvFBC desktop capturer"),
        _ => bail!("capture backend is unavailable in this Linux build"),
    }
}

/// Build a capturer from an already-created virtual output ([`crate::vdisplay::VirtualOutput`]).
/// Explodes the output into the primitives ss-capture needs (so the capturer never depends on the
/// vdisplay type); the capturer takes the keepalive, so dropping it releases the output.
#[cfg(target_os = "linux")]
pub fn capture_virtual_output(
    vout: crate::vdisplay::VirtualOutput,
    want: OutputFormat,
    capture: crate::session_plan::CaptureBackend,
) -> Result<Box<dyn Capturer>> {
    // The virtual-output source is the compositor-created PipeWire node. Keep the resolved backend
    // in the call so the display and capture decision remains one pipeline record. The capture
    // method preference applies only to an existing desktop mirror, where there is no output we
    // created and no PipeWire node selected by the virtual-display backend.
    // `want.gpu` gates GPU zero-copy capture and `want.chroma_444` selects the worker's planar-YUV444
    // GPU convert. `gpu = false` (4:4:4 without zero-copy) forces the CPU mmap path so the encoder
    // gets CPU-resident RGB to swscale into YUV444P.
    //
    // `want.hdr` runs the 10-bit PQ/BT.2020 offer. It is only ever set for a gamescope output off
    // our `pipewire-hdr` build — every other Linux virtual output is SDR-only upstream — and the
    // handshake already resolved that through [`capturer_supports_hdr_for`] before the Welcome,
    // so passing it through here is the whole of this arm's HDR logic. It used to be dropped on
    // the floor, which is what kept the Linux native plane at 8 bits.
    let pipeline = crate::session_plan::LinuxDisplayPipeline::for_virtual_output(
        crate::vdisplay::detect().ok(),
        capture,
    );
    tracing::info!(
        pipeline = %pipeline.label(),
        zero_copy = want.gpu,
        hdr = want.hdr,
        "resolved Linux virtual-output display pipeline"
    );
    ss_capture::open_virtual_output(
        vout.remote_fd,
        vout.node_id,
        vout.preferred_mode,
        vout.keepalive,
        want.gpu,
        want.chroma_444,
        want.hdr,
        zero_copy_policy(want.pyrowave, want.nv12_native),
        vout.expect_exact_dims,
    )
}

/// Can the NATIVE-plane capture source this session will drive deliver a 10-bit PQ/BT.2020 frame?
/// The capture-side half of the slipstream/1 bit-depth gate (`native::handshake`), and the single
/// source-aware answer — `ss_capture::capturer_supports_hdr()` alone cannot answer it on Linux,
/// where it depends on which compositor is resolved and which gamescope binary is installed.
///
/// **Must be truthful, because the Welcome is irrevocable**: `bit_depth` is decided before the
/// display exists, and PQ frames handed to an 8-bit encoder are a deliberate hard error
/// (`ss-encode/src/enc/linux/mod.rs`). So every term here is a STATIC fact resolvable before the
/// spawn — never "spawn it and find out".
///
/// - **Linux + gamescope**: true when the host knob allows it, the resolved gamescope binary
///   offers 10-bit BT.2020/PQ capture formats (`packaging/gamescope`), the sub-mode is one we
///   SPAWN (an attach to a foreign gamescope tells us nothing about how it was started — §3.6
///   stretch), and no earlier virtual-output HDR negotiation on this host has latched a downgrade.
/// - **Linux, anything else**: false. Mutter/KWin/wlroots virtual outputs are 8-bit upstream. The
///   other Linux HDR path — the GNOME 50+ portal monitor mirror — belongs to the GameStream plane
///   and is gated by `gamestream::host_hdr_capable` + the live monitor colour-mode probe instead.
pub fn capturer_supports_hdr_for(compositor: Option<crate::vdisplay::Compositor>) -> bool {
    if compositor == Some(crate::vdisplay::Compositor::Gamescope) {
        return ss_host_config::config().gamescope_hdr
            && ss_vdisplay::gamescope_hdr_available()
            && !ss_capture::hdr_capture_failed(ss_capture::HdrSource::VirtualOutput);
    }
    ss_capture::capturer_supports_hdr()
}
