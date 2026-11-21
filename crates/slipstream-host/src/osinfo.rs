//! Host OS identity for client-facing surfaces: an icon-friendly **specificity chain** plus a
//! human-readable name.
//!
//! The chain is slash-separated, generic → specific, `family[/like][/id]`:
//! `linux`, `linux/debian/ubuntu`, `linux/fedora/bazzite`,
//! `linux/arch/steamos`. Clients walk it most-specific-first and show the first token they have
//! art for — a client with no Bazzite mark lands on `fedora`, then generic `linux` — so the host
//! always emits the *full* chain and clients need zero distro→parent knowledge. On Linux the
//! middle token is the first `ID_LIKE` ancestor we recognize ([`FAMILIES`]; `ID_LIKE` is ordered
//! most-similar-first per the os-release spec) and the leaf is `ID` verbatim, so even a distro we
//! never heard of still degrades to its family's icon.
//!
//! Windows and macOS have no distro ladder, so their chain is a single truthful token
//! (`windows`, `macos`) and their human name comes from the OS itself: the registry's
//! `CurrentVersion` product name on Windows, the `SystemVersion.plist` on macOS. A client with no
//! art for those tokens falls back to its generic monitor mark, which is *honest* — the previous
//! behaviour handed it the Linux fallback (`linux` / `Linux`) on every non-Linux host, so a
//! Windows machine advertised a Tux.
//!
//! The chain rides the mDNS `os=` TXT record ([`crate::discovery`]) and the mgmt API's
//! `HostInfo.os`; the pretty name (`PRETTY_NAME`) is REST-only — TXT stays small. Both values are
//! advisory identity, same trust posture as the `mac` TXT key: unauthenticated, and a wrong value
//! only draws a wrong icon.

use std::sync::OnceLock;

/// The host's OS identity, detected once per process (os-release is static for its lifetime).
pub struct OsInfo {
    /// `linux[/<family>][/<id>]` (or a single `windows`/`macos`) — see the module doc.
    pub chain: String,
    /// Human-readable OS name (os-release `PRETTY_NAME`; the OS's own product name elsewhere).
    pub pretty: String,
}

/// Detect (once) and return the host's OS identity.
pub fn detect() -> &'static OsInfo {
    static OS: OnceLock<OsInfo> = OnceLock::new();
    OS.get_or_init(platform_os_info)
}

/// Per-OS detection entry point.
///
/// Linux is a straight call into the unchanged [`linux_os_info`] chain (`/etc/os-release` over
/// `/usr/lib/os-release`, else `linux`/`Linux`), so the emitted `os_name` is byte-identical to
/// every release before this dispatch existed.
#[cfg(target_os = "linux")]
fn platform_os_info() -> OsInfo {
    linux_os_info()
}

#[cfg(target_os = "macos")]
fn platform_os_info() -> OsInfo {
    macos_os_info()
}

#[cfg(target_os = "windows")]
fn platform_os_info() -> OsInfo {
    windows_os_info()
}

/// Targets with no first-class identity: report the platform token truthfully rather than
/// borrowing Linux's. (`std::env::consts::OS` is a compile-time constant, e.g. `freebsd`.)
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn platform_os_info() -> OsInfo {
    // Every `env::consts::OS` value is lowercase ASCII (`freebsd`, `openbsd`, `android`, …), so it
    // is already TXT-safe and needs no sanitizing.
    let tok = std::env::consts::OS.to_string();
    OsInfo {
        pretty: tok.clone(),
        chain: tok,
    }
}

/// `ID_LIKE` ancestors worth keeping as the chain's middle token — the distros clients plausibly
/// have icon art for. Anything else (or no `ID_LIKE` at all) yields a two-segment chain and the
/// leaf still identifies the distro exactly.
#[cfg(any(target_os = "linux", test))]
const FAMILIES: &[&str] = &[
    "debian", "ubuntu", "fedora", "rhel", "arch", "opensuse", "suse", "gentoo", "alpine", "nixos",
];

/// Read os-release from its two spec'd locations (`/etc` overrides the `/usr/lib` fallback);
/// a host with neither (or unreadable) is simply `linux`.
#[cfg(any(target_os = "linux", test))]
fn linux_os_info() -> OsInfo {
    ["/etc/os-release", "/usr/lib/os-release"]
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok())
        .map(|s| parse_os_release(&s))
        .unwrap_or_else(|| OsInfo {
            chain: "linux".into(),
            pretty: "Linux".into(),
        })
}

/// The `CurrentVersion` key every Windows release since NT 4 has carried.
#[cfg(target_os = "windows")]
const WINDOWS_CURRENT_VERSION: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";

