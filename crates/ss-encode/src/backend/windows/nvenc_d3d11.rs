//! Windows direct-SDK NVENC. Capture delivers tightly packed CPU BGRA; this backend
//! uploads that into a D3D11 texture and registers it as `NV_ENC_INPUT_RESOURCE_TYPE_DIRECTX`.
//! NVENC does the RGB→YUV conversion, including HEVC 4:4:4 when the chip advertises it.
//!
//! The driver library (`nvEncodeAPI64.dll`) is loaded at runtime. A missing DLL, a
//! non-NVIDIA adapter, or a failed session open is a clean `None` from [`probe`] — the
//! selector then stays on openh264 and advertises H.264 only. Caps that do probe
//! (codec GUIDs, HEVC 4:4:4, 10-bit) are what the host advertises. 10-bit encode is
//! reported for the GPU, but Windows capture is 8-bit BGRA, so a 10-bit *session* is
//! refused here rather than labeled HDR while the pixels are SDR.

// Same exemption as `nvenc_core`: this body is raw `nvEncodeAPI` entry-table calls. Wrapping
// each one in `unsafe {}` would only restate the signature. The blocks that are NOT entry-table
// calls (D3D11, the bitstream copy) stay explicit and carry a SAFETY comment.
#![allow(unsafe_op_in_unsafe_fn)]
#![deny(clippy::undocumented_unsafe_blocks)]

use super::nvenc_core::{
    apply_low_latency_config, build_init_params, codec_guid, plan_range_recovery, resolve_slices,
    resolve_split_mode, resolve_split_subframe, resolve_subframe, subframe_env_forced,
    LowLatencyConfig, NvStatusExt, RangePlan,
};
use super::nvenc_status;
use super::{Codec, EncodedFrame, Encoder, EncoderCaps};
use crate::ChromaFormat;
use anyhow::{anyhow, bail, Context, Result};
use nvidia_video_codec_sdk::sys::nvEncodeAPI as nv;
use ss_frame::{CapturedFrame, FramePayload, PixelFormat};
use ss_gpu::VENDOR_NVIDIA;
use std::ffi::c_void;
use std::ptr;
use windows::{
    core::Interface,
    Win32::{
        Foundation::HMODULE,
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0},
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Resource,
                ID3D11Texture2D, D3D11_BIND_RENDER_TARGET, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
            },
            Dxgi::{CreateDXGIFactory1, IDXGIAdapter, IDXGIAdapter1, IDXGIFactory1},
        },
    },
};

/// Codecs and chroma/depth bits the driver reported for this process's NVIDIA adapter.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ProbedSupport {
    pub codecs: crate::CodecSupport,
    pub hevc_444: bool,
    pub ten_bit: bool,
}

/// Cached driver probe. `None` = NVENC cannot be used (no DLL, no NVIDIA adapter, or the
/// session open failed). One throwaway D3D11 session per process.
pub(crate) fn probe() -> Option<ProbedSupport> {
    static CACHE: std::sync::OnceLock<Option<ProbedSupport>> = std::sync::OnceLock::new();
    *CACHE.get_or_init(probe_uncached)
}

/// Whether dispatch should open NVENC rather than openh264.
///
/// Software is forced by an explicit `SLIPSTREAM_ENCODER=software` pin. `nvenc`/`nvidia`/
/// `cuda` force the hardware path (the open then fails loudly if the driver is absent).
/// `auto` (the default) takes NVENC only when the selected adapter is NVIDIA and the probe
/// succeeded. Linux-only names (`vaapi`, `vulkan`, `pyrowave`) are not a hardware selection;
/// [`open`] rejects them.
pub(crate) fn hardware_selected() -> bool {
    match dispatch() {
        Dispatch::Nvenc => probe().is_some(),
        Dispatch::NvencForced => true,
        Dispatch::Software | Dispatch::Unsupported => false,
    }
}

/// The codec mask to advertise, or `None` when this host must stay on the software H.264 bit.
pub(crate) fn advertised_wire_caps() -> Option<u8> {
    if !hardware_selected() {
        return None;
    }
    // A forced NVENC pin whose probe failed still must not advertise codecs we cannot
    // open. Empty probe → H.264 only would be a lie if the client then gets a hard
    // error at open; advertising nothing-but-H.264 lets a forced pin fail at open with
    // the real driver error while auto falls back. Forced + failed probe: no mask, so
    // the caller advertises H.264 and `open` still tries NVENC and surfaces the error
    // for any codec the client picked. Prefer the probed mask whenever the driver answered.
    probe().and_then(|p| p.codecs.wire_mask())
}

pub(crate) fn hevc_444_supported() -> bool {
    hardware_selected() && probe().is_some_and(|p| p.hevc_444)
}

