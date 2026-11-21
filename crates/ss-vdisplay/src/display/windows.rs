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
    let origins = monitor_origins().context("enumerate Windows display monitor origins")?;
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
        // P83-7: never invent an origin. Duplicate `(0,0)` values make absolute input and monitor
        // resolution treat two heads as one; skip the head when its origin could not be read.
        let Some((x, y)) = origins.get(&device).copied() else {
            tracing::warn!(
                device = %device,
                "skipping monitor with no readable GDI origin"
            );
            continue;
        };
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
            // A refusal is not fatal. Clients ask for their own window geometry — a MacBook at
            // 1512x982@120, for instance — and a TV or fixed-mode panel simply has no such mode.
            // Propagating the error aborts the pipeline build, and the client sees a black screen
            // with no frames (observed 2026-11-17: `display mode 1512x982@120 refused for
            // \\.\DISPLAY1 (test: DISP_CHANGE(-2))`, session dropped after one attempt).
            //
            // The readback below already exists to make the head's *actual* mode the source of
            // truth, precisely because a panel may ignore or refuse an ask. So the honest path is
            // to leave the head where it is, warn, and let the readback pick the real dimensions;
            // the client scales to its window. Changing the operator's display to a mode they did
            // not ask for would be worse than streaming at the mode it is already in.
            if let Err(e) = set_mode(&self.device, mode.width, mode.height, mode.refresh_hz) {
                tracing::warn!(
                    device = %self.device,
                    requested = %fmt_mode((mode.width, mode.height, mode.refresh_hz)),
                    error = %format!("{e:#}"),
                    "windows: the head refused the client mode  -  leaving it as it is and \
                     capturing where the display actually is (the client scales)"
                );
            }
        } else {
            tracing::info!(
                device = %self.device,
                width = mode.width,
                height = mode.height,
                refresh_hz = mode.refresh_hz,
                "windows: head already at the client mode"
            );
        }
        // `ChangeDisplaySettingsExW`'s success verdict is a statement about the call, not about the
        // head: on an RTX 3090 Ti / `\\.\DISPLAY1` panel it returns SUCCESSFUL for a
        // `1920x1080@144` ask that the panel cannot hold, and the head silently stays at its native
        // `2560x1440@144`. Capture configured from the *requested* mode then waits forever on a
        // frame pool for a mode that was never programmed: the session negotiates, opens NVENC,
        // streams audio and ships zero AUs with no error anywhere. Read the actual mode back and
        // make it the source of truth for everything downstream.
        let effective = read_mode(&self.device).context("read back the head mode after the set")?;
        let requested = (mode.width, mode.height, mode.refresh_hz);
        let effective_dims = effective.dims();
        // P83-6: WGC is the configured Windows capture backend. Probe it once here rather than
        // claiming every effective mode is capturable and failing later with zero frames.
        let capture_supported = ss_capture::probe_wgc();
        let (width, height, hz) = match decide_mode_readback(
            requested,
            effective_dims,
            capture_supported,
        ) {
            ModeReadback::UseRequested { refresh_hz } => {
                tracing::info!(
                    device = %self.device,
                    requested = %fmt_mode(requested),
                    effective = %fmt_mode(effective_dims),
                    used_hz = refresh_hz,
                    mismatch = false,
                    "head mode readback"
                );
                (requested.0, requested.1, refresh_hz)
            }
            ModeReadback::UseEffective => {
                tracing::warn!(
                    device = %self.device,
                    requested = %fmt_mode(requested),
                    effective = %fmt_mode(effective_dims),
                    mismatch = true,
                    "head mode readback  -  the display did not hold the requested mode; capturing \
                     where it actually is"
                );
                effective_dims
            }
            ModeReadback::Fail { reason } => anyhow::bail!(
                "head mode readback for {}: requested={} effective={}: {reason}",
                self.device,
                fmt_mode(requested),
                fmt_mode(effective_dims)
            ),
        };
        // A driver may report `dmDisplayFrequency` 0/1 for "default". Pacing needs a real rate;
        // use the effective mode's own Hz (the same source as the pixels above), never the
        // request's Hz, which would fabricate a WxH@Hz mode the head never advertised.
        let hz = if hz > 1 { hz } else { effective.hz };
        let hz = if hz > 1 { hz } else { 60 };
        Ok(VirtualOutput {
            node_id: 0, // no PipeWire node — capture resolves `windows_head` instead
            preferred_mode: Some((width, height, hz)),
            keepalive: Box::new(restore),
            ownership: DisplayOwnership::External,
            reused_gen: None,
            pool_gen: None,
            windows_head: Some(self.device.clone()),
        })
    }
}

