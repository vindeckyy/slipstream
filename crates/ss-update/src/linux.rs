//! Linux root-helper entry for `ss-update`.

use crate::apply::{apply_for_kind, gate_version};
use crate::detect::detect_kind;
use crate::mode::Mode;
use crate::result::{now_unix, write_result, HelperResult};

pub fn main() {
    let arg = std::env::args().nth(1).unwrap_or_default();
    let mode = match arg.as_str() {
        "apply" => Mode::Host,
        "apply-client" => Mode::Client,
        _ => {
            eprintln!(
                "usage: ss-update apply | apply-client   (normally via \
                 slipstream-update.service / slipstream-client-update.service)"
            );
            std::process::exit(2);
        }
    };
    // Effective root is required for every leg; refuse early with a clear message
    // rather than half-running.
    // SAFETY: geteuid has no preconditions.
    if unsafe { libc_geteuid() } != 0 {
        eprintln!("ss-update: must run as root (start slipstream-update.service)");
        std::process::exit(1);
    }

    let kind = match detect_kind(mode) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("ss-update: {e}");
            write_result(
                mode,
                &HelperResult {
                    ok: false,
                    kind: "unknown".into(),
                    before_version: String::new(),
                    after_version: String::new(),
                    changed: false,
                    staged: false,
                    error: Some(e),
                    finished_unix: now_unix(),
                },
            );
            std::process::exit(1);
        }
    };
    println!("ss-update: {} install kind {kind}", mode.as_str());
    let before = gate_version(mode).unwrap_or_default();

    let outcome = apply_for_kind(kind).and_then(|staged| {
        // The run-the-binary gate: the freshly installed binary must actually run.
        // Skipped for staged (rpm-ostree) — the new binary isn't in /usr until reboot.
        let after = if staged {
            before.clone()
        } else {
            gate_version(mode)
                .map_err(|e| format!("run-the-binary gate: {e} — the update did NOT stick"))?
        };
        Ok((staged, after))
    });

    let result = match outcome {
        Ok((staged, after)) => HelperResult {
            ok: true,
            kind: kind.into(),
            changed: staged || after != before,
            staged,
            before_version: before,
            after_version: after,
            error: None,
            finished_unix: now_unix(),
        },
        Err(e) => {
            eprintln!("ss-update: {e}");
            HelperResult {
                ok: false,
                kind: kind.into(),
                before_version: before.clone(),
                after_version: before,
                changed: false,
                staged: false,
                error: Some(e),
                finished_unix: now_unix(),
            }
        }
    };
    let ok = result.ok;
    write_result(mode, &result);
    println!(
        "ss-update: {} ({} -> {}, changed: {}, staged: {})",
        if ok { "ok" } else { "FAILED" },
        result.before_version,
        result.after_version,
        result.changed,
        result.staged,
    );
    std::process::exit(if ok { 0 } else { 1 });
}


// One libc symbol, declared directly — not worth a libc dependency in a root helper.
extern "C" {
    #[link_name = "geteuid"]
    fn libc_geteuid() -> u32;
}
