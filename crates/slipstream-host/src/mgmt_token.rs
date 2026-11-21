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
            return validate(v).with_context(|| {
                format!(
                    "{env_var} is not a valid management token (want {} hex characters); \
                     unset it to fall back to the persisted file",
                    TOKEN_HEX_BYTES * 2
                )
            });
        }
    }
    let path = ss_paths::config_dir().join(file);
    if let Ok(contents) = fs::read_to_string(&path) {
        if let Some(tok) = parse_token(&contents, env_var).with_context(|| {
            // Fail closed, but never leave the operator guessing: name the file and the two
            // ways out. Deleting the file is safe — the token is regenerated and persisted on
            // the next start — and the env var overrides the file entirely.
            format!(
                "{}: unusable management token. To recover: delete that file to mint a new \
                 token on the next start, or set {env_var} to {} hex characters (it overrides \
                 the file and is not persisted)",
                path.display(),
                TOKEN_HEX_BYTES * 2
            )
        })? {
            return Ok(tok);
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

/// Parse the token from the persisted file.
///
/// Accepts every shape an operator or script can reasonably produce without editing the file
/// format we write: a bare value, `<KEY>=<value>` (what we write, also a valid EnvironmentFile),
/// surrounding whitespace, and CRLF line endings. `Ok(None)` means "nothing persisted yet" —
/// the caller mints one. Anything else is an explicit error naming the offending shape, so a
/// malformed file fails at startup with a fixable message instead of surfacing later as a 401
/// on every request.
fn parse_token(contents: &str, env_var: &str) -> Result<Option<String>> {
    let Some(line) = contents.lines().map(str::trim).find(|l| !l.is_empty()) else {
        return Ok(None);
    };
    let value = match line.strip_prefix(env_var) {
        Some(rest) => rest
            .trim_start()
            .strip_prefix('=')
            .with_context(|| format!("`{env_var}` line is missing the `=` separator"))?
            .trim(),
        None if line.contains('=') => {
            let key = line.split('=').next().unwrap_or(line);
            anyhow::bail!("found `{key}=...` but this file holds the {env_var} credential");
        }
        None => line,
    };
    validate(value).map(Some).with_context(|| {
        format!(
            "persisted token is not {} hex characters",
            TOKEN_HEX_BYTES * 2
        )
    })
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

    fn hex(n: usize) -> String {
        let alphabet = b"0123456789abcdef";
        (0..n).map(|i| alphabet[i % 16] as char).collect()
    }

    /// Every shape an operator or script can produce is read, and the parsed *value* is pinned
    /// (not just "did not panic").
    #[test]
    fn accepts_bare_keyvalue_padded_and_crlf_forms() {
        let token = hex(TOKEN_HEX_BYTES * 2);
        let cases: [(&str, String); 6] = [
            ("raw value + LF", format!("{token}\n")),
            ("raw value, no newline", token.clone()),
            ("KEY=VALUE + LF", format!("{ENV_VAR}={token}\n")),
            (
                "KEY=VALUE, whitespace-padded",
                format!("  \t{ENV_VAR} =  {token}  \r\n"),
            ),
            ("CRLF", format!("{ENV_VAR}={token}\r\n")),
            ("leading blank lines", format!("\n \n{token}\n")),
        ];
        for (label, contents) in cases {
            assert_eq!(
                parse_token(&contents, ENV_VAR).unwrap().as_deref(),
                Some(token.as_str()),
                "case {label}"
            );
        }
        // The plugin credential parses through the same reader with its own key.
        assert_eq!(
            parse_token(&format!("{PLUGIN_ENV_VAR}={token}\n"), PLUGIN_ENV_VAR)
                .unwrap()
                .as_deref(),
            Some(token.as_str())
        );
    }

    /// An empty file is not malformed — it means "nothing persisted yet", so the caller mints
    /// one. Blank-only content is the same case.
    #[test]
    fn empty_file_means_mint_a_token() {
        assert_eq!(parse_token("", ENV_VAR).unwrap(), None);
        assert_eq!(parse_token("\n  \n", ENV_VAR).unwrap(), None);
    }

    /// Malformed content is rejected here, with a message naming the shape, rather than being
    /// passed through to fail later as an opaque 401.
    #[test]
    fn rejects_malformed_forms_explicitly() {
        let token = hex(TOKEN_HEX_BYTES * 2);
        let truncated = &token[..TOKEN_HEX_BYTES * 2 - 1];
        let cases: [(&str, String); 6] = [
            ("truncated 63-hex", truncated.to_string()),
            ("empty value after `=`", format!("{ENV_VAR}=\n")),
            ("key without separator", ENV_VAR.to_string()),
            ("garbage line", "definitely not a token\n".to_string()),
            (
                "wrong credential's key",
                format!("{PLUGIN_ENV_VAR}={token}\n"),
            ),
            ("token with trailing junk", format!("{token}zz\n")),
        ];
        for (label, contents) in cases {
            let err = parse_token(&contents, ENV_VAR)
                .expect_err(&format!("case {label} must be rejected"))
                .to_string()
                .to_ascii_lowercase();
            assert!(
                err.contains("token") || err.contains("hex") || err.contains("separator"),
                "case {label}: unhelpful error {err:?}"
            );
        }
    }

    #[test]
    fn generated_token_round_trips_through_the_file() {
        let dir = std::env::temp_dir().join(format!("ss-mgmt-token-test-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(FILE);
        let token = "cafe".repeat(TOKEN_HEX_BYTES / 2);
        write_token(&path, ENV_VAR, &token).unwrap();
        let read = fs::read_to_string(&path).unwrap();
        assert_eq!(
            parse_token(&read, ENV_VAR).unwrap().as_deref(),
            Some(token.as_str())
        );
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
