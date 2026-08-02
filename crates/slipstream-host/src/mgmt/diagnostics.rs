//! Read-only host preflight checks for the console and support workflow.
//!
//! The checks intentionally report facts that can be evaluated without creating a display or
//! opening a stream. A failed check includes one operator action, while a warning means the host
//! can still start but will use a fallback or needs a session-specific setting.

use super::shared::*;
use crate::encode::Codec;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CheckStatus {
    Pass,
    Warn,
    Fail,
    Skip,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct PreflightCheck {
    /// Stable identifier suitable for UI filtering and support reports.
    id: String,
    label: String,
    status: CheckStatus,
    detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    remediation: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct PreflightReport {
    schema: u32,
    generated_unix_ms: u64,
    ready: bool,
    checks: Vec<PreflightCheck>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn check(
    id: &str,
    label: &str,
    status: CheckStatus,
    detail: impl Into<String>,
    remediation: Option<&str>,
) -> PreflightCheck {
    PreflightCheck {
        id: id.into(),
        label: label.into(),
        status,
        detail: detail.into(),
        remediation: remediation.map(str::to_string),
    }
}

fn config_check() -> PreflightCheck {
    let store = crate::host_config_file::store();
    let cfg = store.get();
    let errors = cfg.validate();
    if !errors.is_empty() {
        return check(
            "host-config",
            "Host configuration",
            CheckStatus::Fail,
            errors.join("; "),
            Some("Open Settings, correct the marked fields, and save again."),
        );
    }
    if store.configured() {
        check(
            "host-config",
            "Host configuration",
            CheckStatus::Pass,
            "Saved host configuration is valid.",
            None,
        )
    } else {
        check(
            "host-config",
            "Host configuration",
            CheckStatus::Warn,
            "No saved host configuration; built-in defaults are active.",
            Some("Save the desired host settings before installing a service."),
        )
    }
}

fn config_storage_check() -> PreflightCheck {
    let dir = ss_paths::config_dir();
    match std::fs::metadata(&dir) {
        Ok(meta) if meta.is_dir() && !meta.permissions().readonly() => check(
            "config-storage",
            "Config storage",
            CheckStatus::Pass,
            "The Slipstream config directory is readable and writable.",
            None,
        ),
        Ok(meta) if !meta.is_dir() => check(
            "config-storage",
            "Config storage",
            CheckStatus::Fail,
            "The Slipstream config path is not a directory.",
            Some("Move the conflicting file and restart Slipstream."),
        ),
        Ok(_) => check(
            "config-storage",
            "Config storage",
            CheckStatus::Fail,
            "The Slipstream config directory is read-only.",
            Some("Grant the service account write access to its config directory."),
        ),
        Err(_) => check(
            "config-storage",
            "Config storage",
            CheckStatus::Warn,
            "The config directory does not exist yet; it will be created on first write.",
            Some("Verify the service account can create its config directory."),
        ),
    }
}

fn encoder_check() -> PreflightCheck {
    let caps = Codec::host_wire_caps();
    if caps == 0 {
        return check(
            "encoder",
            "Video encoder",
            CheckStatus::Fail,
            "No usable video encoder was detected.",
            Some("Check the GPU driver, encoder preference, and host logs."),
        );
    }
    use slipstream_core::quic::{CODEC_AV1, CODEC_H264, CODEC_HEVC, CODEC_PYROWAVE};
    let mut codecs = Vec::new();
    for (bit, codec) in [
        (CODEC_H264, Codec::H264),
        (CODEC_HEVC, Codec::H265),
        (CODEC_AV1, Codec::Av1),
        (CODEC_PYROWAVE, Codec::PyroWave),
    ] {
        if caps & bit != 0 {
            codecs.push(codec.label());
        }
    }
    check(
        "encoder",
        "Video encoder",
        CheckStatus::Pass,
        format!("Available codecs: {}.", codecs.join(", ")),
        None,
    )
}

fn conflicts_check() -> PreflightCheck {
    let conflicts = crate::detect::current_summary_labels();
    if conflicts.is_empty() {
        check(
            "conflicts",
            "Competing host processes",
            CheckStatus::Pass,
            "No competing Moonlight-compatible host is running.",
            None,
        )
    } else {
        check(
            "conflicts",
            "Competing host processes",
            CheckStatus::Fail,
            format!("Running now: {}.", conflicts.join(", ")),
            Some("Stop the other host before starting a Slipstream stream."),
        )
    }
}

#[cfg(target_os = "linux")]
fn graphics_checks(out: &mut Vec<PreflightCheck>) {
    let available = crate::vdisplay::available();
    let headless = crate::vdisplay::headless_available();
    if available.is_empty() && !headless.iter().any(|(_, _, ok)| *ok) {
        out.push(check(
            "compositor",
            "Compositor session",
            CheckStatus::Fail,
            "No usable compositor or headless backend was detected.",
            Some("Start a Wayland session or install labwc, krfb, or gamescope for headless use."),
        ));
    } else {
        let names = available
            .iter()
            .map(|c| format!("{c:?}"))
            .collect::<Vec<_>>();
        let detail = if names.is_empty() {
            "No desktop compositor is visible; a headless backend is available.".to_string()
        } else {
            format!("Available compositor backends: {}.", names.join(", "))
        };
        out.push(check(
            "compositor",
            "Compositor session",
            CheckStatus::Pass,
            detail,
            None,
        ));
    }

    let runtime = std::env::var_os("XDG_RUNTIME_DIR").is_some();
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let portal = runtime;
    let status = if runtime && (wayland || portal) {
        CheckStatus::Pass
    } else {
        CheckStatus::Warn
    };
    out.push(check(
        "capture-environment",
        "Capture environment",
        status,
        format!(
            "XDG_RUNTIME_DIR={}, Wayland display={}, portal session={}",
            if runtime { "present" } else { "missing" },
            if wayland { "present" } else { "missing" },
            if portal { "possible" } else { "missing" },
        ),
        (status == CheckStatus::Warn).then_some(
            "Run the service inside the graphical user session or configure a headless compositor.",
        ),
    ));

    let kms = ss_capture::probe_kms();
    let nvfbc = ss_capture::probe_nvfbc();
    out.push(check(
        "capture-backends",
        "Capture backends",
        if kms || nvfbc || wayland || portal {
            CheckStatus::Pass
        } else {
            CheckStatus::Warn
        },
        format!(
            "Portal={}, wlroots={}, KMS={}, NvFBC={}",
            if portal { "available" } else { "unavailable" },
            if wayland { "possible" } else { "unavailable" },
            if kms { "detected" } else { "unavailable" },
            if nvfbc { "detected" } else { "unavailable" },
        ),
        Some("Use the capture method picker to choose a backend supported by this session."),
    ));
}

#[cfg(not(target_os = "linux"))]
fn graphics_checks(out: &mut Vec<PreflightCheck>) {
    out.push(check(
        "compositor",
        "Display session",
        CheckStatus::Skip,
        "Linux compositor checks do not apply on this host.",
        None,
    ));
}

fn report() -> PreflightReport {
    let mut checks = vec![
        config_storage_check(),
        config_check(),
        encoder_check(),
        conflicts_check(),
    ];
    graphics_checks(&mut checks);
    let ready = checks.iter().all(|c| c.status != CheckStatus::Fail);
    PreflightReport {
        schema: 1,
        generated_unix_ms: now_ms(),
        ready,
        checks,
    }
}

/// Run the offline preflight check from the host CLI. This shares the same report builder as the
/// management route, so install scripts and the web console cannot disagree about readiness.
pub(crate) fn run_cli(args: &[String]) -> anyhow::Result<()> {
    let json = match args {
        [] => false,
        [flag] if flag == "--json" => true,
        _ => anyhow::bail!("usage: slipstream-host doctor [--json]"),
    };
    let result = report();
    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Slipstream host preflight");
        for item in &result.checks {
            let status = match item.status {
                CheckStatus::Pass => "PASS",
                CheckStatus::Warn => "WARN",
                CheckStatus::Fail => "FAIL",
                CheckStatus::Skip => "SKIP",
            };
            println!("[{status}] {}: {}", item.label, item.detail);
            if let Some(remediation) = &item.remediation {
                println!("       {remediation}");
            }
        }
        println!(
            "Result: {}",
            if result.ready {
                "ready to stream"
            } else {
                "blocked"
            }
        );
    }
    if result.ready {
        Ok(())
    } else {
        anyhow::bail!("host preflight has blocked checks")
    }
}

/// Evaluate host readiness without changing display or stream state.
#[utoipa::path(
    get,
    path = "/diagnostics/preflight",
    tag = "diagnostics",
    operation_id = "getDiagnosticsPreflight",
    responses(
        (status = OK, description = "Read-only host preflight report", body = PreflightReport),
        (status = UNAUTHORIZED, description = "Missing or invalid bearer token", body = ApiError),
    )
)]
pub(crate) async fn get_preflight() -> Json<PreflightReport> {
    Json(report())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_is_ready_when_no_check_fails() {
        let report = report();
        assert_eq!(report.schema, 1);
        assert_eq!(
            report.ready,
            !report.checks.iter().any(|c| c.status == CheckStatus::Fail)
        );
    }
}
