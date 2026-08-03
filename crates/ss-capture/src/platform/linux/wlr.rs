//! wlroots desktop capture via `zwlr_screencopy_manager_v1` (SHM path).
//!
//! Connects as an ordinary Wayland client (same shape as `ss-inject`'s wlroots injector), binds
//! the screencopy manager + `wl_shm` + the first usable `wl_output`, and copies each frame into a
//! client SHM buffer. The capturer then memcpy's that buffer into a tightly packed CPU
//! [`PixelFormat::Bgrx`] / [`PixelFormat::Bgra`] payload — no dmabuf, no PipeWire, no portal.
//!
//! This is the direct-screencopy source for Sway / River / Hyprland when the portal path is
//! unwanted or unavailable. Cost matches the protocol: one compositor blit into SHM per frame,
//! plus a host-side copy out of the mmap. Prefer the portal/PipeWire path when zero-copy matters.
//!
//! **Fail-closed.** Open refuses without `WAYLAND_DISPLAY`, without `zwlr_screencopy_manager_v1`,
//! without `wl_shm`, or without a usable output. Per-frame failures that look permanent (copy
//! failed, connection died) sticky-kill the capturer so a pooled session rebuilds.

// Every `unsafe` block in this file carries a `// SAFETY:` proof; enforce it (unsafe-proof program).
#![deny(clippy::undocumented_unsafe_blocks)]

use anyhow::{anyhow, bail, Context, Result};
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};
use std::time::{Duration, Instant};
use wayland_client::protocol::{
    wl_buffer::WlBuffer,
    wl_output::{self, WlOutput},
    wl_registry,
    wl_shm::{self, WlShm},
    wl_shm_pool::WlShmPool,
};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum};
use wayland_protocols_wlr::screencopy::v1::client::{
    zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
    zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1,
};

use super::{CaptureTelemetry, CapturedFrame, Capturer, FramePayload, PixelFormat};
use crate::capture_now_ns;

/// Cap on waiting for one screencopy handshake (buffer params or ready/failed).
const FRAME_WAIT: Duration = Duration::from_secs(5);

/// SHM buffer parameters from the frame's `buffer` event.
#[derive(Clone, Copy)]
struct BufferInfo {
    format: wl_shm::Format,
    width: u32,
    height: u32,
    stride: u32,
}

/// Per-capture handshake state, reset before every `capture_output`.
#[derive(Default)]
struct FramePending {
    info: Option<BufferInfo>,
    /// v3+: all buffer types enumerated. Ignored when the manager is bound below v3.
    buffer_done: bool,
    y_invert: bool,
    ready: bool,
    failed: bool,
}

struct OutputSlot {
    output: WlOutput,
    width: i32,
    height: i32,
}

