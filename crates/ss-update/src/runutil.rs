//! Command runners for package-manager legs.

use std::process::Command;

pub(crate) fn run(cmd: &mut Command, what: &str) -> Result<(), String> {
    println!("ss-update: running {cmd:?}");
    let status = cmd
        .status()
        .map_err(|e| format!("{what}: failed to launch: {e}"))?;
    if !status.success() {
        return Err(format!("{what}: exited {status}"));
    }
    Ok(())
}

pub(crate) fn run_capture(cmd: &mut Command, what: &str) -> Result<String, String> {
    let out = cmd
        .output()
        .map_err(|e| format!("{what}: failed to launch: {e}"))?;
    if !out.status.success() {
        return Err(format!("{what}: exited {}", out.status));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The installed slipstream packages, from the LOCAL package database — upgrade exactly
/// what this box has (host-only installs don't grow a web console out of nowhere).
pub(crate) fn installed_packages(query: &mut Command, what: &str) -> Result<Vec<String>, String> {
    let out = run_capture(query, what)?;
    let pkgs: Vec<String> = out
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("slipstream"))
        .map(str::to_string)
        .collect();
    if pkgs.is_empty() {
        return Err(format!("{what}: no installed slipstream packages found"));
    }
    Ok(pkgs)
}
