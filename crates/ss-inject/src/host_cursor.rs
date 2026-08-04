//! Hide the host's local OS cursor while clients stream, without blanking the stream overlay.
//!
//! - Windows: `ShowCursor(FALSE)` until the display count is negative; restore on Drop.
//! - Linux X11: `XFixesHideCursor` on the root window.
//! - Linux Wayland (GNOME): temporary invisible XCursor theme via gsettings; capture keeps the
//!   last non-blank `SPA_META_Cursor` bitmap so the client still sees a pointer.

use ss_capture::host_cursor_flag;

/// RAII hide of the host-local OS cursor. Restores on drop.
pub struct PlatformHide {
    #[cfg(target_os = "linux")]
    inner: linux::Inner,
    #[cfg(target_os = "windows")]
    inner: windows::Inner,
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    _inner: (),
}

impl PlatformHide {
    /// Best-effort hide. Returns `None` when the platform cannot hide (caller still holds the
    /// refcount share; stream continues).
    pub fn acquire() -> Option<Self> {
        #[cfg(target_os = "linux")]
        {
            let inner = linux::Inner::acquire()?;
            host_cursor_flag::set_hidden_for_stream(true);
            return Some(Self { inner });
        }
        #[cfg(target_os = "windows")]
        {
            let inner = windows::Inner::acquire()?;
            host_cursor_flag::set_hidden_for_stream(true);
            return Some(Self { inner });
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            tracing::debug!("host cursor hide: unsupported on this platform");
            None
        }
    }
}

impl Drop for PlatformHide {
    fn drop(&mut self) {
        host_cursor_flag::set_hidden_for_stream(false);
        // Platform restore runs via Inner's Drop.
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::path::PathBuf;
    use std::process::Command;

    pub(super) enum Inner {
        Xfixes(XfixesHide),
        Theme(ThemeHide),
    }

    impl Inner {
        pub(super) fn acquire() -> Option<Self> {
            // On Wayland, XFixes against Xwayland often "succeeds" without hiding the compositor
            // cursor. Prefer the GNOME theme path whenever WAYLAND_DISPLAY is set.
            let on_wayland = std::env::var_os("WAYLAND_DISPLAY")
                .filter(|s| !s.is_empty())
                .is_some();
            if on_wayland {
                match ThemeHide::try_acquire() {
                    Ok(t) => {
                        tracing::info!(
                            "host cursor hide: GNOME invisible cursor theme (Wayland)"
                        );
                        return Some(Self::Theme(t));
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "host cursor hide: Wayland hide unavailable; streaming continues"
                        );
                        return None;
                    }
                }
            }
            if let Some(x) = XfixesHide::try_acquire() {
                tracing::info!("host cursor hide: XFixesHideCursor (X11)");
                return Some(Self::Xfixes(x));
            }
            tracing::warn!(
                "host cursor hide: no X11 display and not a supported Wayland desktop; streaming continues"
            );
            None
        }
    }

    pub(super) struct XfixesHide {
        // Keep the connection alive for the hide duration; Drop shows the cursor again.
        conn: x11rb::rust_connection::RustConnection,
        root: u32,
    }

    impl XfixesHide {
        fn try_acquire() -> Option<Self> {
            // Wayland-native sessions often still expose Xwayland on DISPLAY - try it, but do not
            // treat failure as fatal (ThemeHide is the Wayland path).
            let (conn, screen_num) =
                x11rb::rust_connection::RustConnection::connect(None).ok()?;
            use x11rb::connection::Connection;
            use x11rb::protocol::xfixes::ConnectionExt as _;
            let screen = &conn.setup().roots.get(screen_num)?;
            let root = screen.root;
            // XFixes 2.0+ required for HideCursor.
            let _ = conn.xfixes_query_version(5, 0).ok()?.reply().ok()?;
            conn.xfixes_hide_cursor(root).ok()?.check().ok()?;
            let _ = conn.flush();
            Some(Self { conn, root })
        }
    }

    impl Drop for XfixesHide {
        fn drop(&mut self) {
            use x11rb::connection::Connection;
            use x11rb::protocol::xfixes::ConnectionExt as _;
            let _ = self.conn.xfixes_show_cursor(self.root);
            let _ = self.conn.flush();
        }
    }

    pub(super) struct ThemeHide {
        prev_theme: String,
        prev_size: Option<String>,
    }

