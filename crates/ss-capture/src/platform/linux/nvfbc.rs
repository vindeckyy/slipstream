//! NVIDIA NvFBC capture through the optional shared-CUDA API.
//!
//! NvFBC is loaded at runtime because its library is only present on NVIDIA installations. The
//! session uses the driver's push model and `NVFBC_CAPTURE_SHARED_CUDA`, then copies each returned
//! CUDA pointer into a pooled Slipstream device buffer. The copy is device-to-device, so the
//! compositor/X server path never crosses host memory and the NvFBC-owned scratch buffer can be
//! reused safely for the next frame.

#![deny(clippy::undocumented_unsafe_blocks)]

use anyhow::{anyhow, bail, ensure, Context, Result};
use libloading::Library;
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use super::{CaptureTelemetry, CapturedFrame, Capturer, FramePayload, PixelFormat};
use crate::capture_now_ns;

const NVFBC_VERSION: u32 = 0x107;
const NVFBC_CAPTURE_SHARED_CUDA: u32 = 1;
const NVFBC_TRACKING_DEFAULT: u32 = 0;
const NVFBC_TRACKING_OUTPUT: u32 = 1;
const NVFBC_BUFFER_FORMAT_BGRA: u32 = 5;
const NVFBC_TRUE: u32 = 1;
const NVFBC_FALSE: u32 = 0;
const NVFBC_ERR_MUST_RECREATE: i32 = 16;
const NVFBC_TOCUDA_GRAB_FLAGS_NOWAIT: u32 = 1 << 0;
const NVFBC_OUTPUT_MAX: usize = 5;
const NVFBC_OUTPUT_NAME_LEN: usize = 128;

const LIBRARY_CANDIDATES: &[&str] = &[
    "libnvidia-fbc.so.1",
    "libnvidia-fbc.so",
    "/usr/lib/x86_64-linux-gnu/libnvidia-fbc.so.1",
    "/usr/lib64/libnvidia-fbc.so.1",
    "/usr/lib/libnvidia-fbc.so.1",
];

