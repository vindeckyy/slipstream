//! Per-install-kind apply legs and the run-the-binary gate.

use std::path::Path;
use std::process::Command;

use crate::detect::PACMAN_OPTIN_CONF;
use crate::mode::Mode;
use crate::runutil::{installed_packages, run, run_capture};

/// The run-the-binary gate's reading: execute what we just installed and take its
/// `--version`. A binary that cannot run is an update that did NOT stick, whatever the
/// package manager reported (the 0.22.0 clobber lesson).
pub(crate) fn gate_version(mode: Mode) -> Result<String, String> {
    let bin = mode.gate_binary();
    run_capture(
        Command::new(bin).arg("--version"),
        &format!("{bin} --version"),
    )
}

/// The per-kind command tables (design §5). Returns `staged` (activation needs a reboot).
pub(crate) fn apply_for_kind(kind: &str) -> Result<bool, String> {
    match kind {
        "apt" => {
            // Refresh only OUR index when the documented list file exists (S5);
            // otherwise a full refresh — normal admin behavior, just slower.
            let ours = "/etc/apt/sources.list.d/slipstream.list";
            let mut update = Command::new("apt-get");
            update.env("DEBIAN_FRONTEND", "noninteractive");
            if Path::new(ours).exists() {
                update.args([
                    "update",
                    "-o",
                    &format!("Dir::Etc::sourcelist={ours}"),
                    "-o",
                    "Dir::Etc::sourceparts=-",
                ]);
            } else {
                update.arg("update");
            }
            run(&mut update, "apt-get update")?;
            let pkgs = installed_packages(
                Command::new("dpkg-query").args(["-W", "-f", "${Package}\n", "slipstream*"]),
                "dpkg-query",
            )?;
            let mut install = Command::new("apt-get");
            install
                .env("DEBIAN_FRONTEND", "noninteractive")
                .args(["install", "--only-upgrade", "-y"])
                .args(&pkgs);
            run(&mut install, "apt-get install --only-upgrade")?;
            Ok(false)
        }
        "dnf" => {
            let pkgs = installed_packages(
                Command::new("rpm").args(["-qa", "--qf", "%{NAME}\n", "slipstream*"]),
                "rpm -qa",
            )?;
            let mut upgrade = Command::new("dnf");
            upgrade.args(["-y", "upgrade"]).args(&pkgs);
            run(&mut upgrade, "dnf upgrade")?;
            Ok(false)
        }
        "rpm-ostree" => {
            // A layered package only re-resolves when forced — the single-transaction
            // uninstall+install dance (packaging/bazzite/update-slipstream.sh). Staged;
            // a reboot activates it.
            let pkgs = installed_packages(
                Command::new("rpm").args(["-qa", "--qf", "%{NAME}\n", "slipstream*"]),
                "rpm -qa",
            )?;
            run(
                Command::new("rpm-ostree").args(["refresh-md", "--force"]),
                "rpm-ostree refresh-md",
            )?;
            let mut update = Command::new("rpm-ostree");
            update.arg("update");
            for p in &pkgs {
                update.args(["--uninstall", p, "--install", p]);
            }
            run(&mut update, "rpm-ostree update (re-resolve)")?;
            Ok(true)
        }
        "sysext" => {
            // The proven signed-feed updater; it refreshes the merged /usr in place.
            run(
                Command::new("slipstream-sysext").arg("update"),
                "slipstream-sysext update",
            )?;
            Ok(false)
        }
        "pacman" => {
            // Arch doctrine: partial upgrades break boxes, so the ONLY thing this
            // helper will run is a full -Syu — and only when the operator opted into
            // that explicitly (root-owned config, not the API).
            let optin = std::fs::read_to_string(PACMAN_OPTIN_CONF)
                .ok()
                .map(|c| c.lines().any(|l| l.trim() == "PACMAN_FULL_SYSUPGRADE=1"))
                .unwrap_or(false);
            if !optin {
                return Err(format!(
                    "pacman full-sysupgrade is not opted in — set PACMAN_FULL_SYSUPGRADE=1 \
                     in {PACMAN_OPTIN_CONF} (this runs `pacman -Syu` for the WHOLE system)"
                ));
            }
            run(
                Command::new("pacman").args(["-Syu", "--noconfirm"]),
                "pacman -Syu",
            )?;
            Ok(false)
        }
        other => Err(format!("no apply leg for install kind {other}")),
    }
}