pub(crate) fn ten_bit_supported() -> bool {
    hardware_selected() && probe().is_some_and(|p| p.ten_bit)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dispatch {
    /// Auto, and the selected GPU is NVIDIA with a live probe.
    Nvenc,
    /// Operator pinned NVENC. Open even when the probe cache is empty, so the error is real.
    NvencForced,
    Software,
    /// `vaapi` / `vulkan` / `pyrowave` / an unknown name — Linux-only on this host.
    Unsupported,
}

fn pref() -> String {
    ss_host_config::config()
        .encoder_pref
        .trim()
        .to_ascii_lowercase()
}

fn dispatch() -> Dispatch {
    match pref().as_str() {
        "software" | "cpu" | "openh264" | "sw" => Dispatch::Software,
        "nvenc" | "nvidia" | "cuda" => Dispatch::NvencForced,
        "" | "auto" => {
            if nvidia_selected() {
                Dispatch::Nvenc
            } else {
                Dispatch::Software
            }
        }
        _ => Dispatch::Unsupported,
    }
}

fn nvidia_selected() -> bool {
    ss_gpu::selected_gpu().is_some_and(|g| g.info.vendor_id == VENDOR_NVIDIA)
}

struct EncodeApi {
    open_encode_session_ex: unsafe extern "C" fn(
        *mut nv::NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS,
        *mut *mut c_void,
    ) -> nv::NVENCSTATUS,
    initialize_encoder:
        unsafe extern "C" fn(*mut c_void, *mut nv::NV_ENC_INITIALIZE_PARAMS) -> nv::NVENCSTATUS,
    destroy_encoder: unsafe extern "C" fn(*mut c_void) -> nv::NVENCSTATUS,
    reconfigure_encoder:
        unsafe extern "C" fn(*mut c_void, *mut nv::NV_ENC_RECONFIGURE_PARAMS) -> nv::NVENCSTATUS,
    get_encode_caps: unsafe extern "C" fn(
        *mut c_void,
        nv::GUID,
        *mut nv::NV_ENC_CAPS_PARAM,
        *mut std::ffi::c_int,
    ) -> nv::NVENCSTATUS,
    get_encode_guid_count: unsafe extern "C" fn(*mut c_void, *mut u32) -> nv::NVENCSTATUS,
    get_encode_guids:
        unsafe extern "C" fn(*mut c_void, *mut nv::GUID, u32, *mut u32) -> nv::NVENCSTATUS,
    get_encode_preset_config_ex: unsafe extern "C" fn(
        *mut c_void,
        nv::GUID,
        nv::GUID,
        nv::NV_ENC_TUNING_INFO,
        *mut nv::NV_ENC_PRESET_CONFIG,
    ) -> nv::NVENCSTATUS,
    create_bitstream_buffer: unsafe extern "C" fn(
        *mut c_void,
        *mut nv::NV_ENC_CREATE_BITSTREAM_BUFFER,
    ) -> nv::NVENCSTATUS,
    destroy_bitstream_buffer:
        unsafe extern "C" fn(*mut c_void, nv::NV_ENC_OUTPUT_PTR) -> nv::NVENCSTATUS,
    lock_bitstream:
        unsafe extern "C" fn(*mut c_void, *mut nv::NV_ENC_LOCK_BITSTREAM) -> nv::NVENCSTATUS,
    unlock_bitstream: unsafe extern "C" fn(*mut c_void, nv::NV_ENC_OUTPUT_PTR) -> nv::NVENCSTATUS,
    register_resource:
        unsafe extern "C" fn(*mut c_void, *mut nv::NV_ENC_REGISTER_RESOURCE) -> nv::NVENCSTATUS,
    unregister_resource:
        unsafe extern "C" fn(*mut c_void, nv::NV_ENC_REGISTERED_PTR) -> nv::NVENCSTATUS,
    map_input_resource:
        unsafe extern "C" fn(*mut c_void, *mut nv::NV_ENC_MAP_INPUT_RESOURCE) -> nv::NVENCSTATUS,
    unmap_input_resource:
        unsafe extern "C" fn(*mut c_void, nv::NV_ENC_INPUT_PTR) -> nv::NVENCSTATUS,
    encode_picture:
        unsafe extern "C" fn(*mut c_void, *mut nv::NV_ENC_PIC_PARAMS) -> nv::NVENCSTATUS,
    invalidate_ref_frames: unsafe extern "C" fn(*mut c_void, u64) -> nv::NVENCSTATUS,
}

fn try_api() -> std::result::Result<&'static EncodeApi, &'static str> {
    static TABLE: std::sync::OnceLock<std::result::Result<EncodeApi, String>> =
        std::sync::OnceLock::new();
    TABLE
        .get_or_init(|| {
            let table = load_api();
            if let Err(e) = &table {
                tracing::warn!(error = %e, "NVENC (Windows) API unavailable");
            }
            table
        })
        .as_ref()
        .map_err(|e| e.as_str())
}

fn api() -> &'static EncodeApi {
    try_api().expect("NVENC call before a successful try_api() gate")
}

fn load_api() -> std::result::Result<EncodeApi, String> {
    // SAFETY: `Library::new` runs `nvEncodeAPI64.dll`'s initializers — the NVIDIA driver
    // library, so loading has no unexpected effects; `map_err` handles its absence.
    // Each `lib.get` asserts the symbol's ABI equals the documented `nvEncodeAPI.h`
    // prototype. `NvEncodeAPIGetMaxSupportedVersion` writes one u32 through a live
    // pointer; `NvEncodeAPICreateInstance` fills `list` during the call only. Each
    // extracted fn pointer is copied out of its borrowing `Symbol` before `forget(lib)`
    // leaks the mapping, so every address stays valid for the process lifetime.
    unsafe {
        let lib = libloading::Library::new("nvEncodeAPI64.dll")
            .map_err(|e| format!("nvEncodeAPI64.dll not loadable (no NVIDIA driver?): {e}"))?;
        let get_version: libloading::Symbol<unsafe extern "C" fn(*mut u32) -> nv::NVENCSTATUS> =
            lib.get(b"NvEncodeAPIGetMaxSupportedVersion\0")
                .map_err(|e| {
                    format!("nvEncodeAPI64.dll exports no NvEncodeAPIGetMaxSupportedVersion: {e}")
                })?;
        let create_instance: libloading::Symbol<
            unsafe extern "C" fn(*mut nv::NV_ENCODE_API_FUNCTION_LIST) -> nv::NVENCSTATUS,
        > = lib
            .get(b"NvEncodeAPICreateInstance\0")
            .map_err(|e| format!("nvEncodeAPI64.dll exports no NvEncodeAPICreateInstance: {e}"))?;
        let get_version = *get_version;
        let create_instance = *create_instance;

        let mut version = 0u32;
        get_version(&mut version)
            .nv_ok()
            .map_err(|e| format!("NvEncodeAPIGetMaxSupportedVersion: {e:?}"))?;
        let (major, minor) = (version >> 4, version & 0xf);
        if (major, minor) < (nv::NVENCAPI_MAJOR_VERSION, nv::NVENCAPI_MINOR_VERSION) {
            return Err(format!(
                "driver NVENC API {major}.{minor} is older than the host's headers {}.{} — \
                 update the NVIDIA driver",
                nv::NVENCAPI_MAJOR_VERSION,
                nv::NVENCAPI_MINOR_VERSION
            ));
        }

        let mut list = nv::NV_ENCODE_API_FUNCTION_LIST {
            version: nv::NV_ENCODE_API_FUNCTION_LIST_VER,
            ..Default::default()
        };
        create_instance(&mut list)
            .nv_ok()
            .map_err(|e| format!("NvEncodeAPICreateInstance: {e:?}"))?;
        const MISSING: &str = "NvEncodeAPICreateInstance left an entry point unfilled";
        let api = EncodeApi {
            open_encode_session_ex: list.nvEncOpenEncodeSessionEx.ok_or(MISSING)?,
            initialize_encoder: list.nvEncInitializeEncoder.ok_or(MISSING)?,
            destroy_encoder: list.nvEncDestroyEncoder.ok_or(MISSING)?,
            reconfigure_encoder: list.nvEncReconfigureEncoder.ok_or(MISSING)?,
            get_encode_caps: list.nvEncGetEncodeCaps.ok_or(MISSING)?,
            get_encode_guid_count: list.nvEncGetEncodeGUIDCount.ok_or(MISSING)?,
            get_encode_guids: list.nvEncGetEncodeGUIDs.ok_or(MISSING)?,
            get_encode_preset_config_ex: list.nvEncGetEncodePresetConfigEx.ok_or(MISSING)?,
            create_bitstream_buffer: list.nvEncCreateBitstreamBuffer.ok_or(MISSING)?,
            destroy_bitstream_buffer: list.nvEncDestroyBitstreamBuffer.ok_or(MISSING)?,
            lock_bitstream: list.nvEncLockBitstream.ok_or(MISSING)?,
            unlock_bitstream: list.nvEncUnlockBitstream.ok_or(MISSING)?,
            register_resource: list.nvEncRegisterResource.ok_or(MISSING)?,
            unregister_resource: list.nvEncUnregisterResource.ok_or(MISSING)?,
            map_input_resource: list.nvEncMapInputResource.ok_or(MISSING)?,
            unmap_input_resource: list.nvEncUnmapInputResource.ok_or(MISSING)?,
            encode_picture: list.nvEncEncodePicture.ok_or(MISSING)?,
            invalidate_ref_frames: list.nvEncInvalidateRefFrames.ok_or(MISSING)?,
        };
        std::mem::forget(lib);
        Ok(api)
    }
}

