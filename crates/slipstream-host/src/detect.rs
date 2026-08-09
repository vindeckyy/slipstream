//! Conflicting game-streaming host detection.
//!
//! Slipstream is one of a family of Moonlight-compatible desktop-streaming hosts. The others —
//! Sunshine and its many forks (Apollo, Vibeshine, Vibepollo, LuminalShine, …) — all impersonate
//! NVIDIA GameStream: they bind the **same** ports (47984/47989 nvhttp, 47998-48010 stream,
//! 47990 web UI — which is also our management API), advertise the **same** `_nvstream._tcp`
//! mDNS service, and frequently install a **conflicting virtual-display driver**. Running one of
//! them alongside Slipstream is unsupported — the symptoms are `address already in use` bind
//! failures, pairing that silently fails, and capture/virtual-display glitches.
//!
//! This module proactively detects such a host (installed and/or running) so we can surface it as
//! early as possible: at host startup (a `warn!` into the log ring + tray/console summary) and via
//! the `detect-conflicts` support subcommand.
//!
//! Detection is **fingerprint-first by name**: a small table of the known products (extend
//! [`KNOWN`] as new forks appear) matched against running processes, systemd units, and on-disk
//! install markers. The Linux backend provides the raw facts; the matching and rendering here are
//! unit-tested.

use std::sync::OnceLock;

/// Lowercased executable basenames (no `.exe`) of every running process — the same snapshot the
/// conflicting-host scan uses, exposed for the few callers that need to ask "is X running?"
/// without duplicating a Toolhelp walk. Best-effort: an empty vec means "could not tell", never
/// "nothing is running", so callers must not read absence as proof.
pub(crate) fn running_process_names() -> Vec<String> {
    platform::running_processes()
}

#[cfg(target_os = "linux")]
#[path = "detect/linux.rs"]
mod platform;

/// A known competing GameStream/Moonlight host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Product {
    Sunshine,
    Apollo,
    Vibeshine,
    Vibepollo,
    Luminalshine,
}

impl Product {
    /// The name shown to the user.
    pub fn label(self) -> &'static str {
        match self {
            Product::Sunshine => "Sunshine",
            Product::Apollo => "Apollo",
            Product::Vibeshine => "Vibeshine",
            Product::Vibepollo => "Vibepollo",
            Product::Luminalshine => "LuminalShine",
        }
    }
}

/// How a conflicting host was observed on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Evidence {
    /// A matching process is running **right now** (process/executable basename).
    Running { process: String },
    /// An OS service / systemd unit for the product is registered (installed; may be stopped).
    Service { name: String },
    /// Installed on disk — a Flatpak app id or a binary on `PATH`.
    Installed { at: String },
}

impl Evidence {
    fn render(&self) -> String {
        match self {
            Evidence::Running { process } => format!("running now ({process})"),
            Evidence::Service { name } => format!("service {name}"),
            Evidence::Installed { at } => format!("installed at {at}"),
        }
    }
}

/// A detected conflicting host, with every piece of corroborating evidence found.
#[derive(Clone, Debug)]
pub struct Detection {
    pub product: Product,
    pub evidence: Vec<Evidence>,
}

impl Detection {
    /// True when a matching process is live — the acute case (a guaranteed resource clash the
    /// moment Slipstream tries to bind its ports).
    pub fn is_running(&self) -> bool {
        self.evidence
            .iter()
            .any(|e| matches!(e, Evidence::Running { .. }))
    }

    /// A compact one-line label for the tray/console summary, e.g. `Sunshine (running)`.
    pub fn label(&self) -> String {
        if self.is_running() {
            format!("{} (running)", self.product.label())
        } else {
            self.product.label().to_string()
        }
    }
}

/// One row of the known-conflicting-host table. Names are matched case-insensitively; process /
/// binary basenames are given **without** an extension (the platform code lowercases + strips
/// `.exe`). Extend this as new Sunshine forks appear — the runtime, the subcommand, and the
/// tray/console summary all key off this one list.
pub struct Known {
    pub product: Product,
    /// Process / executable basenames (lowercase, no extension) that identify this host.
    pub processes: &'static [&'static str],
    /// Linux systemd unit basenames (without `.service`), checked in the standard unit dirs.
    pub linux_units: &'static [&'static str],
    /// Linux flatpak application ids.
    pub flatpaks: &'static [&'static str],
}

