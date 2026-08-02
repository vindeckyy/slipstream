//! DRM/KMS primary-plane capture.
//!
//! KMS is the compositor/capture boundary for a scanout desktop: the compositor has already
//! selected the active primary plane, and this backend exports that plane's current framebuffer
//! as a DMA-BUF. The encoder can then import the same allocation through the existing Linux
//! dmabuf paths without a CPU readback or a second compositor connection.
//!
//! The implementation intentionally uses the small libdrm mode-setting ABI directly. The
//! userspace `drm` crate does not cover every framebuffer query needed here, while linking against
//! libdrm is already a normal Linux graphics dependency. Only single-plane packed RGB framebuffers
//! are accepted; multi-plane scanout can be added once the frame vocabulary carries separate
//! GEM/prime handles for every plane.

use anyhow::{anyhow, bail, ensure, Context, Result};
use std::ffi::{CStr, OsStr};
use std::fs::{File, OpenOptions};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::raw::{c_char, c_int, c_ulong, c_void};
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use super::{CaptureTelemetry, CapturedFrame, Capturer, DmabufFrame, FramePayload, PixelFormat};
use crate::capture_now_ns;
use ss_frame::CursorOverlay;

const DRM_CLIENT_CAP_UNIVERSAL_PLANES: u64 = 2;
const DRM_MODE_OBJECT_PLANE: u32 = 0xeeee_eeee;
const DRM_PLANE_TYPE_PRIMARY: u64 = 1;
const DRM_PLANE_TYPE_CURSOR: u64 = 2;
const DRM_MODE_FB_MODIFIERS: u32 = 1 << 1;
const DRM_FORMAT_MOD_LINEAR: u64 = 0;
const DRM_VBLANK_RELATIVE: u32 = 1;
const DRM_VBLANK_HIGH_CRTC_SHIFT: u32 = 1;
const DRM_VBLANK_HIGH_CRTC_MASK: u32 = 0x0000_003e;

const DRM_FORMAT_XRGB8888: u32 = fourcc(*b"XR24");
const DRM_FORMAT_XBGR8888: u32 = fourcc(*b"XB24");
const DRM_FORMAT_ARGB8888: u32 = fourcc(*b"AR24");
const DRM_FORMAT_ABGR8888: u32 = fourcc(*b"AB24");
const DRM_FORMAT_XRGB2101010: u32 = fourcc(*b"XR30");
const DRM_FORMAT_XBGR2101010: u32 = fourcc(*b"XB30");

const fn fourcc(bytes: [u8; 4]) -> u32 {
    u32::from_le_bytes(bytes)
}

#[repr(C)]
struct DrmModePlane {
    count_formats: u32,
    formats: *mut u32,
    plane_id: u32,
    crtc_id: u32,
    fb_id: u32,
    crtc_x: u32,
    crtc_y: u32,
    x: u32,
    y: u32,
    possible_crtcs: u32,
    gamma_size: u32,
}

#[repr(C)]
struct DrmModePlaneResources {
    count_planes: u32,
    planes: *mut u32,
}

#[repr(C)]
struct DrmModeResources {
    count_fbs: c_int,
    fbs: *mut u32,
    count_crtcs: c_int,
    crtcs: *mut u32,
    count_connectors: c_int,
    connectors: *mut u32,
    count_encoders: c_int,
    encoders: *mut u32,
    min_width: u32,
    max_width: u32,
    min_height: u32,
    max_height: u32,
}

#[repr(C)]
struct DrmModeConnector {
    connector_id: u32,
    encoder_id: u32,
    connector_type: u32,
    connector_type_id: u32,
    connection: u32,
    mm_width: u32,
    mm_height: u32,
    subpixel: u32,
    count_modes: c_int,
    modes: *mut c_void,
    count_props: c_int,
    props: *mut u32,
    prop_values: *mut u64,
    count_encoders: c_int,
    encoders: *mut u32,
}

