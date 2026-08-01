//! X11 desktop capture: `GetImage` on the root window (or the primary RandR output's CRTC
//! region), handed over as packed BGRx/BGRA CPU frames.
//!
//! This is the fallback source for a plain Xorg session, where the two real Linux paths are both
//! unavailable: there is no xdg-ScreenCast portal to talk to, and no compositor virtual output to
//! bind a PipeWire node to. It is deliberately the simple thing — one synchronous `GetImage` per
//! frame over the pure-Rust [`RustConnection`], no MIT-SHM segment, no damage tracking — because
//! the paths that matter for throughput (portal + PipeWire, dmabuf zero-copy) already exist and
//! this one exists so a bare X session streams at all.
//!
//! Cost, so nobody is surprised: `GetImage` copies the whole framebuffer through the X socket
//! every frame (~8 MB at 1080p, ~33 MB at 4K), which is a memcpy the compositor paths avoid
//! entirely. It is fine at 1080p60 on a local socket and gets expensive above that.
//!
//! **Pixel layout.** X hands back Z-format rows for the root's depth. We accept only the ordinary
//! Xorg TrueColor case — LSB-first byte order, 32 bits per pixel, `0xff0000/0xff00/0xff` RGB
//! masks — because that is exactly the `[B,G,R,x]` byte order [`PixelFormat::Bgrx`] already
//! names, so the reply buffer becomes the frame payload with no repack. Anything else fails at
//! [`X11Capturer::open`] with the layout it found rather than emitting scrambled pixels.
//!
//! **Cursor.** `GetImage` does not include the pointer (it is composited by the server, not part
//! of the root's contents), and this capturer publishes no [`ss_frame::CursorOverlay`] yet, so an
//! X11-captured stream is cursorless unless the client draws its own. The XFixes read that would
//! fill that gap already exists next door in [`super::xfixes_cursor`], but it is wired to the
//! gamescope target provider; pointing it at an ordinary display is the obvious follow-up.

use anyhow::{anyhow, Context, Result};
use std::time::Instant;
use x11rb::connection::Connection;
use x11rb::errors::ReplyError;
use x11rb::protocol::randr::ConnectionExt as _;
use x11rb::protocol::xproto::{ConnectionExt as _, ImageFormat, ImageOrder, Screen, Setup, Window};
use x11rb::rust_connection::RustConnection;

use super::{CapturedFrame, Capturer, FramePayload, PixelFormat};

/// The RandR version whose `GetOutputPrimary` we rely on (1.3). Asked for, not required: a server
/// without the extension just loses the primary-output crop and captures the whole root.
const RANDR_VERSION: (u32, u32) = (1, 3);

/// All planes — the reply's 4th byte is then the visual's unused/alpha byte, which both
/// [`PixelFormat::Bgrx`] (ignored) and [`PixelFormat::Bgra`] (honoured) already describe.
const ALL_PLANES: u32 = !0;

/// A capture region in root-window coordinates, resolved once at [`X11Capturer::open`].
#[derive(Clone, Copy)]
struct Region {
    x: i16,
    y: i16,
    width: u16,
    height: u16,
}

/// Root-window capturer for a plain X11 session. See the module docs for the cost and the
/// (single) accepted pixel layout.
pub struct X11Capturer {
    conn: RustConnection,
    root: Window,
    region: Region,
    format: PixelFormat,
    /// Frame timestamps are relative to the capturer's own start, like every other source here.
    start: Instant,
    /// Gated by [`Capturer::set_active`]: a pooled capturer between streams must not keep pulling
    /// whole framebuffers through the X socket.
    active: bool,
    /// Sticky: a failed `GetImage` on a fixed region means the connection died or the screen was
    /// reconfigured underneath us. Neither recovers in place — the region was resolved at open —
    /// so the capturer reports itself dead and the caller rebuilds against the new geometry.
    dead: bool,
}

impl X11Capturer {
    /// Connect to `$DISPLAY` and bind to the primary RandR output's CRTC region, falling back to
    /// the whole root window when the server has no RandR, no primary output, or the primary is
    /// not driving a CRTC.
    pub fn open() -> Result<Self> {
        let (conn, screen_num) =
            RustConnection::connect(None).context("connect to the X server ($DISPLAY)")?;
        let screen = conn
            .setup()
            .roots
            .get(screen_num)
            .ok_or_else(|| anyhow!("the X server reported no screen {screen_num}"))?
            .clone();
        let format = probe_format(conn.setup(), &screen)?;
        let root = screen.root;
        let full = Region {
            x: 0,
            y: 0,
            width: screen.width_in_pixels,
            height: screen.height_in_pixels,
        };
        let region = primary_region(&conn, root).unwrap_or(full);
        tracing::info!(
            display = ?std::env::var("DISPLAY").ok(),
            width = region.width,
            height = region.height,
            x = region.x,
            y = region.y,
            ?format,
            whole_root = region.width == full.width && region.height == full.height,
            "X11 capture: GetImage source open"
        );
        Ok(X11Capturer {
            conn,
            root,
            region,
            format,
            start: Instant::now(),
            active: true,
            dead: false,
        })
    }

