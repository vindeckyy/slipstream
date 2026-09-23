//! DXGI Desktop Duplication: the fallback Windows source for sessions where WGC is
//! unavailable (older Windows, RDP sessions, secure-desktop edges).
//!
//! `IDXGIOutputDuplication::AcquireNextFrame` on one worker thread, staging-texture
//! readback to CPU `Bgra`, published into the shared [`FrameSlot`]. Acquired frames
//! already contain the cursor, so like WGC this backend always embeds it. A topology
//! change surfaces as `DXGI_ERROR_ACCESS_LOST` and terminally fails the capturer — the
//! session rebuild reopens it (the same recover-or-drop contract as a resize).

use super::common::{enumerate_monitors, materialize_bgra_frame, FrameSlot};
use super::{CapturedFrame, Capturer};
use anyhow::{Context, Result};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use windows::core::Interface;
use windows::Win32::Graphics::{
    Direct3D11::{
        ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11Texture2D, D3D11_CPU_ACCESS_READ,
        D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC,
        D3D11_USAGE_STAGING,
    },
    Dxgi::{
        Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC},
        CreateDXGIFactory1, IDXGIAdapter1, IDXGIFactory1, IDXGIOutput1, IDXGIOutputDuplication,
        IDXGIResource, DXGI_ERROR_ACCESS_LOST, DXGI_ERROR_WAIT_TIMEOUT, DXGI_OUTDUPL_FRAME_INFO,
    },
};

/// DXGI Desktop Duplication source. Owns the worker thread; dropping stops it.
pub struct DxgiCapturer {
    slot: Arc<FrameSlot>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl DxgiCapturer {
    /// Duplicate the primary output.
    pub fn open() -> Result<Self> {
        Self::open_for_monitor(None)
    }

    /// Duplicate the output whose GDI device name matches
    /// `SLIPSTREAM_CAPTURE_MONITOR`, else the primary head.
    pub fn open_for_monitor(monitor: Option<&str>) -> Result<Self> {
        // Factory/adapter/output enumeration takes no raw memory; every out-param is a
        // live local, and COM objects are reference-counted by the `windows` wrappers.
        let (adapter_index, output_index) = pick_output(monitor)?;
        // Default-adapter D3D11 device for the duplication; out-params are live locals,
        // no aliasing beyond the call.
        let (device, ctx) = create_device()?;
        let dupl = duplicate_output(&device, adapter_index, output_index)?;
        let (w, h) = output_size(adapter_index, output_index)?;

        let slot = FrameSlot::new();
        let stop = Arc::new(AtomicBool::new(false));
        tracing::info!(width = w, height = h, "dxgi: duplicating output");
        let worker = std::thread::Builder::new()
            .name("slipstream-dxgi".into())
            .spawn({
                let (slot, stop) = (slot.clone(), stop.clone());
                move || dxgi_worker(device, ctx, dupl, w, h, slot, stop)
            })
            .context("dxgi: spawn worker")?;
        Ok(DxgiCapturer {
            slot,
            stop,
            worker: Some(worker),
        })
    }
}

impl Drop for DxgiCapturer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

impl Capturer for DxgiCapturer {
    fn backend_name(&self) -> &'static str {
        "dxgi"
    }

    fn next_frame(&mut self) -> Result<CapturedFrame> {
        self.slot.take_blocking()
    }

    fn try_latest(&mut self) -> Result<Option<CapturedFrame>> {
        if let Some(dead) = self.slot.dead_error() {
            anyhow::bail!("dxgi worker died: {dead}");
        }
        Ok(self.slot.take())
    }

    fn supports_arrival_wait(&self) -> bool {
        true
    }

    fn wait_arrival(&mut self, deadline: std::time::Instant) {
        self.slot.wait_arrival(deadline);
    }

    fn set_active(&mut self, active: bool) {
        if !active {
            self.slot.flush();
        }
    }

    fn is_alive(&self) -> bool {
        self.slot.is_alive()
    }

    fn set_cursor_forward(&mut self, on: bool) {
        // Duplicated frames contain the cursor; no shape channel exists yet.
        if on {
            tracing::warn!(
                "dxgi: cursor-forward requested but the pointer stays embedded (no shape channel yet)"
            );
        }
    }

    fn telemetry(&self) -> super::CaptureTelemetry {
        self.slot.telemetry()
    }
}