#[repr(C)]
struct DrmModeEncoder {
    encoder_id: u32,
    encoder_type: u32,
    crtc_id: u32,
    possible_crtcs: u32,
    possible_clones: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DrmVblankRequest {
    type_: u32,
    sequence: u32,
    signal: c_ulong,
}

#[repr(C)]
union DrmVblank {
    request: DrmVblankRequest,
}

#[repr(C)]
struct DrmModeFb {
    fb_id: u32,
    width: u32,
    height: u32,
    pitch: u32,
    bpp: u32,
    depth: u32,
    handle: u32,
}

#[repr(C)]
struct DrmModeFb2 {
    fb_id: u32,
    width: u32,
    height: u32,
    pixel_format: u32,
    modifier: u64,
    flags: u32,
    handles: [u32; 4],
    pitches: [u32; 4],
    offsets: [u32; 4],
}

#[repr(C)]
struct DrmModeObjectProperties {
    count_props: u32,
    props: *mut u32,
    prop_values: *mut u64,
}

#[repr(C)]
struct DrmModeProperty {
    prop_id: u32,
    flags: u32,
    name: [c_char; 32],
    count_values: c_int,
    values: *mut u64,
    count_enums: c_int,
    enums: *mut c_void,
    count_blobs: c_int,
    blob_ids: *mut u32,
}

#[link(name = "drm")]
unsafe extern "C" {
    fn drmIsKMS(fd: c_int) -> c_int;
    fn drmSetClientCap(fd: c_int, capability: u64, value: u64) -> c_int;
    fn drmModeGetResources(fd: c_int) -> *mut DrmModeResources;
    fn drmModeFreeResources(ptr: *mut DrmModeResources);
    fn drmModeGetConnector(fd: c_int, connector_id: u32) -> *mut DrmModeConnector;
    fn drmModeFreeConnector(ptr: *mut DrmModeConnector);
    fn drmModeGetEncoder(fd: c_int, encoder_id: u32) -> *mut DrmModeEncoder;
    fn drmModeFreeEncoder(ptr: *mut DrmModeEncoder);
    fn drmModeGetPlaneResources(fd: c_int) -> *mut DrmModePlaneResources;
    fn drmModeFreePlaneResources(ptr: *mut DrmModePlaneResources);
    fn drmModeGetPlane(fd: c_int, plane_id: u32) -> *mut DrmModePlane;
    fn drmModeFreePlane(ptr: *mut DrmModePlane);
    fn drmModeGetFB(fd: c_int, buffer_id: u32) -> *mut DrmModeFb;
    fn drmModeFreeFB(ptr: *mut DrmModeFb);
    fn drmModeGetFB2(fd: c_int, buffer_id: u32) -> *mut DrmModeFb2;
    fn drmModeFreeFB2(ptr: *mut DrmModeFb2);
    fn drmModeObjectGetProperties(
        fd: c_int,
        object_id: u32,
        object_type: u32,
    ) -> *mut DrmModeObjectProperties;
    fn drmModeFreeObjectProperties(ptr: *mut DrmModeObjectProperties);
    fn drmModeGetProperty(fd: c_int, property_id: u32) -> *mut DrmModeProperty;
    fn drmModeFreeProperty(ptr: *mut DrmModeProperty);
    fn drmWaitVBlank(fd: c_int, vblank: *mut DrmVblank) -> c_int;
    fn drmPrimeHandleToFD(fd: c_int, handle: u32, flags: u32, prime_fd: *mut c_int) -> c_int;
}

#[derive(Clone, Copy, Debug)]
struct PlaneState {
    fb_id: u32,
    crtc_id: u32,
    possible_crtcs: u32,
    crtc_x: u32,
    crtc_y: u32,
    x: u32,
    y: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CursorSignature {
    fb_id: u32,
    crtc_id: u32,
    crtc_x: u32,
    crtc_y: u32,
    x: u32,
    y: u32,
}

struct CursorCache {
    fb_id: u32,
    width: u32,
    height: u32,
    rgba: Arc<Vec<u8>>,
    serial: u64,
    x: i32,
    y: i32,
    visible: bool,
}

impl CursorCache {
    fn overlay(&self) -> CursorOverlay {
        CursorOverlay {
            x: self.x,
            y: self.y,
            w: self.width,
            h: self.height,
            rgba: Arc::clone(&self.rgba),
            serial: self.serial,
            hot_x: 0,
            hot_y: 0,
            visible: self.visible,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Framebuffer {
    width: u32,
    height: u32,
    fourcc: u32,
    modifier: u64,
    handle: u32,
    offset: u32,
    pitch: u32,
}

/// A capture source bound to one active primary plane on one DRM card.
pub struct KmsCapturer {
    card: File,
    plane_id: u32,
    crtc_id: u32,
    vblank_type: u32,
    cursor_plane_id: Option<u32>,
    framebuffer: Framebuffer,
    active: bool,
    dead: AtomicBool,
    last_cursor_signature: CursorSignature,
    cursor: Option<CursorCache>,
    last_frame_ns: u64,
    frames_published: u64,
}

/// Probe KMS using the monitor selected through the environment-backed compatibility path.
pub fn probe_kms() -> bool {
    let monitor = std::env::var("SLIPSTREAM_CAPTURE_MONITOR").ok();
    probe_kms_for_monitor(monitor.as_deref())
}

/// Probe the complete KMS path for one monitor. This opens the active primary plane and tests
/// Prime export, so capability reporting does not stop at "a DRM card exists".
pub fn probe_kms_for_monitor(monitor: Option<&str>) -> bool {
    let requested_plane = std::env::var("SLIPSTREAM_KMS_PLANE_ID")
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok());
    card_paths().into_iter().any(|path| {
        OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_CLOEXEC)
            .open(path)
            .ok()
            .is_some_and(|card| {
                let requested_crtc = match monitor {
                    Some(name) => match crtc_for_monitor(&card, name) {
                        Ok(Some(crtc)) => Some(crtc),
                        _ => return false,
                    },
                    None => None,
                };
                let Ok(plane_id) = select_plane(&card, requested_crtc, requested_plane, monitor)
                else {
                    return false;
                };
                KmsCapturer::open(card, plane_id).is_ok()
            })
    })
}

/// Open the first active primary plane that libdrm exposes. A plane id can be pinned for diagnosis
/// with `SLIPSTREAM_KMS_PLANE_ID`; the normal path follows the driver's primary-plane property.
pub fn open_kms_desktop() -> Result<Box<dyn Capturer>> {
    let monitor = std::env::var("SLIPSTREAM_CAPTURE_MONITOR").ok();
    open_kms_desktop_for_monitor(monitor.as_deref())
}

/// Open an active primary plane for the effective physical-monitor pin, when one is supplied.
/// The host passes persisted policy here because this crate only sees environment-backed config.
pub fn open_kms_desktop_for_monitor(monitor: Option<&str>) -> Result<Box<dyn Capturer>> {
    let requested_plane = std::env::var("SLIPSTREAM_KMS_PLANE_ID")
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok());
    let mut errors = Vec::new();

    for path in card_paths() {
        let card = match OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_CLOEXEC)
            .open(&path)
        {
            Ok(card) => card,
            Err(error) => {
                errors.push(format!("{}: {error}", path.display()));
                continue;
            }
        };
        if let Err(error) = ensure_kms(&card) {
            errors.push(format!("{}: {error}", path.display()));
            continue;
        }
        let requested_crtc = match monitor {
            Some(name) => match crtc_for_monitor(&card, name) {
                Ok(Some(crtc_id)) => Some(crtc_id),
                Ok(None) => {
                    errors.push(format!(
                        "{}: monitor {name:?} is not an active DRM connector",
                        path.display()
                    ));
                    continue;
                }
                Err(error) => {
                    errors.push(format!(
                        "{}: find monitor {name:?}: {error}",
                        path.display()
                    ));
                    continue;
                }
            },
            None => None,
        };
        let plane_id = match select_plane(&card, requested_crtc, requested_plane, monitor) {
            Ok(id) => id,
            Err(error) => {
                errors.push(format!("{}: {error}", path.display()));
                continue;
            }
        };
        match KmsCapturer::open(card, plane_id) {
            Ok(capturer) => {
                return Ok(Box::new(capturer));
            }
            Err(error) => errors.push(format!("{} plane {plane_id}: {error}", path.display())),
        }
    }

