//! Windows desktop backend: mirror a real head at the client's mode, or plug an indirect
//! display when `SLIPSTREAM_VIRTUAL_DISPLAY=idd` and a compatible driver is installed.
//!
//! Mirror is the default. Every Windows session streams a physical head — the same
//! "mirror, don't create" shape as the Linux `MirrorDisplay` (and like it, this
//! backend reports [`DisplayOwnership::External`], so none of the keep-alive pooling
//! applies to someone else's monitor).
//!
//! `create(mode)` sets the head to the client's WxH@Hz with `ChangeDisplaySettingsExW`
//! (a no-op when the head is already there) and hands back a RAII keepalive that
//! restores the previous mode on drop — a session never leaves the desktop stranded
//! at the client's resolution, including service-stop and host-crash paths that skip
//! the session teardown (best-effort: the restore itself can fail, and then the mode
//! sticks until the next session or a manual change).
//!
//! The mode change is display-wide (every session sees the same head), so concurrent
//! sessions at different modes last-writer-win — the admission layer (`display/admission.rs`)
//! keeps them apart, exactly like the shared-desktop Linux backends.

use crate::monitors::PhysicalMonitor;
use crate::{DisplayOwnership, Mode, VirtualDisplay, VirtualOutput};

#[path = "windows_idd.rs"]
mod idd;

use anyhow::{Context, Result};
pub use idd::{driver_present as idd_driver_present, use_virtual_display, WindowsIddDisplay};
use windows::Win32::{
    Foundation::{BOOL, HWND, LPARAM, RECT},
    Graphics::Gdi::*,
};

/// Whether the box has any attached head to mirror (the `available`/`detect` gate).
pub fn has_heads() -> bool {
    list_monitors().map(|m| !m.is_empty()).unwrap_or(false)
}

/// Every attached head, in GDI enumeration order.
pub fn list_monitors() -> Result<Vec<PhysicalMonitor>> {
    // Origins come from the monitor walk (matched by device name below).
    let origins = monitor_origins();
    let mut out = Vec::new();
    let mut index = 0u32;
    loop {
        let mut adapter = DISPLAY_DEVICEW {
            cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };
        // SAFETY: `adapter` is a live struct filled synchronously; iteration ends at
        // the first `false` (no more devices). Null adapter name enumerates displays.
        let more = unsafe {
            use windows::core::PCWSTR;
            EnumDisplayDevicesW(PCWSTR::null(), index, &mut adapter, 0).as_bool()
        };
        if !more {
            break;
        }
        index += 1;
        if adapter.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP == 0 {
            continue;
        }
        let device = wide_to_string(&adapter.DeviceName);
        let description = {
            let s = wide_to_string(&adapter.DeviceString);
            super::monitors::describe("", &s, &device)
        };
        let (width, height, freq) =
            current_mode(&device).with_context(|| format!("read the current mode of {device}"))?;
        let (x, y) = origins.get(&device).copied().unwrap_or((0, 0));
        out.push(PhysicalMonitor {
            connector: device,
            description,
            width,
            height,
            refresh_mhz: freq * 1000,
            x,
            y,
            // Per-monitor DPI (`GetDpiForMonitor`) lands separately; 1.0 keeps the
            // console picker honest about what is known today.
            scale: 1.0,
            primary: adapter.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE != 0,
            enabled: true,
            managed: false,
        });
    }
    Ok(out)
}

/// The mirror backend: one real head, mode-set per session, mode-restored on drop.
pub struct WindowsMirrorDisplay {
    device: String,
}

impl WindowsMirrorDisplay {
    /// Resolve the head: the `SLIPSTREAM_CAPTURE_MONITOR` pin when set (a miss is a
    /// hard error — streaming the wrong screen is worse than refusing), else primary,
    /// else the first attached head.
    pub fn new() -> Result<Self> {
        let heads = list_monitors()?;
        let head = match crate::capture_monitor() {
            Some(want) => crate::monitors::resolve(&heads, &want)?.connector.clone(),
            None => heads
                .iter()
                .find(|m| m.primary)
                .or(heads.first())
                .context("no attached monitors")?
                .connector
                .clone(),
        };
        Ok(WindowsMirrorDisplay { device: head })
    }
}

impl VirtualDisplay for WindowsMirrorDisplay {
    fn name(&self) -> &'static str {
        "windows"
    }

    fn create(&mut self, mode: Mode) -> Result<VirtualOutput> {
        let current = read_mode(&self.device)?;
        let restore = ModeRestore {
            device: self.device.clone(),
            original: current.clone(),
            restore_on_drop: current.dims() != (mode.width, mode.height, mode.refresh_hz),
        };
        if restore.restore_on_drop {
            tracing::info!(
                device = %self.device,
                from_w = current.width,
                from_h = current.height,
                from_hz = current.hz,
                to_w = mode.width,
                to_h = mode.height,
                to_hz = mode.refresh_hz,
                "windows: setting head to the client mode (restored on session end)"
            );
            set_mode(&self.device, mode.width, mode.height, mode.refresh_hz)?;
        } else {
            tracing::info!(
                device = %self.device,
                width = mode.width,
                height = mode.height,
                refresh_hz = mode.refresh_hz,
                "windows: head already at the client mode"
            );
        }
        Ok(VirtualOutput {
            node_id: 0, // no PipeWire node — capture resolves `windows_head` instead
            preferred_mode: Some((mode.width, mode.height, mode.refresh_hz)),
            keepalive: Box::new(restore),
            ownership: DisplayOwnership::External,
            reused_gen: None,
            pool_gen: None,
            windows_head: Some(self.device.clone()),
        })
    }
}

