//! DDC/CI monitor panel power control on Linux — the EXPERIMENTAL `ddc_power_off` axis.
//!
//! Same intent as the Windows `display/ddc.rs` path: VCP 0xD6 DPMS-off (`0x04`) before an
//! Exclusive topology drops physical heads, and DPMS-on (`0x01`) after restore. Implemented by
//! shelling out to `ddcutil` when it is on PATH (best-effort; missing binary or unsupported
//! panels are skipped with an info log). Never on the frame path.

use std::process::Command;

const VCP_POWER_MODE: &str = "0xD6";
const POWER_ON: &str = "0x01";
const POWER_OFF: &str = "0x04";

fn ddcutil_available() -> bool {
    Command::new("ddcutil")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Detect display numbers `ddcutil` can talk to (`Display N` lines from `ddcutil detect`).
fn detect_displays() -> Vec<u32> {
    let out = match Command::new("ddcutil").args(["detect", "--brief"]).output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        Ok(o) => {
            tracing::debug!(
                status = ?o.status,
                stderr = %String::from_utf8_lossy(&o.stderr),
                "DDC/CI: ddcutil detect failed"
            );
            return Vec::new();
        }
        Err(e) => {
            tracing::debug!(error = %e, "DDC/CI: ddcutil not runnable");
            return Vec::new();
        }
    };
    let mut ids = Vec::new();
    for line in out.lines() {
        // Brief form: "Display 1" / "Display 2"
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Display ") {
            if let Ok(n) = rest.split_whitespace().next().unwrap_or("").parse::<u32>() {
                ids.push(n);
            }
        }
    }
    ids
}

fn set_power(display_id: u32, value: &str) -> bool {
    let status = Command::new("ddcutil")
        .args([
            "--display",
            &display_id.to_string(),
            "setvcp",
            VCP_POWER_MODE,
            value,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status();
    match status {
        Ok(s) if s.success() => {
            tracing::info!(
                display_id,
                value,
                "DDC/CI: panel power mode commanded via ddcutil"
            );
            true
        }
        Ok(s) => {
            tracing::debug!(display_id, value, status = ?s, "DDC/CI: ddcutil setvcp failed");
            false
        }
        Err(e) => {
            tracing::debug!(display_id, error = %e, "DDC/CI: ddcutil setvcp not runnable");
            false
        }
    }
}

/// Command every DDC-capable panel off (VCP 0xD6 → DPMS off). `exclude` is unused on Linux
/// (virtual outputs do not answer DDC); kept so call sites match the Windows signature.
pub fn panel_off_except(_exclude: &str) -> u32 {
    if !ddcutil_available() {
        tracing::info!(
            "DDC/CI: ddcutil not on PATH — the ddc_power_off axis did nothing \
             (install ddcutil to enable monitor panel power control)"
        );
        return 0;
    }
    let displays = detect_displays();
    if displays.is_empty() {
        tracing::info!(
            "DDC/CI: no panel answered ddcutil detect — the ddc_power_off axis did nothing \
             (internal eDP panels often have no DDC; externals may have it disabled in the OSD)"
        );
        return 0;
    }
    let mut acked = 0u32;
    for d in displays {
        if set_power(d, POWER_OFF) {
            acked += 1;
        }
    }
    if acked == 0 {
        tracing::info!(
            "DDC/CI: no panel accepted the DPMS-off command — the ddc_power_off axis did nothing"
        );
    }
    acked
}

/// Best-effort wake: DPMS-on to every panel `ddcutil` can see.
pub fn panel_on_all() -> u32 {
    if !ddcutil_available() {
        return 0;
    }
    let mut acked = 0u32;
    for d in detect_displays() {
        if set_power(d, POWER_ON) {
            acked += 1;
        }
    }
    acked
}