    if errors.is_empty() {
        bail!("no DRM card with an active primary plane was found")
    }
    bail!(
        "KMS capture could not open an active primary plane: {}",
        errors.join("; ")
    )
}

fn select_plane(
    card: &File,
    requested_crtc: Option<u32>,
    requested_plane: Option<u32>,
    monitor: Option<&str>,
) -> Result<u32> {
    ensure_kms(card)?;
    match requested_plane {
        Some(id) => {
            let state = plane_on_card(card, id)?;
            ensure!(
                state.fb_id != 0 && state.crtc_id != 0,
                "requested plane {id} has no active framebuffer"
            );
            ensure!(
                requested_crtc.is_none_or(|crtc| crtc == state.crtc_id),
                "requested plane {id} does not drive monitor {monitor:?}"
            );
            ensure!(
                plane_type(card, id) == Some(DRM_PLANE_TYPE_PRIMARY),
                "requested plane {id} is not a primary plane"
            );
            Ok(id)
        }
        None => find_primary_plane(card, requested_crtc),
    }
}

impl KmsCapturer {
    fn open(card: File, plane_id: u32) -> Result<Self> {
        let state = plane_on_card(&card, plane_id)?;
        ensure!(
            state.fb_id != 0 && state.crtc_id != 0,
            "KMS primary plane {plane_id} is not active"
        );
        let vblank_type = crtc_vblank_type(&card, state.crtc_id)?;
        let framebuffer = framebuffer(&card, state.fb_id)?;
        let format = pixel_format(framebuffer.fourcc).ok_or_else(|| {
            anyhow!(
                "primary plane {plane_id} uses unsupported DRM format {:#x}",
                framebuffer.fourcc
            )
        })?;
        ensure!(
            framebuffer.handle != 0,
            "primary plane {plane_id} framebuffer {} has no GEM handle",
            state.fb_id
        );
        let _ = prime_fd(&card, framebuffer.handle)
            .context("export the KMS primary framebuffer as a dma-buf")?;
        let cursor_plane_id = find_cursor_plane(&card, state.crtc_id);
        if cursor_plane_id.is_none() {
            tracing::warn!(
                plane_id,
                crtc_id = state.crtc_id,
                "KMS capture: no cursor plane found; pointer metadata may be unavailable"
            );
        }
        tracing::info!(
            card = ?card,
            plane_id,
            framebuffer = state.fb_id,
            width = framebuffer.width,
            height = framebuffer.height,
            fourcc = ?format,
            modifier = framebuffer.modifier,
            "KMS capture: primary plane open"
        );
        Ok(Self {
            card,
            plane_id,
            crtc_id: state.crtc_id,
            vblank_type,
            cursor_plane_id,
            framebuffer,
            active: true,
            dead: AtomicBool::new(false),
            last_cursor_signature: CursorSignature::default(),
            cursor: None,
            last_frame_ns: 0,
            frames_published: 0,
        })
    }

    fn current_state(&self) -> Result<PlaneState> {
        plane_on_card(&self.card, self.plane_id)
    }

    fn current_cursor_state(&self) -> Option<PlaneState> {
        self.cursor_plane_id
            .and_then(|plane_id| plane_on_card(&self.card, plane_id).ok())
    }

    fn cursor_signature(&self) -> CursorSignature {
        self.current_cursor_state()
            .map(|state| CursorSignature {
                fb_id: state.fb_id,
                crtc_id: state.crtc_id,
                crtc_x: state.crtc_x,
                crtc_y: state.crtc_y,
                x: state.x,
                y: state.y,
            })
            .unwrap_or_default()
    }

    fn refresh_cursor(&mut self) -> Option<CursorOverlay> {
        let state = self.current_cursor_state()?;
        self.last_cursor_signature = CursorSignature {
            fb_id: state.fb_id,
            crtc_id: state.crtc_id,
            crtc_x: state.crtc_x,
            crtc_y: state.crtc_y,
            x: state.x,
            y: state.y,
        };
        if state.fb_id == 0 || state.crtc_id == 0 {
            if let Some(cursor) = self.cursor.as_mut() {
                cursor.visible = false;
                cursor.x = state.crtc_x as i32;
                cursor.y = state.crtc_y as i32;
                return Some(cursor.overlay());
            }
            return None;
        }

        let needs_image = self
            .cursor
            .as_ref()
            .is_none_or(|cursor| cursor.fb_id != state.fb_id);
        if needs_image {
            let framebuffer = match framebuffer(&self.card, state.fb_id) {
                Ok(framebuffer) => framebuffer,
                Err(error) => {
                    tracing::debug!(error = %error, "KMS cursor framebuffer disappeared during refresh");
                    return self.cursor.as_ref().map(CursorCache::overlay);
                }
            };
            let (width, height, rgba) = match read_cursor_image(&self.card, framebuffer) {
                Ok(image) => image,
                Err(error) => {
                    tracing::debug!(error = %error, "KMS cursor framebuffer is not readable");
                    return self.cursor.as_ref().map(CursorCache::overlay);
                }
            };
            let serial = self
                .cursor
                .as_ref()
                .map_or(1, |cursor| cursor.serial.saturating_add(1));
            self.cursor = Some(CursorCache {
                fb_id: state.fb_id,
                width,
                height,
                rgba,
                serial,
                x: state.crtc_x as i32,
                y: state.crtc_y as i32,
                visible: true,
            });
        } else if let Some(cursor) = self.cursor.as_mut() {
            cursor.x = state.crtc_x as i32;
            cursor.y = state.crtc_y as i32;
            cursor.visible = true;
        }
        self.cursor.as_ref().map(CursorCache::overlay)
    }