/// Choose the (adapter, output) to duplicate: the `SLIPSTREAM_CAPTURE_MONITOR` device name
/// when it matches a DXGI output, else the GDI primary head's output, else the first
/// attached output.
/// Enumerate every attached output as `(adapter, output, device_name, _)`: the shared
/// enumeration behind picking and probing, so the two cannot drift.
fn enumerate_attached_outputs() -> Result<Vec<(u32, u32, String, bool)>> {
    // SAFETY: factory/adapter/output enumeration takes no raw memory — every out-param
    // is a live local, and the enumerated interfaces are reference-counted. `GetDesc`
    // returns its struct by value; each output outlives its desc. No frame is acquired.
    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1().context("dxgi: factory")?;
        // Collect every output with its GDI device name for matching.
        let mut outputs: Vec<(u32, u32, String, bool)> = Vec::new();
        let mut adapter_index = 0u32;
        while let Ok(adapter) = factory.EnumAdapters1(adapter_index) {
            let mut output_index = 0u32;
            while let Ok(output) = adapter.EnumOutputs(output_index) {
                let desc = output.GetDesc()?;
                if desc.AttachedToDesktop.as_bool() {
                    let name = String::from_utf16_lossy(&desc.DeviceName)
                        .trim_end_matches('\0')
                        .to_string();
                    outputs.push((adapter_index, output_index, name, false));
                }
                output_index += 1;
            }
            adapter_index += 1;
        }
        if outputs.is_empty() {
            anyhow::bail!("dxgi: no attached outputs found");
        }
        Ok(outputs)
    }
}

fn pick_output(monitor: Option<&str>) -> Result<(u32, u32)> {
    let mut outputs = enumerate_attached_outputs()?;
    // Mark the GDI primary head (same `\\.\DISPLAYn` namespace).
    if let Ok(monitors) = enumerate_monitors() {
        if let Some(primary) = monitors.iter().find(|m| m.primary) {
            for out in outputs.iter_mut() {
                if out.2.eq_ignore_ascii_case(&primary.device) {
                    out.3 = true;
                }
            }
        }
    }
    if let Some(name) = monitor.filter(|n| !n.is_empty()) {
        if let Some(out) = outputs.iter().find(|o| o.2.eq_ignore_ascii_case(name)) {
            return Ok((out.0, out.1));
        }
        anyhow::bail!(
            "capture monitor {name:?} not found (dxgi outputs: {})",
            outputs
                .iter()
                .map(|o| o.2.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    if let Some(out) = outputs.iter().find(|o| o.3) {
        return Ok((out.0, out.1));
    }
    Ok((outputs[0].0, outputs[0].1))
}

/// Default-adapter D3D11 device + immediate context for the duplication.
fn create_device() -> Result<(ID3D11Device, ID3D11DeviceContext)> {
    use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_UNKNOWN;
    use windows::Win32::Graphics::Direct3D11::D3D11CreateDevice;
    use windows::Win32::Graphics::Dxgi::IDXGIAdapter;
    let mut device = None;
    let mut ctx = None;
    // Out-params are live locals written synchronously; default adapter.
    // SAFETY: see above; the returned interfaces are reference-counted.
    unsafe {
        D3D11CreateDevice(
            None::<&IDXGIAdapter>,
            D3D_DRIVER_TYPE_UNKNOWN,
            windows::Win32::Foundation::HMODULE::default(),
            windows::Win32::Graphics::Direct3D11::D3D11_CREATE_DEVICE_FLAG(0),
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut ctx),
        )
        .context("dxgi: D3D11CreateDevice")?;
    }
    Ok((
        device.context("dxgi: no D3D11 device")?,
        ctx.context("dxgi: no immediate context")?,
    ))
}

/// Duplicate the chosen output against our device.
fn duplicate_output(
    device: &ID3D11Device,
    adapter_index: u32,
    output_index: u32,
) -> Result<IDXGIOutputDuplication> {
    // The duplication is bound to our device and released with the returned interface.
    // SAFETY: all enumerated objects are live; the duplication is reference-counted and
    // dropped on worker exit.
    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1().context("dxgi: factory")?;
        let adapter: IDXGIAdapter1 = factory
            .EnumAdapters1(adapter_index)
            .context("dxgi: adapter")?;
        let output = adapter.EnumOutputs(output_index).context("dxgi: output")?;
        let output1: IDXGIOutput1 = output.cast().context("dxgi: IDXGIOutput1")?;
        output1
            .DuplicateOutput(device)
            .context("dxgi: DuplicateOutput")
    }
}

/// The output's desktop size, for the staging texture + frame dims.
fn output_size(adapter_index: u32, output_index: u32) -> Result<(u32, u32)> {
    // SAFETY: enumerated objects are live; the returned desc owns its data.
    let (right, left, bottom, top) = unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1().context("dxgi: factory")?;
        let adapter: IDXGIAdapter1 = factory
            .EnumAdapters1(adapter_index)
            .context("dxgi: adapter")?;
        let output = adapter.EnumOutputs(output_index).context("dxgi: output")?;
        let desc = output.GetDesc().context("dxgi: GetDesc")?;
        let r = desc.DesktopCoordinates;
        (r.right, r.left, r.bottom, r.top)
    };
    Ok((
        right.saturating_sub(left).max(1) as u32,
        bottom.saturating_sub(top).max(1) as u32,
    ))
}

