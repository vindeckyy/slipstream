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
    std::env::var(name).ok().map(|s| {
        !matches!(
            s.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "off" | "no"
        )
    })
}