    fn capture_current(&mut self, state: PlaneState) -> Result<CapturedFrame> {
        let framebuffer = framebuffer(&self.card, state.fb_id)?;
        let format = pixel_format(framebuffer.fourcc).ok_or_else(|| {
            anyhow!(
                "KMS plane {} changed to unsupported DRM format {:#x}",
                self.plane_id,
                framebuffer.fourcc
            )
        })?;
        ensure!(
            framebuffer.handle != 0,
            "KMS plane {} framebuffer {} has no GEM handle",
            self.plane_id,
            state.fb_id
        );
        ensure!(
            framebuffer.pitch
                >= framebuffer
                    .width
                    .saturating_mul(format.bytes_per_pixel() as u32),
            "KMS framebuffer {} pitch {} is too small for {}x{} {:?}",
            state.fb_id,
            framebuffer.pitch,
            framebuffer.width,
            framebuffer.height,
            format
        );

        let fd = prime_fd(&self.card, framebuffer.handle)?;
        let pts_ns = capture_now_ns();
        let cursor = self.refresh_cursor();
        self.framebuffer = framebuffer;
        self.last_frame_ns = pts_ns;
        self.frames_published = self.frames_published.saturating_add(1);
        Ok(CapturedFrame {
            width: framebuffer.width,
            height: framebuffer.height,
            pts_ns,
            format,
            payload: FramePayload::Dmabuf(DmabufFrame {
                fd,
                fourcc: framebuffer.fourcc,
                modifier: framebuffer.modifier,
                plane1: None,
                offset: framebuffer.offset,
                stride: framebuffer.pitch,
            }),
            cursor,
        })
    }

    fn mark_dead<T>(&self, result: Result<T>) -> Result<T> {
        if result.is_err() {
            self.dead.store(true, Ordering::Release);
        }
        result
    }

    fn wait_vblank(&mut self) -> Result<()> {
        let mut vblank = DrmVblank {
            request: DrmVblankRequest {
                type_: self.vblank_type,
                sequence: 1,
                signal: 0,
            },
        };
        // SAFETY: `self.card` owns a live DRM descriptor and `vblank` is a correctly sized C ABI
        // union whose request arm asks the kernel to block for one relative vblank. The ioctl does
        // not retain the pointer after returning.
        let rc = unsafe { drmWaitVBlank(self.card.as_raw_fd(), &mut vblank) };
        if rc != 0 {
            bail!(
                "DRM vblank wait failed for CRTC {}: {}",
                self.crtc_id,
                std::io::Error::last_os_error()
            )
        }
        Ok(())
    }
}

impl Capturer for KmsCapturer {
    fn backend_name(&self) -> &'static str {
        "kms-primary-plane"
    }

    fn next_frame(&mut self) -> Result<CapturedFrame> {
        if self.dead.load(Ordering::Acquire) {
            bail!("KMS capture is dead; rebuild the DRM plane source")
        }
        let state = self.mark_dead(self.current_state())?;
        if state.fb_id == 0 || state.crtc_id == 0 {
            self.dead.store(true, Ordering::Release);
            return Err(anyhow!(
                "KMS primary plane {} has no framebuffer",
                self.plane_id
            ));
        }
        if state.crtc_id != self.crtc_id {
            self.dead.store(true, Ordering::Release);
            bail!(
                "KMS plane {} moved from CRTC {} to {}",
                self.plane_id,
                self.crtc_id,
                state.crtc_id
            );
        }
        let result = self.capture_current(state);
        if result.is_err() {
            self.dead.store(true, Ordering::Release);
        }
        result
    }

    fn try_latest(&mut self) -> Result<Option<CapturedFrame>> {
        if !self.active {
            return Ok(None);
        }
        if self.dead.load(Ordering::Acquire) {
            bail!("KMS capture is dead; rebuild the DRM plane source")
        }
        let state = self.mark_dead(self.current_state())?;
        if state.fb_id == 0 || state.crtc_id == 0 {
            self.dead.store(true, Ordering::Release);
            bail!("KMS primary plane {} is no longer active", self.plane_id);
        }
        if state.crtc_id != self.crtc_id {
            self.dead.store(true, Ordering::Release);
            bail!(
                "KMS plane {} moved from CRTC {} to {}",
                self.plane_id,
                self.crtc_id,
                state.crtc_id
            );
        }
        self.capture_current(state).map(Some).inspect_err(|_| {
            self.dead.store(true, Ordering::Release);
        })
    }

    fn supports_arrival_wait(&self) -> bool {
        true
    }

    fn wait_arrival(&mut self, deadline: Instant) {
        if !self.active || self.dead.load(Ordering::Acquire) || Instant::now() >= deadline {
            return;
        }
        match self.current_state() {
            Ok(state) if state.fb_id != 0 && state.crtc_id == self.crtc_id => {
                if let Err(error) = self.wait_vblank() {
                    tracing::debug!(error = %error, "KMS vblank wait ended");
                    self.dead.store(true, Ordering::Release);
                }
            }
            Ok(_) => {
                self.dead.store(true, Ordering::Release);
            }
            Err(error) => {
                tracing::debug!(error = %error, "KMS plane query ended");
                self.dead.store(true, Ordering::Release);
            }
        }
    }

    fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    fn is_alive(&self) -> bool {
        !self.dead.load(Ordering::Acquire)
    }

    fn cursor(&mut self) -> Option<CursorOverlay> {
        if !self.active || self.dead.load(Ordering::Acquire) {
            return None;
        }
        if self.cursor_signature() == self.last_cursor_signature {
            return self.cursor.as_ref().map(CursorCache::overlay);
        }
        self.refresh_cursor()
    }

    fn telemetry(&self) -> CaptureTelemetry {
        CaptureTelemetry {
            last_frame_ns: self.last_frame_ns,
            frames_published: self.frames_published,
            width: self.framebuffer.width,
            height: self.framebuffer.height,
            modifier: self.framebuffer.modifier,
            ..CaptureTelemetry::default()
        }
    }
}

fn card_paths() -> Vec<PathBuf> {
    let mut paths = std::fs::read_dir("/dev/dri")
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| {
                    name.strip_prefix("card")
                        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
                })
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn ensure_kms(card: &File) -> Result<()> {
    let fd = card.as_raw_fd();
    // SAFETY: `fd` is the live O_RDWR descriptor owned by `card`; libdrm only reads the integer
    // and queries the kernel. No Rust pointer is passed.
    if unsafe { drmIsKMS(fd) } == 0 {
        bail!("the DRM node is not a KMS device")
    }
    // SAFETY: the descriptor is live and the capability/value are by-value ABI arguments. The
    // capability enables the plane resources used by the subsequent read-only enumeration.
    if unsafe { drmSetClientCap(fd, DRM_CLIENT_CAP_UNIVERSAL_PLANES, 1) } != 0 {
        bail!(
            "DRM universal planes are unavailable: {}",
            std::io::Error::last_os_error()
        )
    }
    Ok(())
}

