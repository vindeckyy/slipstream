/// Create `dir` and its parents owner-private at mode 0700. Tightens an already-existing dir too.
pub fn create_private_dir(dir: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        let r = std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir);
        // `recursive` doesn't re-chmod an existing dir — tighten it so an old 0755 dir gets locked.
        if dir.exists() {
            let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
        }
        r
    }
}

/// Write `contents` to `path` as an owner-only secret, created and re-chmod'd at mode 0600. Mirrors
/// the mgmt-token hardening for the host private key and persisted trust stores.
pub fn write_secret_file(path: &std::path::Path, contents: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    f.write_all(contents)?;
    f.flush()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}