    /// One `GetImage` round-trip. The reply's rows are already tightly packed (32 bpp, 32-bit
    /// scanline pad ⇒ `width * 4` bytes per row), so its buffer IS the frame payload.
    fn capture(&mut self) -> Result<CapturedFrame> {
        if self.dead {
            return Err(anyhow!(
                "X11 capture is dead: an earlier GetImage failed (screen reconfigured, or the X \
                 connection dropped) — rebuild the capturer"
            ));
        }
        let Region {
            x,
            y,
            width,
            height,
        } = self.region;
        let reply = self
            .conn
            .get_image(
                ImageFormat::Z_PIXMAP,
                self.root,
                x,
                y,
                width,
                height,
                ALL_PLANES,
            )
            .map_err(ReplyError::from)
            .and_then(|c| c.reply())
            .map_err(|e| {
                self.dead = true;
                anyhow!("X11 GetImage failed ({width}x{height}+{x}+{y}): {e}")
            })?;
        let want = width as usize * height as usize * 4;
        if reply.data.len() < want {
            self.dead = true;
            return Err(anyhow!(
                "X11 GetImage returned {} bytes for a {width}x{height} region, expected {want}",
                reply.data.len()
            ));
        }
        let mut data = reply.data;
        data.truncate(want);
        Ok(CapturedFrame {
            width: u32::from(width),
            height: u32::from(height),
            pts_ns: self.start.elapsed().as_nanos() as u64,
            format: self.format,
            payload: FramePayload::Cpu(data),
            cursor: None,
        })
    }
}

impl Capturer for X11Capturer {
    fn next_frame(&mut self) -> Result<CapturedFrame> {
        self.capture()
    }

    /// `GetImage` is synchronous and always produces the CURRENT framebuffer, so there is no
    /// mailbox to drain and no such thing as "no frame since last call" — the only `None` is an
    /// inactive (pooled) capturer, which must not pull framebuffers between streams.
    fn try_latest(&mut self) -> Result<Option<CapturedFrame>> {
        if !self.active {
            return Ok(None);
        }
        self.capture().map(Some)
    }

    fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    fn is_alive(&self) -> bool {
        !self.dead
    }
}

/// The one pixel layout this capturer accepts, resolved from the server's setup. See the module
/// docs: matching `[B,G,R,x]` exactly is what lets the `GetImage` reply become the payload
/// untouched, so anything else is rejected here rather than repacked per frame.
fn probe_format(setup: &Setup, screen: &Screen) -> Result<PixelFormat> {
    if setup.image_byte_order != ImageOrder::LSB_FIRST {
        return Err(anyhow!(
            "X11 capture needs an LSB-first server (this one is MSB-first): the reply bytes would \
             be R,G,B,x, which no CPU capture format here names"
        ));
    }
    let bpp = setup
        .pixmap_formats
        .iter()
        .find(|f| f.depth == screen.root_depth)
        .map(|f| f.bits_per_pixel)
        .ok_or_else(|| {
            anyhow!(
                "the X server lists no pixmap format for the root depth ({})",
                screen.root_depth
            )
        })?;
    if bpp != 32 {
        return Err(anyhow!(
            "X11 capture needs a 32-bit-per-pixel root (this root is depth {} at {bpp} bpp)",
            screen.root_depth
        ));
    }
    let visual = screen
        .allowed_depths
        .iter()
        .flat_map(|d| &d.visuals)
        .find(|v| v.visual_id == screen.root_visual)
        .ok_or_else(|| anyhow!("the root visual is missing from the X server's depth list"))?;
    if (visual.red_mask, visual.green_mask, visual.blue_mask) != (0xff0000, 0xff00, 0xff) {
        return Err(anyhow!(
            "X11 capture needs the usual TrueColor masks (R 0xff0000, G 0xff00, B 0xff); this \
             root visual reports R {:#x}, G {:#x}, B {:#x}",
            visual.red_mask,
            visual.green_mask,
            visual.blue_mask
        ));
    }
    // Depth 32 means the 4th byte is a real alpha channel; depth 24 leaves it unused padding.
    Ok(if screen.root_depth == 32 {
        PixelFormat::Bgra
    } else {
        PixelFormat::Bgrx
    })
}

/// The primary RandR output's CRTC geometry, or `None` when there is nothing to crop to — no
/// RandR extension, no primary set (a single-head server often sets none), or the primary output
/// is disconnected/unmapped. Every failure is a fallback, not an error: capturing the whole root
/// is a correct answer, just a bigger one on a multi-head desktop.
fn primary_region(conn: &RustConnection, root: Window) -> Option<Region> {
    let (major, minor) = RANDR_VERSION;
    if let Err(e) = conn
        .randr_query_version(major, minor)
        .ok()?
        .reply()
        .map_err(|e| e.to_string())
    {
        tracing::debug!(error = %e, "X11 capture: no usable RandR — capturing the whole root");
        return None;
    }
    let primary = conn.randr_get_output_primary(root).ok()?.reply().ok()?;
    if primary.output == 0 {
        tracing::debug!("X11 capture: no primary RandR output — capturing the whole root");
        return None;
    }
    let res = conn
        .randr_get_screen_resources_current(root)
        .ok()?
        .reply()
        .ok()?;
    let info = conn
        .randr_get_output_info(primary.output, res.config_timestamp)
        .ok()?
        .reply()
        .ok()?;
    if info.crtc == 0 {
        tracing::debug!("X11 capture: the primary output drives no CRTC — capturing the whole root");
        return None;
    }
    let crtc = conn
        .randr_get_crtc_info(info.crtc, res.config_timestamp)
        .ok()?
        .reply()
        .ok()?;
    if crtc.width == 0 || crtc.height == 0 {
        return None;
    }
    tracing::debug!(
        output = %String::from_utf8_lossy(&info.name),
        width = crtc.width,
        height = crtc.height,
        "X11 capture: cropping to the primary RandR output"
    );
    Some(Region {
        x: crtc.x,
        y: crtc.y,
        width: crtc.width,
        height: crtc.height,
    })
}
