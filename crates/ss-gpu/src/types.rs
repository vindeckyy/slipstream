#[cfg(target_os = "linux")]
use std::path::PathBuf;

/// PCI vendor ids of the GPU vendors the encode backends know (NVENC / AMF / QSV, VAAPI on Linux).
pub const VENDOR_NVIDIA: u32 = 0x10DE;
pub const VENDOR_AMD: u32 = 0x1002;
pub const VENDOR_INTEL: u32 = 0x8086;

/// Platform handle of an enumerated GPU — how the pipeline actually addresses it. Not part of the
/// stable identity (Windows LUIDs are per-boot; a render node can renumber across kernel updates).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GpuHandle {
    /// DXGI `AdapterLuid` of this adapter (this boot only).
    #[cfg(target_os = "windows")]
    pub luid_low: u32,
    #[cfg(target_os = "windows")]
    pub luid_high: i32,
    /// DRM render node (`/dev/dri/renderD*`) of this GPU.
    #[cfg(target_os = "linux")]
    pub render_node: Option<PathBuf>,
}

/// One hardware GPU as enumerated on this host.
#[derive(Clone, Debug)]
pub struct GpuInfo {
    /// Stable identifier for the API/UI: `"{vendor:04x}-{device:04x}-{occurrence}"`. Occurrence
    /// disambiguates identical cards (two of the same model) by enumeration order among their
    /// twins — the best available tiebreaker (PCI order), imperfect but honest.
    pub id: String,
    /// Adapter description (Windows) / synthesized vendor label + node (Linux).
    pub name: String,
    pub vendor_id: u32,
    pub device_id: u32,
    /// Index among enumerated GPUs with the same (vendor_id, device_id).
    pub occurrence: u32,
    /// Dedicated VRAM in bytes (0 where the platform doesn't expose it — non-amdgpu Linux sysfs).
    pub vram_bytes: u64,
    pub handle: GpuHandle,
}

/// Lowercase vendor tag for the API (`nvidia` / `amd` / `intel` / `other`).
pub fn vendor_tag(vendor_id: u32) -> &'static str {
    match vendor_id {
        VENDOR_NVIDIA => "nvidia",
        VENDOR_AMD => "amd",
        VENDOR_INTEL => "intel",
        _ => "other",
    }
}

impl GpuInfo {
    /// Lowercase vendor tag for the API (`nvidia` / `amd` / `intel` / `other`).
    pub fn vendor_tag(&self) -> &'static str {
        vendor_tag(self.vendor_id)
    }

    /// The DXGI LUID this adapter had at enumeration time.
    #[cfg(target_os = "windows")]
    pub fn luid(&self) -> windows::Win32::Foundation::LUID {
        windows::Win32::Foundation::LUID {
            LowPart: self.handle.luid_low,
            HighPart: self.handle.luid_high,
        }
    }
}

/// Assign the stable `id` + `occurrence` fields after enumeration (occurrence = index among
/// same-(vendor,device) twins, in inventory order — Windows sorts the inventory by LUID first so
/// twin numbering is stable for the boot, see [`enumerate`]).
// Called only by the Linux/Windows `enumerate()` arms; the stub `enumerate()` on other targets
// (macOS dev host) doesn't, so it's dead there.
#[cfg_attr(not(any(target_os = "linux", target_os = "windows")), allow(dead_code))]
pub(crate) fn assign_ids(gpus: &mut [GpuInfo]) {
    for i in 0..gpus.len() {
        let occ = gpus[..i]
            .iter()
            .filter(|g| g.vendor_id == gpus[i].vendor_id && g.device_id == gpus[i].device_id)
            .count() as u32;
        gpus[i].occurrence = occ;
        gpus[i].id = format!(
            "{:04x}-{:04x}-{}",
            gpus[i].vendor_id, gpus[i].device_id, occ
        );
    }
}
