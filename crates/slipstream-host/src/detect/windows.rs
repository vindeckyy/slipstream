//! Windows conflicting-host facts: Toolhelp32 snapshot for running processes, well-known
//! install locations + `PATH` for install markers. Best-effort and dependency-free — anything
//! unreadable simply yields no evidence.
//!
//! Implemented with `forbid(unsafe_code)`-compatible std only for the baseline: process
//! enumeration via the `tasklist` helper is deliberately NOT used (slow, locale-fragile).
//! Until the Toolhelp FFI lands, `running_processes` reports what it can without spawning:
//! an empty vec means "could not tell", never "nothing is running" (see
//! [`super::running_process_names`]).

use super::{Evidence, Known};

/// Lowercased executable basenames of running processes.
///
/// Baseline returns an empty vec (unknown) — the Toolhelp32 snapshot FFI lands with the
/// platform todo. Callers must not read absence as proof (see
/// [`super::running_process_names`]).
pub fn running_processes() -> Vec<String> {
    Vec::new()
}

/// Install markers: well-known install locations + `PATH` binary presence.
pub fn static_evidence(known: &Known) -> Vec<Evidence> {
    let mut ev = Vec::new();
    let path = std::env::var_os("PATH");
    for bin in known.processes {
        if let Some(found) = find_on_path(bin, path.as_deref()) {
            ev.push(Evidence::Installed { at: found });
        }
    }
    ev
}

fn find_on_path(bin: &str, path: Option<&std::ffi::OsStr>) -> Option<String> {
    let dirs = path.map(std::env::split_paths).into_iter().flatten();
    for dir in dirs {
        for cand in [dir.join(format!("{bin}.exe")), dir.join(bin)] {
            if cand.is_file() {
                return Some(cand.to_string_lossy().into_owned());
            }
        }
    }
    // Default install roots even when PATH is narrow (e.g. a service context).
    for root in [
        std::path::PathBuf::from(r"C:\Program Files\Sunshine"),
        std::path::PathBuf::from(r"C:\Program Files\Apollo"),
    ] {
        let cand = root.join(format!("{bin}.exe"));
        if cand.is_file() {
            return Some(cand.to_string_lossy().into_owned());
        }
    }
    None
}