/// The known Moonlight-compatible hosts that clash with Slipstream. All are Sunshine or Sunshine
/// forks; add new forks here (one row) and every surface picks them up.
pub const KNOWN: &[Known] = &[
    Known {
        product: Product::Sunshine,
        processes: &["sunshine"],
        linux_units: &["sunshine"],
        flatpaks: &["dev.lizardbyte.app.Sunshine"],
    },
    Known {
        product: Product::Apollo,
        processes: &["apollo"],
        linux_units: &["apollo"],
        flatpaks: &["dev.lizardbyte.app.Apollo"],
    },
    Known {
        product: Product::Vibeshine,
        processes: &["vibeshine"],
        linux_units: &["vibeshine"],
        flatpaks: &[],
    },
    Known {
        product: Product::Vibepollo,
        processes: &["vibepollo"],
        linux_units: &["vibepollo"],
        flatpaks: &[],
    },
    Known {
        product: Product::Luminalshine,
        processes: &["luminalshine"],
        linux_units: &["luminalshine"],
        flatpaks: &[],
    },
];

/// Why running side-by-side breaks, shared by every surface (log and support subcommand).
pub const UNSUPPORTED_BLURB: &str =
    "Running Slipstream alongside another Moonlight-compatible host \
(Sunshine and its forks) is UNSUPPORTED: they bind the same GameStream ports (47984/47989, \
47998-48010), advertise the same _nvstream mDNS name, and often install a conflicting \
virtual-display driver. Expect \"address already in use\" errors, failed pairing, and capture \
glitches. Stop and uninstall the other host, or don't run them at the same time.";

/// Scan the machine for conflicting hosts. Portable; dispatches into the platform back-end. Does
/// real OS work (process enumeration, systemd and filesystem checks) — cheap, but not
/// free, so use [`init`]/[`snapshot`] for the startup report and
/// [`current_summary_labels`] for the live status path.
pub fn scan() -> Vec<Detection> {
    let procs = platform::running_processes();
    let mut out = Vec::new();
    for known in KNOWN {
        let mut evidence: Vec<Evidence> = Vec::new();
        for p in &procs {
            if known.processes.iter().any(|n| p == n) {
                evidence.push(Evidence::Running { process: p.clone() });
            }
        }
        evidence.extend(platform::static_evidence(known));
        if !evidence.is_empty() {
            out.push(Detection {
                product: known.product,
                evidence,
            });
        }
    }
    out
}

static SNAPSHOT: OnceLock<Vec<Detection>> = OnceLock::new();

/// Scan once and cache the result for the life of the process. The snapshot is used by the startup
/// report; the management summary uses a fresh process-only scan so a host that stops later does
/// not remain reported as running.
pub fn init() -> &'static [Detection] {
    SNAPSHOT.get_or_init(scan)
}

/// The cached snapshot, or empty if [`init`] hasn't run. Non-scanning: safe to call from hot paths
/// and from tests without touching the OS.
pub fn snapshot() -> &'static [Detection] {
    SNAPSHOT.get().map(Vec::as_slice).unwrap_or(&[])
}

/// Compact labels for the tray / web-console summary: **running detections only**.
///
/// Install-only / service-registered evidence stays in [`scan`] / [`render_report`] for
/// `detect-conflicts`; the console field means "another server is running", so
/// static evidence must not leak through here. This filters the detections it is given; callers
/// that need current state should use [`current_summary_labels`].
pub fn summary_labels(detections: &[Detection]) -> Vec<String> {
    detections
        .iter()
        .filter(|d| d.is_running())
        .map(Detection::label)
        .collect()
}

/// Compact labels for compatible servers whose process is running now.
///
/// Unlike [`summary_labels`], this does not use the startup [`snapshot`]. It performs only the
/// current process scan and deliberately omits installed/service evidence, so a stopped but
/// installed Sunshine cannot keep the web console warning alive.
pub fn current_summary_labels() -> Vec<String> {
    running_summary_labels(&running_process_names())
}

