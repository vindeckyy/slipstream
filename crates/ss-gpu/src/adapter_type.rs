/// `RenderSupported` — clear on display-only adapters (an IDD ghost has no render engine).
const RENDER_SUPPORTED: u32 = 1 << 0;
/// `SoftwareDevice` — WARP/Basic Render (normally already dropped via the DXGI flag).
const SOFTWARE_DEVICE: u32 = 1 << 2;
/// `IndirectDisplayDevice` — an IddCx virtual-display adapter (ss-vdisplay, Parsec VDD, …).
const INDIRECT_DISPLAY_DEVICE: u32 = 1 << 6;

/// True when these bits describe an adapter that can never be the render/encode GPU:
/// indirect-display, software, or anything without render support.
pub fn hidden(bits: u32) -> bool {
    bits & INDIRECT_DISPLAY_DEVICE != 0
        || bits & SOFTWARE_DEVICE != 0
        || bits & RENDER_SUPPORTED == 0
}
