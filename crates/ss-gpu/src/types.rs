use std::path::PathBuf;

/// PCI vendor ids of the GPU vendors the encode backends know.
pub const VENDOR_NVIDIA: u32 = 0x10DE;
pub const VENDOR_AMD: u32 = 0x1002;
pub const VENDOR_INTEL: u32 = 0x8086;

/// Linux render node used to address an enumerated GPU.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GpuHandle {
    pub render_node: Option<PathBuf>,
}

/// One hardware GPU as enumerated on this host.
#[derive(Clone, Debug)]
pub struct GpuInfo {
    /// Stable identifier for the API/UI: `"{vendor:04x}-{device:04x}-{occurrence}"`.
    pub id: String,
    pub name: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub occurrence: u32,
    pub vram_bytes: u64,
    pub handle: GpuHandle,
}

/// Lowercase vendor tag for the API.
pub fn vendor_tag(vendor_id: u32) -> &'static str {
    match vendor_id {
        VENDOR_NVIDIA => "nvidia",
        VENDOR_AMD => "amd",
        VENDOR_INTEL => "intel",
        _ => "other",
    }
}

impl GpuInfo {
    pub fn vendor_tag(&self) -> &'static str {
        vendor_tag(self.vendor_id)
    }
}

/// Assign stable ids and occurrence numbers after enumeration.
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