fn probe_uncached() -> Option<ProbedSupport> {
    let Ok(api) = try_api() else {
        return None;
    };
    let Ok((device, _ctx)) = nvidia_device() else {
        tracing::info!("NVENC (Windows): no NVIDIA adapter — software H.264");
        return None;
    };
    // SAFETY: `try_api()` returned Ok, so every fn pointer is a live entry point. `device`
    // is a live `ID3D11Device` held for this function (NVENC does not take ownership).
    // `params`/`enc`/`guids` are live locals that outlive their synchronous calls. The
    // session is destroyed on every path out, including a failed open (the driver may
    // have taken the slot before erroring).
    unsafe {
        let mut params = nv::NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS {
            version: nv::NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS_VER,
            deviceType: nv::NV_ENC_DEVICE_TYPE::NV_ENC_DEVICE_TYPE_DIRECTX,
            device: device.as_raw(),
            apiVersion: nv::NVENCAPI_VERSION,
            ..Default::default()
        };
        let mut enc: *mut c_void = ptr::null_mut();
        if let Err(e) = (api.open_encode_session_ex)(&mut params, &mut enc).nv_ok() {
            if !enc.is_null() {
                let _ = (api.destroy_encoder)(enc);
            }
            tracing::warn!(
                error = %format!("{:#}", nvenc_status::call_err("open_encode_session_ex (codec probe)", e)),
                "NVENC (Windows) codec probe failed — advertising H.264 only"
            );
            return None;
        }
        nvenc_status::note_session_opened();
        let mut count = 0u32;
        let counted = (api.get_encode_guid_count)(enc, &mut count).nv_ok().is_ok();
        let mut guids = vec![nv::GUID::default(); count as usize];
        let mut written = 0u32;
        let listed = counted
            && count > 0
            && (api.get_encode_guids)(enc, guids.as_mut_ptr(), count, &mut written)
                .nv_ok()
                .is_ok();
        guids.truncate(written as usize);
        let cap = |guid: nv::GUID, which: nv::NV_ENC_CAPS| -> bool {
            let mut param = nv::NV_ENC_CAPS_PARAM {
                version: nv::NV_ENC_CAPS_PARAM_VER,
                capsToQuery: which,
                reserved: [0; 62],
            };
            let mut val: std::ffi::c_int = 0;
            (api.get_encode_caps)(enc, guid, &mut param, &mut val)
                .nv_ok()
                .is_ok()
                && val != 0
        };
        let hevc = guids.contains(&nv::NV_ENC_CODEC_HEVC_GUID);
        let hevc_444 = hevc
            && cap(
                nv::NV_ENC_CODEC_HEVC_GUID,
                nv::NV_ENC_CAPS::NV_ENC_CAPS_SUPPORT_YUV444_ENCODE,
            );
        let ten_bit = (hevc
            && cap(
                nv::NV_ENC_CODEC_HEVC_GUID,
                nv::NV_ENC_CAPS::NV_ENC_CAPS_SUPPORT_10BIT_ENCODE,
            ))
            || (guids.contains(&nv::NV_ENC_CODEC_AV1_GUID)
                && cap(
                    nv::NV_ENC_CODEC_AV1_GUID,
                    nv::NV_ENC_CAPS::NV_ENC_CAPS_SUPPORT_10BIT_ENCODE,
                ));
        let probed = ProbedSupport {
            codecs: crate::CodecSupport {
                h264: guids.contains(&nv::NV_ENC_CODEC_H264_GUID),
                h265: hevc,
                av1: guids.contains(&nv::NV_ENC_CODEC_AV1_GUID),
            },
            hevc_444,
            ten_bit,
        };
        let _ = (api.destroy_encoder)(enc);
        if !listed {
            tracing::warn!("NVENC (Windows) codec probe listed no encode GUIDs");
            return None;
        }
        tracing::info!(
            h264 = probed.codecs.h264,
            h265 = probed.codecs.h265,
            av1 = probed.codecs.av1,
            hevc_444 = probed.hevc_444,
            ten_bit = probed.ten_bit,
            "NVENC (Windows) encode capabilities probed"
        );
        Some(probed)
    }
}

