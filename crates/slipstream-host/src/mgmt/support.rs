//! Local, redacted support-bundle export.
//!
//! A bundle is deliberately JSON rather than a general archive. It stays inspectable from the
//! console and CLI, has no decompression attack surface, and can be attached to a report without
//! granting the host a network upload path. The file is written owner-private and the management
//! API exposes it only on the bearer-token admin lane.

use super::shared::*;
use crate::log_capture::{LogEntry, MAX_PAGE};
use crate::stats_recorder::{CaptureMeta, StatsStatus};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const SCHEMA: u32 = 1;
const MAX_LOGS: usize = 1000;
const MAX_BUNDLE_BYTES: usize = 8 * 1024 * 1024;
const MAX_BUNDLES: usize = 10;
const MAX_BUNDLE_STORAGE_BYTES: u64 = 80 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct SupportBundle {
    pub schema: u32,
    pub id: String,
    pub generated_unix_ms: u64,
    pub host: SupportHost,
    pub configuration: crate::host_config_file::HostConfigFile,
    pub runtime: SupportRuntime,
    pub logs: Vec<LogEntry>,
    pub recordings: Vec<CaptureMeta>,
    pub redactions: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct SupportHost {
    pub version: String,
    pub abi_version: u32,
    pub os: String,
    pub os_name: String,
    pub gamestream: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub(crate) struct SupportRuntime {
    pub video_streaming: bool,
    pub audio_streaming: bool,
    pub active_game: bool,
    pub stats: StatsStatus,
    pub conflicts: Vec<String>,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

fn unix_ms_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn bundle_dir() -> PathBuf {
    ss_paths::config_dir().join("support-bundles")
}

fn next_id(now: u64) -> String {
    let serial = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    format!("{now}-{}-{serial}", std::process::id())
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id != "."
        && id != ".."
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

/// Redact values from key=value log fields and known local identity strings while keeping useful
/// backend/error context. Logs are not treated as a trusted serialization boundary: this removes
/// the identities the host knows about and records the limitation in the bundle metadata.
fn redact_message(message: &str, identities: &[String]) -> String {
    let sensitive = [
        "token",
        "pin",
        "password",
        "secret",
        "private_key",
        "cert_pem",
        "key_pem",
        "bearer",
    ];
    let mut identities = identities.to_vec();
    identities.sort_by_key(|value| std::cmp::Reverse(value.len()));
    let scrubbed = identities
        .iter()
        .fold(message.to_string(), |text, identity| {
            if identity.is_empty() {
                text
            } else {
                text.replace(identity, "<redacted-local>")
            }
        });
    scrubbed
        .split_whitespace()
        .map(|part| {
            let Some((key, value)) = part.split_once('=') else {
                return redact_endpoint(part).unwrap_or_else(|| part.to_string());
            };
            if sensitive
                .iter()
                .any(|needle| key.to_ascii_lowercase().contains(needle))
            {
                format!("{key}=<redacted>")
            } else if let Some(value) = redact_endpoint(value) {
                format!("{key}={value}")
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Replace standalone IPv4/IPv6 literals while preserving an endpoint's port and trailing comma.
fn redact_endpoint(value: &str) -> Option<String> {
    let core = value.trim_end_matches([',', ';', ')']);
    let suffix = &value[core.len()..];
    if let Ok(addr) = core.parse::<SocketAddr>() {
        let host = if addr.is_ipv6() {
            "[<redacted-ip>]"
        } else {
            "<redacted-ip>"
        };
        return Some(format!("{host}:{}{}", addr.port(), suffix));
    }
    if core.starts_with('[')
        && core.ends_with(']')
        && core[1..core.len() - 1].parse::<IpAddr>().is_ok()
    {
        return Some(format!("[<redacted-ip>]{}", &value[core.len()..]));
    }
    if core.parse::<IpAddr>().is_ok() {
        return Some(format!("<redacted-ip>{}", &value[core.len()..]));
    }
    None
}

fn redact_logs(st: &MgmtState) -> Vec<LogEntry> {
    let mut identities = vec![st.app.host.local_ip.to_string()];
    for key in ["HOME", "USER", "USERNAME", "USERPROFILE", "XDG_RUNTIME_DIR"] {
        if let Some(value) = std::env::var_os(key).and_then(|v| v.into_string().ok()) {
            identities.push(value);
        }
    }
    identities.sort_by_key(|value| std::cmp::Reverse(value.len()));
    crate::log_capture::ring()
        .since(0, MAX_PAGE.min(MAX_LOGS))
        .entries
        .into_iter()
        .map(|mut entry| {
            entry.msg = redact_message(&entry.msg, &identities);
            entry
        })
        .collect()
}

fn redact_recordings(st: &MgmtState) -> Vec<CaptureMeta> {
    st.stats
        .list()
        .into_iter()
        .map(|mut meta| {
            meta.client = "<redacted>".into();
            meta
        })
        .collect()
}

fn make_bundle(st: &MgmtState) -> SupportBundle {
    let now = unix_ms_now();
    let app = &st.app;
    let active_game = app
        .launch
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some();
    SupportBundle {
        schema: SCHEMA,
        id: next_id(now),
        generated_unix_ms: now,
        host: SupportHost {
            version: env!("SLIPSTREAM_VERSION").into(),
            abi_version: slipstream_core::ABI_VERSION,
            os: app.host.os_chain.clone(),
            os_name: app.host.os_name.clone(),
            gamestream: st.gamestream_enabled,
        },
        configuration: crate::host_config_file::get(),
        runtime: SupportRuntime {
            video_streaming: app.streaming.load(std::sync::atomic::Ordering::Relaxed),
            audio_streaming: app
                .audio_streaming
                .load(std::sync::atomic::Ordering::Relaxed),
            active_game,
            stats: st.stats.status(),
            conflicts: crate::detect::current_summary_labels(),
        },
        logs: redact_logs(st),
        recordings: redact_recordings(st),
        redactions: vec![
            "pairing PINs, bearer tokens, secrets, and private key material".into(),
            "paired client fingerprints and recording client labels".into(),
            "known host IP addresses, home paths, runtime paths, and local usernames in log fields"
                .into(),
            "support-bundle files are written owner-private under the Slipstream config directory"
                .into(),
        ],
    }
}

fn bundle_path(id: &str) -> std::io::Result<PathBuf> {
    if !valid_id(id) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "invalid support bundle id",
        ));
    }
    Ok(bundle_dir().join(format!("{id}.json")))
}

fn save_bundle(bundle: &SupportBundle) -> std::io::Result<()> {
    let dir = bundle_dir();
    ss_paths::create_private_dir(&dir)?;
    let bytes = serde_json::to_vec_pretty(bundle).map_err(std::io::Error::other)?;
    if bytes.len() > MAX_BUNDLE_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::OutOfMemory,
            "support bundle exceeded its size limit",
        ));
    }
    let path = bundle_path(&bundle.id)?;
    let tmp = path.with_extension("json.tmp");
    ss_paths::write_secret_file(&tmp, &bytes)?;
    std::fs::rename(tmp, path)?;
    prune_bundles(&dir, &bundle.id);
    Ok(())
}

/// Keep support export storage bounded. Only generated `*.json` children in the private support
/// directory are eligible, and the bundle just written is always preserved.
fn prune_bundles(dir: &std::path::Path, keep_id: &str) {
    let mut files = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                return None;
            }
            let id = path.file_stem()?.to_str()?.to_string();
            if !valid_id(&id) {
                return None;
            }
            let size = entry.metadata().ok()?.len();
            let modified = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .unwrap_or(UNIX_EPOCH);
            Some((id, path, size, modified))
        })
        .collect::<Vec<_>>();
    files.sort_by_key(|(_, _, _, modified)| *modified);
    let mut total = files.iter().map(|(_, _, size, _)| *size).sum::<u64>();
    while files.len() > MAX_BUNDLES || total > MAX_BUNDLE_STORAGE_BYTES {
        let Some(index) = files.iter().position(|(id, _, _, _)| id != keep_id) else {
            break;
        };
        let (_, path, size, _) = files.remove(index);
        if std::fs::remove_file(path).is_ok() {
            total = total.saturating_sub(size);
        }
    }
}