type Status = i32;
type SessionHandle = u64;
type NvFbcBool = u32;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct BoxRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Size {
    w: u32,
    h: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct FrameGrabInfo {
    width: u32,
    height: u32,
    byte_size: u32,
    current_frame: u32,
    is_new_frame: NvFbcBool,
    timestamp_us: u64,
    missed_frames: u32,
    required_post_processing: NvFbcBool,
    direct_capture: NvFbcBool,
}

#[repr(C)]
struct CreateHandleParams {
    version: u32,
    private_data: *const c_void,
    private_data_size: u32,
    externally_managed_context: NvFbcBool,
    glx_context: *mut c_void,
    glx_fb_config: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RandrOutput {
    id: u32,
    name: [c_char; NVFBC_OUTPUT_NAME_LEN],
    tracked_box: BoxRect,
}

#[repr(C)]
struct GetStatusParams {
    version: u32,
    capture_possible: NvFbcBool,
    currently_capturing: NvFbcBool,
    can_create_now: NvFbcBool,
    screen_size: Size,
    xrandr_available: NvFbcBool,
    outputs: [RandrOutput; NVFBC_OUTPUT_MAX],
    output_count: u32,
    nvfbc_version: u32,
    in_modeset: NvFbcBool,
}

#[repr(C)]
struct CreateCaptureSessionParams {
    version: u32,
    capture_type: u32,
    tracking_type: u32,
    output_id: u32,
    capture_box: BoxRect,
    frame_size: Size,
    with_cursor: NvFbcBool,
    disable_auto_modeset_recovery: NvFbcBool,
    round_frame_size: NvFbcBool,
    sampling_rate_ms: u32,
    push_model: NvFbcBool,
    allow_direct_capture: NvFbcBool,
}

#[repr(C)]
struct VersionParams {
    version: u32,
}

#[repr(C)]
struct ToCudaSetupParams {
    version: u32,
    buffer_format: u32,
}

#[repr(C)]
struct ToCudaGrabFrameParams {
    version: u32,
    flags: u32,
    cuda_device_buffer: *mut c_void,
    frame_info: *mut FrameGrabInfo,
    timeout_ms: u32,
}

type GetLastError = unsafe extern "C" fn(SessionHandle) -> *const c_char;
type CreateHandle = unsafe extern "C" fn(*mut SessionHandle, *mut CreateHandleParams) -> Status;
type DestroyHandle = unsafe extern "C" fn(SessionHandle, *mut VersionParams) -> Status;
type BindContext = unsafe extern "C" fn(SessionHandle, *mut VersionParams) -> Status;
type ReleaseContext = unsafe extern "C" fn(SessionHandle, *mut VersionParams) -> Status;
type GetStatus = unsafe extern "C" fn(SessionHandle, *mut GetStatusParams) -> Status;
type CreateCapture = unsafe extern "C" fn(SessionHandle, *mut CreateCaptureSessionParams) -> Status;
type DestroyCapture = unsafe extern "C" fn(SessionHandle, *mut VersionParams) -> Status;
type ToCudaSetup = unsafe extern "C" fn(SessionHandle, *mut ToCudaSetupParams) -> Status;
type ToCudaGrab = unsafe extern "C" fn(SessionHandle, *mut ToCudaGrabFrameParams) -> Status;
type CreateInstance = unsafe extern "C" fn(*mut ApiFunctionList) -> Status;

#[repr(C)]
#[derive(Default)]
struct ApiFunctionList {
    version: u32,
    get_last_error: Option<GetLastError>,
    create_handle: Option<CreateHandle>,
    destroy_handle: Option<DestroyHandle>,
    get_status: Option<GetStatus>,
    create_capture: Option<CreateCapture>,
    destroy_capture: Option<DestroyCapture>,
    to_sys_setup: *mut c_void,
    to_sys_grab: *mut c_void,
    to_cuda_setup: Option<ToCudaSetup>,
    to_cuda_grab: Option<ToCudaGrab>,
    pad1: *mut c_void,
    pad2: *mut c_void,
    pad3: *mut c_void,
    bind_context: Option<BindContext>,
    release_context: Option<ReleaseContext>,
    pad4: *mut c_void,
    pad5: *mut c_void,
    pad6: *mut c_void,
    pad7: *mut c_void,
    to_gl_setup: *mut c_void,
    to_gl_grab: *mut c_void,
}

struct NvFbcApi {
    _library: Library,
    functions: ApiFunctionList,
}

// SAFETY: the function table contains immutable ABI function pointers and padding pointers
// returned by NvFBC. The complete table is transferred to the single capture thread that owns the
// session; no table field is concurrently mutated or dereferenced from another thread.
unsafe impl Send for ApiFunctionList {}

impl NvFbcApi {
    fn load() -> Result<Self> {
        let mut errors = Vec::new();
        for candidate in LIBRARY_CANDIDATES {
            // SAFETY: `candidate` is a fixed NUL-free library name/path. The returned Library owns
            // the handle and stays alive in `NvFbcApi` for every copied function pointer's lifetime.
            let library = match unsafe { Library::new(candidate) } {
                Ok(library) => library,
                Err(error) => {
                    errors.push(format!("{candidate}: {error}"));
                    continue;
                }
            };
            // SAFETY: the symbol name is NUL-terminated and the loaded NvFBC ABI defines this
            // function with the exact `CreateInstance` signature below. The Symbol borrow ends
            // before the library is moved into the returned owner.
            let create = match unsafe { library.get::<CreateInstance>(b"NvFBCCreateInstance\0") } {
                Ok(symbol) => *symbol,
                Err(error) => {
                    errors.push(format!("{candidate}: missing NvFBCCreateInstance: {error}"));
                    continue;
                }
            };
            let mut functions = ApiFunctionList {
                version: NVFBC_VERSION,
                ..ApiFunctionList::default()
            };
            // SAFETY: `functions` is a live C ABI out-parameter and `create` came from the live
            // library handle retained below. The call writes only that struct synchronously.
            let status = unsafe { create(&mut functions) };
            if status != 0 {
                errors.push(format!(
                    "{candidate}: NvFBCCreateInstance returned {status}"
                ));
                continue;
            }
            return Ok(Self {
                _library: library,
                functions,
            });
        }
        bail!("NvFBC shared library is unavailable: {}", errors.join("; "))
    }

    fn required<T: Copy>(&self, value: Option<T>, name: &str) -> Result<T> {
        value.ok_or_else(|| anyhow!("NvFBC API did not provide {name}"))
    }

    fn error(&self, handle: SessionHandle, operation: &str, status: Status) -> anyhow::Error {
        let detail = self
            .functions
            .get_last_error
            .and_then(|get_error| {
                // SAFETY: `handle` is the live NvFBC session handle used for the failed call, and
                // NvFBC returns a pointer to an internal NUL-terminated diagnostic string. We copy
                // it immediately, without retaining the pointer.
                let ptr = unsafe { get_error(handle) };
                (!ptr.is_null()).then(|| {
                    // SAFETY: NvFBC returned this pointer from the live handle's error accessor;
                    // it is documented as an internal NUL-terminated diagnostic string and is
                    // copied immediately without escaping the call.
                    unsafe { CStr::from_ptr(ptr) }
                        .to_string_lossy()
                        .into_owned()
                })
            })
            .unwrap_or_else(|| "no driver diagnostic".to_string());
        anyhow!("NvFBC {operation} failed with status {status}: {detail}")
    }

    fn check(&self, handle: SessionHandle, operation: &str, status: Status) -> Result<()> {
        if status == 0 {
            Ok(())
        } else {
            Err(self.error(handle, operation, status))
        }
    }
}

fn struct_version<T>(revision: u32) -> u32 {
    std::mem::size_of::<T>() as u32 | (revision << 16) | (NVFBC_VERSION << 24)
}

fn magic_private_data() -> [u32; 4] {
    [0xAEF5_7AC5, 0x401D_1A39, 0x1B85_6BBE, 0x9ED0_CEBA]
}

struct Grab {
    ptr: ss_zerocopy::cuda::CUdeviceptr,
    info: FrameGrabInfo,
}

struct HandleGuard<'a> {
    api: &'a NvFbcApi,
    handle: SessionHandle,
    armed: bool,
}

impl<'a> HandleGuard<'a> {
    fn new(api: &'a NvFbcApi, handle: SessionHandle) -> Self {
        Self {
            api,
            handle,
            armed: true,
        }
    }

    fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for HandleGuard<'_> {
    fn drop(&mut self) {
        if !self.armed || self.handle == 0 {
            return;
        }
        if let Some(destroy) = self.api.functions.destroy_handle {
            let mut params = VersionParams {
                version: struct_version::<VersionParams>(1),
            };
            // SAFETY: the guard owns the successful create result and destroys it once if
            // initialization exits before ownership moves into `NvFbcCapturer`.
            let _ = unsafe { destroy(self.handle, &mut params) };
        }
    }
}

/// NVIDIA framebuffer source. NvFBC's returned pointer belongs to the driver; `copy_grab` copies it
/// into a Slipstream-owned pooled buffer before the driver is allowed to reuse it.
pub struct NvFbcCapturer {
    api: NvFbcApi,
    handle: SessionHandle,
    capture_live: bool,
    context_bound: bool,
    pool: ss_zerocopy::cuda::BufferPool,
    _warmup_texture: ss_zerocopy::cuda::CudaWarmupTexture,
    width: u32,
    height: u32,
    active: bool,
    dead: AtomicBool,
    pending: Option<Grab>,
    last_frame_ns: u64,
    frames_published: u64,
    target_tracking_type: u32,
    target_output_id: u32,
}

impl NvFbcCapturer {
    fn open() -> Result<Self> {
        let monitor = std::env::var("SLIPSTREAM_CAPTURE_MONITOR").ok();
        Self::open_for_monitor(monitor.as_deref())
    }

    fn open_for_monitor(monitor: Option<&str>) -> Result<Self> {
        if std::env::var_os("DISPLAY").is_none() {
            bail!("NvFBC requires an X11 DISPLAY")
        }
        ss_zerocopy::cuda::make_current()
            .context("make the shared CUDA context current for NvFBC")?;
        let api = NvFbcApi::load()?;
        let private_data = magic_private_data();
        let mut handle = 0;
        let mut create_handle = CreateHandleParams {
            version: struct_version::<CreateHandleParams>(2),
            private_data: private_data.as_ptr().cast(),
            private_data_size: std::mem::size_of_val(&private_data) as u32,
            externally_managed_context: NVFBC_FALSE,
            glx_context: std::ptr::null_mut(),
            glx_fb_config: std::ptr::null_mut(),
        };
        let create_handle_fn = api.required(api.functions.create_handle, "NvFBCCreateHandle")?;
        // SAFETY: the function pointer came from `NvFBCCreateInstance`, `&mut handle` and
        // `&mut create_handle` are live C ABI out-parameters for the synchronous call, and the
        // private-data pointer remains valid for the duration of this call.
        let status = unsafe { create_handle_fn(&mut handle, &mut create_handle) };
        api.check(handle, "NvFBCCreateHandle", status)?;

        let guard = HandleGuard::new(&api, handle);
        let (pool, width, height, tracking_type, output_id) =
            Self::from_handle(&api, handle, monitor)?;
        let warmup_texture = ss_zerocopy::cuda::CudaWarmupTexture::new(width, height)
            .context("create the NvFBC CUDA texture warmup")?;
        guard.disarm();
        let mut session = Self {
            api,
            handle,
            capture_live: false,
            context_bound: false,
            pool,
            _warmup_texture: warmup_texture,
            width,
            height,
            active: true,
            dead: AtomicBool::new(false),
            pending: None,
            last_frame_ns: 0,
            frames_published: 0,
            target_tracking_type: tracking_type,
            target_output_id: output_id,
        };
        if let Err(error) = session.bind_and_start() {
            session.destroy();
            return Err(error);
        }
        Ok(session)
    }

    fn from_handle(
        api: &NvFbcApi,
        handle: SessionHandle,
        monitor: Option<&str>,
    ) -> Result<(ss_zerocopy::cuda::BufferPool, u32, u32, u32, u32)> {
        let mut status = GetStatusParams {
            version: struct_version::<GetStatusParams>(2),
            capture_possible: 0,
            currently_capturing: 0,
            can_create_now: 0,
            screen_size: Size::default(),
            xrandr_available: 0,
            outputs: [RandrOutput {
                id: 0,
                name: [0; NVFBC_OUTPUT_NAME_LEN],
                tracked_box: BoxRect::default(),
            }; NVFBC_OUTPUT_MAX],
            output_count: 0,
            nvfbc_version: 0,
            in_modeset: 0,
        };
        let get_status = api.required(api.functions.get_status, "NvFBCGetStatus")?;
        // SAFETY: `status` is a live correctly-versioned C out-parameter, and `get_status` points
        // into the library retained by `api`.
        let result = unsafe { get_status(handle, &mut status) };
        api.check(handle, "NvFBCGetStatus", result)?;
        if status.capture_possible == NVFBC_FALSE {
            bail!("the NVIDIA driver reports that NvFBC capture is unavailable")
        }
        if status.can_create_now == NVFBC_FALSE {
            bail!("the NVIDIA driver cannot create an NvFBC session right now")
        }
        if status.in_modeset != NVFBC_FALSE {
            bail!("the NVIDIA X server is in modeset")
        }
        let (tracking_type, output_id, width, height) = select_target(&status, monitor)?;
        ensure_dimensions(width, height)?;
        let pool = ss_zerocopy::cuda::BufferPool::new(width, height)
            .context("allocate the NvFBC CUDA frame pool")?;
        Ok((pool, width, height, tracking_type, output_id))
    }

    fn bind_and_start(&mut self) -> Result<()> {
        let bind = self
            .api
            .required(self.api.functions.bind_context, "NvFBCBindContext")?;
        let mut bind_params = VersionParams {
            version: struct_version::<VersionParams>(1),
        };
        // SAFETY: the bind parameter is a live versioned struct and the function pointer belongs
        // to the retained NvFBC library; the call touches only the current capture thread.
        self.api.check(self.handle, "NvFBCBindContext", unsafe {
            bind(self.handle, &mut bind_params)
        })?;
        self.context_bound = true;

        let mut params = CreateCaptureSessionParams {
            version: struct_version::<CreateCaptureSessionParams>(6),
            capture_type: NVFBC_CAPTURE_SHARED_CUDA,
            tracking_type: self.target_tracking_type,
            output_id: self.target_output_id,
            capture_box: BoxRect::default(),
            frame_size: Size::default(),
            // Include the pointer in the captured image. This keeps the KMS/NvFBC desktop sources
            // honest until a separate cursor channel is added for NvFBC.
            with_cursor: NVFBC_TRUE,
            disable_auto_modeset_recovery: NVFBC_TRUE,
            round_frame_size: NVFBC_FALSE,
            sampling_rate_ms: 16,
            push_model: NVFBC_TRUE,
            // NvFBC direct capture cannot be combined with compositor cursor capture.
            allow_direct_capture: NVFBC_FALSE,
        };
        let create_capture = self.api.required(
            self.api.functions.create_capture,
            "NvFBCCreateCaptureSession",
        )?;
        // SAFETY: `params` is a live versioned in/out struct and `create_capture` is a live API
        // function from the retained NvFBC library; the synchronous call does not retain it.
        let status = unsafe { create_capture(self.handle, &mut params) };
        self.api
            .check(self.handle, "NvFBCCreateCaptureSession", status)?;
        self.capture_live = true;
        let setup = self
            .api
            .required(self.api.functions.to_cuda_setup, "NvFBCToCudaSetUp")?;
        let mut setup_params = ToCudaSetupParams {
            version: struct_version::<ToCudaSetupParams>(1),
            buffer_format: NVFBC_BUFFER_FORMAT_BGRA,
        };
        // SAFETY: `setup_params` is a live versioned struct and `setup` belongs to the retained
        // library. The call initializes the driver's CUDA capture buffer.
        if let Err(error) = self.api.check(self.handle, "NvFBCToCudaSetUp", unsafe {
            setup(self.handle, &mut setup_params)
        }) {
            self.stop_capture();
            return Err(error);
        }
        Ok(())
    }

    fn grab(&mut self, flags: u32, timeout_ms: u32) -> Result<Option<Grab>> {
        ss_zerocopy::cuda::make_current()
            .context("make the shared CUDA context current for NvFBC grab")?;
        let grab_fn = self
            .api
            .required(self.api.functions.to_cuda_grab, "NvFBCToCudaGrabFrame")?;
        let mut ptr: ss_zerocopy::cuda::CUdeviceptr = 0;
        let mut info = FrameGrabInfo::default();
        let mut params = ToCudaGrabFrameParams {
            version: struct_version::<ToCudaGrabFrameParams>(2),
            flags,
            cuda_device_buffer: (&mut ptr as *mut ss_zerocopy::cuda::CUdeviceptr).cast(),
            frame_info: &mut info,
            timeout_ms,
        };
        // SAFETY: all pointers in `params` point to live stack out-parameters for this synchronous
        // call. `grab_fn` came from the retained NvFBC function table; the driver writes the CUDA
        // pointer and frame metadata before returning.
        let status = unsafe { grab_fn(self.handle, &mut params) };
        if status == NVFBC_ERR_MUST_RECREATE {
            bail!("NvFBC requires capture-session recreation after a display modeset")
        }
        self.api
            .check(self.handle, "NvFBCToCudaGrabFrame", status)?;
        if info.is_new_frame == NVFBC_FALSE {
            return Ok(None);
        }
        if ptr == 0 {
            bail!("NvFBC returned a null CUDA frame pointer")
        }
        Ok(Some(Grab { ptr, info }))
    }

    fn copy_grab(&mut self, grab: Grab) -> Result<CapturedFrame> {
        ensure_dimensions(grab.info.width, grab.info.height)?;
        if grab.info.width != self.width || grab.info.height != self.height {
            self.pool = ss_zerocopy::cuda::BufferPool::new(grab.info.width, grab.info.height)
                .context("resize the NvFBC CUDA frame pool after a modeset")?;
            self.width = grab.info.width;
            self.height = grab.info.height;
        }
        let destination = self.pool.get().context("get a pooled NvFBC CUDA frame")?;
        ss_zerocopy::cuda::copy_raw_device_to_device(
            grab.ptr,
            self.width as usize * 4,
            &destination,
            true,
        )?;
        let pts_ns = capture_now_ns();
        self.last_frame_ns = pts_ns;
        self.frames_published = self.frames_published.saturating_add(1);
        Ok(CapturedFrame {
            width: self.width,
            height: self.height,
            pts_ns,
            format: PixelFormat::Bgra,
            payload: FramePayload::Cuda(destination),
            cursor: None,
            stage_ns: ss_frame::CaptureStageTimes::default(),
        })
    }

    fn stop_capture(&mut self) {
        if !self.capture_live {
            return;
        }
        if let Some(destroy) = self.api.functions.destroy_capture {
            let mut params = VersionParams {
                version: struct_version::<VersionParams>(1),
            };
            // SAFETY: the handle and versioned destruction parameters belong to this live session;
            // teardown is best-effort and the function pointer is from the retained library.
            let _ = unsafe { destroy(self.handle, &mut params) };
        }
        self.capture_live = false;
    }

    fn destroy(&mut self) {
        self.stop_capture();
        if self.handle == 0 {
            return;
        }
        if self.context_bound {
            if let Some(release) = self.api.functions.release_context {
                let mut params = VersionParams {
                    version: struct_version::<VersionParams>(1),
                };
                // SAFETY: release is paired with the successful bind and uses a live session handle.
                let _ = unsafe { release(self.handle, &mut params) };
            }
            self.context_bound = false;
        }
        if let Some(destroy) = self.api.functions.destroy_handle {
            let mut params = VersionParams {
                version: struct_version::<VersionParams>(1),
            };
            // SAFETY: destroy is the final operation on this live session handle.
            let _ = unsafe { destroy(self.handle, &mut params) };
        }
        self.handle = 0;
    }
}

impl Drop for NvFbcCapturer {
    fn drop(&mut self) {
        self.destroy();
    }
}

impl Capturer for NvFbcCapturer {
    fn backend_name(&self) -> &'static str {
        "nvfbc-shared-cuda"
    }

    fn next_frame(&mut self) -> Result<CapturedFrame> {
        if self.dead.load(Ordering::Acquire) {
            bail!("NvFBC capture is dead; rebuild the source")
        }
        let grab = self
            .grab(0, 0)?
            .ok_or_else(|| anyhow!("NvFBC returned no initial frame"))?;
        self.copy_grab(grab).inspect_err(|_| {
            self.dead.store(true, Ordering::Release);
        })
    }

    fn try_latest(&mut self) -> Result<Option<CapturedFrame>> {
        if !self.active {
            return Ok(None);
        }
        if self.dead.load(Ordering::Acquire) {
            bail!("NvFBC capture is dead; rebuild the source")
        }
        let grab = if let Some(grab) = self.pending.take() {
            grab
        } else {
            let Some(grab) = self.grab(NVFBC_TOCUDA_GRAB_FLAGS_NOWAIT, 0)? else {
                return Ok(None);
            };
            grab
        };
        self.copy_grab(grab).map(Some).inspect_err(|_| {
            self.dead.store(true, Ordering::Release);
        })
    }

    fn supports_arrival_wait(&self) -> bool {
        true
    }

    fn wait_arrival(&mut self, deadline: Instant) {
        if !self.active || self.dead.load(Ordering::Acquire) || self.pending.is_some() {
            return;
        }
        let timeout_ms = deadline
            .saturating_duration_since(Instant::now())
            .as_millis()
            .min(u32::MAX as u128) as u32;
        if timeout_ms == 0 {
            return;
        }
        match self.grab(0, timeout_ms) {
            Ok(Some(grab)) => self.pending = Some(grab),
            Ok(None) => {}
            Err(error) => {
                tracing::debug!(error = %error, "NvFBC arrival wait failed");
                self.dead.store(true, Ordering::Release);
            }
        }
    }

    fn set_active(&mut self, active: bool) {
        self.active = active;
        if !active {
            self.pending = None;
        }
    }

    fn is_alive(&self) -> bool {
        !self.dead.load(Ordering::Acquire)
    }

    fn telemetry(&self) -> CaptureTelemetry {
        CaptureTelemetry {
            last_frame_ns: self.last_frame_ns,
            frames_published: self.frames_published,
            width: self.width,
            height: self.height,
            ..CaptureTelemetry::default()
        }
    }
}

