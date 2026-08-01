use crate::types::{assign_ids, GpuHandle, GpuInfo};
#[cfg(target_os = "linux")]
use crate::types::{VENDOR_AMD, VENDOR_INTEL, VENDOR_NVIDIA};
#[cfg(target_os = "linux")]
use std::path::PathBuf;

/// Enumerate this host's hardware GPUs. Windows: DXGI adapters minus WARP/Basic-Render (the same
/// filter `win_adapter` always applied) and minus indirect-display/display-only adapters (see the
/// ghost-twin filter inside). Linux: `/dev/dri/renderD*` with PCI ids from sysfs.
/// Other platforms (the macOS dev/test host build): empty — the endpoints still exist, they just
/// report no GPUs.
#[cfg(target_os = "windows")]
pub fn enumerate() -> Vec<GpuInfo> {
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE,
    };
    let mut out = Vec::new();
    // SAFETY: `CreateDXGIFactory1` returns an owned COM factory (Released when the local drops) or
    // errs (→ empty inventory). `EnumAdapters1(i)` yields owned `IDXGIAdapter1`s until it errors
    // past the last adapter; `GetDesc1()` returns the descriptor by value. Only locals are touched,
    // nothing escapes, no raw pointer is dereferenced.
    unsafe {
        let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() else {
            return out;
        };
        let mut i = 0u32;
        while let Ok(adapter) = factory.EnumAdapters1(i) {
            i += 1;
            let Ok(d) = adapter.GetDesc1() else { continue };
            if (d.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32) != 0 {
                continue; // Microsoft Basic Render / WARP
            }
            let name = String::from_utf16_lossy(&d.Description)
                .trim_end_matches('\u{0}')
                .to_string();
            let lname = name.to_ascii_lowercase();
            if lname.contains("basic render") || lname.contains("warp") {
                continue;
            }
            // ss-vdisplay's IddCx adapter enumerates as a perfect DXGI twin of the GPU it renders
            // on — same Description, VendorId/DeviceId/SubSysId, even DedicatedVideoMemory; only
            // the LUID differs (verified on an RTX 4090 host). Left in, every slipstream host
            // lists each render GPU twice: the picker offers a dead twin (display-only — D3D11
            // device creation on it fails) and auto-select's max-VRAM tie between the twins is
            // settled by enumeration order. The kernel adapter type is the only field that
            // separates them, so drop indirect-display (any vendor's virtual display, ours
            // included), display-only, and software adapters here. A failed query keeps the
            // adapter (fail open).
            match crate::kmt::adapter_type_bits(d.AdapterLuid.LowPart, d.AdapterLuid.HighPart) {
                Some(bits) if crate::adapter_type::hidden(bits) => {
                    tracing::debug!(
                        name = %name,
                        bits = %format!("{bits:#x}"),
                        "skipping non-render DXGI adapter (indirect display / display-only / software)"
                    );
                    continue;
                }
                _ => {}
            }
            out.push(GpuInfo {
                id: String::new(),
                name,
                vendor_id: d.VendorId,
                device_id: d.DeviceId,
                occurrence: 0,
                vram_bytes: d.DedicatedVideoMemory as u64,
                handle: GpuHandle {
                    luid_low: d.AdapterLuid.LowPart,
                    luid_high: d.AdapterLuid.HighPart,
                },
            });
        }
    }
    // Deterministic inventory order. DXGI enumerates the adapter owning the primary display first,
    // and the vdisplay flow itself MOVES the primary mid-session (exclusive mode turns the physical
    // screens off) — which flipped every order-relative selection between IDENTICAL twin GPUs from
    // one call to the next (the max-VRAM tie, the env-substring first match, occurrence numbering):
    // monitor ADD pinned the driver to one twin, the ring open picked the other → TEX_FAIL. LUIDs
    // are creation-ordered and stable for the boot, so sorting by them makes selection stable too.
    out.sort_by_key(|g| ((g.handle.luid_high as i64) << 32) | g.handle.luid_low as i64);
    assign_ids(&mut out);
    out
}

#[cfg(target_os = "linux")]
pub fn enumerate() -> Vec<GpuInfo> {
    let mut nodes: Vec<String> = std::fs::read_dir("/dev/dri")
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.starts_with("renderD"))
                .collect()
        })
        .unwrap_or_default();
    nodes.sort();
    let mut out = Vec::new();
    for node in nodes {
        let sys = format!("/sys/class/drm/{node}/device");
        let read_hex = |f: &str| -> u32 {
            std::fs::read_to_string(format!("{sys}/{f}"))
                .ok()
                .and_then(|s| u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok())
                .unwrap_or(0)
        };
        let vendor_id = read_hex("vendor");
        let device_id = read_hex("device");
        // Only amdgpu exposes a VRAM total in sysfs; 0 elsewhere (display-only — Linux auto
        // selection is NVIDIA-presence + render node, not VRAM).
        let vram_bytes = std::fs::read_to_string(format!("{sys}/mem_info_vram_total"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);
        let vendor_label = match vendor_id {
            VENDOR_NVIDIA => "NVIDIA".to_string(),
            VENDOR_AMD => "AMD".to_string(),
            VENDOR_INTEL => "Intel".to_string(),
            other => format!("GPU 0x{other:04x}"),
        };
        out.push(GpuInfo {
            id: String::new(),
            name: format!("{vendor_label} GPU ({node})"),
            vendor_id,
            device_id,
            occurrence: 0,
            vram_bytes,
            handle: GpuHandle {
                render_node: Some(PathBuf::from(format!("/dev/dri/{node}"))),
            },
        });
    }
    assign_ids(&mut out);
    out
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
pub fn enumerate() -> Vec<GpuInfo> {
    Vec::new()
}