/// The NVIDIA adapter NVENC should bind, preferring the selected GPU's LUID.
fn nvidia_device() -> Result<(ID3D11Device, ID3D11DeviceContext)> {
    let want = ss_gpu::selected_gpu()
        .filter(|g| g.info.vendor_id == VENDOR_NVIDIA)
        .and_then(|g| g.info.handle.dxgi_luid);
    // SAFETY: factory/adapter enumeration uses live out-params only. The chosen adapter
    // is reference-counted and outlives `D3D11CreateDevice`, which returns its own
    // device/context references. No frame is created.
    unsafe {
        let factory = CreateDXGIFactory1::<IDXGIFactory1>().context("CreateDXGIFactory1")?;
        let mut index = 0u32;
        let mut fallback: Option<IDXGIAdapter1> = None;
        let mut matched: Option<IDXGIAdapter1> = None;
        while let Ok(adapter) = factory.EnumAdapters1(index) {
            if let Ok(desc) = adapter.GetDesc() {
                if desc.VendorId == VENDOR_NVIDIA {
                    let luid = ((desc.AdapterLuid.HighPart as u64) << 32)
                        | desc.AdapterLuid.LowPart as u64;
                    if want == Some(luid) {
                        matched = Some(adapter);
                        break;
                    }
                    if fallback.is_none() {
                        fallback = Some(adapter);
                    }
                }
            }
            index += 1;
        }
        let adapter = matched.or(fallback).context("no NVIDIA DXGI adapter")?;
        let adapter: IDXGIAdapter = adapter.cast().context("IDXGIAdapter")?;
        let mut device = None;
        let mut ctx = None;
        let mut level = D3D_FEATURE_LEVEL_11_0;
        D3D11CreateDevice(
            Some(&adapter),
            D3D_DRIVER_TYPE_UNKNOWN,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&[D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&mut device),
            Some(&mut level),
            Some(&mut ctx),
        )
        .context("D3D11CreateDevice (NVIDIA)")?;
        Ok((
            device.context("D3D11CreateDevice returned no device")?,
            ctx.context("D3D11CreateDevice returned no context")?,
        ))
    }
}

/// D3D11 + NVENC session. Sync retrieve: `encode_picture` blocks, then `poll` locks the
/// bitstream. One input texture, re-uploaded every frame — capture is already a CPU copy,
/// and a depth-1 submit matches the low-latency contract.
pub struct NvencD3d11Encoder {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    texture: ID3D11Texture2D,
    encoder: *mut c_void,
    registered: nv::NV_ENC_REGISTERED_PTR,
    bitstream: nv::NV_ENC_OUTPUT_PTR,
    codec: Codec,
    codec_guid: nv::GUID,
    width: u32,
    height: u32,
    fps: u32,
    bitrate_bps: u64,
    chroma_444: bool,
    rfi_supported: bool,
    custom_vbv: bool,
    slices: u32,
    split_mode: u32,
    subframe_on: bool,
    frame_idx: i64,
    force_kf: bool,
    pending_anchor: bool,
    last_rfi_range: Option<(i64, i64)>,
    /// The AU from the last `submit`, waiting for `poll`. Sync encode produces exactly one.
    pending: Option<EncodedFrame>,
}

// SAFETY: the `!Send` fields are the raw NVENC session handle and the bitstream/registration
// pointers. `ID3D11Device` / `ID3D11DeviceContext` / `ID3D11Texture2D` are COM pointers the
// windows crate does not mark `Send`. The encoder is moved onto the host encode thread once
// and every method runs there; no NVENC or immediate-context call is in flight during the
// move. `Send` introduces no data race.
unsafe impl Send for NvencD3d11Encoder {}

impl NvencD3d11Encoder {
    #[allow(clippy::too_many_arguments)]
    pub fn open(
        codec: Codec,
        _format: PixelFormat,
        width: u32,
        height: u32,
        fps: u32,
        bitrate_bps: u64,
        _cuda: bool,
        bit_depth: u8,
        chroma: ChromaFormat,
        _cursor_blend: bool,
        max_slices: u32,
    ) -> Result<Self> {
        if bit_depth >= 10 {
            bail!(
                "Windows capture is 8-bit BGRA, so a {bit_depth}-bit encode would be labeled \
                 HDR while the pixels are SDR — refusing the session (HDR capture is not \
                 available on this host yet)"
            );
        }
        let probed = probe().context(
            "NVENC (Windows) is unavailable (no nvEncodeAPI64.dll, or no NVIDIA adapter)",
        )?;
        let codec_ok = match codec {
            Codec::H264 => probed.codecs.h264,
            Codec::H265 => probed.codecs.h265,
            Codec::Av1 => probed.codecs.av1,
            Codec::PyroWave => false,
        };
        if !codec_ok {
            bail!(
                "this NVIDIA GPU's NVENC does not encode {codec:?} (probed h264={} h265={} av1={})",
                probed.codecs.h264,
                probed.codecs.h265,
                probed.codecs.av1
            );
        }
        let mut chroma_444 = chroma.is_444() && codec == Codec::H265;
        if chroma_444 && !probed.hevc_444 {
            tracing::warn!(
                "HEVC 4:4:4 requested but this NVENC does not advertise YUV444 — encoding 4:2:0"
            );
            chroma_444 = false;
        }
        try_api().map_err(|e| anyhow!("NVENC (Windows) unavailable: {e}"))?;
        let (device, context) = nvidia_device()?;
        // SAFETY: `device` is the live NVIDIA device created just above and kept in `Self`.
        // Every NVENC pointer comes from the runtime table (`try_api` succeeded). The
        // texture, registration, and bitstream are destroyed in `Drop`.
        let built = unsafe {
            open_session(
                &device,
                codec,
                width,
                height,
                fps,
                bitrate_bps,
                chroma_444,
                max_slices.max(1),
            )
        }?;
        tracing::info!(
            codec = codec.nvenc_name(),
            width,
            height,
            fps,
            mbps = built.bitrate_bps / 1_000_000,
            chroma_444,
            rfi = built.rfi,
            "NVENC (Windows D3D11) encoder opened"
        );
        Ok(Self {
            device,
            context,
            texture: built.texture,
            encoder: built.encoder,
            registered: built.registered,
            bitstream: built.bitstream,
            codec,
            codec_guid: codec_guid(codec),
            width,
            height,
            fps,
            bitrate_bps: built.bitrate_bps,
            chroma_444,
            rfi_supported: built.rfi,
            custom_vbv: built.custom_vbv,
            slices: built.slices,
            split_mode: built.split_mode,
            subframe_on: built.subframe_on,
            frame_idx: 0,
            force_kf: false,
            pending_anchor: false,
            last_rfi_range: None,
            pending: None,
        })
    }
}