/// Read the product identity out of the registry; each value is independently optional so a
/// locked-down or unusual install still reports `windows` rather than nothing.
#[cfg(target_os = "windows")]
fn windows_os_info() -> OsInfo {
    let read = |name: &str| registry_string(WINDOWS_CURRENT_VERSION, name);
    windows_identity(
        read("ProductName").as_deref(),
        read("DisplayVersion").as_deref(),
        read("CurrentBuildNumber").as_deref(),
    )
}

/// Compose the Windows identity from the three `CurrentVersion` values (pure, so it is tested on
/// every target). `ProductName` still reads "Windows 10 …" on Windows 11 — a documented Redmond
/// quirk — so a build ≥ 22000 (the first 11 build) upgrades that token: the version name we print
/// has to match what Settings shows.
#[cfg(any(target_os = "windows", test))]
fn windows_identity(
    product: Option<&str>,
    display_version: Option<&str>,
    build: Option<&str>,
) -> OsInfo {
    let product = product.map(str::trim).filter(|s| !s.is_empty());
    let mut name = product.unwrap_or("Windows").to_string();
    if build
        .and_then(|b| b.trim().parse::<u32>().ok())
        .is_some_and(|b| b >= 22_000)
    {
        if let Some(rest) = name.strip_prefix("Windows 10") {
            name = format!("Windows 11{rest}");
        }
    }
    if let Some(v) = display_version.map(str::trim).filter(|v| !v.is_empty()) {
        name = format!("{name} ({v})");
    }
    OsInfo {
        chain: "windows".into(),
        pretty: name,
    }
}

/// Read one `REG_SZ` value from `HKLM\<subkey>`; `None` for a missing key/value or a non-string
/// type (identity is advisory — never fail the host over it).
#[cfg(target_os = "windows")]
fn registry_string(subkey: &str, value: &str) -> Option<String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }
    let (subkey, value) = (wide(subkey), wide(value));
    let mut buf = [0u16; 256];
    let mut len = (buf.len() * std::mem::size_of::<u16>()) as u32;
    // SAFETY: both strings are NUL-terminated for the duration of the call; `buf` is exclusively
    // borrowed and `len` describes its capacity in bytes, per RegGetValueW's contract.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr().cast()),
            Some(&mut len),
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    // The API reports bytes; a truncated buffer is not NUL-terminated, hence the `min` + `unwrap_or`.
    let chars = (len as usize / 2).min(buf.len());
    let end = buf[..chars].iter().position(|&c| c == 0).unwrap_or(chars);
    let s = String::from_utf16_lossy(&buf[..end]);
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

/// macOS keeps its identity in a small XML plist; no plist crate needed for two flat strings.
#[cfg(target_os = "macos")]
fn macos_os_info() -> OsInfo {
    let plist = std::fs::read_to_string("/System/Library/CoreServices/SystemVersion.plist")
        .unwrap_or_default();
    macos_identity(
        plist_value(&plist, "ProductName").as_deref(),
        plist_value(&plist, "ProductVersion").as_deref(),
    )
}

/// Compose the macOS identity (pure, so it is tested on every target): `macOS 14.5`.
#[cfg(any(target_os = "macos", test))]
fn macos_identity(product: Option<&str>, version: Option<&str>) -> OsInfo {
    let product = product
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("macOS");
    let pretty = match version.map(str::trim).filter(|v| !v.is_empty()) {
        Some(v) => format!("{product} {v}"),
        None => product.to_string(),
    };
    OsInfo {
        chain: "macos".into(),
        pretty,
    }
}

/// The `<string>` that follows `<key>key</key>` in an Apple XML plist (`SystemVersion.plist` is
/// flat: one key, one scalar, repeated).
#[cfg(any(target_os = "macos", test))]
fn plist_value(plist: &str, key: &str) -> Option<String> {
    let after_key = plist.split(&format!("<key>{key}</key>")).nth(1)?;
    let rest = &after_key[after_key.find("<string>")? + "<string>".len()..];
    let value = rest[..rest.find("</string>")?].trim();
    (!value.is_empty()).then(|| value.to_string())
}