fn ensure_dimensions(width: u32, height: u32) -> Result<()> {
    if width == 0 || height == 0 {
        bail!("NvFBC returned an invalid frame size {width}x{height}")
    }
    Ok(())
}

fn select_target(
    status: &GetStatusParams,
    requested: Option<&str>,
) -> Result<(u32, u32, u32, u32)> {
    if let Some(requested) = requested.filter(|value| !value.is_empty()) {
        if status.xrandr_available != NVFBC_FALSE {
            if let Some(output) = status.outputs
                [..status.output_count.min(NVFBC_OUTPUT_MAX as u32) as usize]
                .iter()
                .find(|output| {
                    output_name(output).is_some_and(|name| name.eq_ignore_ascii_case(requested))
                })
            {
                return Ok((
                    NVFBC_TRACKING_OUTPUT,
                    output.id,
                    output.tracked_box.w,
                    output.tracked_box.h,
                ));
            }
        }
        if let Ok(index) = requested.parse::<usize>() {
            let output_count = status.output_count.min(NVFBC_OUTPUT_MAX as u32) as usize;
            if status.xrandr_available != NVFBC_FALSE && index < output_count {
                let output = &status.outputs[index];
                return Ok((
                    NVFBC_TRACKING_OUTPUT,
                    output.id,
                    output.tracked_box.w,
                    output.tracked_box.h,
                ));
            }
        }
        bail!("SLIPSTREAM_CAPTURE_MONITOR={requested} does not match an NvFBC output")
    }
    Ok((
        NVFBC_TRACKING_DEFAULT,
        0,
        status.screen_size.w,
        status.screen_size.h,
    ))
}

