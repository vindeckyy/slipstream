//! CLI argument parsing for the Windows client. Pure: no sockets, no env beyond the default name,
//! so every rule below is directly testable (`cargo test -p slipstream-windows`).
//!
//! Flags mirror the sibling clients and the probe so an operator's muscle memory transfers:
//! `--connect HOST[:PORT]`, `--discover [SECS]`, `--pair PIN`, `--pin HEX`, `--name LABEL`,
//! `--mode WxHxFPS`, `--bitrate KBPS`, `--codec auto|h264|hevc|av1`, `--seconds N`,
//! `--record FILE`, `--play FILE`.

use slipstream_core::Mode;

/// The host's data-plane port is always 9777; a bare `--connect 10.0.0.5` means that port.
pub const DEFAULT_PORT: u16 = 9777;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    /// `--connect HOST[:PORT]` (mutually exclusive with `--discover` / `--pair`).
    pub connect: Option<String>,
    /// `--discover [SECS]`: browse for `_slipstream._udp` and exit.
    pub discover: Option<u64>,
    /// `--pair PIN`: run the SPAKE2 pairing ceremony and print the fingerprint to pin, then exit.
    pub pair: Option<String>,
    /// `--pin HEX`: the host certificate fingerprint (64 hex chars). A malformed value aborts —
    /// never silently downgrades to trust-on-first-use.
    pub pin: Option<[u8; 32]>,
    /// `--trust HEX`: non-interactive answer to the trust-on-first-use card. The value must equal
    /// the fingerprint the host actually presents, otherwise the session is refused. Interactive
    /// runs omit it and confirm at the prompt.
    pub trust: Option<[u8; 32]>,
    /// `--name LABEL`: how the host labels this client in its pending-approval list.
    pub name: String,
    pub mode: Mode,
    pub bitrate_kbps: u32,
    /// 0 = auto (host decides); otherwise a `CODEC_*` preference byte.
    pub preferred_codec: u8,
    pub audio_channels: u8,
    /// `--seconds N`: stop the stream after N seconds (0 = run until the window is closed).
    pub seconds: Option<u64>,
    /// `--record FILE`: write received AUs (annex-B, playable: `.h265`/`.h264`/`.av1`).
    pub record: Option<String>,
    /// `--play FILE`: decode a recorded Annex-B elementary stream (with AUD NALs) into a window,
    /// with no network at all. Proves the decode+present stack on its own.
    pub play: Option<String>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            connect: None,
            discover: None,
            pair: None,
            pin: None,
            trust: None,
            name: default_name(),
            mode: Mode {
                width: 1920,
                height: 1080,
                refresh_hz: 144,
            },
            bitrate_kbps: 0,
            preferred_codec: 0,
            audio_channels: 2,
            seconds: None,
            record: None,
            play: None,
        }
    }
}

/// `COMPUTERNAME` is the Windows analogue of the probe's `--name` default.
pub fn default_name() -> String {
    std::env::var("COMPUTERNAME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Windows PC".to_string())
}

/// What the parsed arguments ask the binary to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Discover(u64),
    Pair,
    Connect,
    Play,
}

impl Args {
    pub fn action(&self) -> Option<Action> {
        if let Some(secs) = self.discover {
            return Some(Action::Discover(secs));
        }
        if self.pair.is_some() {
            return Some(Action::Pair);
        }
        if self.play.is_some() {
            return Some(Action::Play);
        }
        self.connect.as_ref().map(|_| Action::Connect)
    }
}

/// A `host:port` pair before DNS resolution (kept separate so parsing stays pure).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub host: String,
    pub port: u16,
}

impl Target {
    /// `1.2.3.4:9777`, `1.2.3.4` (port defaults to 9777), `[::1]:9777`, `host.lan:9777`.
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s.is_empty() {
            return Err("--connect needs a host".into());
        }
        // IPv6 literal without a port: `[::1]` or a bare `::1` (contains several colons).
        if let Some(rest) = s.strip_prefix('[') {
            let (host, tail) = rest
                .split_once(']')
                .ok_or_else(|| format!("unterminated '[' in {s:?}"))?;
            let port = match tail.strip_prefix(':') {
                Some(p) => p.parse::<u16>().map_err(|_| format!("bad port in {s:?}"))?,
                None if tail.is_empty() => DEFAULT_PORT,
                None => return Err(format!("unexpected {tail:?} after ] in {s:?}")),
            };
            return Ok(Self {
                host: host.to_string(),
                port,
            });
        }
        // Bare IPv6 (two or more colons) is not supported without brackets: fail loudly instead
        // of guessing which group is the port.
        if s.matches(':').count() > 1 {
            return Err(format!(
                "ambiguous IPv6 address {s:?}; bracket it, e.g. [::1]:{DEFAULT_PORT}"
            ));
        }
        match s.rsplit_once(':') {
            // `host:port` where the tail really is a port.
            Some((host, p)) if !host.is_empty() && p.parse::<u16>().is_ok() => Ok(Self {
                host: host.to_string(),
                port: p.parse().unwrap(),
            }),
            _ => Ok(Self {
                host: s.to_string(),
                port: DEFAULT_PORT,
            }),
        }
    }
}