struct Opened {
    encoder: *mut c_void,
    texture: ID3D11Texture2D,
    registered: nv::NV_ENC_REGISTERED_PTR,
    bitstream: nv::NV_ENC_OUTPUT_PTR,
    rfi: bool,
    custom_vbv: bool,
    slices: u32,
    split_mode: u32,
    subframe_on: bool,
    bitrate_bps: u64,
}

#[allow(clippy::too_many_arguments)]
unsafe fn open_session(
    device: &ID3D11Device,
    codec: Codec,
    width: u32,
    height: u32,
    fps: u32,
    bitrate_bps: u64,
    chroma_444: bool,
    max_slices: u32,
) -> Result<Opened> {
    // Caps come from a throwaway session. `nvEncInitializeEncoder` is once-per-session:
    // a rejected config destroys that session and the next attempt opens a new one.
    let (rfi, custom_vbv, slices, split, subframe_on) = {
        let enc = open_encode_session(device)?;
        let guid = codec_guid(codec);
        let cap = |which: nv::NV_ENC_CAPS| -> i32 {
            let mut param = nv::NV_ENC_CAPS_PARAM {
                version: nv::NV_ENC_CAPS_PARAM_VER,
                capsToQuery: which,
                reserved: [0; 62],
            };
            let mut val: std::ffi::c_int = 0;
            if (api().get_encode_caps)(enc, guid, &mut param, &mut val)
                .nv_ok()
                .is_err()
            {
                0
            } else {
                val
            }
        };
        let rfi = cap(nv::NV_ENC_CAPS::NV_ENC_CAPS_SUPPORT_REF_PIC_INVALIDATION) != 0;
        let custom_vbv = cap(nv::NV_ENC_CAPS::NV_ENC_CAPS_SUPPORT_CUSTOM_VBV_BUF_SIZE) != 0;
        let subframe_cap = cap(nv::NV_ENC_CAPS::NV_ENC_CAPS_SUPPORT_SUBFRAME_READBACK) != 0;
        let slices = resolve_slices(codec, 4.min(max_slices));
        let subframe_on = resolve_subframe(subframe_cap);
        let pixel_rate = width as u64 * height as u64 * fps as u64;
        let split = resolve_split_mode(8, pixel_rate);
        let (split, subframe_on) =
            resolve_split_subframe(codec, split, subframe_on, subframe_env_forced());
        let _ = (api().destroy_encoder)(enc);
        (rfi, custom_vbv, slices, split, subframe_on)
    };

    let texture = create_input_texture(device, width, height)?;
    let disable = nv::NV_ENC_SPLIT_ENCODE_MODE::NV_ENC_SPLIT_DISABLE_MODE as u32;
    let mut attempts = vec![(bitrate_bps, split, subframe_on)];
    if split != disable {
        attempts.push((bitrate_bps, disable, subframe_on));
    }
    let mut rate = bitrate_bps.min(codec.max_bitrate_bps());
    let floor = 50_000_000u64;
    while rate > floor {
        rate = rate * 3 / 4;
        attempts.push((rate, disable, false));
    }

    let mut last = None;
    for (i, &(bps, split_mode, subframe)) in attempts.iter().enumerate() {
        let enc = open_encode_session(device)?;
        match try_initialize(
            enc,
            codec,
            codec_guid(codec),
            width,
            height,
            fps,
            bps,
            chroma_444,
            rfi,
            custom_vbv,
            slices,
            split_mode,
            subframe,
        ) {
            Ok(()) => {
                if i > 0 {
                    tracing::warn!(
                        requested_mbps = bitrate_bps / 1_000_000,
                        opened_mbps = bps / 1_000_000,
                        "NVENC (Windows) refused the first config — opened at a lower bitrate / split-off"
                    );
                }
                let mut rr = nv::NV_ENC_REGISTER_RESOURCE {
                    version: nv::NV_ENC_REGISTER_RESOURCE_VER,
                    resourceType:
                        nv::NV_ENC_INPUT_RESOURCE_TYPE::NV_ENC_INPUT_RESOURCE_TYPE_DIRECTX,
                    width,
                    height,
                    pitch: width * 4,
                    resourceToRegister: texture.as_raw(),
                    bufferFormat: nv::NV_ENC_BUFFER_FORMAT::NV_ENC_BUFFER_FORMAT_ARGB,
                    bufferUsage: nv::NV_ENC_BUFFER_USAGE::NV_ENC_INPUT_IMAGE,
                    ..Default::default()
                };
                if let Err(e) = (api().register_resource)(enc, &mut rr).nv_ok() {
                    let _ = (api().destroy_encoder)(enc);
                    return Err(nvenc_status::call_err("register_resource", e));
                }
                let mut bs = nv::NV_ENC_CREATE_BITSTREAM_BUFFER {
                    version: nv::NV_ENC_CREATE_BITSTREAM_BUFFER_VER,
                    ..Default::default()
                };
                if let Err(e) = (api().create_bitstream_buffer)(enc, &mut bs).nv_ok() {
                    let _ = (api().unregister_resource)(enc, rr.registeredResource);
                    let _ = (api().destroy_encoder)(enc);
                    return Err(nvenc_status::call_err("create_bitstream_buffer", e));
                }
                return Ok(Opened {
                    encoder: enc,
                    texture,
                    registered: rr.registeredResource,
                    bitstream: bs.bitstreamBuffer,
                    rfi,
                    custom_vbv,
                    slices,
                    split_mode,
                    subframe_on: subframe,
                    bitrate_bps: bps,
                });
            }
            Err(e) => {
                let _ = (api().destroy_encoder)(enc);
                last = Some(e);
            }
        }
    }
    Err(last.unwrap_or_else(|| anyhow!("NVENC (Windows) initialize failed")))
}

