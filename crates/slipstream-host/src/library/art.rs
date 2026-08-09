//! Artwork helpers for the local cache, provider URLs, and the GameStream art proxy.

use super::*;

/// Keep the existing startup hook while providers supply their own URLs and local art directly.
pub fn start_art_warmer() -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("ss-art-warmer".into())
        .spawn(|| {})
        .expect("spawn art warmer thread")
}

/// Fetch one image URL for the GameStream `/appasset` cover proxy, as `(bytes, content-type)`. Handles
/// `data:` URLs (Lutris inlines art that way) by decoding inline, and `http(s)` URLs by a bounded GET
/// (8 MiB cap so a hostile/huge art URL can't balloon host memory). `None` on any non-image scheme,
/// network/decoder error, or empty body. Blocking (ureq) — call off the async runtime.
pub(crate) fn fetch_image(url: &str) -> Option<(Vec<u8>, String)> {
    use base64::Engine as _;
    use std::io::Read as _;
    if let Some(rest) = url.strip_prefix("data:") {
        // data:[<mediatype>][;base64],<payload>
        let (meta, data) = rest.split_once(',')?;
        let ctype = meta
            .split(';')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or("image/jpeg")
            .to_string();
        let bytes = if meta.contains(";base64") {
            base64::engine::general_purpose::STANDARD
                .decode(data)
                .ok()?
        } else {
            data.as_bytes().to_vec()
        };
        return (!bytes.is_empty()).then_some((bytes, ctype));
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return None;
    }
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        // Don't follow redirects (SSRF pivot): this is called on launcher-cache- and custom-entry-
        // supplied URLs, so a `3xx` to an internal/metadata endpoint must not be chased by the
        // privileged host. Matches the webhook path (security-review 2026-07-17).
        .redirects(0)
        .build();
    let resp = agent.get(url).call().ok()?;
    let ctype = resp
        .header("Content-Type")
        .unwrap_or("image/jpeg")
        .to_string();
    let mut bytes = Vec::new();
    resp.into_reader()
        .take(8 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .ok()?;
    (!bytes.is_empty()).then_some((bytes, ctype))
}

/// A stored [`Artwork`] value that is a **local filesystem path** to an image on the host — as
/// opposed to an `http(s)`/`data:` URL or an already-relative host proxy path. Provider plugins that
/// run on the host set these: the reconcile payload stays small
/// (paths, not inlined bytes, so it scales to thousands of titles) and the host serves the bytes
/// through the art proxy, exactly like Steam's cache art. Absolute Linux paths are accepted;
/// proxy paths and URLs are not.
pub fn is_local_art_path(v: &str) -> bool {
    if v.starts_with("http://") || v.starts_with("https://") || v.starts_with("data:") {
        return false;
    }
    if v.starts_with("/api/v1/library/art/") {
        return false;
    }
    std::path::Path::new(v).is_absolute()
}

/// Read a local image file into `(bytes, content-type)` for the art proxy. `None` if it isn't an
/// existing regular file, is empty, or exceeds 16 MiB (a cover never approaches that; the cap bounds
/// host memory). Content-type is guessed from the extension.
pub fn local_art_bytes(path: &str) -> Option<(Vec<u8>, String)> {
    let p = std::path::Path::new(path);
    let meta = std::fs::metadata(p).ok()?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > 16 * 1024 * 1024 {
        return None;
    }
    let ctype = match p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("bmp") => "image/bmp",
        Some("ico") => "image/x-icon",
        Some("tga") => "image/x-tga",
        _ => "application/octet-stream",
    }
    .to_string();
    Some((std::fs::read(p).ok()?, ctype))
}

/// Resolve one art value to bytes for the Moonlight `/appasset` proxy: a local host file
/// ([`is_local_art_path`]) is read directly, anything else is a URL fetched by [`fetch_image`].
fn resolve_art_bytes(v: &str) -> Option<(Vec<u8>, String)> {
    if is_local_art_path(v) {
        local_art_bytes(v)
    } else {
        fetch_image(v)
    }
}

/// Rewrite any **local-file** art paths on an entry into host art-proxy URLs
/// (`/api/v1/library/art/<id>/<kind>`, the same relative-proxy shape Steam art uses, resolved by the
/// client against the host). `http(s)`/`data:` URLs and already-relative proxy paths are left as-is.
/// Applied to the `GET /library` response so a client fetches a provider's local covers from the host
/// instead of receiving an unreachable `C:\…` path.
pub fn proxy_local_art(id: &str, art: &mut Artwork) {
    let rw = |field: &mut Option<String>, kind: &str| {
        if field.as_deref().is_some_and(is_local_art_path) {
            *field = Some(format!("/api/v1/library/art/{id}/{kind}"));
        }
    };
    rw(&mut art.portrait, "portrait");
    rw(&mut art.hero, "hero");
    rw(&mut art.logo, "logo");
    rw(&mut art.header, "header");
}

