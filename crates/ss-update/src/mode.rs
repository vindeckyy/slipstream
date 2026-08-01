//! Product mode (host vs client) — verb selects which marker and gate binary.

/// Which product this run was started for. It comes from the VERB in a root-owned unit's
/// fixed `ExecStart` — never from an unprivileged caller — so it stays inside the
/// zero-attacker-influenceable-parameters rule: the two units differ only in which
/// product's marker they read and which binary the run-the-binary gate executes.
///
/// Two units exist rather than one because the host and the client are separate packages
/// and every packaging format we ship treats two packages owning one path as a hard
/// conflict — a client-only box (a Steam Deck, a handheld) must be able to install the
/// helper without the host package.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Host,
    Client,
}

impl Mode {
    pub(crate) fn marker(self) -> &'static str {
        match self {
            Mode::Host => "/usr/share/slipstream/install-kind",
            Mode::Client => "/usr/share/slipstream-client/install-kind",
        }
    }

    pub(crate) fn sysext_marker(self) -> &'static str {
        match self {
            Mode::Host => "/usr/lib/extension-release.d/extension-release.slipstream",
            Mode::Client => "/usr/lib/extension-release.d/extension-release.slipstream-client",
        }
    }

    /// The binary the run-the-binary gate executes after a package-manager run.
    pub(crate) fn gate_binary(self) -> &'static str {
        match self {
            Mode::Host => "/usr/bin/slipstream-host",
            Mode::Client => "/usr/bin/slipstream-client",
        }
    }

    pub(crate) fn result_path(self) -> &'static str {
        match self {
            Mode::Host => "/var/lib/slipstream/update-result.json",
            Mode::Client => "/var/lib/slipstream/client-update-result.json",
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Mode::Host => "host",
            Mode::Client => "client",
        }
    }
}