/// Create and persist a redacted support bundle.
#[utoipa::path(
    post,
    path = "/support-bundles",
    tag = "support",
    operation_id = "supportBundleCreate",
    responses(
        (status = OK, description = "Redacted support bundle created", body = SupportBundle),
        (status = INTERNAL_SERVER_ERROR, description = "Could not write the bundle", body = ApiError),
        (status = UNAUTHORIZED, description = "Missing or invalid bearer token", body = ApiError),
    )
)]
pub(crate) async fn support_bundle_create(State(st): State<Arc<MgmtState>>) -> Response {
    let bundle = make_bundle(&st);
    if let Err(e) = save_bundle(&bundle) {
        return api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("could not save support bundle: {e}"),
        );
    }
    tracing::info!(id = %bundle.id, "management API: support bundle created");
    Json(bundle).into_response()
}

/// Read a previously generated support bundle.
#[utoipa::path(
    get,
    path = "/support-bundles/{id}",
    tag = "support",
    operation_id = "supportBundleGet",
    params(("id" = String, Path, description = "The support bundle id")),
    responses(
        (status = OK, description = "Redacted support bundle", body = SupportBundle),
        (status = NOT_FOUND, description = "No bundle with that id", body = ApiError),
        (status = INTERNAL_SERVER_ERROR, description = "The bundle is unreadable", body = ApiError),
        (status = UNAUTHORIZED, description = "Missing or invalid bearer token", body = ApiError),
    )
)]
pub(crate) async fn support_bundle_get(Path(id): Path<String>) -> Response {
    let path = match bundle_path(&id) {
        Ok(path) => path,
        Err(_) => return api_error(StatusCode::NOT_FOUND, "no support bundle with that id"),
    };
    match std::fs::read(path) {
        Ok(bytes) => match serde_json::from_slice::<SupportBundle>(&bytes) {
            Ok(bundle) => Json(bundle).into_response(),
            Err(e) => api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("could not read support bundle: {e}"),
            ),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            api_error(StatusCode::NOT_FOUND, "no support bundle with that id")
        }
        Err(e) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("could not read support bundle: {e}"),
        ),
    }
}