unsafe fn open_encode_session(device: &ID3D11Device) -> Result<*mut c_void> {
    let mut params = nv::NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS {
        version: nv::NV_ENC_OPEN_ENCODE_SESSION_EX_PARAMS_VER,
        deviceType: nv::NV_ENC_DEVICE_TYPE::NV_ENC_DEVICE_TYPE_DIRECTX,
        device: device.as_raw(),
        apiVersion: nv::NVENCAPI_VERSION,
        ..Default::default()
    };
    let mut enc: *mut c_void = ptr::null_mut();
    if let Err(e) = (api().open_encode_session_ex)(&mut params, &mut enc).nv_ok() {
        if !enc.is_null() {
            let _ = (api().destroy_encoder)(enc);
        }
        return Err(nvenc_status::call_err("open_encode_session_ex", e));
    }
    nvenc_status::note_session_opened();
    Ok(enc)
}

#[allow(clippy::too_many_arguments)]
unsafe fn build_config(
    enc: *mut c_void,
    codec: Codec,
    guid: nv::GUID,
    fps: u32,
    bitrate: u64,
    chroma_444: bool,
    rfi: bool,
    custom_vbv: bool,
    slices: u32,
) -> Result<nv::NV_ENC_CONFIG> {
    let mut preset = nv::NV_ENC_PRESET_CONFIG {
        version: nv::NV_ENC_PRESET_CONFIG_VER,
        presetCfg: nv::NV_ENC_CONFIG {
            version: nv::NV_ENC_CONFIG_VER,
            ..Default::default()
        },
        ..Default::default()
    };
    (api().get_encode_preset_config_ex)(
        enc,
        guid,
        nv::NV_ENC_PRESET_P1_GUID,
        nv::NV_ENC_TUNING_INFO::NV_ENC_TUNING_INFO_ULTRA_LOW_LATENCY,
        &mut preset,
    )
    .nv_ok()
    .map_err(|e| nvenc_status::call_err("get_encode_preset_config_ex", e))?;
    let mut cfg = preset.presetCfg;
    apply_low_latency_config(
        &mut cfg,
        LowLatencyConfig {
            codec,
            bitrate,
            fps,
            custom_vbv,
            chroma_444,
            // BGRA/ARGB is full chroma; 4:4:4 engages only when the session asked for it
            // AND this chip's caps probe said yes (the caller cleared `chroma_444` otherwise).
            full_chroma_input: true,
            bit_depth: 8,
            av1_input_depth_minus8: 0,
            hdr: false,
            rfi_supported: rfi,
            slices,
            vbv_frames: crate::LatencyProfile::current().config().vbv_frames,
        },
    );
    Ok(cfg)
}

#[allow(clippy::too_many_arguments)]
unsafe fn try_initialize(
    enc: *mut c_void,
    codec: Codec,
    guid: nv::GUID,
    width: u32,
    height: u32,
    fps: u32,
    bitrate: u64,
    chroma_444: bool,
    rfi: bool,
    custom_vbv: bool,
    slices: u32,
    split_mode: u32,
    subframe: bool,
) -> Result<()> {
    let mut cfg = build_config(
        enc, codec, guid, fps, bitrate, chroma_444, rfi, custom_vbv, slices,
    )?;
    let mut init = build_init_params(guid, width, height, fps, &mut cfg, split_mode, subframe);
    (api().initialize_encoder)(enc, &mut init)
        .nv_ok()
        .map_err(|e| nvenc_status::call_err("initialize_encoder", e))?;
    Ok(())
}

fn create_input_texture(device: &ID3D11Device, width: u32, height: u32) -> Result<ID3D11Texture2D> {
    let desc = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32,
        CPUAccessFlags: 0,
        MiscFlags: 0,
    };
    let mut tex = None;
    // SAFETY: `desc` is a live value; the out-texture is returned by reference count.
    // Initial data is `None` — `UpdateSubresource` fills it before the first encode.
    unsafe {
        device
            .CreateTexture2D(&desc, None, Some(&mut tex))
            .context("NVENC input texture")?;
    }
    tex.context("CreateTexture2D returned no texture")
}

/// Packed BGRA bytes NVENC's `ARGB` buffer format consumes (memory order B,G,R,A).
fn bgra_bytes<'a>(
    format: PixelFormat,
    width: u32,
    height: u32,
    src: &'a [u8],
    scratch: &'a mut Vec<u8>,
) -> Result<&'a [u8]> {
    let pixels = width as usize * height as usize;
    match format {
        PixelFormat::Bgra | PixelFormat::Bgrx => {
            if src.len() < pixels * 4 {
                bail!("BGRA frame is short ({} bytes for {width}x{height})", src.len());
            }
            Ok(&src[..pixels * 4])
        }
        PixelFormat::Rgba | PixelFormat::Rgbx => {
            if src.len() < pixels * 4 {
                bail!("RGBA frame is short ({} bytes for {width}x{height})", src.len());
            }
            scratch.clear();
            scratch.reserve(pixels * 4);
            for px in src[..pixels * 4].chunks_exact(4) {
                scratch.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
            }
            Ok(scratch.as_slice())
        }
        PixelFormat::Bgr => {
            if src.len() < pixels * 3 {
                bail!("BGR frame is short ({} bytes for {width}x{height})", src.len());
            }
            scratch.clear();
            scratch.reserve(pixels * 4);
            for px in src[..pixels * 3].chunks_exact(3) {
                scratch.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
            Ok(scratch.as_slice())
        }
        PixelFormat::Rgb => {
            if src.len() < pixels * 3 {
                bail!("RGB frame is short ({} bytes for {width}x{height})", src.len());
            }
            scratch.clear();
            scratch.reserve(pixels * 4);
            for px in src[..pixels * 3].chunks_exact(3) {
                scratch.extend_from_slice(&[px[2], px[1], px[0], 255]);
            }
            Ok(scratch.as_slice())
        }
        other => bail!(
            "NVENC (Windows) ingests packed RGB/BGR only; got {other:?} (Windows capture delivers BGRA)"
        ),
    }
}

