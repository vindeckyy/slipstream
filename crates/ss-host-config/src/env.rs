/// Whether a `SLIPSTREAM_*` env var reads as ON, or `None` when it is unset — the host's
/// **explicit-off** grammar: `0` / `false` / `off` / `no` (trimmed, case-insensitive) are off and ANY
/// other value is on, so a presence-style `=1` keeps working. Every "default ON" knob below shares
/// it.
///
/// Exported because callers in other crates need the SAME grammar. A hand-rolled
/// `var(k).as_deref() != Ok("0")` accepts `"0 "` (trailing space, trivially produced by a systemd
/// drop-in or a shell heredoc) and `"false"` as ON — the bug class of ed525c4c, and the reason
/// `SLIPSTREAM_PIPEWIRE_NV12` in ss-capture now routes through here.
///
/// Note this is deliberately NOT the grammar `ss-zerocopy` uses for its own flags (truthy:
/// `1|true|yes|on`, everything else off) — see the module docs: independent features that share a
/// name prefix.
pub fn env_on(name: &str) -> Option<bool> {
    parse_env_on(std::env::var(name).ok().as_deref())
}

/// Pure explicit-off parse for tests and callers that already hold the raw value.
///
/// `None` (unset) stays `None` so default-ON vs default-OFF knobs can `unwrap_or` themselves.
pub fn parse_env_on(raw: Option<&str>) -> Option<bool> {
    raw.map(|s| {
        !matches!(
            s.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "off" | "no"
        )
    })
}

/// Default-ON policy gate: unset → on; explicit off grammar → off; any other value → on.
pub fn default_on_gate(raw: Option<&str>) -> bool {
    parse_env_on(raw).unwrap_or(true)
}
