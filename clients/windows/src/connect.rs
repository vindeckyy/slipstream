//! The client's decision layer: what a `_slipstream._udp` TXT record means, whether a handshake
//! result may stream, and the connection phase machine. Pure — no sockets, no window.
//!
//! These mirror the two seams `clients/apple` extracted (`HostAdvert`, `ConnectOutcome`) so the
//! same rules are reviewable and testable on every platform instead of being inline in the shell.

// The full admission table is exercised by the tests below and consumed by the window backend in
// the next change; not every branch is reachable from today's CLI, which is by design.
#![allow(dead_code)]

/// The host's advertised pairing policy. FAIL-CLOSED: anything that is not a literal
/// `pair=optional` means pairing is required, because a missing TXT key on an old host is
/// indistinguishable from an attacker's stripped one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingPolicy {
    Required,
    Optional,
}

impl PairingPolicy {
    pub fn from_txt(v: &str) -> Self {
        if v.trim().eq_ignore_ascii_case("optional") {
            Self::Optional
        } else {
            Self::Required
        }
    }
}

/// A parsed `_slipstream._udp` advert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostAdvert {
    pub id: String,
    /// Cert SHA-256, hex, lowercased. Advisory only (mDNS is unauthenticated) — it is what the
    /// user is shown to compare and pin, never a trust decision on its own.
    pub fp: String,
    pub pair: PairingPolicy,
    pub macs: Vec<String>,
    pub os: Vec<String>,
}

impl HostAdvert {
    /// Decode the TXT key/value pairs. Unknown keys are ignored; duplicate keys keep the first.
    pub fn from_txt(entries: &[(String, String)]) -> Self {
        let get = |key: &str| {
            entries
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v.trim().to_string())
                .unwrap_or_default()
        };
        let list = |key: &str| -> Vec<String> {
            get(key)
                .split([',', ';'])
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect()
        };
        Self {
            id: get("id"),
            fp: get("fp").to_ascii_lowercase(),
            pair: PairingPolicy::from_txt(&get("pair")),
            macs: list("mac"),
            os: list("os"),
        }
    }

    /// Whether this advert offers trust-on-first-use. Only an explicit `pair=optional` does.
    pub fn allows_tofu(&self) -> bool {
        self.pair == PairingPolicy::Optional
    }
}

/// The post-handshake decision (the `ConnectionAdmission.decide` table on Apple).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// Pinned, or the host already trusts this device: start streaming.
    Stream,
    /// Unpinned, but the host offers TOFU: show the fingerprint and stream once confirmed.
    ConfirmTrust,
    /// Unpinned and the host requires pairing: refuse and drop the connection.
    RefuseUnpaired,
}

/// `pinned` = the operator supplied (or a previous pairing stored) the host fingerprint.
/// `advert` is the advert used to reach the host; `None` for manual `ip:port` entry, where there
/// is no advert to consult.
/// `manual_entry` = the operator typed the address. There is no advert, so the certificate the
/// host presents is the only anchor: gate it behind the explicit confirmation card rather than
/// letting it stream. This is the sole trust decision in the client — core's `client_pinned(None)`
/// accepts any certificate by design, so nothing else can enforce it.
pub fn decide(pinned: bool, advert: Option<&HostAdvert>, manual_entry: bool) -> Admission {
    if pinned {
        return Admission::Stream;
    }
    if manual_entry {
        return Admission::ConfirmTrust;
    }
    match advert {
        Some(a) if a.allows_tofu() => Admission::ConfirmTrust,
        _ => Admission::RefuseUnpaired,
    }
}

impl Admission {
    /// A refused connection must be closed before the UI returns to idle, or the next connect
    /// races the previous socket.
    pub fn requires_drop(&self) -> bool {
        matches!(self, Admission::RefuseUnpaired)
    }

    pub fn message(&self, host: &str) -> String {
        match self {
            Admission::Stream => format!("streaming from {host}"),
            Admission::ConfirmTrust => {
                format!("{host} is not paired yet; confirm its fingerprint to trust it")
            }
            Admission::RefuseUnpaired => {
                format!("{host} is not paired yet. Pair with its PIN (--pair) before streaming.")
            }
        }
    }
}

/// How a connect attempt ended, classified into what the user can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectOutcome {
    /// Handshake + admission succeeded.
    Streaming,
    /// The host rejected us with one of its `REJECTED_*` statuses.
    Rejected(u32),
    /// No QUIC handshake: wrong pin, wrong address, or the host is not running.
    Unreachable,
    /// A pin mismatch is reported separately: it means the host is reachable but not who you
    /// pinned, which is the one failure a user must not retry blindly.
    PinMismatch,
    /// The 180 s delegated-approval window elapsed with no console decision.
    ApprovalTimeout,
}