fn find_primary_plane(card: &File, requested_crtc: Option<u32>) -> Result<u32> {
    ensure_kms(card)?;
    let plane_ids = plane_ids(card)?;
    for plane_id in plane_ids {
        let Some(state) = plane_on_card(card, plane_id).ok() else {
            continue;
        };
        if state.fb_id == 0
            || state.crtc_id == 0
            || requested_crtc.is_some_and(|crtc| crtc != state.crtc_id)
        {
            continue;
        }
        match plane_type(card, plane_id) {
            Some(DRM_PLANE_TYPE_PRIMARY) => return Ok(plane_id),
            Some(_) => {}
            None => {}
        };
    }
    Err(anyhow!("no active primary DRM plane was found"))
}

fn plane_ids(card: &File) -> Result<Vec<u32>> {
    let fd = card.as_raw_fd();
    // SAFETY: libdrm returns either a null pointer or an allocated plane-resource object owned by
    // this call. We check for null, read the advertised array while it is alive, and free it on
    // every path below.
    let resources = unsafe { drmModeGetPlaneResources(fd) };
    if resources.is_null() {
        bail!(
            "drmModeGetPlaneResources failed: {}",
            std::io::Error::last_os_error()
        )
    }
    // SAFETY: `resources` is the live non-null result of `drmModeGetPlaneResources`; these scalar
    // fields are copied before the matching free below.
    let (count, pointer) = unsafe { ((*resources).count_planes, (*resources).planes) };
    if count == 0 || pointer.is_null() {
        // SAFETY: `resources` is the live object returned by libdrm and is freed exactly once.
        unsafe { drmModeFreePlaneResources(resources) };
        bail!("DRM plane resources are empty")
    }
    // SAFETY: `resources` was checked non-null and its `planes` array contains `count_planes`
    // entries per libdrm's ABI. The array is borrowed only until the matching free below.
    let plane_ids = unsafe { std::slice::from_raw_parts(pointer, count as usize).to_vec() };
    // SAFETY: `resources` is the live pointer returned by the matching libdrm allocator and is
    // freed exactly once after the borrowed array has been copied.
    unsafe { drmModeFreePlaneResources(resources) };
    Ok(plane_ids)
}

fn find_cursor_plane(card: &File, crtc_id: u32) -> Option<u32> {
    let crtc_index = crtc_index(card, crtc_id);
    plane_ids(card).ok()?.into_iter().find(|&plane_id| {
        if plane_type(card, plane_id) != Some(DRM_PLANE_TYPE_CURSOR) {
            return false;
        }
        let Ok(state) = plane_on_card(card, plane_id) else {
            return false;
        };
        cursor_plane_matches_crtc(state.crtc_id, state.possible_crtcs, crtc_id, crtc_index)
    })
}

fn cursor_plane_matches_crtc(
    plane_crtc_id: u32,
    possible_crtcs: u32,
    crtc_id: u32,
    crtc_index: Option<u32>,
) -> bool {
    plane_crtc_id == crtc_id
        || (plane_crtc_id == 0
            && crtc_index
                .is_some_and(|index| index < u32::BITS && possible_crtcs & (1u32 << index) != 0))
}

fn crtc_index(card: &File, crtc_id: u32) -> Option<u32> {
    let resources = resources(card).ok()?;
    let ids = unsafe_resource_ids(resources, |resources| {
        (resources.crtcs, resources.count_crtcs)
    });
    // SAFETY: `resources` is the live result of `drmModeGetResources` and is freed exactly once
    // after the CRTC ids have been copied.
    unsafe { drmModeFreeResources(resources) };
    ids.iter()
        .position(|&id| id == crtc_id)
        .map(|index| index as u32)
}

fn crtc_vblank_type(card: &File, crtc_id: u32) -> Result<u32> {
    let index = crtc_index(card, crtc_id)
        .ok_or_else(|| anyhow!("CRTC {crtc_id} is not present in DRM resources"))?;
    ensure!(
        index <= 31,
        "CRTC index {index} cannot be addressed by the DRM vblank ABI"
    );
    let high_crtc = if index == 0 {
        0
    } else {
        (index << DRM_VBLANK_HIGH_CRTC_SHIFT) & DRM_VBLANK_HIGH_CRTC_MASK
    };
    Ok(DRM_VBLANK_RELATIVE | high_crtc)
}

fn crtc_for_monitor(card: &File, wanted: &str) -> Result<Option<u32>> {
    let resources = resources(card)?;
    let connector_ids = unsafe_resource_ids(resources, |resources| {
        (resources.connectors, resources.count_connectors)
    });
    // SAFETY: `resources` was returned by `drmModeGetResources` and the copied ids no longer
    // borrow it. The matching free is performed exactly once before returning.
    unsafe { drmModeFreeResources(resources) };

    for connector_id in connector_ids {
        let fd = card.as_raw_fd();
        // SAFETY: `connector_id` came from the live DRM resource list. libdrm returns an owned
        // connector object that is checked and freed below.
        let connector = unsafe { drmModeGetConnector(fd, connector_id) };
        if connector.is_null() {
            continue;
        }
        // SAFETY: the connector object is live for this block; all scalar fields and encoder ids
        // are copied before its matching free.
        let (name, encoder_id, encoder_ids, connection) = unsafe {
            let count = (*connector).count_encoders.max(0) as usize;
            let ids = if count == 0 || (*connector).encoders.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts((*connector).encoders, count).to_vec()
            };
            (
                connector_name((*connector).connector_type, (*connector).connector_type_id),
                (*connector).encoder_id,
                ids,
                (*connector).connection,
            )
        };
        // SAFETY: `connector` came from `drmModeGetConnector` and is freed exactly once here.
        unsafe { drmModeFreeConnector(connector) };
        if !name
            .as_deref()
            .is_some_and(|name| name.eq_ignore_ascii_case(wanted))
            || connection == 2
        {
            continue;
        }
        let mut candidates = encoder_ids;
        if encoder_id != 0 {
            candidates.insert(0, encoder_id);
        }
        for encoder_id in candidates {
            // SAFETY: `encoder_id` came from a live connector object. The pointer is copied and
            // released before the next candidate is examined.
            let encoder = unsafe { drmModeGetEncoder(fd, encoder_id) };
            if encoder.is_null() {
                continue;
            }
            // SAFETY: `encoder` is live and only its scalar C ABI field is read here.
            let crtc_id = unsafe { (*encoder).crtc_id };
            // SAFETY: `encoder` came from `drmModeGetEncoder` and is freed exactly once.
            unsafe { drmModeFreeEncoder(encoder) };
            if crtc_id != 0 {
                return Ok(Some(crtc_id));
            }
        }
    }
    Ok(None)
}