pub fn parse_hex32(s: &str) -> Option<[u8; 32]> {
    let s = s.trim();
    if s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(out)
}

pub fn hex(fp: &[u8; 32]) -> String {
    fp.iter().map(|b| format!("{b:02x}")).collect()
}

fn parse_mode(m: &str) -> Option<Mode> {
    let mut it = m.split('x');
    let mode = Mode {
        width: it.next()?.parse().ok()?,
        height: it.next()?.parse().ok()?,
        refresh_hz: it.next()?.parse().ok()?,
    };
    if it.next().is_some() || mode.width == 0 || mode.height == 0 || mode.refresh_hz == 0 {
        return None;
    }
    Some(mode)
}

fn parse_codec(c: &str) -> Option<u8> {
    use slipstream_core::quic::{CODEC_AV1, CODEC_H264, CODEC_HEVC};
    match c.to_ascii_lowercase().as_str() {
        "auto" => Some(0),
        "h264" | "avc" => Some(CODEC_H264),
        "hevc" | "h265" => Some(CODEC_HEVC),
        "av1" => Some(CODEC_AV1),
        _ => None,
    }
}

/// Parse an argument vector (without argv[0]). Every malformed *present* flag is an error: the
/// client never silently drops a request the user made.
pub fn parse<I: IntoIterator<Item = String>>(argv: I) -> Result<Args, String> {
    let argv: Vec<String> = argv.into_iter().collect();
    let mut args = Args::default();
    let mut i = 0usize;
    let value = |i: &mut usize, flag: &str| -> Result<String, String> {
        *i += 1;
        argv.get(*i)
            .cloned()
            .ok_or_else(|| format!("{flag} needs a value"))
    };
    while i < argv.len() {
        match argv[i].as_str() {
            "--connect" => args.connect = Some(value(&mut i, "--connect")?),
            "--discover" => {
                // Optional numeric argument: `--discover 4`, but a following flag is not one.
                let next = argv.get(i + 1).and_then(|s| s.parse::<u64>().ok());
                if next.is_some() {
                    i += 1;
                }
                args.discover = Some(next.unwrap_or(4));
            }
            "--pair" => args.pair = Some(value(&mut i, "--pair")?),
            "--pin" => {
                let raw = value(&mut i, "--pin")?;
                args.pin = Some(parse_hex32(&raw).ok_or_else(|| {
                    "--pin must be exactly 64 hex chars (the host logs its fingerprint at startup)"
                        .to_string()
                })?);
            }
            "--trust" => {
                let raw = value(&mut i, "--trust")?;
                args.trust = Some(parse_hex32(&raw).ok_or_else(|| {
                    "--trust must be exactly 64 hex chars (the fingerprint the client printed)"
                        .to_string()
                })?);
            }
            "--name" => args.name = value(&mut i, "--name")?,
            "--mode" => {
                let raw = value(&mut i, "--mode")?;
                args.mode = parse_mode(&raw)
                    .ok_or_else(|| format!("--mode must be WxHxFPS, got {raw:?}"))?;
            }
            "--bitrate" => {
                let raw = value(&mut i, "--bitrate")?;
                args.bitrate_kbps = raw
                    .parse()
                    .map_err(|_| format!("--bitrate needs kbps as a number, got {raw:?}"))?;
            }
            "--codec" => {
                let raw = value(&mut i, "--codec")?;
                args.preferred_codec = parse_codec(&raw)
                    .ok_or_else(|| format!("--codec must be auto|h264|hevc|av1, got {raw:?}"))?;
            }
            "--audio-channels" => {
                let raw = value(&mut i, "--audio-channels")?;
                let n: u8 = raw
                    .parse()
                    .map_err(|_| format!("--audio-channels needs 2, 6 or 8, got {raw:?}"))?;
                if !matches!(n, 2 | 6 | 8) {
                    return Err(format!("--audio-channels needs 2, 6 or 8, got {n}"));
                }
                args.audio_channels = n;
            }
            "--seconds" => {
                let raw = value(&mut i, "--seconds")?;
                args.seconds = Some(
                    raw.parse()
                        .map_err(|_| format!("--seconds needs a number, got {raw:?}"))?,
                );
            }
            "--record" => args.record = Some(value(&mut i, "--record")?),
            "--play" => args.play = Some(value(&mut i, "--play")?),
            "-h" | "--help" => return Err(usage().to_string()),
            other if other.starts_with("--") => return Err(format!("unknown flag {other}")),
            other => {
                // A bare positional is the host, the way `ssh host` works.
                if args.connect.is_some() {
                    return Err(format!("unexpected argument {other:?}"));
                }
                args.connect = Some(other.to_string());
            }
        }
        i += 1;
    }
    if args.connect.is_some()
        && (args.discover.is_some() || args.pair.is_some() || args.play.is_some())
    {
        return Err("--connect cannot be combined with --discover, --pair or --play".into());
    }
    Ok(args)
}