/// Pure os-release parser (compiled on every target so the tests run on any dev box).
///
/// Grammar per the freedesktop spec subset we need: `KEY=value` lines, values optionally
/// `"`/`'`-quoted. `ID`/`ID_LIKE` tokens are lowercased and sanitized before they feed a DNS TXT
/// record; `PRETTY_NAME` falls back `NAME` → capitalized `ID` → `"Linux"`.
#[cfg(any(target_os = "linux", test))]
fn parse_os_release(contents: &str) -> OsInfo {
    let (mut id, mut id_like, mut pretty, mut name) = (None, None, None, None);
    for line in contents.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("ID=") {
            id = Some(unquote(v));
        } else if let Some(v) = line.strip_prefix("ID_LIKE=") {
            id_like = Some(unquote(v));
        } else if let Some(v) = line.strip_prefix("PRETTY_NAME=") {
            pretty = Some(unquote(v));
        } else if let Some(v) = line.strip_prefix("NAME=") {
            name = Some(unquote(v));
        }
    }

    let id_tok = id.as_deref().and_then(sanitize_token);
    // First *recognized* ID_LIKE ancestor wins (the list is ordered most-similar-first), skipping
    // a token that merely repeats ID (some distros write `ID_LIKE` reflexively).
    let like_tok = id_like
        .as_deref()
        .unwrap_or("")
        .split_whitespace()
        .filter_map(sanitize_token)
        .find(|t| FAMILIES.contains(&t.as_str()) && Some(t) != id_tok.as_ref());

    let mut chain = String::from("linux");
    for tok in [&like_tok, &id_tok].into_iter().flatten() {
        chain.push('/');
        chain.push_str(tok);
    }

    let pretty = [pretty, name]
        .into_iter()
        .flatten()
        .map(|s| s.trim().to_string())
        .find(|s| !s.is_empty())
        .or_else(|| id_tok.as_deref().map(capitalize))
        .unwrap_or_else(|| "Linux".into());

    OsInfo { chain, pretty }
}

/// Strip one matching pair of surrounding `"` or `'` quotes.
#[cfg(any(target_os = "linux", test))]
fn unquote(v: &str) -> String {
    let v = v.trim();
    for q in ['"', '\''] {
        if let Some(inner) = v.strip_prefix(q).and_then(|s| s.strip_suffix(q)) {
            return inner.to_string();
        }
    }
    v.to_string()
}

/// Lowercase and reduce an `ID`/`ID_LIKE` token to TXT-safe `[a-z0-9._-]`, capped at 32 chars;
/// `None` when nothing survives (a hostile or garbage value must not reach the advert).
#[cfg(any(target_os = "linux", test))]
fn sanitize_token(raw: &str) -> Option<String> {
    let tok: String = raw
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
        .take(32)
        .collect();
    (!tok.is_empty()).then_some(tok)
}