/// The head's mode as (width, height, Hz), for compare + restore.
#[derive(Clone)]
struct HeadMode {
    width: u32,
    height: u32,
    hz: u32,
    /// Full device mode, re-applied verbatim on restore (color depth, flags, …).
    devmode: DEVMODEW,
}

impl HeadMode {
    fn dims(&self) -> (u32, u32, u32) {
        (self.width, self.height, self.hz)
    }
}

/// RAII mode restore: re-applies the pre-session mode when the session ends.
struct ModeRestore {
    device: String,
    original: HeadMode,
    restore_on_drop: bool,
}

impl Drop for ModeRestore {
    fn drop(&mut self) {
        if !self.restore_on_drop {
            return;
        }
        if let Err(e) = apply_mode(
            &self.device,
            self.original.width,
            self.original.height,
            self.original.hz,
            &self.original.devmode,
        ) {
            tracing::warn!(
                device = %self.device,
                error = %format!("{e:#}"),
                "windows: could not restore the head mode — it stays at the session mode"
            );
        } else {
            tracing::info!(
                device = %self.device,
                "windows: restored the head mode after the session"
            );
        }
    }
}

/// Read the head's current mode.
fn read_mode(device: &str) -> Result<HeadMode> {
    // SAFETY: `dm` is a live struct (sized via `dmSize`) filled synchronously; the
    // device name lives for the call.
    unsafe {
        let name = to_wide_nul(device);
        let mut dm = DEVMODEW {
            dmSize: std::mem::size_of::<DEVMODEW>() as u16,
            ..Default::default()
        };
        EnumDisplaySettingsW(
            windows::core::PCWSTR(name.as_ptr()),
            ENUM_CURRENT_SETTINGS,
            &mut dm,
        )
        .ok()
        .with_context(|| format!("EnumDisplaySettingsW({device})"))?;
        Ok(HeadMode {
            width: dm.dmPelsWidth,
            height: dm.dmPelsHeight,
            hz: dm.dmDisplayFrequency,
            devmode: dm,
        })
    }
}

/// Current WxH@Hz of `device` (for the monitor list).
fn current_mode(device: &str) -> Result<(u32, u32, u32)> {
    read_mode(device).map(|m| (m.width, m.height, m.hz))
}

/// Set the head to WxH@Hz, testing the mode first so a refusal names the mode.
fn set_mode(device: &str, width: u32, height: u32, hz: u32) -> Result<()> {
    let current = read_mode(device)?;
    apply_mode(device, width, height, hz, &current.devmode)
}

/// Apply WxH@Hz onto `device`, cloned from `template` (preserves depth/orientation).
fn apply_mode(device: &str, width: u32, height: u32, hz: u32, template: &DEVMODEW) -> Result<()> {
    // SAFETY: `dm` is a live struct (sized, fields-only change); the device name lives
    // for the call; `CDS_TEST` changes nothing, `CDS_FULLSCREEN` applies process-wide
    // like every display-mode tool. A `RESTART` verdict (driver wants a reboot for the
    // mode) is surfaced, not forced.
    unsafe {
        let name = to_wide_nul(device);
        let mut dm = *template;
        dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY;
        dm.dmPelsWidth = width;
        dm.dmPelsHeight = height;
        dm.dmDisplayFrequency = hz;
        let tested = ChangeDisplaySettingsExW(
            windows::core::PCWSTR(name.as_ptr()),
            Some(&dm),
            HWND::default(),
            CDS_TEST,
            None,
        );
        if tested != DISP_CHANGE_SUCCESSFUL {
            anyhow::bail!(
                "display mode {width}x{height}@{hz} refused for {device} (test: {tested:?})"
            );
        }
        match ChangeDisplaySettingsExW(
            windows::core::PCWSTR(name.as_ptr()),
            Some(&dm),
            HWND::default(),
            CDS_FULLSCREEN,
            None,
        ) {
            DISP_CHANGE_SUCCESSFUL => Ok(()),
            other => {
                anyhow::bail!("display mode {width}x{height}@{hz} failed for {device} ({other:?})")
            }
        }
    }
}

/// `device → (x, y)` origins from the monitor walk, matched by GDI device name.
fn monitor_origins() -> std::collections::HashMap<String, (i32, i32)> {
    struct Origins(std::collections::HashMap<String, (i32, i32)>);
    // SAFETY: synchronous enumeration; the map outlives the walk (moved out after).
    unsafe extern "system" fn enum_proc(
        hmon: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        // SAFETY: `lparam` is the live `Origins` handed to the synchronous walk below.
        let out = unsafe { &mut *(lparam.0 as *mut Origins) };
        let mut ex = MONITORINFOEXW::default();
        ex.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        // SAFETY: live struct filled synchronously (see the capture crate's GDI walk).
        if unsafe { GetMonitorInfoW(hmon, &mut ex.monitorInfo).as_bool() } {
            let device = wide_to_string(&ex.szDevice);
            let r = ex.monitorInfo.rcMonitor;
            out.0.insert(device, (r.left, r.top));
        }
        true.into()
    }
    let mut origins = Origins(std::collections::HashMap::new());
    // SAFETY: the walk is synchronous; `origins` outlives it.
    unsafe {
        let _ = EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(enum_proc),
            LPARAM(&mut origins as *mut Origins as isize),
        );
    }
    origins.0
}

/// NUL-terminated UTF-16 device name for Win32 calls.
fn to_wide_nul(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// GDI fixed wide buffer → `String` (NUL-trimmed).
fn wide_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}