fn output_name(output: &RandrOutput) -> Option<String> {
    let length = output
        .name
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(output.name.len());
    // SAFETY: `output.name` is a live fixed-size C character array copied into this Rust value;
    // the bounded slice never reads beyond that array, and the bytes are converted without a
    // borrowed pointer escaping this function.
    let bytes = unsafe { std::slice::from_raw_parts(output.name.as_ptr().cast::<u8>(), length) };
    std::str::from_utf8(bytes).ok().map(str::to_owned)
}

/// Probe NvFBC using the monitor selected through the environment-backed compatibility path.
pub fn probe_nvfbc() -> bool {
    let monitor = std::env::var("SLIPSTREAM_CAPTURE_MONITOR").ok();
    probe_nvfbc_for_monitor(monitor.as_deref())
}

/// Probe the complete NvFBC availability chain for one monitor. A library sitting on disk is
/// insufficient: consumer drivers may omit the API, deny capture, or be in modeset.
pub fn probe_nvfbc_for_monitor(monitor: Option<&str>) -> bool {
    if std::env::var_os("DISPLAY").is_none() {
        return false;
    }
    if ss_zerocopy::cuda::make_current().is_err() {
        return false;
    }
    let Ok(api) = NvFbcApi::load() else {
        return false;
    };
    let private_data = magic_private_data();
    let mut handle = 0;
    let mut params = CreateHandleParams {
        version: struct_version::<CreateHandleParams>(2),
        private_data: private_data.as_ptr().cast(),
        private_data_size: std::mem::size_of_val(&private_data) as u32,
        externally_managed_context: NVFBC_FALSE,
        glx_context: std::ptr::null_mut(),
        glx_fb_config: std::ptr::null_mut(),
    };
    let Ok(create) = api.required(api.functions.create_handle, "NvFBCCreateHandle") else {
        return false;
    };
    // SAFETY: `params` and `handle` are live C ABI call arguments, and `create` came from the
    // retained runtime library.
    if unsafe { create(&mut handle, &mut params) } != 0 {
        return false;
    }
    // Exercise the same bind, texture warmup, capture creation, and CUDA setup sequence as the
    // real opener. A library/status-only probe otherwise advertises NvFBC on drivers that fail at
    // `NvFBCToCudaSetUp`, which leaves auto-selection to discover the failure only at session open.
    let usable = probe_capture_session(&api, handle, monitor).is_ok();
    if let Some(destroy) = api.functions.destroy_handle {
        let mut destroy_params = VersionParams {
            version: struct_version::<VersionParams>(1),
        };
        // SAFETY: the handle was returned by the successful create call and is destroyed once.
        let _ = unsafe { destroy(handle, &mut destroy_params) };
    }
    usable
}

