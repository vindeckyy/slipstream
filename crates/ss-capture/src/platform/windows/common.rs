//! Shared Windows capture plumbing: monitor enumeration (GDI), the one-deep
//! overwriting frame slot with arrival wakeup, and BGRA frame materialization.

use super::{capture_now_ns, CapturedFrame, FramePayload, PixelFormat};
use anyhow::{Context, Result};
use ss_frame::CaptureStageTimes;
use std::sync::{Arc, Condvar, Mutex};
use windows::Win32::{
    Foundation::{BOOL, LPARAM, RECT},
    Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW},
    UI::WindowsAndMessaging::MONITORINFOF_PRIMARY,
};

/// One physical head, as enumerated via GDI.
#[derive(Clone, Debug)]
pub struct MonitorInfo {
    /// The Win32 monitor handle, for `GraphicsCaptureItem::TryCreateFromMonitor`.
    pub hmon: HMONITOR,
    /// GDI device name (`\\.\DISPLAY1`) — what `SLIPSTREAM_CAPTURE_MONITOR` names.
    pub device: String,
    /// Whether this is the primary head.
    pub primary: bool,
    /// Desktop-space size. (The origin matters for the absolute-input anchor, which
    /// belongs to the vdisplay monitor enumeration landing with its todo — capture only
    /// needs the size here.)
    pub width: u32,
    pub height: u32,
}

/// Enumerate every display monitor via `EnumDisplayMonitors`.
pub fn enumerate_monitors() -> Result<Vec<MonitorInfo>> {
    let mut out: Vec<MonitorInfo> = Vec::new();
    // `EnumDisplayMonitors` calls `enum_proc` synchronously before returning, so the
    // `*mut Vec` in `lparam` is live for every call; the callback pushes one entry and
    // returns TRUE (continue). `HDC::default()` is the null HDC (whole virtual screen).
    // SAFETY: all arguments are live locals and the enumeration is synchronous; the only
    // raw pointer is the `lparam` round-trip documented on the callback itself.
    unsafe {
        EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(enum_proc),
            LPARAM(&mut out as *mut Vec<MonitorInfo> as isize),
        )
        .ok()
        .context("EnumDisplayMonitors")?;
    }
    Ok(out)
}