/// What the mode readback says capture should be configured from.
///
/// Pure and separable from Win32 on purpose: the API boundary (`read_mode`) cannot be unit tested,
/// while this decision is the whole defect. `UseRequested` and `UseEffective` differ only where the
/// head ignored the ask; both mean "capture here", but only one is a warning.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ModeReadback {
    /// The head is at the requested pixel size (or the ask was already satisfied): capture there.
    /// Carries the rate the head actually runs, which differs from the request when only the
    /// refresh was refused.
    UseRequested { refresh_hz: u32 },
    /// The head ignored or refused the ask but its actual mode is usable: capture there.
    UseEffective,
    /// The head reports a mode capture cannot be configured for: refuse the session loudly.
    Fail { reason: String },
}

/// Decide what a `requested`/`effective` pair means for capture configuration.
///
/// The measured case that motivates this: requesting `1920x1080@144` on a panel whose EDID lists
/// only `2560x1440@144` leaves the head at `2560x1440@144` while `ChangeDisplaySettingsExW` returns
/// SUCCESSFUL. Capturing at the requested 1080p then produces zero frames forever, so an effective
/// mode that differs must win  -  `UseEffective`  -  never be silently reported as the request.
fn decide_mode_readback(
    requested: (u32, u32, u32),
    effective: (u32, u32, u32),
    capture_supported: bool,
) -> ModeReadback {
    if effective.0 == 0 || effective.1 == 0 {
        return ModeReadback::Fail {
            reason: "the display reports a zero-sized mode, so no frame can be captured".into(),
        };
    }
    if !capture_supported {
        return ModeReadback::Fail {
            reason: "capture cannot be configured for the display's effective mode".into(),
        };
    }
    // Pixels decide whether frames arrive at all, so agreeing pixels means the ask is satisfied. The
    // RATE is not interchangeable: a 1080p120 ask on a 60 Hz head used to be reported as held, and
    // the encoder then opened at 120 Hz on a 60 Hz source, pacing double and duplicating frames.
    // Capture at the rate the head really runs. A driver may report a fractional rate rounded to a
    // neighbour (143.998 Hz as 143 or 144); that is the real rate either way.
    if (effective.0, effective.1) == (requested.0, requested.1) {
        ModeReadback::UseRequested {
            refresh_hz: if effective.2 == 0 {
                requested.2
            } else {
                effective.2
            },
        }
    } else {
        ModeReadback::UseEffective
    }
}

/// `WxH@Hz` for a log line.
fn fmt_mode(mode: (u32, u32, u32)) -> String {
    format!("{}x{}@{}", mode.0, mode.1, mode.2)
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
fn monitor_origins() -> Result<std::collections::HashMap<String, (i32, i32)>> {
    struct Origins {
        map: std::collections::HashMap<String, (i32, i32)>,
        failed: bool,
    }
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
            out.map.insert(device, (r.left, r.top));
        } else {
            // P83-7: remember the failure instead of leaving this head to be defaulted to (0,0).
            out.failed = true;
        }
        true.into()
    }
    let mut origins = Origins {
        map: std::collections::HashMap::new(),
        failed: false,
    };
    // SAFETY: the walk is synchronous; `origins` outlives it.
    let ok = unsafe {
        EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(enum_proc),
            LPARAM(&mut origins as *mut Origins as isize),
        )
        .as_bool()
    };
    if !ok || origins.failed {
        anyhow::bail!("GetMonitorInfoW failed for at least one attached monitor")
    }
    Ok(origins.map)
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