fn probe_capture_session(
    api: &NvFbcApi,
    handle: SessionHandle,
    monitor: Option<&str>,
) -> Result<()> {
    let get_status = api.required(api.functions.get_status, "NvFBCGetStatus")?;
    let mut status = GetStatusParams {
        version: struct_version::<GetStatusParams>(2),
        capture_possible: 0,
        currently_capturing: 0,
        can_create_now: 0,
        screen_size: Size::default(),
        xrandr_available: 0,
        outputs: [RandrOutput {
            id: 0,
            name: [0; NVFBC_OUTPUT_NAME_LEN],
            tracked_box: BoxRect::default(),
        }; NVFBC_OUTPUT_MAX],
        output_count: 0,
        nvfbc_version: 0,
        in_modeset: 0,
    };
    // SAFETY: `status` is a live versioned C out-parameter and `get_status` belongs to `api`.
    api.check(handle, "NvFBCGetStatus", unsafe {
        get_status(handle, &mut status)
    })?;
    ensure!(
        status.capture_possible != NVFBC_FALSE,
        "the NVIDIA driver reports that NvFBC capture is unavailable"
    );
    ensure!(
        status.can_create_now != NVFBC_FALSE,
        "the NVIDIA driver cannot create an NvFBC session right now"
    );
    ensure!(
        status.in_modeset == NVFBC_FALSE,
        "the NVIDIA X server is in modeset"
    );
    let (tracking_type, output_id, width, height) = select_target(&status, monitor)?;
    ensure_dimensions(width, height)?;
    let _warmup_texture = ss_zerocopy::cuda::CudaWarmupTexture::new(width, height)
        .context("create the NvFBC CUDA texture warmup")?;

    let bind = api.required(api.functions.bind_context, "NvFBCBindContext")?;
    let mut context_bound = false;
    let mut capture_live = false;
    let result = (|| -> Result<()> {
        let mut bind_params = VersionParams {
            version: struct_version::<VersionParams>(1),
        };
        // SAFETY: the bind parameter is a live versioned struct and the function pointer belongs
        // to the retained NvFBC library; the call touches only the current probe thread.
        api.check(handle, "NvFBCBindContext", unsafe {
            bind(handle, &mut bind_params)
        })?;
        context_bound = true;

        let mut capture_params = CreateCaptureSessionParams {
            version: struct_version::<CreateCaptureSessionParams>(6),
            capture_type: NVFBC_CAPTURE_SHARED_CUDA,
            tracking_type,
            output_id,
            capture_box: BoxRect::default(),
            frame_size: Size::default(),
            with_cursor: NVFBC_TRUE,
            disable_auto_modeset_recovery: NVFBC_TRUE,
            round_frame_size: NVFBC_FALSE,
            sampling_rate_ms: 16,
            push_model: NVFBC_TRUE,
            allow_direct_capture: NVFBC_FALSE,
        };
        let create_capture =
            api.required(api.functions.create_capture, "NvFBCCreateCaptureSession")?;
        // SAFETY: `capture_params` is a live versioned in/out struct and `create_capture` belongs
        // to the retained NvFBC library.
        api.check(handle, "NvFBCCreateCaptureSession", unsafe {
            create_capture(handle, &mut capture_params)
        })?;
        capture_live = true;

        let setup = api.required(api.functions.to_cuda_setup, "NvFBCToCudaSetUp")?;
        let mut setup_params = ToCudaSetupParams {
            version: struct_version::<ToCudaSetupParams>(1),
            buffer_format: NVFBC_BUFFER_FORMAT_BGRA,
        };
        // SAFETY: `setup_params` is a live versioned struct and `setup` belongs to the retained
        // library. The warmup texture is alive on this same current CUDA context.
        api.check(handle, "NvFBCToCudaSetUp", unsafe {
            setup(handle, &mut setup_params)
        })
    })();

    if capture_live {
        if let Some(destroy) = api.functions.destroy_capture {
            let mut params = VersionParams {
                version: struct_version::<VersionParams>(1),
            };
            // SAFETY: the probe owns the successful capture session and destroys it once before
            // releasing the bound context.
            let _ = unsafe { destroy(handle, &mut params) };
        }
    }
    if context_bound {
        if let Some(release) = api.functions.release_context {
            let mut params = VersionParams {
                version: struct_version::<VersionParams>(1),
            };
            // SAFETY: release is paired with the successful probe bind and uses the live handle.
            let _ = unsafe { release(handle, &mut params) };
        }
    }
    result
}

