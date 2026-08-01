/// Best-effort Windows DACL lockdown of the config *directory* (the companion to
/// [`restrict_to_system_admins`] for files). The default `%ProgramData%` ACL lets `BUILTIN\Users`
/// create subfolders/files (and become `CREATOR OWNER`), so a non-admin could pre-create the
/// `slipstream` dir or plant a `host.env`/`apps.json` that the privileged SYSTEM service then trusts
/// (LPE; security-review 2026-06-28 #3). This re-owns the dir to Administrators (defeating a
/// pre-creation), strips inheritance, and sets an explicit DACL: SYSTEM/Administrators/OWNER full
/// (object+container inherit so child files/dirs inherit it), and Users **read-only** (so existing
/// reads of non-secret config keep working but a local user can no longer write/plant). Secret files
/// are additionally locked to SYSTEM/Admins by [`crate::write_secret_file`]. Hard-coded SIDs
/// (locale-independent) via the absolute `%SystemRoot%` path; never fatal.
pub(crate) fn restrict_dir_to_system_admins(dir: &std::path::Path) {
    let icacls = std::env::var("SystemRoot")
        .map(|r| format!("{r}\\System32\\icacls.exe"))
        .unwrap_or_else(|_| "icacls".to_string());
    // Reset ownership of the directory object to Administrators first, so a dir a non-admin may have
    // pre-created can't keep OWNER control (an owner can always rewrite the DACL). No `/T` — re-owning
    // the dir itself is what defeats the pre-creation; recursing a large captures tree each call is
    // needless churn (secret files are individually owner-locked by `write_secret_file`).
    let _ = std::process::Command::new(&icacls)
        .arg(dir.as_os_str())
        .args(["/setowner", "*S-1-5-32-544"]) // BUILTIN\Administrators
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    let status = std::process::Command::new(&icacls)
        .arg(dir.as_os_str())
        .args([
            "/inheritance:r",
            "/grant:r",
            "*S-1-5-18:(OI)(CI)(F)", // NT AUTHORITY\SYSTEM
            "/grant:r",
            "*S-1-5-32-544:(OI)(CI)(F)", // BUILTIN\Administrators
            "/grant:r",
            "*S-1-3-4:(OI)(CI)(F)", // OWNER RIGHTS
            "/grant:r",
            "*S-1-5-32-545:(OI)(CI)(RX)", // BUILTIN\Users — read-only (no create/write → no plant)
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() => {}
        _ => tracing::warn!(
            dir = %dir.display(),
            "config-dir DACL hardening did not fully succeed — a local user may be able to plant config files"
        ),
    }
}

/// Best-effort Windows DACL lockdown of a secret file: strip inherited ACEs and grant Full only to
/// SYSTEM, Administrators, and OWNER RIGHTS (the creating account — the SYSTEM service or a manually
/// running user keeps access). Without this the host key under the default Users-readable
/// `%ProgramData%` ACL is readable by ANY local user. Uses `icacls` with hard-coded SIDs
/// (locale-independent) via the absolute `%SystemRoot%` path (a privileged service must not trust
/// `PATH`). Never fatal — on failure the file is simply left at the inherited ACL (today's behaviour).
pub(crate) fn restrict_to_system_admins(path: &std::path::Path) {
    let icacls = std::env::var("SystemRoot")
        .map(|r| format!("{r}\\System32\\icacls.exe"))
        .unwrap_or_else(|_| "icacls".to_string());
    let status = std::process::Command::new(icacls)
        .arg(path.as_os_str())
        .args([
            "/inheritance:r",
            "/grant:r",
            "*S-1-5-18:(F)", // NT AUTHORITY\SYSTEM
            "/grant:r",
            "*S-1-5-32-544:(F)", // BUILTIN\Administrators
            "/grant:r",
            "*S-1-3-4:(F)", // OWNER RIGHTS
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    match status {
        Ok(s) if s.success() => {}
        _ => tracing::warn!(
            path = %path.display(),
            "icacls hardening did not succeed — this secret may be readable by other local users"
        ),
    }
}