/// The DXGI worker: acquire → stage → publish. `WAIT_TIMEOUT` (no update) just loops;
/// `ACCESS_LOST` (topology change) terminally fails so the session rebuilds by reopening.
fn dxgi_worker(
    device: ID3D11Device,
    ctx: ID3D11DeviceContext,
    dupl: IDXGIOutputDuplication,
    width: u32,
    height: u32,
    slot: Arc<FrameSlot>,
    stop: Arc<AtomicBool>,
) {
    let staging = match create_staging(&device, width, height) {
        Ok(t) => t,
        Err(e) => {
            slot.fail(format!("dxgi staging: {e:#}"));
            return;
        }
    };
    // The staging upcast keeps the texture alive for the whole loop.
    let stage_res: ID3D11Resource = match staging.cast() {
        Ok(r) => r,
        Err(e) => {
            slot.fail(format!("dxgi staging resource: {e:#}"));
            return;
        }
    };
    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let mut info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut resource: Option<IDXGIResource> = None;
        // `info`/`resource` are live locals filled synchronously; the acquired texture is
        // released below before the next acquire.
        // SAFETY: see above; every acquired frame is released exactly once (success,
        // idle, and failure paths all release).
        let acquired = unsafe { dupl.AcquireNextFrame(100, &mut info, &mut resource) };
        match acquired {
            Ok(()) => {}
            Err(e) if e.code() == DXGI_ERROR_WAIT_TIMEOUT => continue,
            Err(e) if e.code() == DXGI_ERROR_ACCESS_LOST => {
                slot.fail(
                    "dxgi: display topology changed (ACCESS_LOST) — reopen to recover".to_string(),
                );
                break;
            }
            Err(e) => {
                slot.fail(format!("dxgi: AcquireNextFrame: {e}"));
                break;
            }
        }
        // No new pixels (cursor-only moves still count as accumulated frames, so this is
        // genuinely idle) — release and keep waiting.
        if info.AccumulatedFrames == 0 {
            // Releases the frame acquired above, balancing the acquire.
            // SAFETY: exactly one release per acquire; the duplication outlives the loop.
            unsafe {
                let _ = dupl.ReleaseFrame();
            }
            continue;
        }
        let frame = (|| -> Result<CapturedFrame> {
            let texture: ID3D11Texture2D = resource
                .as_ref()
                .context("dxgi: no resource")?
                .cast()
                .context("dxgi: texture")?;
            // Both textures live on this thread's exclusive immediate context.
            let texture_res: ID3D11Resource = texture.cast().context("dxgi: resource")?;
            // SAFETY: live objects, exclusive context, same-size textures.
            unsafe {
                ctx.CopyResource(&stage_res, &texture_res);
            }
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            // Staging is CPU-readable; the pointer is valid until `Unmap` below.
            // SAFETY: staging is CPU-readable and exclusively owned here; unmapped below.
            unsafe {
                ctx.Map(&stage_res, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                    .context("dxgi: Map")?;
            }
            let out = materialize_bgra_frame(
                mapped.pData as *const u8,
                mapped.RowPitch as usize,
                width,
                height,
            );
            // Unmaps the texture mapped above; bytes were copied out.
            // SAFETY: balances the `Map` above on the same object.
            unsafe {
                ctx.Unmap(&stage_res, 0);
            }
            // Releases the frame acquired above, balancing the acquire.
            // SAFETY: exactly one release per acquire (the failure path below also
            // releases, and only runs when this line did not).
            unsafe {
                let _ = dupl.ReleaseFrame();
            }
            Ok(out)
        })();
        match frame {
            Ok(f) => slot.publish(f),
            Err(e) => {
                // The closure releases only on success (its release is the last step
                // before `Ok`), so an `Err` means the frame is still held — release it
                // here for exactly-once balance.
                // SAFETY: one release per acquire; the duplication outlives the loop.
                unsafe {
                    let _ = dupl.ReleaseFrame();
                }
                slot.fail(format!("{e:#}"));
                break;
            }
        }
    }
}

/// One CPU-readable staging texture at the output size.
fn create_staging(device: &ID3D11Device, width: u32, height: u32) -> Result<ID3D11Texture2D> {
    let desc = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_STAGING,
        BindFlags: 0,
        CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
        MiscFlags: 0,
    };
    // `desc` is a plain value struct; the texture is returned, not borrowed.
    let mut tex = None;
    // SAFETY: out-param is a live local written synchronously; the texture is
    // reference-counted.
    unsafe {
        device
            .CreateTexture2D(&desc, None, Some(&mut tex))
            .context("dxgi: staging")?;
    }
    tex.context("dxgi: no staging texture")
}
