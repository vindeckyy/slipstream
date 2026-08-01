//! Outcome written for the unprivileged caller to read.

use serde::Serialize;
use std::path::Path;

use crate::mode::Mode;

/// What the host reads back. Field meanings mirror the mgmt API's `UpdateResultInfo`
/// where they overlap; `changed=false` is the "your package source has nothing newer
/// yet" case (not an error), `staged=true` means a reboot finishes the update
/// (rpm-ostree).
#[derive(Serialize)]
pub(crate) struct HelperResult {
    pub ok: bool,
    pub kind: String,
    pub before_version: String,
    pub after_version: String,
    pub changed: bool,
    pub staged: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub finished_unix: u64,
}

pub(crate) fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub(crate) fn write_result(mode: Mode, result: &HelperResult) {
    let path = Path::new(mode.result_path());
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = path.with_extension("json.tmp");
    if let Ok(bytes) = serde_json::to_vec_pretty(result) {
        if std::fs::write(&tmp, &bytes).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}
