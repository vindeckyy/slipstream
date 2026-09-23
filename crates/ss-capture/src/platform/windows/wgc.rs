//! Windows Graphics Capture (WGC): the primary Windows source.
//!
//! Per-monitor `GraphicsCaptureItem` + a free-threaded `Direct3D11CaptureFramePool`
//! (`Bgra8`, cursor embedded), drained by one worker thread into the shared [`FrameSlot`].
//! The pool's `FrameArrived` event only wakes the worker — all D3D11 work (one immediate
//! context) stays on that single thread, so no multithreaded-device locking is needed.
//!
//! 8-bit SDR only (the pool negotiates `Bgra8`); HDR arrives with the quality todo. The
//! cursor is always embedded (`IsCursorCaptureEnabled`); cursor-forward sessions keep the
//! embedded pointer until the shape channel lands (the handshake never negotiates the
//! channel on a software host today, so this is unreachable, not silent).

use super::common::{enumerate_monitors, materialize_bgra_frame, pick_monitor, FrameSlot};
use super::{CapturedFrame, Capturer};
use anyhow::{Context, Result};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use windows::{
    core::Interface,
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{
            Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem,
            GraphicsCaptureSession,
        },
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
        SizeInt32,
    },
    Win32::{
        Foundation::HMODULE,
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0},
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Resource,
                ID3D11Texture2D, D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC,
                D3D11_USAGE_STAGING,
            },
            Dxgi::{IDXGIAdapter, IDXGIDevice},
        },
        System::WinRT::{
            Direct3D11::CreateDirect3D11DeviceFromDXGIDevice, RoInitialize, RO_INIT_MULTITHREADED,
        },
    },
};

/// Windows Graphics Capture source. Owns the worker thread; dropping stops it.
pub struct WgcCapturer {
    slot: Arc<FrameSlot>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl WgcCapturer {
    /// Capture the primary monitor.
    pub fn open() -> Result<Self> {
        Self::open_for_monitor(None)
    }

    /// Capture the monitor named by `SLIPSTREAM_CAPTURE_MONITOR` (`\\.\DISPLAY1`), else the
    /// primary head. Fails loudly when the pin names nothing on this box.
    pub fn open_for_monitor(monitor: Option<&str>) -> Result<Self> {
        // WinRT init is process-wide and idempotent (`S_FALSE` when already
        // initialized); the outcome is deliberately ignored.
        // SAFETY: no pointers; process-wide init only.
        let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
        let monitors = enumerate_monitors()?;
        let picked = pick_monitor(&monitors, monitor)?;
        tracing::info!(
            device = %picked.device,
            width = picked.width,
            height = picked.height,
            "wgc: capturing monitor"
        );
        let item = create_capture_item_for_monitor(picked.hmon)
            .with_context(|| format!("wgc: capture item for {}", picked.device))?;
        let (device, ctx) = create_d3d11_device()?;
        let direct_device: IDirect3DDevice = {
            let dxgi: IDXGIDevice = device.cast().context("wgc: IDXGIDevice")?;
            // SAFETY: `dxgi` is our live device; the returned WinRT device wraps it
            // with its own reference, recast below for exactly this scope.
            let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi) }
                .context("wgc: IDirect3DDevice")?;
            inspectable.cast().context("wgc: IDirect3DDevice cast")?
        };
        let size = item.Size().context("wgc: item size")?;
        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &direct_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            size,
        )
        .context("wgc: frame pool")?;
        let session = pool
            .CreateCaptureSession(&item)
            .context("wgc: capture session")?;
        session
            .SetIsCursorCaptureEnabled(true)
            .context("wgc: cursor capture")?;

        let slot = FrameSlot::new();
        let stop = Arc::new(AtomicBool::new(false));
        // The arrival event only wakes the worker — frame processing stays on the worker
        // thread, where the immediate context is exclusively used.
        let waker = slot.clone();
        let _token = pool
            .FrameArrived(&TypedEventHandler::new(
                move |_pool: &Option<Direct3D11CaptureFramePool>,
                      _args: &Option<windows::core::IInspectable>| {
                    waker.note_arrival();
                    Ok(())
                },
            ))
            .context("wgc: FrameArrived")?;
        session.StartCapture().context("wgc: StartCapture")?;

        let worker = std::thread::Builder::new()
            .name("slipstream-wgc".into())
            .spawn({
                let (slot, stop) = (slot.clone(), stop.clone());
                move || wgc_worker(pool, session, device, ctx, slot, stop)
            })
            .context("wgc: spawn worker")?;
        Ok(WgcCapturer {
            slot,
            stop,
            worker: Some(worker),
        })
    }
}