/// `ubuntu` → `Ubuntu` (ASCII-only; the fallback label when os-release names are absent).
#[cfg(any(target_os = "linux", test))]
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(contents: &str) -> (String, String) {
        let info = parse_os_release(contents);
        (info.chain, info.pretty)
    }

    #[test]
    fn ubuntu_chains_through_debian() {
        let (chain, pretty) = parsed(
            "PRETTY_NAME=\"Ubuntu 24.04.2 LTS\"\nNAME=\"Ubuntu\"\nID=ubuntu\nID_LIKE=debian\n",
        );
        assert_eq!(chain, "linux/debian/ubuntu");
        assert_eq!(pretty, "Ubuntu 24.04.2 LTS");
    }

    #[test]
    fn bazzite_quoted_id_like() {
        let (chain, pretty) =
            parsed("ID=bazzite\nID_LIKE=\"fedora\"\nPRETTY_NAME=\"Bazzite 42 (Kinoite)\"\n");
        assert_eq!(chain, "linux/fedora/bazzite");
        assert_eq!(pretty, "Bazzite 42 (Kinoite)");
    }

    #[test]
    fn steamos_is_arch_family() {
        let (chain, _) = parsed("ID=steamos\nID_LIKE=arch\nPRETTY_NAME=\"SteamOS\"\n");
        assert_eq!(chain, "linux/arch/steamos");
    }

    #[test]
    fn cachyos_is_arch_family() {
        let (chain, _) = parsed("ID=cachyos\nID_LIKE=\"arch\"\nNAME=\"CachyOS Linux\"\n");
        assert_eq!(chain, "linux/arch/cachyos");
    }

    #[test]
    fn nobara_takes_rhel_the_first_recognized_ancestor() {
        // Verified against a real Nobara 44 box: it declares `ID_LIKE="rhel centos fedora"`,
        // so the family token is *rhel*, not the fedora one might expect. Clients ship a
        // `nobara` mark, so the leaf matches before the family ever gets a say — but a
        // RHEL-family distro we have no art for lands on plain Tux, not on Fedora.
        let (chain, pretty) = parsed(
            "NAME=\"Nobara Linux\"\nID=nobara\nID_LIKE=\"rhel centos fedora\"\nPRETTY_NAME=\"Nobara Linux 44 (KDE Plasma Desktop Edition)\"\n",
        );
        assert_eq!(chain, "linux/rhel/nobara");
        assert_eq!(pretty, "Nobara Linux 44 (KDE Plasma Desktop Edition)");
    }

    #[test]
    fn nixos_without_id_like_is_two_segments() {
        let (chain, _) = parsed("ID=nixos\nPRETTY_NAME=\"NixOS 25.05 (Warbler)\"\n");
        assert_eq!(chain, "linux/nixos");
    }

    #[test]
    fn plain_root_distro() {
        let (chain, _) = parsed("ID=arch\nNAME=\"Arch Linux\"\n");
        assert_eq!(chain, "linux/arch");
    }

    #[test]
    fn multi_ancestor_id_like_takes_first_recognized() {
        // centos: ID_LIKE is most-similar-first — rhel wins over fedora.
        let (chain, _) = parsed("ID=\"centos\"\nID_LIKE=\"rhel fedora\"\n");
        assert_eq!(chain, "linux/rhel/centos");
    }

    #[test]
    fn pop_takes_ubuntu_over_debian() {
        let (chain, _) = parsed("ID=pop\nID_LIKE=\"ubuntu debian\"\n");
        assert_eq!(chain, "linux/ubuntu/pop");
    }

    #[test]
    fn reflexive_id_like_is_skipped() {
        let (chain, _) = parsed("ID=fedora\nID_LIKE=fedora\n");
        assert_eq!(chain, "linux/fedora");
    }

    #[test]
    fn unrecognized_id_like_is_dropped_leaf_kept() {
        let (chain, _) = parsed("ID=chimera\nID_LIKE=frontier\n");
        assert_eq!(chain, "linux/chimera");
    }

    #[test]
    fn garbage_and_empty_fall_back_to_linux() {
        assert_eq!(parsed("").0, "linux");
        assert_eq!(parsed("not an os-release file at all\n===\n").0, "linux");
        assert_eq!(parsed("").1, "Linux");
    }

    #[test]
    fn hostile_id_is_sanitized_for_txt() {
        // Quotes/spaces/slashes must never reach the TXT record or split the chain.
        let (chain, _) = parsed("ID=\"Ubu ntu/EVIL=1\"\n");
        assert_eq!(chain, "linux/ubuntuevil1");
    }

    #[test]
    fn overlong_id_is_capped() {
        let long = "x".repeat(80);
        let (chain, _) = parsed(&format!("ID={long}\n"));
        assert_eq!(chain, format!("linux/{}", "x".repeat(32)));
    }

    #[test]
    fn pretty_falls_back_name_then_id() {
        assert_eq!(
            parsed("ID=debian\nNAME=\"Debian GNU/Linux\"\n").1,
            "Debian GNU/Linux"
        );
        assert_eq!(parsed("ID=debian\n").1, "Debian");
    }

    #[test]
    fn single_quoted_values() {
        let (chain, pretty) = parsed("ID='opensuse-tumbleweed'\nID_LIKE='opensuse suse'\nPRETTY_NAME='openSUSE Tumbleweed'\n");
        assert_eq!(chain, "linux/opensuse/opensuse-tumbleweed");
        assert_eq!(pretty, "openSUSE Tumbleweed");
    }

    #[test]
    fn crlf_and_padding_tolerated() {
        let (chain, _) = parsed("  ID=ubuntu \r\n\tID_LIKE=debian\r\n");
        assert_eq!(chain, "linux/debian/ubuntu");
    }

    /// Byte-compat pin: the exact fleet identities the pre-dispatch code produced, restated here
    /// as a single table. Dispatch (`detect` → `platform_os_info` → `linux_os_info`) added an
    /// indirection only; nothing in this chain was touched, and this test fails loudly if the
    /// emitted `os_name` for a shipped distro ever drifts.
    #[test]
    fn linux_identities_are_byte_identical_to_pre_dispatch() {
        let cases: &[(&str, &str, &str)] = &[
            (
                "PRETTY_NAME=\"Ubuntu 24.04.2 LTS\"\nNAME=\"Ubuntu\"\nID=ubuntu\nID_LIKE=debian\n",
                "linux/debian/ubuntu",
                "Ubuntu 24.04.2 LTS",
            ),
            (
                "ID=bazzite\nID_LIKE=\"fedora\"\nPRETTY_NAME=\"Bazzite 42 (Kinoite)\"\n",
                "linux/fedora/bazzite",
                "Bazzite 42 (Kinoite)",
            ),
            (
                "ID=steamos\nID_LIKE=arch\nPRETTY_NAME=\"SteamOS\"\n",
                "linux/arch/steamos",
                "SteamOS",
            ),
            (
                "ID=nixos\nPRETTY_NAME=\"NixOS 25.06 (Warbler)\"\n",
                "linux/nixos",
                "NixOS 25.06 (Warbler)",
            ),
            ("", "linux", "Linux"),
        ];
        for (input, chain, pretty) in cases {
            assert_eq!(
                &parsed(input),
                &(chain.to_string(), pretty.to_string()),
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn windows_identity_is_truthful_chain_and_name() {
        let info = windows_identity(Some("Windows 11 Pro"), Some("23H2"), Some("22631"));
        assert_eq!(info.chain, "windows");
        assert_eq!(info.pretty, "Windows 11 Pro (23H2)");
    }

    #[test]
    fn windows_product_name_is_upgraded_when_build_says_eleven() {
        // The registry still reports "Windows 10 Pro" on Windows 11; build ≥ 22000 is 11.
        let info = windows_identity(Some("Windows 10 Pro"), Some("24H2"), Some("26100"));
        assert_eq!(info.pretty, "Windows 11 Pro (24H2)");
        let real_ten = windows_identity(Some("Windows 10 Pro"), Some("22H2"), Some("19045"));
        assert_eq!(real_ten.pretty, "Windows 10 Pro (22H2)");
    }

    #[test]
    fn windows_identity_degrades_to_the_platform_token() {
        let bare = windows_identity(None, None, None);
        assert_eq!(bare.chain, "windows");
        assert_eq!(bare.pretty, "Windows");
        let version_only = windows_identity(None, Some("23H2"), None);
        assert_eq!(version_only.pretty, "Windows (23H2)");
        // Garbage build numbers must not panic or fabricate a version.
        let junk = windows_identity(Some("Windows 11 Pro"), Some("  "), Some("n/a"));
        assert_eq!(junk.pretty, "Windows 11 Pro");
    }

    #[test]
    fn macos_identity_reads_the_system_version_plist() {
        let plist = "<plist><dict>\n\t<key>ProductBuildVersion</key>\n\t<string>23F79</string>\n\t<key>ProductName</key>\n\t<string>macOS</string>\n\t<key>ProductVersion</key>\n\t<string>14.5</string>\n</dict></plist>\n";
        assert_eq!(plist_value(plist, "ProductName").as_deref(), Some("macOS"));
        assert_eq!(
            plist_value(plist, "ProductVersion").as_deref(),
            Some("14.5")
        );
        assert_eq!(
            plist_value(plist, "ProductBuildVersion").as_deref(),
            Some("23F79")
        );
        assert_eq!(plist_value(plist, "Missing"), None);

        let info = macos_identity(
            plist_value(plist, "ProductName").as_deref(),
            plist_value(plist, "ProductVersion").as_deref(),
        );
        assert_eq!(info.chain, "macos");
        assert_eq!(info.pretty, "macOS 14.5");
    }

    #[test]
    fn macos_identity_degrades_to_the_platform_token() {
        let bare = macos_identity(None, None);
        assert_eq!(bare.chain, "macos");
        assert_eq!(bare.pretty, "macOS");
        assert_eq!(macos_identity(None, Some("15.1")).pretty, "macOS 15.1");
        assert_eq!(macos_identity(Some(""), Some(" ")).pretty, "macOS");
    }

    /// The actual `detect()` on a Windows build must not advertise Linux: that was the bug.
    #[cfg(target_os = "windows")]
    #[test]
    fn detect_reports_windows_on_windows() {
        let info = detect();
        assert_eq!(
            info.chain, "windows",
            "Windows host must not advertise a Linux chain"
        );
        assert!(!info.chain.contains('/'), "windows has no family ladder");
        assert!(
            info.pretty.starts_with("Windows"),
            "unexpected os_name: {}",
            info.pretty
        );
    }

    /// And on Linux the chain keeps its shape and the unchanged fallback chain.
    #[cfg(target_os = "linux")]
    #[test]
    fn detect_reports_linux_on_linux() {
        let info = detect();
        assert_eq!(info.chain.split('/').next(), Some("linux"));
        assert!(!info.pretty.is_empty(), "os_name must never be empty");
    }

    /// `detect()` is process-stable (one `OnceLock` init), so two calls must agree byte for byte.
    #[test]
    fn detect_is_stable_across_calls() {
        let (a, b) = (detect(), detect());
        assert_eq!((&a.chain, &a.pretty), (&b.chain, &b.pretty));
    }
}
