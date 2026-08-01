/// Create `dir` (and parents) owner-private — **0700** on Unix (so the host's secrets aren't readable
/// by other local users via a traversable config path). On Windows, applies a restrictive DACL
/// ([`crate::windows::restrict_dir_to_system_admins`]) so a local unprivileged user can't pre-create / plant files in
/// the config tree (the default `%ProgramData%` ACL grants Users *create*; security-review
/// 2026-06-28 #3/#11). Tightens (and re-owns) an already-existing dir too.
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
    #[cfg(not(unix))]
    {
        let r = std::fs::create_dir_all(dir);
        #[cfg(windows)]
        crate::windows::restrict_dir_to_system_admins(dir);
        r
    }
}

/// Write `contents` to `path` as an **owner-only secret**: created and re-chmod'd **0600** on Unix
/// (never even briefly group/world-readable), and DACL-restricted to SYSTEM/Administrators/owner on
/// Windows (the default `%ProgramData%` ACL is Users-readable). Mirrors the mgmt-token hardening; used
/// for the host private key and the persisted trust stores so a local unprivileged user can neither
/// read the key (impersonation) nor tamper with the paired allow-list (unauthorized pairing).
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
    #[cfg(windows)]
    crate::windows::restrict_to_system_admins(path);
    Ok(())
}
