//! The present seam. Two traits — [`Decoder`] (bitstream AU → 32-bit BGRA) and [`Presenter`]
//! (BGRA →
//! screen) — so the Windows shell can land in stages without the session loop changing shape:
//! today `PassthroughDecoder` counts AUs and `ConsolePresenter` reports them; the D3D11VA decode
//! backend and the winit/softbuffer window attach at the same two points (see PR.md).
//!
//! The only pixels produced today come from [`TestPattern`], which is deliberately pure and
//! tested: it is what proves the present path on a box with no FFmpeg development SDK.

// The seam is consumed by the tests here and by the window / D3D11VA backend next; the default
// build intentionally reaches only the counting presenter.
#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedFrame {
    pub width: u32,
    pub height: u32,
    /// Tightly packed 32-bit pixels in GDI's 32-bpp `BI_RGB` byte order (B, G, R, X), which is also
    /// what `ffmpeg -pix_fmt bgra` emits: `width * height * 4` bytes, row 0 first. Choosing this
    /// order at the decoder means the presenter never has to swizzle 2 M pixels per frame.
    pub pixels: Vec<u8>,
    pub frame_index: u32,
    pub pts_ns: u64,
}

/// Bitstream access unit in, 32-bit BGRA out.
pub trait Decoder {
    /// `codec` is the host's negotiated `CODEC_*` byte (`welcome.codec`). `None` means "this
    /// backend cannot decode this codec" — the session keeps receiving, it just presents nothing.
    fn decode(&mut self, au: &[u8], codec: u8) -> Option<DecodedFrame>;
    /// The delivered geometry changed (the host corrected the requested mode to what it can
    /// actually encode). Backends that size their output from it rebuild here.
    fn resize(&mut self, _width: u32, _height: u32) {}
    /// Flow-control instrumentation: `(max frame-queue depth seen, milliseconds blocked writing an
    /// AU)`. Backends without a queue report zeros.
    fn flow_stats(&self) -> (usize, u64) {
        (0, 0)
    }
}

/// One surface to draw on. `None` = no new picture (the presenter keeps the last frame).
pub trait Presenter {
    fn present(&mut self, frame: Option<&DecodedFrame>);
    fn status(&mut self, message: &str);
    /// Diagnostic split of the last present's cost: `(convert_ns, blit_ns)`. Backends that do
    /// neither report zeros.
    fn stage_ns(&self) -> (u64, u64) {
        (0, 0)
    }
    /// The delivered geometry changed (the host corrected the requested mode to what it can
    /// actually encode). Backends that own a surface reallocate it here; the next frame is
    /// expected at the new size.
    fn resize(&mut self, _width: u32, _height: u32) {}
    /// True once the user closed the window (the session loop treats it as "stop").
    fn closed(&self) -> bool {
        false
    }
}

/// No decoder: the honest state of this crate before the hardware path lands. The session still
/// validates the whole transport (every AU reaches this method) and `--record` still writes a
/// playable file.
pub struct PassthroughDecoder {
    pub aus: u64,
    pub bytes: u64,
}

impl Default for PassthroughDecoder {
    fn default() -> Self {
        Self { aus: 0, bytes: 0 }
    }
}

impl Decoder for PassthroughDecoder {
    fn decode(&mut self, au: &[u8], _codec: u8) -> Option<DecodedFrame> {
        self.aus += 1;
        self.bytes += au.len() as u64;
        None
    }
}

/// A deterministic moving-bar test pattern. Pure so the present path is testable without a GPU.
pub struct TestPattern {
    pub width: u32,
    pub height: u32,
    pub phase: u32,
}

impl TestPattern {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width: width.max(2),
            height: height.max(2),
            phase: 0,
        }
    }

    /// Eight vertical bars plus one bar that moves with `phase`, so a stalled stream is visibly
    /// stalled.
    pub fn render(&mut self) -> DecodedFrame {
        let (w, h) = (self.width as usize, self.height as usize);
        let mut pixels = vec![0u8; w * h * 4];
        let bar = (w / 9).max(1);
        let moving = (self.phase as usize * bar) % w;
        for y in 0..h {
            // A dim gradient keeps the image from looking like a solid fill.
            let tint = (y * 128 / h) as u8;
            for x in 0..w {
                let i = (y * w + x) * 4;
                let (r, g, b): (u8, u8, u8) = if (x / bar) % 2 == 0 {
                    (235, 235, 235)
                } else {
                    (30, 30, 30)
                };
                let (r, g, b): (u8, u8, u8) = if x >= moving && x < moving + bar {
                    (40, 190, 255)
                } else {
                    (r, g, b)
                };
                // B, G, R, X — the DIB byte order.
                pixels[i] = b.saturating_sub(tint / 4);
                pixels[i + 1] = g.saturating_sub(tint / 4);
                pixels[i + 2] = r.saturating_sub(tint / 4);
                pixels[i + 3] = 255;
            }
        }
        self.phase = self.phase.wrapping_add(1);
        DecodedFrame {
            width: self.width,
            height: self.height,
            pixels,
            frame_index: self.phase,
            pts_ns: 0,
        }
    }
}

/// The headless presenter: logs throughput and the session status. It is what the winit window
/// will replace; the trait is identical so nothing in the session loop changes.
pub struct ConsolePresenter {
    pub frames: u64,
    pub bytes: u64,
    last_log: std::time::Instant,
}

impl Default for ConsolePresenter {
    fn default() -> Self {
        Self {
            frames: 0,
            bytes: 0,
            last_log: std::time::Instant::now(),
        }
    }
}

impl Presenter for ConsolePresenter {
    fn present(&mut self, frame: Option<&DecodedFrame>) {
        if let Some(f) = frame {
            self.frames += 1;
            self.bytes += f.pixels.len() as u64;
        }
        if self.last_log.elapsed() >= std::time::Duration::from_secs(2) {
            tracing::info!(
                framed = self.frames,
                mib = self.bytes / (1024 * 1024),
                "present"
            );
            self.last_log = std::time::Instant::now();
        }
    }

    fn status(&mut self, message: &str) {
        tracing::info!(%message, "status");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_counts_every_au_and_presents_nothing() {
        let mut d = PassthroughDecoder::default();
        assert!(d.decode(&[1, 2, 3], 0).is_none());
        assert!(d.decode(&[4], 0).is_none());
        assert_eq!(d.aus, 2);
        assert_eq!(d.bytes, 4);
    }

    #[test]
    fn test_pattern_is_bgra_and_opaque() {
        let f = TestPattern::new(64, 32).render();
        assert_eq!(f.width, 64);
        assert_eq!(f.height, 32);
        assert_eq!(f.pixels.len(), 64 * 32 * 4);
        assert!(f.pixels.chunks_exact(4).all(|px| px[3] == 255));
    }

    #[test]
    fn test_pattern_moves_between_frames() {
        let mut p = TestPattern::new(90, 10);
        let a = p.render();
        let b = p.render();
        assert_ne!(a.pixels, b.pixels, "the moving bar must change position");
        assert_eq!(b.frame_index, a.frame_index.wrapping_add(1));
    }

    #[test]
    fn tiny_and_degenerate_modes_do_not_panic() {
        let f = TestPattern::new(0, 0).render();
        assert_eq!(f.pixels.len(), 2 * 2 * 4);
    }

    #[test]
    fn console_presenter_status_does_not_count_frames() {
        let mut p = ConsolePresenter::default();
        p.status("hello");
        assert_eq!(p.frames, 0);
        let f = TestPattern::new(2, 2).render();
        p.present(Some(&f));
        assert_eq!(p.frames, 1);
    }
}