/// Resolve + fetch the best box-art cover for a library id (the GameStream `/appasset` proxy — Moonlight
/// fetches per-app covers from the HOST, not the CDN, so we proxy the bytes). Tries the portrait (tall
/// capsule Moonlight wants) → header → hero → logo, returning the first that fetches as
/// `(bytes, content-type)`. Resolves the id against the host's OWN library. Blocking — call off the
/// async runtime (e.g. `spawn_blocking`).
pub fn fetch_box_art(id: &str) -> Option<(Vec<u8>, String)> {
    // Steam's `Artwork` fields are now relative proxy paths (see `steam_art`) the *client* resolves
    // against the host — meaningless to `fetch_image`, which expects an absolute URL. Resolve
    // those kinds directly instead of going through the URL fields.
    if let Some(appid) = id
        .strip_prefix("steam:")
        .and_then(|s| s.parse::<u32>().ok())
    {
        return [
            ArtKind::Portrait,
            ArtKind::Header,
            ArtKind::Hero,
            ArtKind::Logo,
        ]
        .into_iter()
        .find_map(|kind| steam_art_bytes(appid, kind));
    }
    let g = all_games().into_iter().find(|g| g.id == id)?;
    [g.art.portrait, g.art.header, g.art.hero, g.art.logo]
        .into_iter()
        .flatten()
        .find_map(|url| resolve_art_bytes(&url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn art_kind_parses_known_names_only() {
        assert_eq!(ArtKind::parse("portrait"), Some(ArtKind::Portrait));
        assert_eq!(ArtKind::parse("hero"), Some(ArtKind::Hero));
        assert_eq!(ArtKind::parse("logo"), Some(ArtKind::Logo));
        assert_eq!(ArtKind::parse("header"), Some(ArtKind::Header));
        assert_eq!(ArtKind::parse("background"), None);
    }

    #[test]
    fn fetch_image_decodes_data_url() {
        // "Hi" base64 == "SGk=" — the data: branch is pure (no network), so it's deterministic.
        let (bytes, ctype) = fetch_image("data:image/png;base64,SGk=").expect("data url decodes");
        assert_eq!(bytes, b"Hi");
        assert_eq!(ctype, "image/png");
        // A non-image scheme is rejected (no launcher art ever points at file://, but be defensive).
        assert!(fetch_image("file:///etc/passwd").is_none());
        // Empty payload → None (never serve a 0-byte cover).
        assert!(fetch_image("data:image/png;base64,").is_none());
    }

    #[test]
    fn local_art_path_detection() {
        assert!(is_local_art_path("/var/lib/slipstream/cover.jpg"));
        // URLs and the host proxy path are NOT local files.
        assert!(!is_local_art_path("https://cdn/x.jpg"));
        assert!(!is_local_art_path("http://host/x.jpg"));
        assert!(!is_local_art_path("data:image/png;base64,AAAA"));
        assert!(!is_local_art_path(
            "/api/v1/library/art/custom:abc/portrait"
        ));
    }

    #[test]
    fn proxy_local_art_rewrites_only_local_paths() {
        let mut art = Artwork {
            portrait: Some("/var/lib/slipstream/art/p.jpg".into()),
            hero: Some("https://cdn/h.jpg".into()),
            logo: None,
            header: Some("/api/v1/library/art/custom:x/header".into()),
        };
        proxy_local_art("custom:abc", &mut art);
        // The local path becomes a host proxy URL; the remote URL and an already-proxied path stay.
        assert_eq!(
            art.portrait.as_deref(),
            Some("/api/v1/library/art/custom:abc/portrait")
        );
        assert_eq!(art.hero.as_deref(), Some("https://cdn/h.jpg"));
        assert_eq!(
            art.header.as_deref(),
            Some("/api/v1/library/art/custom:x/header")
        );
    }

    #[test]
    fn local_art_bytes_reads_a_real_file() {
        let dir = std::env::temp_dir().join(format!("ss-art-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("cover.png");
        std::fs::write(&f, [1u8, 2, 3, 4]).unwrap();
        let (bytes, ctype) = local_art_bytes(f.to_str().unwrap()).expect("reads file");
        assert_eq!(bytes, vec![1, 2, 3, 4]);
        assert_eq!(ctype, "image/png");
        assert!(local_art_bytes(dir.join("nope.png").to_str().unwrap()).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
