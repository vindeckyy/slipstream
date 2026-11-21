//! This client's persistent identity, shared file-for-file with the other clients on the box:
//! `%APPDATA%\slipstream\client-{cert,key}.pem`. Reusing the same path is what makes an already
//! paired device keep working when the operator switches clients.

use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;

fn config_dir() -> Result<PathBuf> {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return Ok(PathBuf::from(appdata).join("slipstream"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        return Ok(PathBuf::from(home).join(".config").join("slipstream"));
    }
    Err(anyhow!("neither APPDATA nor HOME is set"))
}

/// Load the PEM cert/key pair, minting one on first use. The certificate is public; the private
/// key is the credential a paired host authorizes, so it is created readable only by the owner.
pub fn load_or_create() -> Result<(String, String)> {
    let dir = config_dir()?;
    let (cp, kp) = (dir.join("client-cert.pem"), dir.join("client-key.pem"));
    if let (Ok(c), Ok(k)) = (std::fs::read_to_string(&cp), std::fs::read_to_string(&kp)) {
        return Ok((c, k));
    }
    let (c, k) = slipstream_core::quic::endpoint::generate_identity()
        .map_err(|e| anyhow!("generate identity: {e}"))?;
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    std::fs::write(&cp, &c).with_context(|| format!("write {}", cp.display()))?;
    write_private(&kp, &k)?;
    tracing::info!(cert = %cp.display(), "generated client identity");
    Ok((c, k))
}

/// `fs::write` under the default ACL on Windows; on unix lock it down to 0600 (the key is the
/// mTLS credential, not a public blob).
fn write_private(path: &std::path::Path, contents: &str) -> Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write as _;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        if let Some(dir) = path.parent() {
            let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
        }
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .with_context(|| format!("open {}", path.display()))?;
        f.write_all(contents.as_bytes())
            .with_context(|| format!("write {}", path.display()))
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, contents).with_context(|| format!("write {}", path.display()))
    }
}
