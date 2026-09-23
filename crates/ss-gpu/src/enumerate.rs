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
            },
        });
    }
    assign_ids(&mut out);
    out
}

/// Enumerate GPUs on non-Linux hosts.
///
/// Baseline returns an empty inventory so selection degrades to `None` and the host
/// takes the software-encode path. The Windows DXGI adapter enumeration (vendor/device
/// id, VRAM, LUID) lands with the platform todo; it fills this in without changing callers.
#[cfg(not(target_os = "linux"))]
pub fn enumerate() -> Vec<crate::types::GpuInfo> {
    let mut out = Vec::new();
    // Keep the post-enumeration invariant (stable ids assigned once) so the DXGI
    // implementation inherits it by construction rather than rediscovering it.
    crate::types::assign_ids(&mut out);
    out
}