    impl ThemeHide {
        fn try_acquire() -> anyhow::Result<Self> {
            let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
            if !desktop.to_ascii_uppercase().contains("GNOME") {
                anyhow::bail!("not a GNOME session (XDG_CURRENT_DESKTOP={desktop:?})");
            }
            let prev_theme = gsettings_get("org.gnome.desktop.interface", "cursor-theme")?;
            let prev_size = gsettings_get("org.gnome.desktop.interface", "cursor-size").ok();
            ensure_invisible_theme()?;
            gsettings_set(
                "org.gnome.desktop.interface",
                "cursor-theme",
                "SlipstreamInvisible",
            )?;
            // Mutter often caches the previous sprite; a size poke forces a theme reload.
            if let Some(size) = &prev_size {
                let poke = size.parse::<i32>().ok().map(|n| (n + 1).max(1).to_string());
                if let Some(p) = poke {
                    let _ = gsettings_set("org.gnome.desktop.interface", "cursor-size", &p);
                    let _ = gsettings_set("org.gnome.desktop.interface", "cursor-size", size);
                }
            }
            Ok(Self {
                prev_theme,
                prev_size,
            })
        }
    }

    impl Drop for ThemeHide {
        fn drop(&mut self) {
            let _ = gsettings_set(
                "org.gnome.desktop.interface",
                "cursor-theme",
                &self.prev_theme,
            );
            if let Some(size) = &self.prev_size {
                let _ = gsettings_set("org.gnome.desktop.interface", "cursor-size", size);
            }
            // Leave the theme files in place for the next hide (cheap); do not delete on restore
            // so a crash mid-session still has assets for a later reconnect.
        }
    }

