//! Root-owned install-kind detection.

use std::path::Path;

use crate::mode::Mode;

pub(crate) const OSTREE_BOOTED: &str = "/run/ostree-booted";
pub(crate) const PACMAN_OPTIN_CONF: &str = "/etc/slipstream/update.conf";

/// Root-owned facts → the apply strategy. Mirrors the host's ladder for the kinds a
/// root helper serves (the helper decides for ITSELF — never trusts its caller).
pub(crate) fn detect_kind(mode: Mode) -> Result<&'static str, String> {
    if Path::new(mode.sysext_marker()).exists() {
        return match mode {
            Mode::Host => Ok("sysext"),
            // `slipstream-sysext update` pulls the HOST image from the signed feed; there
            // is no client feed to pull from (a client sysext is the local
            // packaging/arch/build-sysext.sh wrapper). Refusing here is the honest answer
            // — the alternative would install the host over a client-only box.
            Mode::Client => Err(
                "this client is a sysext, and the sysext feed carries the host image only \
                 — rebuild and re-install the client image instead"
                    .to_string(),
            ),
        };
    }
    let marker_path = mode.marker();
    let marker = std::fs::read_to_string(marker_path)
        .map_err(|e| format!("no install-kind marker at {marker_path}: {e}"))?;
    match marker.split_whitespace().next() {
        Some("apt") => Ok("apt"),
        Some("dnf") if Path::new(OSTREE_BOOTED).exists() => Ok("rpm-ostree"),
        Some("dnf") => Ok("dnf"),
        Some("pacman") => Ok("pacman"),
        other => Err(format!(
            "install-kind marker says {other:?} — no root apply leg for it"
        )),
    }
}
