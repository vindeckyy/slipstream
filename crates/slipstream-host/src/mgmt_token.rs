//! Management-API bearer token resolution.
//!
//! The mgmt API always serves HTTPS (the host's identity cert) and now always requires auth — even
//! on a loopback bind. This module guarantees the tokens always exist: an explicit env var wins
//! (operator override, not persisted); otherwise the persisted file under the config dir is used;
//! otherwise a fresh 32-byte hex token is generated and persisted. Files are written in
//! `KEY=<hex>` form (0600) so the bundled web console can source them directly as a systemd
//! `EnvironmentFile` — a single source of truth shared between the host and its consumers.
//!
//! Two tokens, two authorities:
//! - **`mgmt-token`** (`SLIPSTREAM_MGMT_TOKEN`) — the operator/console token; authorizes the full
//!   admin surface.
//! - **`plugin-token`** (`SLIPSTREAM_PLUGIN_TOKEN`) — the scripting runner's capability-limited
//!   credential (`mgmt::auth::plugin_may_access`): everything a plugin legitimately needs, but not
//!   hook registration or pairing administration. The SDK's `connect()` prefers this file, so a
//!   defect in an operator plugin can't rewrite `hooks.json` or admit new devices.

use anyhow::{Context, Result};
use rand::RngCore;
use std::fs;
use std::path::Path;

const TOKEN_HEX_BYTES: usize = 32;
const ENV_VAR: &str = "SLIPSTREAM_MGMT_TOKEN";
const FILE: &str = "mgmt-token";
const PLUGIN_ENV_VAR: &str = "SLIPSTREAM_PLUGIN_TOKEN";
const PLUGIN_FILE: &str = "plugin-token";

/// Resolve the mgmt (full-admin) token (env > persisted file > generate+persist). Hex (not base64)
/// so the persisted `KEY=VALUE` line is safe to source from a shell / systemd `EnvironmentFile`.
pub fn load_or_generate() -> Result<String> {
    load_or_generate_impl(ENV_VAR, FILE)
}

/// Resolve the scripting runner's scoped plugin token, same precedence as [`load_or_generate`].
/// Persisted to `plugin-token` next to `mgmt-token`; the plugin runner receives access only to this
/// scoped credential, never `mgmt-token`.
pub fn load_or_generate_plugin() -> Result<String> {
    load_or_generate_impl(PLUGIN_ENV_VAR, PLUGIN_FILE)
}

/// Validate an operator-supplied bearer token. Tokens are deliberately fixed-size hex so they
/// have 256 bits of entropy and can be carried safely in environment files and command lines.
pub fn validate(token: &str) -> Result<String> {
    let token = token.trim();
    if token.len() != TOKEN_HEX_BYTES * 2 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        anyhow::bail!(
            "management tokens must be exactly {} hexadecimal characters",
            TOKEN_HEX_BYTES * 2
        );
    }
    Ok(token.to_ascii_lowercase())
}

fn load_or_generate_impl(env_var: &str, file: &str) -> Result<String> {
    if let Ok(v) = std::env::var(env_var) {
        let v = v.trim();
        if !v.is_empty() {
            return validate(v).with_context(|| format!("validate {env_var}"));
        }
    }
    let path = ss_paths::config_dir().join(file);
    if let Ok(contents) = fs::read_to_string(&path) {
        if let Some(tok) = parse_token(&contents, env_var) {
            return validate(&tok).with_context(|| format!("validate {}", path.display()));
        }
    }
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    let token = hex::encode(buf);
    let dir = ss_paths::config_dir();
    // Keep the configuration directory owner-private so the token cannot leak through the path.
    ss_paths::create_private_dir(&dir).with_context(|| format!("create {}", dir.display()))?;
    write_token(&path, env_var, &token)?;
    tracing::info!(path = %path.display(), "generated and persisted API token (owner-only)");
    Ok(token)
}

/// Parse the token from the persisted file: accept either a bare token line or a
/// `<KEY>=<token>` line (the form we write, also valid as an EnvironmentFile).
fn parse_token(contents: &str, env_var: &str) -> Option<String> {
    let line = contents.lines().find(|l| !l.trim().is_empty())?.trim();
    let tok = line
        .strip_prefix(env_var)
        .and_then(|rest| rest.strip_prefix('='))
        .unwrap_or(line)
        .trim();
    (!tok.is_empty()).then(|| tok.to_string())
}

/// Write `<KEY>=<token>` to `path` as an owner-only secret through the shared secret-file helper.
fn write_token(path: &Path, env_var: &str, token: &str) -> Result<()> {
    let line = format!("{env_var}={token}\n");
    ss_paths::write_secret_file(path, line.as_bytes())
        .with_context(|| format!("write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare_and_keyvalue_forms() {
        let token = "a".repeat(TOKEN_HEX_BYTES * 2);
        assert_eq!(
            parse_token(&format!("{token}\n"), ENV_VAR).as_deref(),
            Some(token.as_str())
        );
        assert_eq!(
            parse_token(&format!("SLIPSTREAM_MGMT_TOKEN={token}\n"), ENV_VAR).as_deref(),
            Some(token.as_str())
        );
        assert_eq!(
            parse_token(
                &format!("SLIPSTREAM_PLUGIN_TOKEN={token}\n"),
                PLUGIN_ENV_VAR
            )
            .as_deref(),
            Some(token.as_str())
        );
        assert_eq!(parse_token("\n  \n", ENV_VAR), None);
        assert_eq!(parse_token("SLIPSTREAM_MGMT_TOKEN=\n", ENV_VAR), None);
    }

    #[test]
    fn generated_token_round_trips_through_the_file() {
        let dir = std::env::temp_dir().join(format!("ss-mgmt-token-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(FILE);
        let token = "cafe".repeat(TOKEN_HEX_BYTES / 2);
        write_token(&path, ENV_VAR, &token).unwrap();
        let read = fs::read_to_string(&path).unwrap();
        assert_eq!(parse_token(&read, ENV_VAR).as_deref(), Some(token.as_str()));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_short_or_non_hex_tokens() {
        assert!(validate("deadbeef").is_err());
        assert!(validate(&"g".repeat(TOKEN_HEX_BYTES * 2)).is_err());
        let uppercase = "A".repeat(TOKEN_HEX_BYTES * 2);
        assert_eq!(
            validate(&uppercase).unwrap(),
            "a".repeat(TOKEN_HEX_BYTES * 2)
        );
    }
}