impl ConnectOutcome {
    /// Classify a status code from the host's rejection frame.
    pub fn from_status(code: u32) -> Self {
        match code {
            // REJECTED_APPROVAL_TIMEOUT is stable in the pairing contract (`api/openapi.json`).
            0x0107 => Self::ApprovalTimeout,
            _ => Self::Rejected(code),
        }
    }

    pub fn message(&self, host: &str) -> String {
        match self {
            Self::Streaming => format!("connected to {host}"),
            Self::Rejected(code) => format!("{host} rejected the session (status 0x{code:04x})"),
            Self::Unreachable => {
                format!("cannot reach {host}: is slipstream-host running and is the port open?")
            }
            Self::PinMismatch => format!(
                "{host} presented a different certificate than the one pinned. \
                 Clear the pin only if the host was reinstalled."
            ),
            Self::ApprovalTimeout => format!(
                "{host} never approved this device in its console (180 s window elapsed); \
                 approve it there, then connect again"
            ),
        }
    }
}

/// The connection phase machine. Every transition in the sibling clients' tables is represented
/// here so a refactor cannot silently leave a connection open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Connecting,
    /// Admitted, but waiting for the user to accept the fingerprint (TOFU).
    AwaitingTrust,
    Streaming,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transition {
    /// Stay in the new phase; `message` is what the UI shows.
    Stay(Phase, String),
    /// Drop the live connection, then go idle.
    DropAndIdle(String),
}

#[derive(Debug, Clone)]
pub struct Machine {
    pub phase: Phase,
    pub host: Option<String>,
}

impl Default for Machine {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            host: None,
        }
    }
}