impl Drop for WgcCapturer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.slot.note_arrival();
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}

impl Capturer for WgcCapturer {
    fn backend_name(&self) -> &'static str {
        "wgc"
    }

    fn next_frame(&mut self) -> Result<CapturedFrame> {
        self.slot.take_blocking()
    }

    fn try_latest(&mut self) -> Result<Option<CapturedFrame>> {
        if let Some(dead) = self.slot.dead_error() {
            anyhow::bail!("wgc worker died: {dead}");
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
        // No shape channel yet: the pointer stays embedded so it is never silently lost.
        // Unreachable today (the handshake never grants the cursor channel to a host whose
        // encoder cannot blend), logged loudly if it ever fires.
        if on {
            tracing::warn!(
                "wgc: cursor-forward requested but the pointer stays embedded (no shape channel yet)"
            );
        }
    }

    fn telemetry(&self) -> super::CaptureTelemetry {
        self.slot.telemetry()
    }
}

/// Create the per-monitor WGC item via the `IGraphicsCaptureItemInterop` COM
/// interop (the `HMONITOR` factory windows-rs does not project as a static).
fn create_capture_item_for_monitor(
    hmon: windows::Win32::Graphics::Gdi::HMONITOR,
) -> Result<GraphicsCaptureItem> {
    use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
    use windows::Win32::System::WinRT::RoGetActivationFactory;
    // SAFETY: activation of the system capture-item factory by contract name; the
    // returned interop keeps itself alive, and `CreateForMonitor` hands us a
    // reference-counted item for exactly this scope.
    unsafe {
        let interop: IGraphicsCaptureItemInterop = RoGetActivationFactory(
            &windows::core::HSTRING::from("Windows.Graphics.Capture.GraphicsCaptureItem"),
        )
        .context("wgc: capture interop factory")?;
        interop
            .CreateForMonitor::<_, GraphicsCaptureItem>(hmon)
            .context("wgc: CreateForMonitor")
    }
}

/// Create the capture D3D11 device (BGRA-capable for the WGC interop) + immediate context.
fn create_d3d11_device() -> Result<(ID3D11Device, ID3D11DeviceContext)> {
    let mut device = None;
    let mut ctx = None;
    let mut level = D3D_FEATURE_LEVEL_11_0;
    // All out-params are live `Option` locals written synchronously; the adapter and
    // feature-level params are `None` (default adapter, runtime's choice).
    // SAFETY: see above; the returned interfaces are reference-counted.
    unsafe {
        D3D11CreateDevice(
            None::<&IDXGIAdapter>,
            D3D_DRIVER_TYPE_UNKNOWN,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            Some(&mut level),
            Some(&mut ctx),
        )
        .context("wgc: D3D11CreateDevice")?;
    }
    Ok((
        device.context("wgc: no D3D11 device")?,
        ctx.context("wgc: no immediate context")?,
    ))
}