    fn gsettings_get(schema: &str, key: &str) -> anyhow::Result<String> {
        let out = Command::new("gsettings")
            .args(["get", schema, key])
            .output()?;
        if !out.status.success() {
            anyhow::bail!(
                "gsettings get {schema} {key} failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        // gsettings prints strings as 'Adwaita' - strip quotes.
        Ok(s.trim_matches('\'').to_string())
    }

    fn gsettings_set(schema: &str, key: &str, value: &str) -> anyhow::Result<()> {
        let out = Command::new("gsettings")
            .args(["set", schema, key, value])
            .output()?;
        if !out.status.success() {
            anyhow::bail!(
                "gsettings set {schema} {key} failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        Ok(())
    }

    /// Icon theme search roots. `~/.icons` is checked before `~/.local/share/icons`, so a
    /// leftover theme there wins over whatever we write under XDG_DATA_HOME.
    fn theme_install_roots() -> Vec<PathBuf> {
        let mut roots = Vec::with_capacity(2);
        if let Some(home) = std::env::var_os("HOME").filter(|s| !s.is_empty()) {
            roots.push(PathBuf::from(&home).join(".icons"));
        }
        let data = std::env::var_os("XDG_DATA_HOME")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share"))
            })
            .unwrap_or_else(|| PathBuf::from(".local/share"));
        roots.push(data.join("icons"));
        roots
    }

    /// Write a fully transparent XCursor theme into every install root we know about.
    fn ensure_invisible_theme() -> anyhow::Result<()> {
        const NAMES: &[&str] = &[
            "arrow",
            "default",
            "pointer",
            "cursor",
            "hand1",
            "hand2",
            "pointing_hand",
            "openhand",
            "closedhand",
            "grab",
            "grabbing",
            "sb_h_double_arrow",
            "sb_v_double_arrow",
            "sb_up_arrow",
            "sb_down_arrow",
            "sb_left_arrow",
            "sb_right_arrow",
            "double_arrow",
            "top_side",
            "bottom_side",
            "left_side",
            "right_side",
            "top_left_corner",
            "top_right_corner",
            "bottom_left_corner",
            "bottom_right_corner",
            "fd_double_arrow",
            "bd_double_arrow",
            "n-resize",
            "s-resize",
            "e-resize",
            "w-resize",
            "ne-resize",
            "nw-resize",
            "se-resize",
            "sw-resize",
            "col-resize",
            "row-resize",
            "all-scroll",
            "cross",
            "crosshair",
            "text",
            "xterm",
            "ibeam",
            "vertical-text",
            "wait",
            "watch",
            "progress",
            "left_ptr_watch",
            "forbidden",
            "not-allowed",
            "crossed_circle",
            "pirate",
            "help",
            "question_arrow",
            "context-menu",
            "cell",
            "alias",
            "copy",
            "no-drop",
            "move",
            "dnd-move",
            "dnd-copy",
            "dnd-link",
            "dnd-none",
            "zoom-in",
            "zoom-out",
            "pencil",
            "color-picker",
            "center_ptr",
            "circle",
        ];
        let mut wrote_any = false;
        for root in theme_install_roots() {
            let base = root.join("SlipstreamInvisible");
            let cursors = base.join("cursors");
            if let Err(e) = std::fs::create_dir_all(&cursors) {
                tracing::debug!(error = %e, path = %cursors.display(), "skip cursor theme root");
                continue;
            }
            std::fs::write(
                base.join("index.theme"),
                "[Icon Theme]\nName=SlipstreamInvisible\nComment=Slipstream host-cursor hide\nInherits=\n",
            )?;
            write_blank_xcursor(&cursors.join("left_ptr"))?;
            for name in NAMES {
                let dest = cursors.join(name);
                if dest.exists() || dest.is_symlink() {
                    continue;
                }
                let _ = std::os::unix::fs::symlink("left_ptr", &dest);
            }
            wrote_any = true;
        }
        if !wrote_any {
            anyhow::bail!("could not write SlipstreamInvisible under any icon theme root");
        }
        Ok(())
    }

    /// Multi-size fully transparent XCursor (16/24/32/48/64) so GNOME's configured size hits.
    /// Always rewritten so a leftover partial theme from an earlier attempt cannot stick.
    fn write_blank_xcursor(path: &std::path::Path) -> anyhow::Result<PathBuf> {
        const SIZES: &[u32] = &[16, 24, 32, 48, 64];
        // XCursor file format (libXcursor):
        // header: magic(4) header_bytes(4) version(4) ntoc(4)
        // toc entry: type(4) subtype(4) position(4)
        // chunk: header(4) type(4) subtype(4) version(4) width(4) height(4) xhot(4) yhot(4) delay(4)
        //        + width*height*4 ARGB pixels
        let magic: u32 = 0x7275_6358; // "Xcur" LE
        let header_bytes: u32 = 16;
        let version: u32 = 0x1_0000;
        let ntoc: u32 = SIZES.len() as u32;
        let chunk_type: u32 = 0xfffd_0002; // IMAGE
        let toc_bytes = 12 * ntoc;
        let mut pos = header_bytes + toc_bytes;
        let mut toc: Vec<(u32, u32, u32)> = Vec::with_capacity(SIZES.len());
        let mut chunks: Vec<Vec<u8>> = Vec::with_capacity(SIZES.len());
        for &size in SIZES {
            let pixels = (size * size) as usize;
            let mut chunk = Vec::with_capacity(36 + pixels * 4);
            for v in [36u32, chunk_type, size, 1, size, size, 0, 0, 0] {
                chunk.extend_from_slice(&v.to_le_bytes());
            }
            chunk.resize(chunk.len() + pixels * 4, 0);
            toc.push((chunk_type, size, pos));
            pos += chunk.len() as u32;
            chunks.push(chunk);
        }
        let mut buf: Vec<u8> = Vec::with_capacity(pos as usize);
        for v in [magic, header_bytes, version, ntoc] {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        for (typ, subtype, p) in toc {
            for v in [typ, subtype, p] {
                buf.extend_from_slice(&v.to_le_bytes());
            }
        }
        for chunk in chunks {
            buf.extend_from_slice(&chunk);
        }
        std::fs::write(path, &buf)?;
        Ok(path.to_path_buf())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn blank_xcursor_is_multi_size_and_transparent() {
            let dir = std::env::temp_dir().join(format!(
                "ss-cursor-blank-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join("left_ptr");
            write_blank_xcursor(&path).expect("write");
            let bytes = std::fs::read(&path).expect("read");
            assert_eq!(&bytes[0..4], b"Xcur");
            let ntoc = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
            assert_eq!(ntoc, 5, "expected five sizes");
            // Every IMAGE chunk's pixel payload is all zeros (fully transparent).
            let mut off = 16usize;
            for _ in 0..ntoc {
                let pos = u32::from_le_bytes(bytes[off + 8..off + 12].try_into().unwrap()) as usize;
                off += 12;
                let w = u32::from_le_bytes(bytes[pos + 16..pos + 20].try_into().unwrap()) as usize;
                let h = u32::from_le_bytes(bytes[pos + 20..pos + 24].try_into().unwrap()) as usize;
                let pix = &bytes[pos + 36..pos + 36 + w * h * 4];
                assert!(pix.iter().all(|&b| b == 0), "{w}x{h} not blank");
            }
            let _ = std::fs::remove_dir_all(&dir);
        }

        #[test]
        fn ensure_invisible_theme_rewrites_leftover_in_icons_root() {
            let home = std::env::temp_dir().join(format!(
                "ss-cursor-home-{}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&home);
            std::fs::create_dir_all(home.join(".icons")).unwrap();
            std::fs::create_dir_all(home.join(".local/share")).unwrap();
            // SAFETY: test-only env mutate for this process.
            unsafe {
                std::env::set_var("HOME", &home);
                std::env::remove_var("XDG_DATA_HOME");
            }
            let cursors = home.join(".icons/SlipstreamInvisible/cursors");
            std::fs::create_dir_all(&cursors).unwrap();
            std::fs::write(cursors.join("left_ptr"), vec![0u8; 70_000]).unwrap();
            ensure_invisible_theme().expect("ensure");
            let len = std::fs::metadata(cursors.join("left_ptr")).unwrap().len();
            assert!(len < 40_000, "leftover ~/.icons theme not rewritten: {len}");
            let data_len = std::fs::metadata(
                home.join(".local/share/icons/SlipstreamInvisible/cursors/left_ptr"),
            )
            .unwrap()
            .len();
            assert!(data_len < 40_000, "XDG data theme missing/bad: {data_len}");
            unsafe {
                std::env::remove_var("HOME");
            }
            let _ = std::fs::remove_dir_all(&home);
        }
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use windows::Win32::UI::WindowsAndMessaging::ShowCursor;

    pub(super) struct Inner {
        /// How many times we called ShowCursor(FALSE) successfully (restore with ShowCursor(TRUE)).
        hides: i32,
    }

    impl Inner {
        pub(super) fn acquire() -> Option<Self> {
            // ShowCursor returns the display count; keep hiding until the cursor is not shown
            // (count < 0), matching typical "force hide" loops.
            let mut hides = 0i32;
            // SAFETY: ShowCursor is a process-wide counter with no pointer args.
            let mut count = unsafe { ShowCursor(false) };
            hides += 1;
            let mut guard = 0;
            while count >= 0 && guard < 64 {
                count = unsafe { ShowCursor(false) };
                hides += 1;
                guard += 1;
            }
            if count >= 0 {
                tracing::warn!(
                    count,
                    "host cursor hide: ShowCursor could not drive the display count negative"
                );
            } else {
                tracing::info!(hides, "host cursor hide: ShowCursor(FALSE)");
            }
            Some(Self { hides })
        }
    }

    impl Drop for Inner {
        fn drop(&mut self) {
            for _ in 0..self.hides {
                // SAFETY: paired ShowCursor(TRUE) for each FALSE above.
                let _ = unsafe { ShowCursor(true) };
            }
        }
    }
}

#[cfg(all(test, target_os = "linux"))]
mod live_smoke {
    #[test]
    #[ignore = "touches the live GNOME cursor theme; run with --ignored"]
    fn acquire_hides_and_restores() {
        let prev = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "cursor-theme"])
            .output()
            .expect("gsettings get");
        let hide = crate::host_cursor::PlatformHide::acquire();
        assert!(hide.is_some(), "acquire should succeed on GNOME Wayland");
        let now = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "cursor-theme"])
            .output()
            .expect("gsettings get");
        let now_s = String::from_utf8_lossy(&now.stdout);
        assert!(
            now_s.contains("SlipstreamInvisible"),
            "expected SlipstreamInvisible, got {now_s}"
        );
        assert!(ss_capture::host_cursor_flag::is_hidden_for_stream());
        drop(hide);
        let after = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "cursor-theme"])
            .output()
            .expect("gsettings get");
        assert_eq!(
            String::from_utf8_lossy(&after.stdout).trim(),
            String::from_utf8_lossy(&prev.stdout).trim()
        );
        assert!(!ss_capture::host_cursor_flag::is_hidden_for_stream());
        let len = std::fs::metadata(
            dirs_next_or_home().join("icons/SlipstreamInvisible/cursors/left_ptr"),
        )
        .map(|m| m.len())
        .unwrap_or(0);
        assert!(len < 40_000, "blank left_ptr should be rewritten, got {len}");
    }

    fn dirs_next_or_home() -> std::path::PathBuf {
        std::env::var_os("XDG_DATA_HOME")
            .filter(|s| !s.is_empty())
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share")))
            .unwrap_or_else(|| std::path::PathBuf::from(".local/share"))
    }
}