/// Wayland dispatch state: globals + the in-flight frame handshake.
#[derive(Default)]
struct State {
    screencopy: Option<ZwlrScreencopyManagerV1>,
    /// Interface version we bound (caps at 3). Gates waiting for `buffer_done`.
    screencopy_version: u32,
    shm: Option<WlShm>,
    outputs: Vec<OutputSlot>,
    pending: FramePending,
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "zwlr_screencopy_manager_v1" => {
                    let v = version.min(3);
                    state.screencopy = Some(registry.bind(name, v, qh, ()));
                    state.screencopy_version = v;
                }
                "wl_shm" => {
                    state.shm = Some(registry.bind(name, 1, qh, ()));
                }
                "wl_output" => {
                    // v2+ carries `mode`; bind at most 4.
                    let output: WlOutput = registry.bind(name, version.min(4), qh, ());
                    state.outputs.push(OutputSlot {
                        output,
                        width: 0,
                        height: 0,
                    });
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<WlOutput, ()> for State {
    fn event(
        state: &mut Self,
        output: &WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Mode {
            flags: WEnum::Value(flags),
            width,
            height,
            ..
        } = event
        {
            if flags.contains(wl_output::Mode::Current) {
                if let Some(slot) = state.outputs.iter_mut().find(|o| o.output == *output) {
                    slot.width = width;
                    slot.height = height;
                }
            }
        }
    }
}

impl Dispatch<ZwlrScreencopyFrameV1, ()> for State {
    fn event(
        state: &mut Self,
        _: &ZwlrScreencopyFrameV1,
        event: zwlr_screencopy_frame_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use zwlr_screencopy_frame_v1::Event;
        match event {
            Event::Buffer {
                format,
                width,
                height,
                stride,
            } => {
                let format = match format {
                    WEnum::Value(f) => f,
                    WEnum::Unknown(v) => {
                        tracing::warn!(format = v, "screencopy offered an unknown wl_shm format");
                        return;
                    }
                };
                state.pending.info = Some(BufferInfo {
                    format,
                    width,
                    height,
                    stride,
                });
            }
            Event::Flags { flags } => {
                let flags = match flags {
                    WEnum::Value(f) => f,
                    WEnum::Unknown(_) => return,
                };
                state.pending.y_invert = flags.contains(zwlr_screencopy_frame_v1::Flags::YInvert);
            }
            Event::Ready { .. } => state.pending.ready = true,
            Event::Failed => state.pending.failed = true,
            Event::BufferDone => state.pending.buffer_done = true,
            // CPU SHM path only — ignore dmabuf offers and damage boxes.
            Event::LinuxDmabuf { .. } | Event::Damage { .. } => {}
            _ => {}
        }
    }
}

// Globals / pool / buffer emit nothing we use.
macro_rules! ignore_events {
    ($($t:ty),* $(,)?) => {$(
        impl Dispatch<$t, ()> for State {
            fn event(
                _: &mut Self,
                _: &$t,
                _: <$t as Proxy>::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
            }
        }
    )*};
}
ignore_events!(WlShm, WlShmPool, WlBuffer, ZwlrScreencopyManagerV1);

/// A client-owned SHM pool + buffer sized for the last negotiated frame.
struct ShmSlot {
    /// Keeps the memfd alive for the compositor's mmap and ours.
    _fd: OwnedFd,
    ptr: *mut u8,
    len: usize,
    pool: WlShmPool,
    buffer: WlBuffer,
    info: BufferInfo,
}

// SAFETY: `ptr` is an exclusive `MAP_SHARED` mapping of `_fd` for `len` bytes; the Wayland
// proxies are only used from the capturer's owning thread (the capturer is `Send` only because
// the encode loop moves it onto one worker — never shared across threads concurrently).
unsafe impl Send for ShmSlot {}

impl Drop for ShmSlot {
    fn drop(&mut self) {
        self.buffer.destroy();
        self.pool.destroy();
        if !self.ptr.is_null() && self.len > 0 {
            // SAFETY: `ptr`/`len` are exactly the base+length of the successful `mmap` in
            // [`ShmSlot::create`]; nothing else aliases this mapping; `munmap` is the matching
            // teardown. Wayland objects were destroyed above so the compositor is no longer
            // expected to touch the fd's pages through this pool.
            unsafe {
                libc::munmap(self.ptr.cast(), self.len);
            }
        }
    }
}

impl ShmSlot {
    fn create(shm: &WlShm, qh: &QueueHandle<State>, info: BufferInfo) -> Result<Self> {
        let len = info
            .stride
            .checked_mul(info.height)
            .ok_or_else(|| anyhow!("screencopy buffer size overflow"))? as usize;
        if len == 0 {
            bail!(
                "screencopy offered an empty buffer ({}x{}, stride {})",
                info.width,
                info.height,
                info.stride
            );
        }
        let fd = memfd_sized(len)?;
        // SAFETY: `fd` is a fresh memfd sized to `len` above; `mmap` installs a new
        // PROT_READ|PROT_WRITE MAP_SHARED mapping of exactly those bytes and returns that base
        // (or MAP_FAILED). Nothing else has mapped this fd yet. The result is checked below.
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd.as_raw_fd(),
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            return Err(anyhow!(
                "mmap screencopy SHM failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        let pool = shm.create_pool(fd.as_fd(), len as i32, qh, ());
        let buffer = pool.create_buffer(
            0,
            info.width as i32,
            info.height as i32,
            info.stride as i32,
            info.format,
            qh,
            (),
        );
        Ok(Self {
            _fd: fd,
            ptr: ptr.cast(),
            len,
            pool,
            buffer,
            info,
        })
    }

    fn matches(&self, info: &BufferInfo) -> bool {
        self.info.width == info.width
            && self.info.height == info.height
            && self.info.stride == info.stride
            && self.info.format == info.format
    }

    /// Borrow the mapped pages as a byte slice (compositor has finished writing after `ready`).
    fn bytes(&self) -> &[u8] {
        // SAFETY: `ptr`/`len` are a live PROT_READ mapping; called only after the frame's
        // `ready` event, so the compositor has released the buffer back to the client.
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

/// Create an anonymous memfd of exactly `len` bytes (zeros).
fn memfd_sized(len: usize) -> Result<OwnedFd> {
    let name = c"slipstream-screencopy";
    // SAFETY: `name` is a valid NUL-terminated CStr literal; `memfd_create` only reads the name
    // (copying it) and returns a fresh fd (or -1). `MFD_CLOEXEC` is a valid flag. Checked below.
    let raw = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
    if raw < 0 {
        bail!(
            "memfd_create for screencopy SHM failed: {}",
            std::io::Error::last_os_error()
        );
    }
    // SAFETY: `raw` is the fresh memfd just returned and checked `>= 0`; unique open fd, so
    // `OwnedFd` takes sole ownership and closes it exactly once on drop.
    let fd = unsafe { OwnedFd::from_raw_fd(raw) };
    // SAFETY: `fd` is an open memfd we own; `ftruncate` only changes its size. `len` fits in
    // `off_t` for any realistic frame (checked via cast). Result checked below.
    let rc = unsafe { libc::ftruncate(fd.as_raw_fd(), len as libc::off_t) };
    if rc != 0 {
        bail!(
            "ftruncate screencopy SHM to {len} failed: {}",
            std::io::Error::last_os_error()
        );
    }
    Ok(fd)
}

fn pixel_format(format: wl_shm::Format) -> Result<PixelFormat> {
    // Little-endian hosts: wl_shm XRGB8888/ARGB8888 store bytes as [B,G,R,x]/[B,G,R,A].
    match format {
        wl_shm::Format::Xrgb8888 => Ok(PixelFormat::Bgrx),
        wl_shm::Format::Argb8888 => Ok(PixelFormat::Bgra),
        other => Err(anyhow!(
            "wlroots screencopy offered unsupported wl_shm format {other:?} (need Xrgb8888 or Argb8888)"
        )),
    }
}

/// Copy SHM rows into a tightly packed `width * height * 4` CPU buffer, honouring stride and
/// the frame's `y_invert` flag.
fn pack_frame(src: &[u8], info: &BufferInfo, y_invert: bool) -> Result<Vec<u8>> {
    let w = info.width as usize;
    let h = info.height as usize;
    let stride = info.stride as usize;
    let row = w
        .checked_mul(4)
        .ok_or_else(|| anyhow!("screencopy row size overflow"))?;
    if stride < row {
        bail!("screencopy stride {stride} is smaller than row width {row}");
    }
    let need = stride
        .checked_mul(h)
        .ok_or_else(|| anyhow!("screencopy SHM size overflow"))?;
    if src.len() < need {
        bail!(
            "screencopy SHM mapping is {} bytes, need {need} for {}x{} stride {stride}",
            src.len(),
            info.width,
            info.height
        );
    }
    let mut out = vec![0u8; row * h];
    for y in 0..h {
        let src_y = if y_invert { h - 1 - y } else { y };
        let s = src_y * stride;
        let d = y * row;
        out[d..d + row].copy_from_slice(&src[s..s + row]);
    }
    Ok(out)
}

/// SHM screencopy capturer for a wlroots-family compositor. See the module docs.
pub struct WlrCapturer {
    conn: Connection,
    queue: EventQueue<State>,
    state: State,
    /// Index into `state.outputs` of the captured head (resolved at open).
    output_idx: usize,
    shm_slot: Option<ShmSlot>,
    active: bool,
    dead: bool,
    width: u32,
    height: u32,
    last_frame_ns: u64,
    frames_published: u64,
}

impl WlrCapturer {
    /// Connect to `$WAYLAND_DISPLAY`, bind screencopy + SHM, and pick the first output that has
    /// advertised a current mode.
    pub fn open() -> Result<Self> {
        if std::env::var_os("WAYLAND_DISPLAY").is_none() {
            bail!(
                "wlroots screencopy needs WAYLAND_DISPLAY (no Wayland session in this environment)"
            );
        }
        let conn = Connection::connect_to_env().context(
            "connect to Wayland for screencopy (is WAYLAND_DISPLAY / XDG_RUNTIME_DIR set?)",
        )?;
        let mut queue = conn.new_event_queue();
        let qh = queue.handle();
        let _registry = conn.display().get_registry(&qh, ());
        let mut state = State::default();
        queue
            .roundtrip(&mut state)
            .context("Wayland registry roundtrip (screencopy)")?;

        if state.screencopy.is_none() {
            bail!(
                "compositor lacks zwlr_screencopy_manager_v1 (need a wlroots-family compositor \
                 such as Sway, River, or Hyprland)"
            );
        }
        if state.shm.is_none() {
            bail!("compositor advertised no wl_shm (required for screencopy SHM capture)");
        }
        // Output mode events arrive after the registry globals; one more roundtrip settles them.
        queue
            .roundtrip(&mut state)
            .context("Wayland output geometry roundtrip")?;

        let output_idx = state
            .outputs
            .iter()
            .position(|o| o.width > 0 && o.height > 0)
            .ok_or_else(|| {
                anyhow!("compositor advertised no usable wl_output with a current mode")
            })?;
        let (ow, oh) = {
            let o = &state.outputs[output_idx];
            (o.width, o.height)
        };

        tracing::info!(
            wayland = ?std::env::var("WAYLAND_DISPLAY").ok(),
            output_w = ow,
            output_h = oh,
            screencopy_version = state.screencopy_version,
            "wlroots screencopy: SHM source open"
        );

        // Probe one frame so open fails on unsupported formats / permission denial instead of
        // at first encode tick. The capturer then reuses the SHM slot for steady-state copies.
        let mut cap = WlrCapturer {
            conn,
            queue,
            state,
            output_idx,
            shm_slot: None,
            active: true,
            dead: false,
            width: ow as u32,
            height: oh as u32,
            last_frame_ns: 0,
            frames_published: 0,
        };
        let _ = cap.capture().context("screencopy probe frame")?;
        cap.last_frame_ns = 0;
        cap.frames_published = 0;
        Ok(cap)
    }

    fn output(&self) -> &WlOutput {
        &self.state.outputs[self.output_idx].output
    }

    fn dispatch_until(
        &mut self,
        deadline: Instant,
        done: impl Fn(&FramePending) -> bool,
    ) -> Result<()> {
        while !done(&self.state.pending) {
            if Instant::now() >= deadline {
                bail!("timed out waiting for zwlr_screencopy frame events");
            }
            self.queue
                .blocking_dispatch(&mut self.state)
                .context("wayland dispatch (screencopy)")?;
        }
        Ok(())
    }

    fn capture(&mut self) -> Result<CapturedFrame> {
        if self.dead {
            return Err(anyhow!(
                "wlroots screencopy is dead: an earlier frame copy failed (output gone, or the \
                 Wayland connection dropped) — rebuild the capturer"
            ));
        }

        self.state.pending = FramePending::default();
        let qh = self.queue.handle();
        let manager = self
            .state
            .screencopy
            .as_ref()
            .ok_or_else(|| anyhow!("screencopy manager missing"))?
            .clone();
        // overlay_cursor = 1: bake the pointer into the frame (we publish no CursorOverlay).
        let frame = manager.capture_output(1, self.output(), &qh, ());
        self.conn
            .flush()
            .context("wayland flush (capture_output)")?;

        let version = self.state.screencopy_version;
        let deadline = Instant::now() + FRAME_WAIT;
        self.dispatch_until(deadline, |p| {
            if p.failed {
                return true;
            }
            p.info.is_some() && (version < 3 || p.buffer_done)
        })?;
        if self.state.pending.failed {
            frame.destroy();
            self.dead = true;
            bail!("zwlr_screencopy failed while advertising buffer parameters");
        }
        let info = self
            .state
            .pending
            .info
            .ok_or_else(|| anyhow!("screencopy sent no wl_shm buffer event"))?;
        let format = match pixel_format(info.format) {
            Ok(f) => f,
            Err(e) => {
                frame.destroy();
                self.dead = true;
                return Err(e);
            }
        };

        if self.shm_slot.as_ref().is_none_or(|s| !s.matches(&info)) {
            let shm = self
                .state
                .shm
                .as_ref()
                .ok_or_else(|| anyhow!("wl_shm missing"))?
                .clone();
            // Drop the old slot before creating a new one (releases the prior mmap / memfd).
            self.shm_slot = None;
            self.shm_slot = Some(ShmSlot::create(&shm, &qh, info)?);
            self.conn.flush().ok();
        }
        let buffer = self
            .shm_slot
            .as_ref()
            .ok_or_else(|| anyhow!("screencopy SHM slot missing"))?
            .buffer
            .clone();

        self.state.pending.ready = false;
        self.state.pending.failed = false;
        self.state.pending.y_invert = false;
        frame.copy(&buffer);
        self.conn
            .flush()
            .context("wayland flush (screencopy copy)")?;

        let deadline = Instant::now() + FRAME_WAIT;
        self.dispatch_until(deadline, |p| p.ready || p.failed)?;
        if self.state.pending.failed {
            frame.destroy();
            self.dead = true;
            bail!("zwlr_screencopy copy failed (output removed, or compositor denied capture?)");
        }
        let y_invert = self.state.pending.y_invert;
        frame.destroy();

        let slot = self
            .shm_slot
            .as_ref()
            .ok_or_else(|| anyhow!("screencopy SHM slot missing after ready"))?;
        let data = pack_frame(slot.bytes(), &info, y_invert).inspect_err(|_| {
            self.dead = true;
        })?;

        let pts_ns = capture_now_ns();
        self.last_frame_ns = pts_ns;
        self.frames_published = self.frames_published.saturating_add(1);
        self.width = info.width;
        self.height = info.height;
        Ok(CapturedFrame {
            width: info.width,
            height: info.height,
            pts_ns,
            format,
            payload: FramePayload::Cpu(data),
            cursor: None,
            stage_ns: ss_frame::CaptureStageTimes::default(),
        })
    }
}

impl Capturer for WlrCapturer {
    fn backend_name(&self) -> &'static str {
        "wlr-screencopy"
    }

    fn telemetry(&self) -> CaptureTelemetry {
        CaptureTelemetry {
            last_frame_ns: self.last_frame_ns,
            frames_published: self.frames_published,
            width: self.width,
            height: self.height,
            ..CaptureTelemetry::default()
        }
    }

    fn next_frame(&mut self) -> Result<CapturedFrame> {
        self.capture()
    }

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
