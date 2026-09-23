//! Windows host clipboard: sequence-number polling + eager client-content writes.
//!
//! Win32 has no session-clipboard protocol — the clipboard is global, and serving bytes
//! on demand (delayed rendering) needs a message-loop window a service host does not
//! have. So this backend splits the two directions asymmetrically:
//!
//! * **host copy → client**: a poll thread watches `GetClipboardSequenceNumber` and
//!   emits [`ClipEvent::Selection`] with the readable wire MIMEs; `read_current`
//!   serves fetches straight from the live clipboard.
//! * **client copy → host**: `set_offer` emits a synthetic [`ClipEvent::Paste`] for the
//!   preferred format; the coordinator fetches the bytes from the client and the
//!   backend's writer task installs them eagerly with `SetClipboardData` — host apps
//!   then paste natively. Our own write's sequence is recorded so the poll loop never
//!   echoes it back as a new host copy.
//!
//! Formats (both directions): `CF_UNICODETEXT` ⇄ text, `HTML Format` ⇄ html (fragment
//! extraction / envelope), `Rich Text Format` ⇄ rtf (raw), `CF_DIB` ⇄ png (BMP
//! transcode via the `image` crate). Anything else is dropped, never misconverted.

use super::{ClipEvent, PasteResponder, WIRE_HTML, WIRE_PNG, WIRE_RTF, WIRE_TEXT};
use anyhow::{Context, Result};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tokio::sync::{mpsc::UnboundedSender, oneshot};
use windows::Win32::{
    Foundation::{HANDLE, HGLOBAL, HWND},
    System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
            GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard,
            RegisterClipboardFormatW, SetClipboardData,
        },
        Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE},
    },
};

/// Standard clipboard ids (not projected as constants — spelled literally).
const CF_TEXT: u32 = 1;
const CF_DIB: u32 = 8;
const CF_UNICODETEXT: u32 = 13;

/// Poll interval for host-copy detection.
const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);

/// Shared backend state handle (writer tasks + poll loop reach the same state).
type SharedState = Arc<Mutex<BackendState>>;