impl Machine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Only idle can start a connect: a double-click must not open two sessions.
    pub fn connect(&mut self, host: &str) -> Result<(), String> {
        if self.phase != Phase::Idle {
            return Err(format!("already {:?}", self.phase));
        }
        self.phase = Phase::Connecting;
        self.host = Some(host.to_string());
        Ok(())
    }

    pub fn admitted(&mut self, admission: Admission) -> Transition {
        let host = self.host.clone().unwrap_or_else(|| "host".into());
        match admission {
            Admission::Stream => {
                self.phase = Phase::Streaming;
                Transition::Stay(Phase::Streaming, admission.message(&host))
            }
            Admission::ConfirmTrust => {
                self.phase = Phase::AwaitingTrust;
                Transition::Stay(Phase::AwaitingTrust, admission.message(&host))
            }
            Admission::RefuseUnpaired => {
                self.phase = Phase::Idle;
                self.host = None;
                Transition::DropAndIdle(admission.message(&host))
            }
        }
    }

    /// The user confirmed the fingerprint on the TOFU card.
    pub fn trusted(&mut self) -> Result<Transition, String> {
        if self.phase != Phase::AwaitingTrust {
            return Err(format!(
                "no pending trust decision (phase {:?})",
                self.phase
            ));
        }
        self.phase = Phase::Streaming;
        Ok(Transition::Stay(Phase::Streaming, "trusted".into()))
    }

    /// True between `admitted(ConfirmTrust)` and `trusted()`. The caller must not stream or enable
    /// input while this holds.
    pub fn awaiting_trust(&self) -> bool {
        self.phase == Phase::AwaitingTrust
    }

    pub fn failed(&mut self, outcome: &ConnectOutcome) -> Transition {
        let host = self.host.clone().unwrap_or_else(|| "host".into());
        self.phase = Phase::Idle;
        self.host = None;
        Transition::DropAndIdle(outcome.message(&host))
    }

    pub fn disconnected(&mut self) -> Transition {
        self.phase = Phase::Idle;
        self.host = None;
        Transition::DropAndIdle("session ended".into())
    }

    /// Input is only ever sent while streaming — never while connecting, awaiting trust, or after
    /// the connection was refused.
    pub fn can_send_input(&self) -> bool {
        self.phase == Phase::Streaming
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn txt(pairs: &[(&str, &str)]) -> HostAdvert {
        let entries: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        HostAdvert::from_txt(&entries)
    }

    #[test]
    fn realistic_advert_parses() {
        let a = txt(&[
            ("proto", "slipstream/1"),
            ("id", "8c1f"),
            ("fp", "  ABCD1234 "),
            ("mac", "aa-bb, cc-dd;ee-ff"),
            ("os", "Windows 11,Arch=x86_64"),
            ("pair", "required"),
        ]);
        assert_eq!(a.id, "8c1f");
        assert_eq!(a.fp, "abcd1234", "fp is trimmed and lowercased");
        assert_eq!(a.macs, vec!["aa-bb", "cc-dd", "ee-ff"]);
        assert_eq!(a.os, vec!["windows 11", "arch=x86_64"]);
        assert!(!a.allows_tofu());
    }

    #[test]
    fn pairing_policy_is_fail_closed() {
        for v in ["", "REQUIRED", "yes", "0", "garbage"] {
            assert_eq!(
                txt(&[("pair", v)]).pair,
                PairingPolicy::Required,
                "pair={v:?} must require pairing"
            );
        }
        assert!(txt(&[("pair", "Optional")]).allows_tofu());
        assert!(txt(&[("pair", " optional ")]).allows_tofu());
        // A missing key is required, not optional.
        assert_eq!(txt(&[]).pair, PairingPolicy::Required);
    }

    #[test]
    fn missing_keys_are_empty_not_panicky() {
        let a = txt(&[]);
        assert!(a.id.is_empty() && a.fp.is_empty() && a.macs.is_empty());
    }

    #[test]
    fn admission_table() {
        let tofu = txt(&[("pair", "optional")]);
        let strict = txt(&[("pair", "required")]);
        assert_eq!(decide(true, Some(&strict), false), Admission::Stream);
        assert_eq!(decide(true, None, true), Admission::Stream);
        assert_eq!(decide(false, Some(&tofu), false), Admission::ConfirmTrust);
        assert_eq!(
            decide(false, Some(&strict), false),
            Admission::RefuseUnpaired
        );
        // Manual `ip:port` has no advert, so the presented certificate is the only anchor. It must
        // reach the confirmation card, never stream unattended.
        assert_eq!(decide(false, None, true), Admission::ConfirmTrust);
        assert_eq!(decide(false, None, false), Admission::RefuseUnpaired);
        assert!(Admission::RefuseUnpaired.requires_drop());
        assert!(!Admission::Stream.requires_drop());
    }

    #[test]
    fn awaiting_trust_blocks_input_until_confirmed() {
        let mut m = Machine::new();
        assert_eq!(m.connect("h:9777"), Ok(()));
        m.admitted(Admission::ConfirmTrust);
        assert!(m.awaiting_trust());
        assert!(!m.can_send_input(), "no input before the user confirms");
        assert!(m.trusted().is_ok());
        assert!(m.can_send_input());
        assert!(!m.awaiting_trust());
    }

    #[test]
    fn machine_happy_path() {
        let mut m = Machine::new();
        assert_eq!(m.connect("h:9777"), Ok(()));
        assert_eq!(m.phase, Phase::Connecting);
        assert!(!m.can_send_input(), "no input before admission");
        assert_eq!(
            m.admitted(Admission::Stream),
            Transition::Stay(Phase::Streaming, "streaming from h:9777".into())
        );
        assert!(m.can_send_input());
        assert_eq!(m.phase, Phase::Streaming);
    }

    #[test]
    fn double_connect_is_refused() {
        let mut m = Machine::new();
        m.connect("h").unwrap();
        assert!(m.connect("h").is_err());
    }

    #[test]
    fn refused_connection_drops_and_goes_idle() {
        let mut m = Machine::new();
        m.connect("h").unwrap();
        let t = m.admitted(Admission::RefuseUnpaired);
        assert!(matches!(t, Transition::DropAndIdle(_)));
        assert_eq!(m.phase, Phase::Idle);
        assert!(m.host.is_none());
        assert!(!m.can_send_input());
    }

    #[test]
    fn tofu_waits_for_confirmation() {
        let mut m = Machine::new();
        m.connect("h").unwrap();
        assert_eq!(
            m.admitted(Admission::ConfirmTrust),
            Transition::Stay(
                Phase::AwaitingTrust,
                "h is not paired yet; confirm its fingerprint to trust it".into()
            )
        );
        assert!(!m.can_send_input());
        assert!(m.trusted().is_ok());
        assert_eq!(m.phase, Phase::Streaming);
        // Only valid while a decision is pending.
        assert!(m.trusted().is_err());
    }

    #[test]
    fn failures_return_to_idle_with_actionable_text() {
        let mut m = Machine::new();
        m.connect("100.112.143.99:9777").unwrap();
        let t = m.failed(&ConnectOutcome::ApprovalTimeout);
        match t {
            Transition::DropAndIdle(msg) => {
                assert!(msg.contains("180 s") && msg.contains("console"))
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(m.phase, Phase::Idle);

        let msg = ConnectOutcome::PinMismatch.message("h");
        assert!(msg.contains("different certificate"));
        assert!(ConnectOutcome::Unreachable
            .message("h")
            .contains("is slipstream-host running"));
        assert_eq!(
            ConnectOutcome::from_status(0x0107),
            ConnectOutcome::ApprovalTimeout
        );
        assert_eq!(
            ConnectOutcome::from_status(0x0103),
            ConnectOutcome::Rejected(0x0103)
        );
    }

    #[test]
    fn disconnect_from_streaming_is_clean() {
        let mut m = Machine::new();
        m.connect("h").unwrap();
        m.admitted(Admission::Stream);
        assert!(matches!(m.disconnected(), Transition::DropAndIdle(_)));
        assert_eq!(m.phase, Phase::Idle);
    }

    #[test]
    fn every_admission_message_names_the_host() {
        for a in [
            Admission::Stream,
            Admission::ConfirmTrust,
            Admission::RefuseUnpaired,
        ] {
            assert!(a.message("box").contains("box"));
        }
    }
}