fn connector_name(connector_type: u32, connector_type_id: u32) -> Option<String> {
    let prefix = match connector_type {
        1 => "VGA",
        2 => "DVI-I",
        3 => "DVI-D",
        4 => "DVI-A",
        5 => "Composite",
        6 => "SVIDEO",
        7 => "LVDS",
        8 => "Component",
        9 => "9PinDIN",
        10 => "DP",
        11 => "HDMI-A",
        12 => "HDMI-B",
        13 => "TV",
        14 => "eDP",
        15 => "Virtual",
        16 => "DSI",
        17 => "DPI",
        18 => "Writeback",
        19 => "SPI",
        20 => "USB",
        _ => return None,
    };
    Some(format!("{prefix}-{connector_type_id}"))
}

fn resources(card: &File) -> Result<*mut DrmModeResources> {
    // SAFETY: `card` owns a live DRM descriptor and libdrm returns either a null pointer or an
    // allocated resource object owned by this call.
    let resources = unsafe { drmModeGetResources(card.as_raw_fd()) };
    if resources.is_null() {
        bail!(
            "drmModeGetResources failed: {}",
            std::io::Error::last_os_error()
        )
    }
    Ok(resources)
}

fn unsafe_resource_ids(
    resources: *mut DrmModeResources,
    fields: impl FnOnce(&DrmModeResources) -> (*mut u32, c_int),
) -> Vec<u32> {
    // SAFETY: callers pass a live non-null libdrm resource object, and the returned vector owns a
    // copy of the advertised ids before the caller frees that object.
    let (ptr, count) = unsafe { fields(&*resources) };
    if ptr.is_null() || count <= 0 {
        return Vec::new();
    }
    // SAFETY: libdrm's resource object advertises `count` contiguous ids at `ptr`; the copy is
    // completed while the object remains alive.
    unsafe { std::slice::from_raw_parts(ptr, count as usize).to_vec() }
}

fn plane_on_card(card: &File, plane_id: u32) -> Result<PlaneState> {
    let fd = card.as_raw_fd();
    // SAFETY: `fd` is live and `plane_id` came from libdrm's plane-resource list or an explicit
    // operator pin. The returned pointer is checked and released below.
    let plane = unsafe { drmModeGetPlane(fd, plane_id) };
    if plane.is_null() {
        bail!(
            "drmModeGetPlane({plane_id}) failed: {}",
            std::io::Error::last_os_error()
        )
    }
    // SAFETY: `plane` is a live libdrm object. We copy its scalar framebuffer id before freeing it.
    let state = unsafe {
        PlaneState {
            fb_id: (*plane).fb_id,
            crtc_id: (*plane).crtc_id,
            possible_crtcs: (*plane).possible_crtcs,
            crtc_x: (*plane).crtc_x,
            crtc_y: (*plane).crtc_y,
            x: (*plane).x,
            y: (*plane).y,
        }
    };
    // SAFETY: `plane` came from `drmModeGetPlane` and is freed exactly once here.
    unsafe { drmModeFreePlane(plane) };
    Ok(state)
}

fn plane_type(card: &File, plane_id: u32) -> Option<u64> {
    let fd = card.as_raw_fd();
    // SAFETY: the descriptor and object id are live ABI values; libdrm returns an owned object or
    // null. The object is freed after its arrays have been copied/read.
    let properties = unsafe { drmModeObjectGetProperties(fd, plane_id, DRM_MODE_OBJECT_PLANE) };
    if properties.is_null() {
        return None;
    }
    // SAFETY: `properties` is the live non-null result of `drmModeObjectGetProperties`; its
    // arrays remain valid until the matching free below, and only their pointers/count are copied.
    let (count, ids_ptr, values_ptr) = unsafe {
        (
            (*properties).count_props,
            (*properties).props,
            (*properties).prop_values,
        )
    };
    if count == 0 || ids_ptr.is_null() || values_ptr.is_null() {
        // SAFETY: `properties` is the live object returned by libdrm and is freed exactly once.
        unsafe { drmModeFreeObjectProperties(properties) };
        return None;
    }
    // SAFETY: both arrays are owned by `properties` and contain `count_props` entries. We read
    // them only while the object remains alive.
    let (ids, values) = unsafe {
        (
            std::slice::from_raw_parts(ids_ptr, count as usize),
            std::slice::from_raw_parts(values_ptr, count as usize),
        )
    };
    let mut result = None;
    for (&property_id, &value) in ids.iter().zip(values.iter()) {
        // SAFETY: `property_id` came from libdrm's live property array. The returned property is
        // checked before reading its NUL-terminated name and then released below.
        let property = unsafe { drmModeGetProperty(fd, property_id) };
        let Some(property) = (!property.is_null()).then_some(property) else {
            continue;
        };
        // SAFETY: libdrm's fixed 32-byte name is NUL-terminated by the ABI. We convert only that
        // live field and immediately copy the comparison result; no pointer escapes the block.
        let is_type = unsafe { CStr::from_ptr((*property).name.as_ptr()).to_bytes() == b"type" };
        // SAFETY: `property` is the live result of `drmModeGetProperty`, freed exactly once here.
        unsafe { drmModeFreeProperty(property) };
        if is_type {
            result = Some(value);
            break;
        }
    }
    // SAFETY: `properties` is the live result of `drmModeObjectGetProperties`, freed exactly once.
    unsafe { drmModeFreeObjectProperties(properties) };
    result
}