impl Drop for NvencD3d11Encoder {
    fn drop(&mut self) {
        if self.encoder.is_null() {
            return;
        }
        // Keep the D3D11 device alive for the NVENC session that was opened against it.
        let _device = &self.device;
        // SAFETY: encode thread (or drop after the move). The session, registration, and
        // bitstream were created together in `open_session` and are destroyed exactly once.
        unsafe {
            let _ = (api().unregister_resource)(self.encoder, self.registered);
            let _ = (api().destroy_bitstream_buffer)(self.encoder, self.bitstream);
            if let Err(e) = (api().destroy_encoder)(self.encoder).nv_ok() {
                tracing::warn!(status = ?e, "NVENC (Windows) destroy_encoder failed");
            }
        }
        self.encoder = ptr::null_mut();
    }
}

impl Encoder for NvencD3d11Encoder {
    fn submit(&mut self, frame: &CapturedFrame) -> Result<()> {
        if frame.width != self.width || frame.height != self.height {
            bail!(
                "NVENC (Windows) frame {}x{} does not match the open session {}x{}",
                frame.width,
                frame.height,
                self.width,
                self.height
            );
        }
        // Windows capture publishes `FramePayload::Cpu` only. The match is exhaustive
        // for this target; a future GPU payload has to grow an arm here.
        let FramePayload::Cpu(bytes) = &frame.payload;
        let mut scratch = Vec::new();
        let bgra = bgra_bytes(frame.format, frame.width, frame.height, bytes, &mut scratch)?;
        let pts_ns = frame.pts_ns;
        let timestamp = self.frame_idx as u64;
        self.frame_idx += 1;
        let force = std::mem::take(&mut self.force_kf);
        let anchor = std::mem::take(&mut self.pending_anchor) && !force;
        // SAFETY: `self.context` is the immediate context of `self.device`, used only on
        // this encode thread. `UpdateSubresource` reads `bgra` for the duration of the
        // synchronous call (row pitch = width*4, no padding). The texture was registered
        // with this session. Map/encode/unmap/lock/unlock run to completion before `submit`
        // returns — sync mode (`enableEncodeAsync = 0`), so the bitstream is complete when
        // `lock_bitstream` returns. The mapped input is unmapped before the next submit
        // re-uploads the same texture.
        let au = unsafe {
            let resource: ID3D11Resource = self.texture.cast().context("texture as resource")?;
            self.context.UpdateSubresource(
                &resource,
                0,
                None,
                bgra.as_ptr().cast(),
                self.width * 4,
                0,
            );
            let mut mp = nv::NV_ENC_MAP_INPUT_RESOURCE {
                version: nv::NV_ENC_MAP_INPUT_RESOURCE_VER,
                registeredResource: self.registered,
                ..Default::default()
            };
            (api().map_input_resource)(self.encoder, &mut mp)
                .nv_ok()
                .map_err(|e| nvenc_status::call_err("map_input_resource", e))?;
            let flags = if force {
                nv::NV_ENC_PIC_FLAGS::NV_ENC_PIC_FLAG_FORCEIDR as u32
                    | nv::NV_ENC_PIC_FLAGS::NV_ENC_PIC_FLAG_OUTPUT_SPSPPS as u32
            } else {
                0
            };
            let mut pic = nv::NV_ENC_PIC_PARAMS {
                version: nv::NV_ENC_PIC_PARAMS_VER,
                inputWidth: self.width,
                inputHeight: self.height,
                inputPitch: self.width * 4,
                inputBuffer: mp.mappedResource,
                bufferFmt: nv::NV_ENC_BUFFER_FORMAT::NV_ENC_BUFFER_FORMAT_ARGB,
                outputBitstream: self.bitstream,
                pictureStruct: nv::NV_ENC_PIC_STRUCT::NV_ENC_PIC_STRUCT_FRAME,
                inputTimeStamp: timestamp,
                encodePicFlags: flags,
                ..Default::default()
            };
            let enc_status = (api().encode_picture)(self.encoder, &mut pic);
            let _ = (api().unmap_input_resource)(self.encoder, mp.mappedResource);
            enc_status
                .nv_ok()
                .map_err(|e| nvenc_status::call_err("encode_picture", e))?;
            let mut lock = nv::NV_ENC_LOCK_BITSTREAM {
                version: nv::NV_ENC_LOCK_BITSTREAM_VER,
                outputBitstream: self.bitstream,
                ..Default::default()
            };
            (api().lock_bitstream)(self.encoder, &mut lock)
                .nv_ok()
                .map_err(|e| nvenc_status::call_err("lock_bitstream", e))?;
            let size = lock.bitstreamSizeInBytes as usize;
            let data = if lock.bitstreamBufferPtr.is_null() || size == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(lock.bitstreamBufferPtr.cast::<u8>(), size).to_vec()
            };
            let keyframe = matches!(
                lock.pictureType,
                nv::NV_ENC_PIC_TYPE::NV_ENC_PIC_TYPE_IDR | nv::NV_ENC_PIC_TYPE::NV_ENC_PIC_TYPE_I
            );
            let _ = (api().unlock_bitstream)(self.encoder, self.bitstream);
            EncodedFrame {
                data,
                pts_ns,
                keyframe,
                recovery_anchor: anchor && !keyframe,
                chunk_aligned: false,
            }
        };
        self.pending = Some(au);
        Ok(())
    }

    fn submit_indexed(&mut self, frame: &CapturedFrame, wire_index: u32) -> Result<()> {
        // The next timestamp NVENC records — and RFI names — is the wire frame index.
        self.frame_idx = wire_index as i64;
        self.submit(frame)
    }

    fn caps(&self) -> EncoderCaps {
        EncoderCaps {
            supports_rfi: self.rfi_supported,
            chroma_444: self.chroma_444,
            // WGC/DXGI embed the pointer in the BGRA frame. This backend does not
            // composite `CapturedFrame::cursor` (there is no pointer-free capture yet).
            blends_cursor: false,
            ..EncoderCaps::default()
        }
    }

    fn request_keyframe(&mut self) {
        self.force_kf = true;
    }

    fn reconfigure_bitrate(&mut self, bps: u64) -> bool {
        if self.encoder.is_null() {
            return false;
        }
        // SAFETY: encode thread, between submit and poll (the session loop's contract).
        // `cfg` outlives the synchronous reconfigure whose init params point at it.
        // `resetEncoder`/`forceIDR` stay 0 so the reference chain and wire-index prediction
        // survive an adaptive-bitrate step.
        unsafe {
            let mut cfg = match build_config(
                self.encoder,
                self.codec,
                self.codec_guid,
                self.fps,
                bps,
                self.chroma_444,
                self.rfi_supported,
                self.custom_vbv,
                self.slices,
            ) {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!(
                        error = %format!("{e:#}"),
                        "NVENC (Windows) reconfigure: config re-author failed — falling back to a rebuild"
                    );
                    return false;
                }
            };
            let mut params = nv::NV_ENC_RECONFIGURE_PARAMS {
                version: nv::NV_ENC_RECONFIGURE_PARAMS_VER,
                reInitEncodeParams: build_init_params(
                    self.codec_guid,
                    self.width,
                    self.height,
                    self.fps,
                    &mut cfg,
                    self.split_mode,
                    self.subframe_on,
                ),
                ..Default::default()
            };
            params.set_resetEncoder(0);
            params.set_forceIDR(0);
            match (api().reconfigure_encoder)(self.encoder, &mut params).nv_ok() {
                Ok(()) => {
                    self.bitrate_bps = bps;
                    true
                }
                Err(e) => {
                    tracing::warn!(
                        error = %format!("{:#}", nvenc_status::call_err("reconfigure_encoder", e)),
                        "NVENC (Windows) in-place bitrate change rejected — the session will rebuild"
                    );
                    false
                }
            }
        }
    }

    fn invalidate_ref_frames(&mut self, first: i64, last: i64) -> bool {
        if self.encoder.is_null() || !self.rfi_supported {
            return false;
        }
        match plan_range_recovery(first, last, self.frame_idx, self.last_rfi_range) {
            RangePlan::Covered => {
                self.pending_anchor = true;
                true
            }
            RangePlan::Decline => false,
            RangePlan::Invalidate { first, last } => {
                // SAFETY: `invalidate_ref_frames` is a runtime-table pointer; `self.encoder`
                // is the live session; this runs on the encode thread. The plan clamped
                // each timestamp into the DPB window.
                unsafe {
                    for ts in first..=last {
                        if (api().invalidate_ref_frames)(self.encoder, ts as u64)
                            .nv_ok()
                            .is_err()
                        {
                            return false;
                        }
                    }
                }
                self.last_rfi_range = Some((first, last));
                self.pending_anchor = true;
                true
            }
        }
    }

    fn poll(&mut self) -> Result<Option<EncodedFrame>> {
        Ok(self.pending.take())
    }

    fn flush(&mut self) -> Result<()> {
        // Sync retrieve locks the bitstream inside `submit`, so nothing is buffered in
        // the driver when the caller flushes. `poll` still returns the last AU.
        Ok(())
    }
}