pub fn usage() -> &'static str {
    "usage: slipstream-windows [HOST[:PORT]] [--connect HOST[:PORT]] [--pin HEX | --pair PIN]\n\
     \x20      [--trust HEX] [--discover [SECS]] [--name LABEL] [--mode WxHxFPS] [--bitrate KBPS]\n\
     \x20      [--codec auto|h264|hevc|av1] [--audio-channels 2|6|8] [--seconds N] [--record FILE]\n\
     \x20      [--play FILE]"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Result<Args, String> {
        parse(s.split_whitespace().map(String::from))
    }

    #[test]
    fn defaults_match_the_probe_baseline() {
        let a = p("--connect 10.0.0.5").unwrap();
        assert_eq!(a.mode.width, 1920);
        assert_eq!(a.mode.height, 1080);
        assert_eq!(a.mode.refresh_hz, 144);
        assert_eq!(a.audio_channels, 2);
        assert_eq!(a.action(), Some(Action::Connect));
    }

    #[test]
    fn bare_positional_is_the_host() {
        let a = p("100.112.143.99:9777").unwrap();
        assert_eq!(a.connect.as_deref(), Some("100.112.143.99:9777"));
        assert_eq!(a.action(), Some(Action::Connect));
    }

    #[test]
    fn second_positional_is_rejected() {
        assert!(p("a b").is_err());
    }

    #[test]
    fn target_defaults_the_port_and_handles_ipv6() {
        assert_eq!(
            Target::parse("10.0.0.5").unwrap(),
            Target {
                host: "10.0.0.5".into(),
                port: DEFAULT_PORT
            }
        );
        assert_eq!(
            Target::parse("host.lan:1234").unwrap(),
            Target {
                host: "host.lan".into(),
                port: 1234
            }
        );
        assert_eq!(
            Target::parse("[::1]:9777").unwrap(),
            Target {
                host: "::1".into(),
                port: 9777
            }
        );
        assert_eq!(
            Target::parse("[fd00::5]").unwrap(),
            Target {
                host: "fd00::5".into(),
                port: 9777
            }
        );
        assert!(Target::parse("::1").is_err(), "unbracketed IPv6 must fail");
        assert!(Target::parse("").is_err());
        assert!(Target::parse("[::1").is_err());
    }

    #[test]
    fn malformed_pin_fails_closed() {
        assert!(p("--connect h --pin abcd").is_err());
        // Non-hex but the right length must still fail.
        let not_hex = "z".repeat(64);
        assert!(p(&format!("--connect h --pin {not_hex}")).is_err());
        let ok = "a".repeat(64);
        assert_eq!(parse_hex32(&ok), Some([0xaa; 32]));
    }

    #[test]
    fn trust_accepts_only_a_well_formed_fingerprint() {
        let ok = "b".repeat(64);
        let a = p(&format!("--connect h --trust {ok}")).unwrap();
        assert_eq!(a.trust, Some([0xbb; 32]));
        // Absent by default: the interactive prompt is the default path, never an implicit yes.
        assert_eq!(p("--connect h").unwrap().trust, None);
        assert!(p("--connect h --trust abcd").is_err());
        assert!(p(&format!("--connect h --trust {}", "z".repeat(64))).is_err());
    }

    #[test]
    fn discover_takes_an_optional_duration() {
        assert_eq!(p("--discover").unwrap().discover, Some(4));
        assert_eq!(p("--discover 9").unwrap().discover, Some(9));
        assert_eq!(p("--discover --name pc").unwrap().discover, Some(4));
        assert_eq!(p("--discover").unwrap().action(), Some(Action::Discover(4)));
    }

    #[test]
    fn pair_and_connect_conflict() {
        assert!(p("--connect h --pair 1234").is_err());
        assert_eq!(p("--pair 1234").unwrap().action(), Some(Action::Pair));
    }

    #[test]
    fn unknown_flag_and_bad_values_are_errors() {
        assert!(p("--connect h --nope").is_err());
        assert!(p("--connect h --mode 1920x1080").is_err());
        assert!(p("--connect h --codec vp9").is_err());
        assert!(p("--connect h --audio-channels 3").is_err());
        assert!(p("--connect h --seconds x").is_err());
        assert!(p("--connect h --bitrate fast").is_err());
        assert!(p("--connect").is_err());
    }

    #[test]
    fn codec_and_mode_parse() {
        let a = p("--connect h --codec h265 --mode 1280x720x60").unwrap();
        assert_eq!(a.preferred_codec, slipstream_core::quic::CODEC_HEVC);
        assert_eq!(a.mode.width, 1280);
        assert_eq!(a.mode.refresh_hz, 60);
        assert_eq!(p("--connect h --codec AUTO").unwrap().preferred_codec, 0);
    }

    #[test]
    fn record_is_carried_through() {
        assert_eq!(
            p("--connect h --record out.h265")
                .unwrap()
                .record
                .as_deref(),
            Some("out.h265")
        );
    }

    #[test]
    fn help_is_not_silently_ignored() {
        let err = p("--help").unwrap_err();
        assert!(err.contains("usage: slipstream-windows"));
    }
}