fn framebuffer(card: &File, fb_id: u32) -> Result<Framebuffer> {
    let fd = card.as_raw_fd();
    // SAFETY: `fd` and `fb_id` are live DRM identifiers. The returned pointer is checked and
    // released with the matching libdrm function after copying its fields.
    let fb2 = unsafe { drmModeGetFB2(fd, fb_id) };
    if !fb2.is_null() {
        // SAFETY: `fb2` is a live libdrm framebuffer object. We copy the scalar fields and then
        // free the object before returning, so no borrowed pointer escapes.
        let (value, has_extra_plane) = unsafe {
            (
                Framebuffer {
                    width: (*fb2).width,
                    height: (*fb2).height,
                    fourcc: (*fb2).pixel_format,
                    modifier: if (*fb2).flags & DRM_MODE_FB_MODIFIERS != 0 {
                        (*fb2).modifier
                    } else {
                        DRM_FORMAT_MOD_LINEAR
                    },
                    handle: (*fb2).handles[0],
                    offset: (*fb2).offsets[0],
                    pitch: (*fb2).pitches[0],
                },
                (&(*fb2).handles)[1..].iter().any(|&handle| handle != 0),
            )
        };
        // SAFETY: `fb2` came from `drmModeGetFB2` and is freed exactly once here.
        unsafe { drmModeFreeFB2(fb2) };
        ensure!(
            !has_extra_plane,
            "DRM framebuffer {fb_id} is multi-plane and is not supported by the dmabuf frame ABI"
        );
        return Ok(value);
    }

    // Older drivers expose only the legacy framebuffer query. Its implicit format is XRGB8888.
    // SAFETY: same ownership/descriptor proof as the FB2 query above.
    let legacy = unsafe { drmModeGetFB(fd, fb_id) };
    if legacy.is_null() {
        bail!(
            "drmModeGetFB2/FB({fb_id}) failed: {}",
            std::io::Error::last_os_error()
        )
    }
    // SAFETY: `legacy` is a live libdrm object. We copy its scalars before freeing it.
    let value = unsafe {
        Framebuffer {
            width: (*legacy).width,
            height: (*legacy).height,
            fourcc: DRM_FORMAT_XRGB8888,
            modifier: DRM_FORMAT_MOD_LINEAR,
            handle: (*legacy).handle,
            offset: 0,
            pitch: (*legacy).pitch,
        }
    };
    // SAFETY: `legacy` came from `drmModeGetFB` and is freed exactly once here.
    unsafe { drmModeFreeFB(legacy) };
    Ok(value)
}

fn prime_fd(card: &File, handle: u32) -> Result<OwnedFd> {
    let mut fd = -1;
    // SAFETY: `card` owns a live DRM descriptor, `handle` came from its live framebuffer, and
    // `&mut fd` is a valid out-parameter for libdrm to fill. The call does not retain the pointer.
    let rc =
        unsafe { drmPrimeHandleToFD(card.as_raw_fd(), handle, libc::O_CLOEXEC as u32, &mut fd) };
    if rc != 0 || fd < 0 {
        bail!(
            "drmPrimeHandleToFD(handle {handle}) failed: {}",
            std::io::Error::last_os_error()
        )
    }
    // SAFETY: a successful drmPrimeHandleToFD returns one owned file descriptor exactly once;
    // transferring it into OwnedFd makes Rust close it exactly once.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

#[repr(C)]
struct DmaBufSync {
    flags: u64,
}

const DMA_BUF_SYNC_READ: u64 = 1;
const DMA_BUF_SYNC_START: u64 = 0;
const DMA_BUF_SYNC_END: u64 = 1 << 2;
const DMA_BUF_IOCTL_SYNC: libc::c_ulong = ((1u64 << 30)
    | ((std::mem::size_of::<DmaBufSync>() as u64) << 16)
    | ((b'b' as u64) << 8)) as libc::c_ulong;

fn sync_dma_buf(fd: &OwnedFd, flags: u64) -> Result<()> {
    let mut sync = DmaBufSync { flags };
    // SAFETY: `fd` is a live PRIME descriptor and `sync` is the correctly sized writable
    // dma-buf ioctl argument. The kernel reads and updates only this stack value.
    let result = unsafe { libc::ioctl(fd.as_raw_fd(), DMA_BUF_IOCTL_SYNC, &mut sync) };
    if result != 0 {
        bail!(
            "DMA_BUF_IOCTL_SYNC failed: {}",
            std::io::Error::last_os_error()
        )
    }
    Ok(())
}

fn read_cursor_image(card: &File, framebuffer: Framebuffer) -> Result<(u32, u32, Arc<Vec<u8>>)> {
    ensure!(
        matches!(
            framebuffer.fourcc,
            DRM_FORMAT_ARGB8888 | DRM_FORMAT_ABGR8888 | DRM_FORMAT_XRGB8888 | DRM_FORMAT_XBGR8888
        ),
        "unsupported KMS cursor format {:#x}",
        framebuffer.fourcc
    );
    ensure!(
        framebuffer.modifier == DRM_FORMAT_MOD_LINEAR,
        "KMS cursor framebuffer uses unsupported modifier {:#x}",
        framebuffer.modifier
    );
    ensure!(
        framebuffer.handle != 0,
        "KMS cursor framebuffer has no GEM handle"
    );
    let row_bytes = (framebuffer.width as usize)
        .checked_mul(4)
        .ok_or_else(|| anyhow!("KMS cursor row size overflow"))?;
    ensure!(
        framebuffer.pitch as usize >= row_bytes,
        "KMS cursor pitch {} is smaller than {}",
        framebuffer.pitch,
        row_bytes
    );
    let image_bytes = (framebuffer.pitch as usize)
        .checked_mul(framebuffer.height as usize)
        .ok_or_else(|| anyhow!("KMS cursor framebuffer size overflow"))?;
    let output_bytes = row_bytes
        .checked_mul(framebuffer.height as usize)
        .ok_or_else(|| anyhow!("KMS cursor output size overflow"))?;
    let fd = prime_fd(card, framebuffer.handle)?;
    sync_dma_buf(&fd, DMA_BUF_SYNC_START | DMA_BUF_SYNC_READ)?;

    // `mmap` requires a page-aligned file offset. The framebuffer offset is restored by adding
    // `delta` to the mapped base before reading the rows.
    // SAFETY: `_SC_PAGESIZE` is a process-global scalar query with no pointer arguments.
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    ensure!(page_size > 0, "could not determine the system page size");
    let page_size = page_size as usize;
    let offset = framebuffer.offset as usize;
    let map_offset = offset & !(page_size - 1);
    let delta = offset - map_offset;
    let map_len = delta
        .checked_add(image_bytes)
        .ok_or_else(|| anyhow!("KMS cursor mapping size overflow"))?;
    ensure!(
        map_offset <= libc::off_t::MAX as usize,
        "KMS cursor mapping offset is too large"
    );
    // SAFETY: `fd` is a live read-only PRIME export, `map_len` is nonzero and checked for
    // overflow, and the page-aligned offset refers to the framebuffer allocation.
    let mapped = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            map_len,
            libc::PROT_READ,
            libc::MAP_SHARED,
            fd.as_raw_fd(),
            map_offset as libc::off_t,
        )
    };
    if mapped == libc::MAP_FAILED {
        let error = std::io::Error::last_os_error();
        let _ = sync_dma_buf(&fd, DMA_BUF_SYNC_END | DMA_BUF_SYNC_READ);
        return Err(anyhow!("mmap KMS cursor framebuffer: {error}"));
    }

    let mut rgba = vec![0u8; output_bytes];
    for y in 0..framebuffer.height as usize {
        // SAFETY: `mapped` covers `map_len`, and the checked row/delta arithmetic stays inside
        // the mapped framebuffer allocation.
        let source = unsafe {
            std::slice::from_raw_parts(
                (mapped as *const u8).add(delta + y * framebuffer.pitch as usize),
                row_bytes,
            )
        };
        let destination = &mut rgba[y * row_bytes..(y + 1) * row_bytes];
        for (src, dst) in source.chunks_exact(4).zip(destination.chunks_exact_mut(4)) {
            let pixel = [src[0], src[1], src[2], src[3]];
            dst.copy_from_slice(&cursor_pixel_to_rgba(framebuffer.fourcc, pixel));
        }
    }

    let sync_result = sync_dma_buf(&fd, DMA_BUF_SYNC_END | DMA_BUF_SYNC_READ);
    // SAFETY: `mapped` is the exact pointer returned by the successful mmap call and `map_len` is
    // unchanged, so this releases the mapping exactly once.
    let unmap_result = unsafe { libc::munmap(mapped, map_len) };
    sync_result?;
    ensure!(
        unmap_result == 0,
        "munmap KMS cursor framebuffer failed: {}",
        std::io::Error::last_os_error()
    );
    Ok((framebuffer.width, framebuffer.height, Arc::new(rgba)))
}