/// The WGC worker: wait for arrivals, drain every queued frame, publish the freshest.
/// Owns the pool/session/device (drops them on exit); any D3D failure is terminal and is
/// reported through the slot so the session fails loudly instead of stalling.
fn wgc_worker(
    pool: Direct3D11CaptureFramePool,
    _session: GraphicsCaptureSession,
    device: ID3D11Device,
    ctx: ID3D11DeviceContext,
    slot: Arc<FrameSlot>,
    stop: Arc<AtomicBool>,
) {
    let mut staging: Option<(ID3D11Texture2D, u32, u32)> = None;
    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        slot.wait_arrival(std::time::Instant::now() + std::time::Duration::from_millis(500));
        if stop.load(Ordering::SeqCst) {
            break;
        }
        // Drain the pool; only the freshest materialization is published.
        let mut pending: Option<Direct3D11CaptureFrame> = None;
        while let Ok(frame) = pool.TryGetNextFrame() {
            pending = Some(frame);
        }
        let Some(frame) = pending else {
            continue;
        };
        match materialize_frame(&frame, &device, &ctx, &mut staging) {
            Ok(captured) => slot.publish(captured),
            Err(e) => {
                slot.fail(format!("{e:#}"));
                break;
            }
        }
    }
}

/// Materialize one WGC frame: surface → `ID3D11Texture2D` → staging copy → mapped rows.
fn materialize_frame(
    frame: &Direct3D11CaptureFrame,
    device: &ID3D11Device,
    ctx: &ID3D11DeviceContext,
    staging: &mut Option<(ID3D11Texture2D, u32, u32)>,
) -> Result<CapturedFrame> {
    let size: SizeInt32 = frame.ContentSize().context("wgc: ContentSize")?;
    let (w, h) = (size.Width.max(1) as u32, size.Height.max(1) as u32);
    // `GetInterface` queries the surface for `ID3D11Texture2D` through COM; the returned
    // interface keeps the texture alive for exactly this scope.
    let texture: ID3D11Texture2D = {
        use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
        let access: IDirect3DDxgiInterfaceAccess =
            frame.Surface().context("wgc: Surface")?.cast()?;
        // SAFETY: `access` is a live interop object wrapping this frame's surface; the
        // returned texture is reference-counted for exactly this scope.
        unsafe { access.GetInterface().context("wgc: texture")? }
    };
    let stage = match staging {
        Some((tex, sw, sh)) if *sw == w && *sh == h => tex.clone(),
        _ => {
            let desc = D3D11_TEXTURE2D_DESC {
                Width: w,
                Height: h,
                MipLevels: 1,
                ArraySize: 1,
                Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
            };
            // `desc` is a plain value struct; the out-texture is returned, not
            // borrowed. Initial data is `None` (uninitialized staging is fine — every
            // byte is overwritten by the copy below before it is read).
            let mut tex = None;
            // SAFETY: out-param is a live local written synchronously; the texture is
            // reference-counted for exactly this scope.
            unsafe {
                device
                    .CreateTexture2D(&desc, None, Some(&mut tex))
                    .context("wgc: staging")?;
            }
            let tex: ID3D11Texture2D = tex.context("wgc: no staging texture")?;
            *staging = Some((tex.clone(), w, h));
            tex
        }
    };
    // Both textures are live D3D11 objects on this thread's exclusive immediate
    // context; `CopyResource` reads the whole source into the same-size staging texture.
    // The `ID3D11Resource` upcasts keep both objects alive for the call.
    let stage_res: ID3D11Resource = stage.cast().context("wgc: staging resource")?;
    let texture_res: ID3D11Resource = texture.cast().context("wgc: source resource")?;
    // SAFETY: live objects, exclusive immediate context, same-size textures.
    unsafe {
        ctx.CopyResource(&stage_res, &texture_res);
    }
    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    // The staging texture is CPU-readable; `Map` hands us a pointer + pitch valid
    // until the matching `Unmap` below, on this thread only.
    // SAFETY: staging is CPU-readable and exclusively owned here; unmapped below.
    unsafe {
        ctx.Map(&stage_res, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
            .context("wgc: Map")?;
    }
    let frame_out =
        materialize_bgra_frame(mapped.pData as *const u8, mapped.RowPitch as usize, w, h);
    // Unmaps the texture mapped above; the materialized bytes were copied out.
    // SAFETY: balances the `Map` above on the same object.
    unsafe {
        ctx.Unmap(&stage_res, 0);
    }
    Ok(frame_out)
}