pub fn open_nvfbc_desktop() -> Result<Box<dyn Capturer>> {
    Ok(Box::new(NvFbcCapturer::open()?))
}

pub fn open_nvfbc_desktop_for_monitor(monitor: Option<&str>) -> Result<Box<dyn Capturer>> {
    Ok(Box::new(NvFbcCapturer::open_for_monitor(monitor)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(name: &str, id: u32, width: u32, height: u32) -> RandrOutput {
        let mut bytes = [0 as c_char; NVFBC_OUTPUT_NAME_LEN];
        for (destination, source) in bytes.iter_mut().zip(name.bytes()) {
            *destination = source as c_char;
        }
        RandrOutput {
            id,
            name: bytes,
            tracked_box: BoxRect {
                x: 0,
                y: 0,
                w: width,
                h: height,
            },
        }
    }

    fn status() -> GetStatusParams {
        GetStatusParams {
            version: 0,
            capture_possible: NVFBC_TRUE,
            currently_capturing: NVFBC_FALSE,
            can_create_now: NVFBC_TRUE,
            screen_size: Size { w: 3840, h: 2160 },
            xrandr_available: NVFBC_TRUE,
            outputs: [output("DP-1", 7, 1920, 1080); NVFBC_OUTPUT_MAX],
            output_count: 1,
            nvfbc_version: 0,
            in_modeset: NVFBC_FALSE,
        }
    }

    #[test]
    fn monitor_name_selects_tracked_output() {
        let status = status();
        assert_eq!(
            select_target(&status, Some("DP-1")).unwrap(),
            (NVFBC_TRACKING_OUTPUT, 7, 1920, 1080)
        );
    }

    #[test]
    fn monitor_name_matching_is_case_insensitive() {
        let status = status();
        assert_eq!(
            select_target(&status, Some("dp-1")).unwrap(),
            (NVFBC_TRACKING_OUTPUT, 7, 1920, 1080)
        );
    }

    #[test]
    fn numeric_monitor_selects_output_index() {
        let status = status();
        assert_eq!(
            select_target(&status, Some("0")).unwrap(),
            (NVFBC_TRACKING_OUTPUT, 7, 1920, 1080)
        );
    }

    #[test]
    fn unknown_monitor_is_rejected() {
        let status = status();
        assert!(select_target(&status, Some("HDMI-A-1")).is_err());
    }
}