fn cursor_pixel_to_rgba(fourcc: u32, pixel: [u8; 4]) -> [u8; 4] {
    match fourcc {
        DRM_FORMAT_ARGB8888 => [pixel[2], pixel[1], pixel[0], pixel[3]],
        DRM_FORMAT_ABGR8888 => pixel,
        DRM_FORMAT_XRGB8888 => [pixel[2], pixel[1], pixel[0], u8::MAX],
        DRM_FORMAT_XBGR8888 => [pixel[0], pixel[1], pixel[2], u8::MAX],
        _ => unreachable!("cursor format checked above"),
    }
}

fn pixel_format(fourcc: u32) -> Option<PixelFormat> {
    Some(match fourcc {
        DRM_FORMAT_XRGB8888 => PixelFormat::Bgrx,
        DRM_FORMAT_ARGB8888 => PixelFormat::Bgra,
        DRM_FORMAT_XBGR8888 => PixelFormat::Rgbx,
        DRM_FORMAT_ABGR8888 => PixelFormat::Rgba,
        DRM_FORMAT_XRGB2101010 => PixelFormat::X2Rgb10,
        DRM_FORMAT_XBGR2101010 => PixelFormat::X2Bgr10,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fourcc_layout_matches_drm_little_endian() {
        assert_eq!(DRM_FORMAT_XRGB8888, u32::from_le_bytes(*b"XR24"));
        assert_eq!(DRM_FORMAT_XRGB2101010, u32::from_le_bytes(*b"XR30"));
    }

    #[test]
    fn supported_scanout_formats_keep_their_byte_order() {
        assert_eq!(pixel_format(DRM_FORMAT_XRGB8888), Some(PixelFormat::Bgrx));
        assert_eq!(pixel_format(DRM_FORMAT_ARGB8888), Some(PixelFormat::Bgra));
        assert_eq!(pixel_format(DRM_FORMAT_XBGR8888), Some(PixelFormat::Rgbx));
        assert_eq!(pixel_format(DRM_FORMAT_ABGR8888), Some(PixelFormat::Rgba));
        assert_eq!(
            pixel_format(DRM_FORMAT_XRGB2101010),
            Some(PixelFormat::X2Rgb10)
        );
        assert_eq!(
            pixel_format(DRM_FORMAT_XBGR2101010),
            Some(PixelFormat::X2Bgr10)
        );
        assert_eq!(pixel_format(fourcc(*b"NV12")), None);
    }

    #[test]
    fn cursor_formats_become_straight_rgba() {
        assert_eq!(
            cursor_pixel_to_rgba(DRM_FORMAT_ARGB8888, [3, 2, 1, 4]),
            [1, 2, 3, 4]
        );
        assert_eq!(
            cursor_pixel_to_rgba(DRM_FORMAT_ABGR8888, [1, 2, 3, 4]),
            [1, 2, 3, 4]
        );
        assert_eq!(
            cursor_pixel_to_rgba(DRM_FORMAT_XRGB8888, [3, 2, 1, 0]),
            [1, 2, 3, 255]
        );
        assert_eq!(
            cursor_pixel_to_rgba(DRM_FORMAT_XBGR8888, [1, 2, 3, 0]),
            [1, 2, 3, 255]
        );
    }

    #[test]
    fn cursor_plane_selection_stays_on_the_target_crtc() {
        assert!(cursor_plane_matches_crtc(42, 0, 42, Some(1)));
        assert!(!cursor_plane_matches_crtc(43, 0, 42, Some(1)));
        assert!(cursor_plane_matches_crtc(0, 1 << 1, 42, Some(1)));
        assert!(!cursor_plane_matches_crtc(0, 1 << 2, 42, Some(1)));
        assert!(!cursor_plane_matches_crtc(0, u32::MAX, 42, Some(32)));
    }
}
