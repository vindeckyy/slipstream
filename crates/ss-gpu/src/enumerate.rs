#[cfg(target_os = "linux")]
use crate::types::{assign_ids, GpuHandle, GpuInfo, VENDOR_AMD, VENDOR_INTEL, VENDOR_NVIDIA};
#[cfg(target_os = "linux")]
use std::path::PathBuf;

/// Enumerate Linux render nodes and their PCI ids from sysfs.
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
                dxgi_luid: None,
            },
        });
    }
    assign_ids(&mut out);
    out
}

/// Enumerate GPUs on non-Linux hosts.
///
/// Windows: DXGI adapter enumeration (vendor/device id, dedicated VRAM, LUID) — the
/// same inventory shape as Linux, so selection, preference matching, and the console's
/// "in use" display work unchanged. Other platforms: empty inventory (software path).
#[cfg(target_os = "windows")]
pub fn enumerate() -> Vec<crate::types::GpuInfo> {
    let mut out = Vec::new();
    // SAFETY: factory/adapter enumeration takes no raw memory — every out-param is a
    // live local, and adapters are reference-counted. `GetDesc` returns its struct by
    // value; each adapter outlives its desc. No frame or device is created.
    unsafe {
        let Ok(factory) = windows::Win32::Graphics::Dxgi::CreateDXGIFactory1::<
            windows::Win32::Graphics::Dxgi::IDXGIFactory1,
        >() else {
            return out;
        };
        let mut index = 0u32;
        while let Ok(adapter) = factory.EnumAdapters1(index) {
            // Software (WARP) adapters report vendor 0x1414 — skip them so auto-select
            // never prefers a rasterizer over real hardware.
            if let Ok(desc) = adapter.GetDesc() {
                if desc.VendorId != 0x1414 && desc.VendorId != 0 {
                    out.push(crate::types::GpuInfo {
                        id: String::new(),
                        name: String::from_utf16_lossy(&desc.Description)
                            .trim_end_matches('\0')
                            .to_string(),
                        vendor_id: desc.VendorId,
                        device_id: desc.DeviceId,
                        occurrence: 0,
                        vram_bytes: desc.DedicatedVideoMemory as u64,
                        handle: crate::types::GpuHandle {
                            render_node: None,
                            dxgi_luid: Some(
                                ((desc.AdapterLuid.HighPart as u64) << 32)
                                    | desc.AdapterLuid.LowPart as u64,
                            ),
                        },
                    });
                }
            }
            index += 1;
        }
    }
    crate::types::assign_ids(&mut out);
    out
}

/// Enumerate GPUs on hosts with no backend yet: empty inventory (software path).
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn enumerate() -> Vec<crate::types::GpuInfo> {
    Vec::new()
}