/// Link stubs for the two driver exports the `nvidia-video-codec-sdk` safe wrapper
/// names. The Windows host loads `nvEncodeAPI64.dll` with libloading and never calls
/// these. They return "no encode device" so a stray call fails closed. Without them
/// the GNU (and MSVC) link fails: the crate has no Windows import library, and the
/// unused safe wrapper is still in the rlib.
#[no_mangle]
pub extern "C" fn NvEncodeAPIGetMaxSupportedVersion(version: *mut u32) -> nv::NVENCSTATUS {
    let _ = version;
    nv::NVENCSTATUS::NV_ENC_ERR_NO_ENCODE_DEVICE
}

#[no_mangle]
pub extern "C" fn NvEncodeAPICreateInstance(
    function_list: *mut nv::NV_ENCODE_API_FUNCTION_LIST,
) -> nv::NVENCSTATUS {
    let _ = function_list;
    nv::NVENCSTATUS::NV_ENC_ERR_NO_ENCODE_DEVICE
}

/// Keep the stubs in the rlib. Nothing in Rust calls them; the NVIDIA crate's
/// safe wrapper is what the linker resolves them against.
#[used]
static KEEP_NVENC_GET_VERSION: extern "C" fn(*mut u32) -> nv::NVENCSTATUS =
    NvEncodeAPIGetMaxSupportedVersion;
#[used]
static KEEP_NVENC_CREATE_INSTANCE: extern "C" fn(
    *mut nv::NV_ENCODE_API_FUNCTION_LIST,
) -> nv::NVENCSTATUS = NvEncodeAPICreateInstance;

#[cfg(test)]
mod tests {
    use super::bgra_bytes;
    use ss_frame::PixelFormat;

    #[test]
    fn rgb_expands_to_bgra_memory_order() {
        let mut scratch = Vec::new();
        let out = bgra_bytes(PixelFormat::Rgb, 1, 1, &[1, 2, 3], &mut scratch).unwrap();
        assert_eq!(out, &[3, 2, 1, 255]);
    }

    #[test]
    fn bgra_is_passed_through() {
        let mut scratch = Vec::new();
        let src = [4u8, 5, 6, 7];
        let out = bgra_bytes(PixelFormat::Bgra, 1, 1, &src, &mut scratch).unwrap();
        assert_eq!(out, &src);
    }
}