fn running_summary_labels(processes: &[String]) -> Vec<String> {
    KNOWN
        .iter()
        .filter_map(|known| {
            processes
                .iter()
                .find(|process| known.processes.iter().any(|name| process == name))
                .map(|process| Detection {
                    product: known.product,
                    evidence: vec![Evidence::Running {
                        process: process.clone(),
                    }],
                })
        })
        .map(|detection| detection.label())
        .collect()
}

/// A full human-readable report: the blurb + one bullet per detected host with its evidence.
/// Empty string when nothing was detected (callers gate on `is_empty()`).
pub fn render_report(detections: &[Detection]) -> String {
    if detections.is_empty() {
        return String::new();
    }
    let mut s = String::from("Detected another game-streaming host on this machine.\n");
    s.push_str(UNSUPPORTED_BLURB);
    s.push_str("\n\nDetected:\n");
    for d in detections {
        let ev = d
            .evidence
            .iter()
            .map(Evidence::render)
            .collect::<Vec<_>>()
            .join("; ");
        s.push_str(&format!("  \u{2022} {} \u{2014} {ev}\n", d.product.label()));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn det(product: Product, evidence: Vec<Evidence>) -> Detection {
        Detection { product, evidence }
    }

    #[test]
    fn empty_report_and_labels() {
        assert!(render_report(&[]).is_empty());
        assert!(summary_labels(&[]).is_empty());
    }

    #[test]
    fn running_detection_is_flagged_and_labelled() {
        let d = det(
            Product::Sunshine,
            vec![
                Evidence::Running {
                    process: "sunshine".into(),
                },
                Evidence::Service {
                    name: "sunshine.service".into(),
                },
            ],
        );
        assert!(d.is_running());
        assert_eq!(d.label(), "Sunshine (running)");
    }

    #[test]
    fn installed_only_is_not_running() {
        let d = det(
            Product::Apollo,
            vec![Evidence::Installed {
                at: "/usr/bin/apollo".into(),
            }],
        );
        assert!(!d.is_running());
        assert_eq!(d.label(), "Apollo");
    }

    #[test]
    fn report_lists_every_product_and_the_blurb() {
        let report = render_report(&[
            det(
                Product::Sunshine,
                vec![Evidence::Running {
                    process: "sunshine".into(),
                }],
            ),
            det(
                Product::Apollo,
                vec![Evidence::Installed {
                    at: "/usr/bin/apollo".into(),
                }],
            ),
        ]);
        assert!(report.contains("UNSUPPORTED"));
        assert!(report.contains("Sunshine \u{2014} running now (sunshine)"));
        assert!(report.contains("Apollo \u{2014} installed at /usr/bin/apollo"));
    }

    #[test]
    fn summary_labels_only_include_running() {
        let running = det(
            Product::Sunshine,
            vec![
                Evidence::Running {
                    process: "sunshine".into(),
                },
                Evidence::Installed {
                    at: "/usr/bin/sunshine".into(),
                },
            ],
        );
        let installed_only = det(
            Product::Apollo,
            vec![Evidence::Installed {
                at: "/usr/bin/apollo".into(),
            }],
        );
        let service_only = det(
            Product::Vibeshine,
            vec![Evidence::Service {
                name: "vibeshine.service".into(),
            }],
        );
        assert_eq!(
            summary_labels(&[running, installed_only.clone(), service_only.clone()]),
            vec!["Sunshine (running)".to_string()]
        );
        assert!(summary_labels(&[installed_only, service_only]).is_empty());
    }

    #[test]
    fn live_summary_drops_a_process_after_it_stops() {
        assert_eq!(
            running_summary_labels(&["sunshine".into()]),
            vec!["Sunshine (running)".to_string()]
        );
        assert!(
            running_summary_labels(&[]).is_empty(),
            "an old process observation must not survive a fresh empty process scan"
        );
    }

    #[test]
    fn known_table_rows_are_well_formed() {
        // Every known product carries a process name and a Linux unit so the runtime scan and
        // static evidence check stay in agreement.
        for k in KNOWN {
            assert!(
                !k.processes.is_empty(),
                "{:?} has no process name",
                k.product
            );
            assert!(
                !k.linux_units.is_empty(),
                "{:?} has no Linux unit",
                k.product
            );
        }
    }
}