#[cfg(test)]
mod readback_tests {
    use super::{decide_mode_readback, ModeReadback};

    /// The panel's native mode: asked for `2560x1440@144`, held it (measured: 410 frames / 10 s).
    #[test]
    fn readback_uses_the_request_for_the_native_mode() {
        assert_eq!(
            decide_mode_readback((2560, 1440, 144), (2560, 1440, 144), true),
            ModeReadback::UseRequested { refresh_hz: 144 }
        );
    }

    /// A mode the driver can genuinely drive: asked `1920x1080@60`, held it (measured: 275 / 8 s).
    #[test]
    fn readback_uses_the_request_for_a_working_1080p60_mode() {
        assert_eq!(
            decide_mode_readback((1920, 1080, 60), (1920, 1080, 60), true),
            ModeReadback::UseRequested { refresh_hz: 60 }
        );
    }

    /// THE defect, measured on this desk: `1920x1080@144` is absent from the panel's EDID, the
    /// change call still reports SUCCESSFUL, and the head stays at `2560x1440@144` while capture is
    /// configured for the requested 1080p — 0 frames with no error. The effective mode must win.
    #[test]
    fn readback_uses_the_effective_mode_when_the_head_ignored_1080p144() {
        let decision = decide_mode_readback((1920, 1080, 144), (2560, 1440, 144), true);
        assert_eq!(decision, ModeReadback::UseEffective);
        assert!(
            !matches!(decision, ModeReadback::UseRequested { .. }),
            "capturing at the requested 1920x1080 produced zero frames on the measured panel"
        );
    }

    /// A readback with no pixels can never be captured; it must fail the session, not degrade it.
    #[test]
    fn readback_fails_on_a_zero_sized_effective_mode() {
        for effective in [(0, 0, 0), (1920, 0, 60), (0, 1080, 60)] {
            match decide_mode_readback((1920, 1080, 60), effective, true) {
                ModeReadback::Fail { reason } => {
                    assert!(reason.contains("zero-sized"), "reason was {reason:?}")
                }
                other => panic!("expected Fail for effective {effective:?}, got {other:?}"),
            }
        }
    }

    /// If capture cannot be configured for the effective mode, refuse rather than ship audio only.
    #[test]
    fn readback_fails_when_capture_cannot_be_configured() {
        match decide_mode_readback((1920, 1080, 144), (2560, 1440, 144), false) {
            ModeReadback::Fail { reason } => {
                assert!(
                    reason.contains("capture cannot be configured"),
                    "{reason:?}"
                )
            }
            other => panic!("expected Fail, got {other:?}"),
        }
    }

    /// A fractional EDID rate read back as a neighbouring integer is the same mode for capture:
    /// `2560x1440@143.998` may enumerate as 143 or 144 Hz and must not be treated as a mismatch.
    #[test]
    fn readback_tolerates_a_rounded_refresh_for_the_same_pixels() {
        assert_eq!(
            decide_mode_readback((2560, 1440, 144), (2560, 1440, 143), true),
            ModeReadback::UseRequested { refresh_hz: 143 }
        );
    }

    /// Refresh-only refusal: the pixels agree, the head runs slower than asked. The ask counts as
    /// satisfied, but the encoder must pace at the head's rate, not the requested one.
    #[test]
    fn readback_uses_the_effective_refresh_when_only_the_rate_was_refused() {
        assert_eq!(
            decide_mode_readback((1920, 1080, 120), (1920, 1080, 60), true),
            ModeReadback::UseRequested { refresh_hz: 60 }
        );
        // A driver that reports no rate at all must not make the session run at 0 Hz.
        assert_eq!(
            decide_mode_readback((1920, 1080, 60), (1920, 1080, 0), true),
            ModeReadback::UseRequested { refresh_hz: 60 }
        );
    }
}