/// Pick the monitor to capture: the `SLIPSTREAM_CAPTURE_MONITOR` device name when it matches
/// (case-insensitive), else the primary head, else the first enumerated head.
pub fn pick_monitor(monitors: &[MonitorInfo], want: Option<&str>) -> Result<MonitorInfo> {
    if let Some(name) = want.filter(|n| !n.is_empty()) {
        if let Some(m) = monitors
            .iter()
            .find(|m| m.device.eq_ignore_ascii_case(name))
        {
            return Ok(m.clone());
        }
        anyhow::bail!(
            "capture monitor {name:?} not found (available: {})",
            monitors
                .iter()
                .map(|m| m.device.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    monitors
        .iter()
        .find(|m| m.primary)
        .or(monitors.first())
        .cloned()
        .context("no display monitors found")
}

// SAFETY: called synchronously by `EnumDisplayMonitors` on this thread; `lparam` is the
// `*mut Vec<MonitorInfo>` handed to it above, valid for the whole enumeration.
unsafe extern "system" fn enum_proc(
    hmon: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    // SAFETY: `lparam` is the `*mut Vec<MonitorInfo>` handed to the synchronous
    // `EnumDisplayMonitors` call above — live for the whole enumeration.
    let out = unsafe { &mut *(lparam.0 as *mut Vec<MonitorInfo>) };
    let mut ex = MONITORINFOEXW::default();
    ex.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    // SAFETY: `ex.monitorInfo` is a live struct filled synchronously by the OS; the
    // `MONITORINFOEXW` prefix layout matches `MONITORINFO` for this call.
    let ok = unsafe { GetMonitorInfoW(hmon, &mut ex.monitorInfo).as_bool() };
    if !ok {
        return true.into();
    }
    // (Origin deliberately not captured here — see the `width` doc above.)
    let r = ex.monitorInfo.rcMonitor;
    let device = String::from_utf16_lossy(&ex.szDevice)
        .trim_end_matches('\0')
        .to_string();
    out.push(MonitorInfo {
        hmon,
        device,
        primary: ex.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
        width: r.right.saturating_sub(r.left).max(0) as u32,
        height: r.bottom.saturating_sub(r.top).max(0) as u32,
    });
    true.into()
}

/// One-deep overwriting frame slot with an arrival generation counter: publishers never block,
/// consumers take the freshest frame, and `wait_arrival` wakes on publish instead of polling.
pub struct FrameSlot {
    state: Mutex<SlotState>,
    changed: Condvar,
}

struct SlotState {
    /// Arrival generation, bumped on every publish — the `wait_arrival` wakeup condition.
    seq: u64,
    frame: Option<CapturedFrame>,
    published: u64,
    overwritten: u64,
    width: u32,
    height: u32,
    /// Terminal failure: the worker thread died; every consumer call after this is an `Err`.
    dead: Option<String>,
}

impl FrameSlot {
    pub fn new() -> Arc<Self> {
        Arc::new(FrameSlot {
            state: Mutex::new(SlotState {
                seq: 0,
                frame: None,
                published: 0,
                overwritten: 0,
                width: 0,
                height: 0,
                dead: None,
            }),
            changed: Condvar::new(),
        })
    }

    /// Publish a frame, overwriting whatever the consumer has not taken yet.
    pub fn publish(&self, frame: CapturedFrame) {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if st.frame.is_some() {
            st.overwritten += 1;
        }
        st.width = frame.width;
        st.height = frame.height;
        st.frame = Some(frame);
        st.published += 1;
        st.seq += 1;
        self.changed.notify_all();
    }

    /// Wake a `wait_arrival` waiter without publishing (a no-content signal).
    pub fn note_arrival(&self) {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        st.seq += 1;
        self.changed.notify_all();
    }

    /// Take the freshest frame, if any.
    pub fn take(&self) -> Option<CapturedFrame> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .frame
            .take()
    }

    /// Take the freshest frame, blocking until one arrives or the worker dies.
    pub fn take_blocking(&self) -> Result<CapturedFrame> {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if let Some(frame) = st.frame.take() {
                return Ok(frame);
            }
            if let Some(dead) = st.dead.clone() {
                anyhow::bail!("capture worker died: {dead}");
            }
            st = self.changed.wait(st).unwrap_or_else(|e| e.into_inner());
        }
    }

    /// Block until a FRESH publish lands or `deadline` passes. Never consumes the frame —
    /// the caller's `take` does that.
    pub fn wait_arrival(&self, deadline: std::time::Instant) {
        let st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let seen = st.seq;
        let timeout = deadline.saturating_duration_since(std::time::Instant::now());
        let _ = self
            .changed
            .wait_timeout_while(st, timeout, |s| s.seq == seen && s.dead.is_none());
    }

    /// Drop any buffered frame (called when the session parks the capturer).
    pub fn flush(&self) {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).frame = None;
    }

    /// Terminally fail the capturer: every subsequent consumer call is an `Err`.
    pub fn fail(&self, msg: String) {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if st.dead.is_none() {
            tracing::warn!(error = %msg, "windows capture worker died");
            st.dead = Some(msg);
            self.changed.notify_all();
        }
    }

    pub fn is_alive(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .dead
            .is_none()
    }

    pub fn dead_error(&self) -> Option<String> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .dead
            .clone()
    }

    pub fn telemetry(&self) -> super::CaptureTelemetry {
        let st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        super::CaptureTelemetry {
            last_frame_ns: capture_now_ns(),
            frames_published: st.published,
            frames_overwritten: st.overwritten,
            width: st.width,
            height: st.height,
            zerocopy: false,
            zerocopy_reason: "cpu frames (no D3D11 texture sharing yet)",
            ..Default::default()
        }
    }
}

