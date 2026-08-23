//! Session-scoped host-local cursor hide: while at least one client is streaming, hide the
//! host OS cursor (restore when the last session ends). Mirrors [`crate::sleep_inhibit`]:
//! refcounted across native + GameStream planes. Best-effort - platforms that cannot hide
//! log once and stream on. Off when `SLIPSTREAM_HIDE_HOST_CURSOR=0`.

use std::sync::{Mutex, OnceLock};

/// RAII share of the host-wide cursor hide - hold one per live session/stream.
/// `active` tracks whether this guard actually incremented the refcount (tablet/touch present).
pub struct StreamHold(bool);

struct State {
    count: u32,
    /// The platform hide held for the whole 1..N refcount window (dropped on 1→0).
    platform: Option<ss_inject::host_cursor::PlatformHide>,
}

fn state() -> &'static Mutex<State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(|| {
        Mutex::new(State {
            count: 0,
            platform: None,
        })
    })
}

/// Take a share; the underlying OS hide is acquired on the 0→1 edge when the config allows it.
/// When a tablet or touch device is present on the host, hide; for TV-only setups (no touch/tablet)
/// keep the cursor visible so the TV retains its pointer.
pub fn hold() -> StreamHold {
    if !ss_host_config::config().hide_host_cursor {
        return StreamHold(false);
    }
    if !should_hide_for_device() {
        tracing::info!("host cursor stays visible: no tablet/touch device detected (TV mode)");
        return StreamHold(false);
    }
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    st.count += 1;
    if st.count == 1 && st.platform.is_none() {
        st.platform = ss_inject::host_cursor::PlatformHide::acquire();
    }
    StreamHold(true)
}

/// Only hide the host cursor when a tablet or touch device is connected.
/// TV setups with only mouse/keyboard/gamepad keep the cursor visible.
/// Checks `ID_INPUT_TABLET` / `ID_INPUT_TOUCHSCREEN` via udev, with fallback to
/// `SLIPSTREAM_HIDE_HOST_CURSOR_ON_TOUCH_ONLY=0` to force always-hide.
fn should_hide_for_device() -> bool {
    // Explicit override: 0 => always hide when enabled (old behavior)
    if std::env::var("SLIPSTREAM_HIDE_HOST_CURSOR_ON_TOUCH_ONLY").as_deref() == Ok("0") {
        return true;
    }
    // Default: hide only if tablet/touch present
    host_has_tablet_or_touch()
}

/// Scan host input devices for tablet or touchscreen.
/// Uses `udevadm` properties `ID_INPUT_TABLET=1` or `ID_INPUT_TOUCHSCREEN=1`.
/// Falls back to sysfs check; if detection fails, assume no tablet (TV stays).
fn host_has_tablet_or_touch() -> bool {
    // Fast path: env can force the decision for testing
    if let Ok(v) = std::env::var("SLIPSTREAM_FORCE_TOUCH_DEVICE") {
        return v == "1" || v.eq_ignore_ascii_case("true");
    }
    // Try udevadm enumeration of /dev/input/event*
    if let Ok(entries) = std::fs::read_dir("/dev/input") {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.starts_with("event") {
                continue;
            }
            // Prefer udevadm if available
            if let Ok(out) = std::process::Command::new("udevadm")
                .args(["info", "--query=property", "--name"])
                .arg(&path)
                .output()
            {
                if out.status.success() {
                    let txt = String::from_utf8_lossy(&out.stdout);
                    if txt.contains("ID_INPUT_TABLET=1") || txt.contains("ID_INPUT_TOUCHSCREEN=1") {
                        return true;
                    }
                    // If udevadm succeeded, continue to next device
                    continue;
                }
            }
            // Fallback: sysfs capabilities check via device path
            if let Ok(link) = std::fs::read_link(format!("/sys/class/input/{}/device", name)) {
                let dev_path = link.to_string_lossy();
                if dev_path.contains("tablet") || dev_path.contains("touch") {
                    return true;
                }
            }
        }
    }
    // Also check /sys/class/input for tablet/touch in name
    if let Ok(entries) = std::fs::read_dir("/sys/class/input") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.contains("tablet") || name.contains("touch") {
                return true;
            }
            // Check device/modalias for tablet
            let modalias_path = format!(
                "/sys/class/input/{}/device/modalias",
                entry.file_name().to_string_lossy()
            );
            if let Ok(m) = std::fs::read_to_string(&modalias_path) {
                let ml = m.to_lowercase();
                if ml.contains("tablet") || ml.contains("touch") {
                    return true;
                }
            }
        }
    }
    false
}

impl Drop for StreamHold {
    fn drop(&mut self) {
        if !self.0 {
            return;
        }
        // Only decrement if this guard actually contributed (tablet/touch was present at hold time)
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        st.count = st.count.saturating_sub(1);
        if st.count == 0 && st.platform.take().is_some() {
            tracing::info!("restored the host OS cursor (no live sessions)");
        }
    }
}