/// Windows host clipboard backend.
pub struct WindowsClipboard {
    event_tx: UnboundedSender<ClipEvent>,
    shared: SharedState,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

struct BackendState {
    /// Sequence right after our own eager write — the poll loop never echoes it.
    /// (Change detection lives in the poll thread's own local; see `poll_loop`.)
    own_seq: u32,
    /// Latest client offer still current (for `clear_offer` matching).
    pending_mime: Option<String>,
    /// Writer-task generation: a stale writer (superseded offer) drops its bytes.
    generation: u64,
}

impl WindowsClipboard {
    /// Open the backend: verify the clipboard opens once (fail loudly when another
    /// session/desktop owns it), snapshot the sequence, start the poll thread.
    pub fn open() -> Result<(Self, tokio::sync::mpsc::UnboundedReceiver<ClipEvent>)> {
        let (event_tx, rx) = tokio::sync::mpsc::unbounded_channel();
        // Prove the clipboard opens before the coordinator depends on us.
        with_clipboard("probe", || Ok(()))?;
        let shared: SharedState = Arc::new(Mutex::new(BackendState {
            own_seq: 0,
            pending_mime: None,
            generation: 0,
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let worker = std::thread::Builder::new()
            .name("slipstream-clipboard".into())
            .spawn({
                let (stop, event_tx, shared) = (stop.clone(), event_tx.clone(), shared.clone());
                move || poll_loop(stop, event_tx, shared)
            })
            .context("spawn clipboard poll thread")?;
        Ok((
            WindowsClipboard {
                event_tx,
                shared,
                stop,
                worker: Some(worker),
            },
            rx,
        ))
    }

    /// Current host selection's wire MIMEs, read live (empty = nothing offerable).
    pub fn current_wire_mimes(&self) -> Vec<String> {
        readable_wire_mimes().unwrap_or_default()
    }

    /// Install a client offer: remember it, then emit a synthetic `Paste` for the
    /// preferred format so the coordinator fetches the bytes and the writer task
    /// installs them eagerly. Formats we cannot write are recorded but never pasted.
    pub fn set_offer(&self, wire_mimes: &[String]) -> Result<()> {
        let Some(mime) = preferred_mime(wire_mimes) else {
            tracing::debug!("client offered no writable clipboard format — recorded only");
            self.shared.lock().unwrap().pending_mime = None;
            return Ok(());
        };
        let (tx, rx) = oneshot::channel::<Vec<u8>>();
        let (generation, event_tx, shared) = {
            let mut st = self.shared.lock().unwrap();
            st.generation += 1;
            st.pending_mime = Some(mime.to_string());
            (st.generation, self.event_tx.clone(), self.shared.clone())
        };
        tokio::spawn(async move {
            // Coordinator gone (session ending) → nothing to write.
            if let Ok(bytes) = rx.await {
                // Stale writer (a newer offer superseded this one) drops its bytes —
                // last-writer-wins keeps rapid copy-copy-paste coherent.
                let current = shared.lock().unwrap().generation;
                if current != generation {
                    return;
                }
                if let Err(e) = write_clipboard(mime, &bytes) {
                    tracing::warn!(error = %format!("{e:#}"), "eager clipboard write failed");
                    return;
                }
                shared.lock().unwrap().own_seq = current_sequence();
            }
        });
        let _ = event_tx.send(ClipEvent::Paste {
            mime: mime.to_string(),
            responder: PasteResponder::WindowsEager(tx),
        });
        Ok(())
    }

    /// Drop the client offer we hold; clear the system clipboard only if it still
    /// carries our eager write (never someone else's copy).
    pub fn clear_offer(&self) -> Result<()> {
        let mut st = self.shared.lock().unwrap();
        st.pending_mime = None;
        st.generation += 1; // retire any in-flight writer
        if current_sequence() == st.own_seq && st.own_seq != 0 {
            drop(st);
            with_clipboard("clear", || {
                // SAFETY: open clipboard owned by this call; emptying our own write.
                unsafe {
                    EmptyClipboard().context("EmptyClipboard")?;
                }
                Ok(())
            })?;
            self.shared.lock().unwrap().own_seq = 0;
        }
        Ok(())
    }
}

impl Drop for WindowsClipboard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

/// Run `f` with the clipboard open (retried briefly — another app may hold it), always
/// closed afterwards.
fn with_clipboard<T>(what: &str, f: impl FnOnce() -> Result<T>) -> Result<T> {
    let mut last: Option<anyhow::Error> = None;
    for _ in 0..10 {
        // SAFETY: null owner (no window); open/close strictly paired in this scope.
        let opened = unsafe { OpenClipboard(HWND::default()).is_ok() };
        if opened {
            let r = f();
            // SAFETY: balances the successful `OpenClipboard` above on this thread.
            unsafe {
                let _ = CloseClipboard();
            }
            return r.context(what.to_string());
        }
        last = Some(anyhow::anyhow!("clipboard busy"));
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    Err(last.unwrap())
}

/// Current clipboard sequence (changes on every write, by anyone).
fn current_sequence() -> u32 {
    // SAFETY: nullary read, no pointers.
    unsafe { GetClipboardSequenceNumber() }
}

/// Poll loop: announce host copies as `Selection` events, skipping our own eager writes
/// and transient busy states (retried next tick, never announced as cleared).
fn poll_loop(stop: Arc<AtomicBool>, event_tx: UnboundedSender<ClipEvent>, shared: SharedState) {
    let mut last_seen = current_sequence();
    while !stop.load(Ordering::SeqCst) {
        std::thread::sleep(POLL_INTERVAL);
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let seq = current_sequence();
        if seq == last_seen {
            continue;
        }
        last_seen = seq;
        // Our own eager write is not a host copy — never echo it back to the client.
        if seq == shared.lock().unwrap().own_seq {
            continue;
        }
        match readable_wire_mimes() {
            Ok(mimes) => {
                let _ = event_tx.send(ClipEvent::Selection { mimes });
            }
            Err(e) => {
                tracing::debug!(error = %format!("{e:#}"), "clipboard poll read failed — retrying");
                // Leave `last_seen` advanced: a transient busy clipboard must not wedge
                // the loop, and the next real change re-announces anyway.
            }
        }
    }
}

/// Wire MIMEs the live clipboard can serve right now (empty = cleared or unreadable).
/// Fails when the clipboard cannot be opened (caller retries next tick).
fn readable_wire_mimes() -> Result<Vec<String>> {
    with_clipboard("enumerate", || {
        // SAFETY: synchronous enumeration on the open clipboard; format ids only.
        let mut formats = Vec::new();
        unsafe {
            let mut fmt = EnumClipboardFormats(0);
            while fmt != 0 {
                formats.push(fmt);
                fmt = EnumClipboardFormats(fmt);
            }
        }
        let mut out = Vec::new();
        if formats.contains(&CF_UNICODETEXT) || formats.contains(&CF_TEXT) {
            out.push(WIRE_TEXT.to_string());
        }
        if has_registered_format("HTML Format") {
            out.push(WIRE_HTML.to_string());
        }
        if has_registered_format("Rich Text Format") {
            out.push(WIRE_RTF.to_string());
        }
        if formats.contains(&CF_DIB) {
            out.push(WIRE_PNG.to_string());
        }
        Ok(out)
    })
}

/// Whether a registered format (by name) is on the open clipboard.
fn has_registered_format(name: &str) -> bool {
    let id = registered_format_id(name);
    if id == 0 {
        return false;
    }
    // SAFETY: id is a live registered format; availability is a synchronous query.
    unsafe { IsClipboardFormatAvailable(id).is_ok() }
}

/// Registered format id by name (0 = unregistered on this box).
fn registered_format_id(name: &str) -> u32 {
    use windows::core::HSTRING;
    // SAFETY: name is a live string for the call; the id is valid process-wide after.
    unsafe { RegisterClipboardFormatW(&HSTRING::from(name)) }
}

/// Preferred writable wire MIME of a client offer (text > html > rtf > png).
fn preferred_mime(mimes: &[String]) -> Option<&'static str> {
    [WIRE_TEXT, WIRE_HTML, WIRE_RTF, WIRE_PNG]
        .into_iter()
        .find(|want| mimes.iter().any(|m| m == want))
}

/// Read one wire format from the live clipboard.
pub fn read_wire_format(wire_mime: &str) -> Result<Vec<u8>> {
    with_clipboard("read", || match wire_mime {
        WIRE_TEXT => read_text(),
        WIRE_HTML => read_html(),
        WIRE_RTF => read_registered("Rich Text Format"),
        WIRE_PNG => read_dib_as_png(),
        _ => anyhow::bail!("unsupported clipboard format {wire_mime:?}"),
    })
}

/// Read `CF_UNICODETEXT` as UTF-8 (trailing NULs trimmed).
fn read_text() -> Result<Vec<u8>> {
    // SAFETY: handle from `GetClipboardData` on the open clipboard; locked, copied,
    // unlocked synchronously — never retained. Null/empty guards below.
    unsafe {
        let handle = GetClipboardData(CF_UNICODETEXT).context("no unicode text")?;
        let ptr = GlobalLock(HGLOBAL(handle.0));
        if ptr.is_null() {
            anyhow::bail!("clipboard text lock failed");
        }
        let len_chars = (0..)
            .take_while(|&i| *(ptr as *const u16).add(i) != 0)
            .count();
        let slice = std::slice::from_raw_parts(ptr as *const u16, len_chars);
        let text = String::from_utf16_lossy(slice);
        let _ = GlobalUnlock(HGLOBAL(handle.0));
        Ok(text.into_bytes())
    }
}

/// Read the `HTML Format` fragment (between the `StartFragment`/`EndFragment` byte
/// offsets) as UTF-8.
fn read_html() -> Result<Vec<u8>> {
    let raw = read_registered("HTML Format")?;
    let text = String::from_utf8_lossy(&raw);
    let start = fragment_offset(&text, "StartFragment:").context("no StartFragment")?;
    let end = fragment_offset(&text, "EndFragment:").context("no EndFragment")?;
    let (start, end) = (start.min(raw.len()), end.min(raw.len()).max(start));
    Ok(raw[start..end].to_vec())
}

/// Parse a `Name:0000001234` byte offset out of an `HTML Format` header.
fn fragment_offset(header: &str, key: &str) -> Option<usize> {
    let at = header.find(key)? + key.len();
    header[at..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()
}

/// Read a registered raw format's bytes verbatim.
fn read_registered(name: &str) -> Result<Vec<u8>> {
    let id = registered_format_id(name);
    if id == 0 {
        anyhow::bail!("{name} is not registered");
    }
    read_format_bytes(id).with_context(|| format!("no {name}"))
}

/// Read `CF_DIB` and transcode it to PNG (universal fallback — never fabricated JPEG).
fn read_dib_as_png() -> Result<Vec<u8>> {
    let dib = read_format_bytes(CF_DIB).context("no DIB")?;
    let rgba = dib_to_rgba(&dib).context("DIB decode")?;
    let mut png = Vec::new();
    {
        use std::io::Cursor;
        image::DynamicImage::ImageRgba8(rgba)
            .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
            .context("PNG encode")?;
    }
    Ok(png)
}

/// Read raw bytes of one clipboard format id.
fn read_format_bytes(format: u32) -> Result<Vec<u8>> {
    // SAFETY: handle from `GetClipboardData` on the open clipboard; sized, locked,
    // copied, unlocked synchronously — never retained.
    unsafe {
        let handle = GetClipboardData(format).context("GetClipboardData")?;
        let hglobal = HGLOBAL(handle.0);
        let size = GlobalSize(hglobal);
        if size == 0 {
            anyhow::bail!("clipboard format is empty");
        }
        let ptr = GlobalLock(hglobal);
        if ptr.is_null() {
            anyhow::bail!("clipboard lock failed");
        }
        let mut buf = vec![0u8; size];
        std::ptr::copy_nonoverlapping(ptr as *const u8, buf.as_mut_ptr(), size);
        let _ = GlobalUnlock(hglobal);
        Ok(buf)
    }
}

/// BITMAPINFOHEADER (40 bytes) fields read off a DIB, little-endian.
struct BmpHeader {
    width: i32,
    height: i32, // >0 = bottom-up, <0 = top-down
    bit_count: u16,
    compression: u32,
}

/// Parse the DIB header (only uncompressed 24/32-bit — anything else fails loudly
/// instead of misdecoding).
fn parse_bmp_header(dib: &[u8]) -> Result<(BmpHeader, usize)> {
    if dib.len() < 40 {
        anyhow::bail!("DIB too small ({})", dib.len());
    }
    let u32le = |o: usize| u32::from_le_bytes(dib[o..o + 4].try_into().unwrap());
    let u16le = |o: usize| u16::from_le_bytes(dib[o..o + 2].try_into().unwrap());
    let header = BmpHeader {
        width: u32le(4) as i32,
        height: u32le(8) as i32,
        bit_count: u16le(14),
        compression: u32le(16),
    };
    if header.width <= 0 || header.height == 0 {
        anyhow::bail!("bad DIB dims {}x{}", header.width, header.height);
    }
    if header.compression != 0 {
        anyhow::bail!(
            "compressed DIB unsupported (compression {})",
            header.compression
        );
    }
    if header.bit_count != 24 && header.bit_count != 32 {
        anyhow::bail!("DIB depth {} unsupported (want 24/32)", header.bit_count);
    }
    Ok((header, 40))
}

/// Decode an in-memory DIB (`CF_DIB` bytes) to RGBA pixels.
fn dib_to_rgba(dib: &[u8]) -> Result<image::RgbaImage> {
    let (header, pixels_at) = parse_bmp_header(dib)?;
    let (w, h) = (header.width as u32, header.height.unsigned_abs());
    let bpp = header.bit_count as usize / 8;
    let stride = (w as usize * bpp + 3) & !3;
    if dib.len() < pixels_at + stride * h as usize {
        anyhow::bail!("DIB truncated");
    }
    let mut rgba = Vec::with_capacity(w as usize * h as usize * 4);
    for y in 0..h {
        // Bottom-up storage when height > 0; top-down otherwise.
        let src_y = if header.height > 0 { h - 1 - y } else { y };
        let row = &dib[pixels_at + src_y as usize * stride..];
        for x in 0..w as usize {
            let p = &row[x * bpp..];
            rgba.extend_from_slice(&[p[2], p[1], p[0], if bpp == 4 { p[3] } else { 255 }]);
        }
    }
    image::RgbaImage::from_raw(w, h, rgba).context("RGBA assemble")
}

/// Install `bytes` (a wire format) into the system clipboard, replacing our selection.
fn write_clipboard(wire_mime: &str, bytes: &[u8]) -> Result<()> {
    let (format, payload): (u32, Vec<u8>) = match wire_mime {
        WIRE_TEXT => (
            CF_UNICODETEXT,
            utf8_to_utf16_nul(std::str::from_utf8(bytes).context("text is not UTF-8")?)
                .iter()
                .flat_map(|u| u.to_le_bytes())
                .collect(),
        ),
        WIRE_HTML => (
            registered_format_id("HTML Format"),
            html_envelope(std::str::from_utf8(bytes).context("html is not UTF-8")?),
        ),
        WIRE_RTF => (registered_format_id("Rich Text Format"), bytes.to_vec()),
        WIRE_PNG => (CF_DIB, png_to_dib(bytes).context("PNG→DIB")?),
        _ => anyhow::bail!("cannot write clipboard format {wire_mime:?}"),
    };
    if format == 0 {
        anyhow::bail!("clipboard format for {wire_mime:?} is not registered");
    }
    with_clipboard("write", || {
        // SAFETY: `EmptyClipboard` takes ownership of the selection; the allocated
        // block is handed to `SetClipboardData` (system-owned on success, freed here
        // on failure) — never double-owned.
        unsafe {
            EmptyClipboard().context("EmptyClipboard")?;
            let hglobal =
                GlobalAlloc(GMEM_MOVEABLE, payload.len().max(1)).context("GlobalAlloc")?;
            let ptr = GlobalLock(hglobal);
            if ptr.is_null() {
                let _ = windows::Win32::Foundation::GlobalFree(hglobal);
                anyhow::bail!("clipboard lock failed");
            }
            std::ptr::copy_nonoverlapping(payload.as_ptr(), ptr as *mut u8, payload.len());
            let _ = GlobalUnlock(hglobal);
            if SetClipboardData(format, HANDLE(hglobal.0)).is_err() {
                let _ = windows::Win32::Foundation::GlobalFree(hglobal);
                anyhow::bail!("SetClipboardData");
            }
            Ok(())
        }
    })
}

/// UTF-8 → NUL-terminated UTF-16 units.
fn utf8_to_utf16_nul(text: &str) -> Vec<u16> {
    let mut units: Vec<u16> = text.encode_utf16().collect();
    units.push(0);
    units
}

/// Wrap an HTML fragment in the `HTML Format` envelope with byte-accurate offsets.
fn html_envelope(fragment: &str) -> Vec<u8> {
    let pre = "<html><body>\r\n<!--StartFragment-->";
    let post = "<!--EndFragment-->\r\n</body>\r\n</html>";
    // Fixed-width header: all offsets fit in 8 digits for sane fragments.
    let header_len = 105usize;
    let html_start = header_len;
    let frag_start = html_start + pre.len();
    let frag_end = frag_start + fragment.len();
    let html_end = frag_end + post.len();
    let header = format!(
        "Version:0.9\r\nStartHTML:{html_start:08}\r\nEndHTML:{html_end:08}\r\nStartFragment:{frag_start:08}\r\nEndFragment:{frag_end:08}\r\n"
    );
    debug_assert_eq!(header.len(), header_len);
    let mut out = header.into_bytes();
    out.extend_from_slice(pre.as_bytes());
    out.extend_from_slice(fragment.as_bytes());
    out.extend_from_slice(post.as_bytes());
    out
}

/// Decode PNG bytes into a bottom-up BGR(A) DIB (`BITMAPINFOHEADER` + pixels).
fn png_to_dib(png: &[u8]) -> Result<Vec<u8>> {
    let img = image::load_from_memory(png)
        .context("PNG decode")?
        .to_rgba8();
    let (w, h) = img.dimensions();
    let mut dib = Vec::with_capacity(40 + w as usize * h as usize * 4);
    dib.extend_from_slice(&40u32.to_le_bytes()); // biSize
    dib.extend_from_slice(&(w as i32).to_le_bytes());
    dib.extend_from_slice(&(h as i32).to_le_bytes()); // >0 = bottom-up
    dib.extend_from_slice(&1u16.to_le_bytes()); // planes
    dib.extend_from_slice(&32u16.to_le_bytes()); // 32-bit
    dib.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    dib.extend_from_slice(&0u32.to_le_bytes()); // image size (0 = BI_RGB)
    dib.extend_from_slice(&0u32.to_le_bytes()); // XPelsPerMeter
    dib.extend_from_slice(&0u32.to_le_bytes()); // YPelsPerMeter
    dib.extend_from_slice(&0u32.to_le_bytes()); // clr used
    dib.extend_from_slice(&0u32.to_le_bytes()); // clr important
    for y in (0..h).rev() {
        for x in 0..w {
            let p = img.get_pixel(x, y).0;
            dib.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
        }
    }
    Ok(dib)
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn fragment_offsets_parse_ms_style_headers() {
        let header = "Version:0.9\r\nStartHTML:00000097\r\nEndHTML:00000123\r\nStartFragment:00000105\r\nEndFragment:00000117\r\n";
        assert_eq!(fragment_offset(header, "StartFragment:"), Some(105));
        assert_eq!(fragment_offset(header, "EndFragment:"), Some(117));
        assert_eq!(fragment_offset(header, "Nope:"), None);
    }

    #[test]
    fn envelope_round_trips_through_the_reader() {
        let frag = "<p>hi</p>";
        let env = html_envelope(frag);
        let text = String::from_utf8_lossy(&env).into_owned();
        let start = fragment_offset(&text, "StartFragment:").unwrap();
        let end = fragment_offset(&text, "EndFragment:").unwrap();
        assert_eq!(&env[start..end], frag.as_bytes());
        // Offsets really point at the fragment (not just self-consistent math).
        assert!(text[start..].starts_with(frag));
    }

    #[test]
    fn dib_decode_rejects_compressed_and_truncated() {
        assert!(parse_bmp_header(&[0u8; 10]).is_err());
        let mut hdr = vec![0u8; 40];
        hdr[0..4].copy_from_slice(&40u32.to_le_bytes());
        hdr[4..8].copy_from_slice(&2u32.to_le_bytes());
        hdr[8..12].copy_from_slice(&2u32.to_le_bytes());
        hdr[14..16].copy_from_slice(&24u16.to_le_bytes());
        hdr[16..20].copy_from_slice(&1u32.to_le_bytes()); // BI_RLE8
        assert!(parse_bmp_header(&hdr).is_err());
    }

    #[test]
    fn preferred_mime_orders_text_first() {
        assert_eq!(
            preferred_mime(&[WIRE_PNG.into(), WIRE_TEXT.into()]),
            Some(WIRE_TEXT)
        );
        assert_eq!(preferred_mime(&["application/x-foo".into()]), None);
    }
}