/// Delete a locally stored support bundle.
#[utoipa::path(
    delete,
    path = "/support-bundles/{id}",
    tag = "support",
    operation_id = "supportBundleDelete",
    params(("id" = String, Path, description = "The support bundle id")),
    responses(
        (status = NO_CONTENT, description = "Bundle deleted"),
        (status = NOT_FOUND, description = "No bundle with that id", body = ApiError),
        (status = UNAUTHORIZED, description = "Missing or invalid bearer token", body = ApiError),
    )
)]
pub(crate) async fn support_bundle_delete(Path(id): Path<String>) -> Response {
    let path = match bundle_path(&id) {
        Ok(path) => path,
        Err(_) => return api_error(StatusCode::NOT_FOUND, "no support bundle with that id"),
    };
    match std::fs::remove_file(path) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            api_error(StatusCode::NOT_FOUND, "no support bundle with that id")
        }
        Err(e) => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("could not delete support bundle: {e}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_sensitive_key_value_fields() {
        let value = redact_message("pair pin=1234 token=abc backend=vaapi", &[]);
        assert_eq!(value, "pair pin=<redacted> token=<redacted> backend=vaapi");
    }

    #[test]
    fn redacts_known_local_identities() {
        let value = redact_message(
            "connect from 192.0.2.8 user=deck home=/home/deck",
            &["192.0.2.8".into(), "deck".into(), "/home/deck".into()],
        );
        assert_eq!(
            value,
            "connect from <redacted-local> user=<redacted-local> home=<redacted-local>"
        );
    }

    #[test]
    fn redacts_ip_literals_and_keeps_ports() {
        let value = redact_message("addr=192.0.2.8:47990 peer=[2001:db8::8]:9777", &[]);
        assert_eq!(value, "addr=<redacted-ip>:47990 peer=[<redacted-ip>]:9777");
    }

    #[test]
    fn rejects_path_traversal_bundle_ids() {
        assert!(bundle_path("../secret").is_err());
        assert!(bundle_path("a/b").is_err());
        assert!(bundle_path("ok-123").is_ok());
    }

    #[test]
    fn retention_keeps_current_bundle_and_bounds_generated_files() {
        let dir = tempfile::tempdir().unwrap();
        for id in 0..(MAX_BUNDLES + 3) {
            std::fs::write(dir.path().join(format!("{id}.json")), b"{}\n").unwrap();
        }
        std::fs::write(dir.path().join("keep.json"), b"{}\n").unwrap();

        prune_bundles(dir.path(), "keep");

        let remaining = std::fs::read_dir(dir.path()).unwrap().count();
        assert!(remaining <= MAX_BUNDLES);
        assert!(dir.path().join("keep.json").exists());
    }
}