/// Build a CPU `Bgra` [`CapturedFrame`] from a mapped D3D11 staging texture.
///
/// `data` is the mapped pointer with `row_pitch` bytes per row for `height` rows; only the
/// top-left `width × height` pixels are content (the texture may be padded wider).
pub fn materialize_bgra_frame(
    data: *const u8,
    row_pitch: usize,
    width: u32,
    height: u32,
) -> CapturedFrame {
    let (w, h) = (width as usize, height as usize);
    let mut buf = vec![0u8; w * h * 4];
    // SAFETY: `data` points to a live mapped staging texture with at least `row_pitch` bytes
    // per row for `height` rows (guaranteed by the D3D11 `Map` contract while mapped); each
    // row copies `w * 4 <= row_pitch` bytes into the disjoint `buf` row. The caller unmaps
    // after this returns; nothing outlives the call.
    unsafe {
        for y in 0..h {
            let src = data.add(y * row_pitch);
            let dst = buf.as_mut_ptr().add(y * w * 4);
            std::ptr::copy_nonoverlapping(src, dst, w * 4);
        }
    }
    CapturedFrame {
        width,
        height,
        pts_ns: capture_now_ns(),
        format: PixelFormat::Bgra,
        payload: FramePayload::Cpu(buf),
        cursor: None,
        stage_ns: CaptureStageTimes::default(),
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use windows::Win32::Graphics::Gdi::HMONITOR;

    fn mon(device: &str, primary: bool) -> MonitorInfo {
        MonitorInfo {
            hmon: HMONITOR::default(),
            device: device.into(),
            primary,
            width: 1920,
            height: 1080,
        }
    }

    fn frame(w: u32, h: u32, fill: u8) -> CapturedFrame {
        CapturedFrame {
            width: w,
            height: h,
            pts_ns: 0,
            format: PixelFormat::Bgra,
            payload: FramePayload::Cpu(vec![fill; w as usize * h as usize * 4]),
            cursor: None,
            stage_ns: CaptureStageTimes::default(),
        }
    }

    #[test]
    fn pick_honors_the_monitor_pin_case_insensitively() {
        let ms = vec![mon("\\\\.\\DISPLAY1", true), mon("\\\\.\\DISPLAY2", false)];
        assert_eq!(
            pick_monitor(&ms, Some("\\\\.\\display2")).unwrap().device,
            "\\\\.\\DISPLAY2"
        );
        // No pin → primary.
        assert_eq!(pick_monitor(&ms, None).unwrap().device, "\\\\.\\DISPLAY1");
        // A pin that matches nothing fails loudly (never a wrong screen).
        assert!(pick_monitor(&ms, Some("\\\\.\\DISPLAY9")).is_err());
    }

    #[test]
    fn pick_falls_back_to_first_without_a_primary() {
        let ms = vec![mon("\\\\.\\DISPLAY2", false)];
        assert_eq!(pick_monitor(&ms, None).unwrap().device, "\\\\.\\DISPLAY2");
        assert!(pick_monitor(&[], None).is_err());
    }

    #[test]
    fn slot_overwrites_and_hands_out_the_freshest() {
        let slot = FrameSlot::new();
        assert!(slot.take().is_none());
        slot.publish(frame(64, 64, 1));
        slot.publish(frame(64, 64, 2));
        let got = slot.take().expect("frame");
        assert_eq!(got.width, 64);
        let FramePayload::Cpu(buf) = got.payload;
        assert_eq!(buf[0], 2);
        assert!(slot.take().is_none());
        let t = slot.telemetry();
        assert_eq!(t.frames_published, 2);
        assert_eq!(t.frames_overwritten, 1);
        assert!(!t.zerocopy);
    }

    #[test]
    fn slot_wait_arrival_wakes_on_publish_and_times_out() {
        let slot = FrameSlot::new();
        // Past deadline → returns immediately without hanging.
        slot.wait_arrival(std::time::Instant::now());
        // A publish in another thread wakes a waiter before its deadline.
        let waker = slot.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(20));
            waker.publish(frame(16, 16, 7));
        });
        let t0 = std::time::Instant::now();
        slot.wait_arrival(t0 + std::time::Duration::from_secs(5));
        assert!(t0.elapsed() < std::time::Duration::from_secs(5));
        assert!(slot.take().is_some());
    }

    #[test]
    fn slot_failure_is_terminal() {
        let slot = FrameSlot::new();
        assert!(slot.is_alive());
        slot.fail("boom".into());
        assert!(!slot.is_alive());
        assert!(slot.take_blocking().is_err());
    }

    #[test]
    fn materialize_respects_row_pitch() {
        // 2x2 content in a 16-byte-pitch mapping (padded rows).
        let mut mapped = [0u8; 2 * 16];
        mapped[0..8].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        mapped[16..24].copy_from_slice(&[9, 10, 11, 12, 13, 14, 15, 16]);
        let f = materialize_bgra_frame(mapped.as_ptr(), 16, 2, 2);
        assert_eq!(f.format, PixelFormat::Bgra);
        let FramePayload::Cpu(buf) = f.payload;
        assert_eq!(buf, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
    }
}
