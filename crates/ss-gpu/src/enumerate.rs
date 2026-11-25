#[cfg(target_os = "linux")]
use crate::types::{assign_ids, GpuHandle, GpuInfo, VENDOR_AMD, VENDOR_INTEL, VENDOR_NVIDIA};
use std::collections::HashMap;
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
            // sysfs device symlink resolves to the PCI slot (e.g. 0000:01:00.0). Two render
            // nodes on one physical card share it; twin cards have different slots.
            pci_identity: std::fs::canonicalize(&sys)
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())),
        });
    }
    // One physical card can expose several render nodes; `pci_identity` is the PCI slot, so two
    // nodes that resolve to the same slot are the same card and must not both appear. The count
    // map is the Windows PnP fallback; `physical_device_counts` is empty on this platform, so the
    // identity check carries the whole decision.
    out = dedupe_to_physical_cards(out, &physical_device_counts());
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
                        // PCI bus/device/function is the physical slot identity; the DXGI LUID is
                        // an adapter-instance handle and must NOT be used to distinguish cards.
                        // Leaving identity unset on Windows keeps the registry-count fallback,
                        // which is the source of truth for multi-card hosts there.
                        pci_identity: None,
                    });
                }
            }
            index += 1;
        }
    }
    // DXGI enumerates *adapter instances*, not physical cards: this desk reports its single
    // RTX 3090 Ti three times (three LUIDs — one with a display output, two headless siblings)
    // while PnP knows exactly one PCI device with that vendor/device id. Nothing in the DXGI
    // descriptor separates the siblings, so the real card count comes from the PCI device
    // tree. Capping per (vendor, device) by that count keeps genuinely distinct cards — a
    // second physical 3090 Ti has its own PCI devnode and therefore its own allowance.
    out = dedupe_to_physical_cards(out, &physical_device_counts());
    crate::types::assign_ids(&mut out);
    out
}

/// Collapse adapters that one physical card is reported as more than once.
///
/// Identity is the PCI bus/slot (or DXGI LUID on Windows), never enumeration position: duplicate
/// instances of card A must not consume the allowance that belongs to a physically distinct card B.
/// `physical_devices` remains a secondary cap for backends that cannot provide identity. An
/// unknown model, absent identity, or partial PCI data is never grounds to drop an adapter:
/// showing a duplicate is safer than hiding a real card.
pub(crate) fn dedupe_to_physical_cards(
    gpus: Vec<crate::types::GpuInfo>,
    physical_devices: &HashMap<(u32, u32), usize>,
) -> Vec<crate::types::GpuInfo> {
    let mut kept: Vec<crate::types::GpuInfo> = Vec::with_capacity(gpus.len());
    let mut seen_identity: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut used: HashMap<(u32, u32), usize> = HashMap::new();
    for gpu in gpus {
        let key = (gpu.vendor_id, gpu.device_id);
        // Identity-based dedup takes precedence over the count fallback. A partial PCI count map
        // only disables a card whose identity is unknown; an identified card always survives.
        if let Some(identity) = &gpu.pci_identity {
            if !seen_identity.insert(identity.clone()) {
                tracing::debug!(
                    name = %gpu.name,
                    identity,
                    "dropping duplicate adapter instance for the same physical GPU"
                );
                continue;
            }
            kept.push(gpu);
            continue;
        }
        // No physical identity: fall back to the registry count, but only when the count is a
        // complete answer for this model. A missing/zero partial count must not disable a card.
        let Some(&limit) = physical_devices.get(&key).filter(|&&n| n > 0) else {
            kept.push(gpu);
            continue;
        };
        let seen = used.entry(key).or_default();
        if *seen < limit {
            *seen += 1;
            kept.push(gpu);
        } else {
            tracing::debug!(
                name = %gpu.name,
                luid = ?gpu.handle.dxgi_luid,
                "dropping duplicate DXGI adapter instance for one physical GPU"
            );
        }
    }
    kept
}

/// Count the PCI display devices the OS has per `(vendor_id, device_id)`.
///
/// Reads the PCI enum tree (`HKLM\SYSTEM\CurrentControlSet\Enum\PCI`): every hardware id key is
/// `VEN_xxxx&DEV_xxxx&SUBSYS_...&REV_...`, and each child key is one device instance (one
/// physical card). A genuine twin adds a second instance. Any failure yields an empty map, which
/// disables dedupe rather than guessing: a *partial* count would cap a model at the instances that
/// happened to be readable and drop a physically distinct card whose sibling key failed to open.
#[cfg(target_os = "windows")]
fn physical_device_counts() -> HashMap<(u32, u32), usize> {
    use windows::core::PWSTR;
    use windows::Win32::Foundation::ERROR_NO_MORE_ITEMS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegEnumKeyExW, RegOpenKeyExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ,
    };

    fn parse_pci_ids(name: &str) -> Option<(u32, u32)> {
        let field = |tag: &str| -> Option<u32> {
            let start = name.find(tag)? + tag.len();
            let hex = name.get(start..start + 4)?;
            u32::from_str_radix(hex, 16).ok()
        };
        Some((field("VEN_")?, field("DEV_")?))
    }

    /// Visit every subkey of `key` in registry order.
    ///
    /// `false` means the walk did not finish: only `ERROR_NO_MORE_ITEMS` is a normal end, and
    /// anything else (a key removed mid-walk, access denied) leaves a partial list the caller must
    /// not treat as complete.
    fn enum_subkeys(key: HKEY, mut visit: impl FnMut(&str)) -> bool {
        let mut index = 0u32;
        loop {
            let mut name = [0u16; 256];
            let mut len = name.len() as u32;
            // SAFETY: `name` is a live 256-UTF-16 buffer and `len` its length in code units;
            // the API only writes into them within those bounds. No class/time out-params.
            let rc = unsafe {
                RegEnumKeyExW(
                    key,
                    index,
                    PWSTR(name.as_mut_ptr()),
                    &mut len,
                    None,
                    PWSTR::null(),
                    None,
                    None,
                )
            };
            if rc == ERROR_NO_MORE_ITEMS {
                return true;
            }
            if rc.is_err() {
                return false;
            }
            visit(&String::from_utf16_lossy(&name[..len as usize]));
            index += 1;
        }
    }

    /// Device instances per `(vendor_id, device_id)` under the opened PCI enum key.
    ///
    /// `None` means the tree could not be read completely; the caller then disables dedupe instead
    /// of capping a model at a count it only partly observed.
    fn collect_counts(root: HKEY) -> Option<HashMap<(u32, u32), usize>> {
        let mut hardware_ids = Vec::new();
        if !enum_subkeys(root, |name| {
            if let Some(ids) = parse_pci_ids(name) {
                hardware_ids.push((ids, name.to_owned()));
            }
        }) {
            return None;
        }
        let mut counts: HashMap<(u32, u32), usize> = HashMap::new();
        for (ids, name) in hardware_ids {
            let mut sub = HKEY::default();
            let mut wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
            // SAFETY: `wide` is NUL-terminated for the duration of the call; `sub` is a live
            // out-param and is closed below on the path that opened it.
            let rc =
                unsafe { RegOpenKeyExW(root, PWSTR(wide.as_mut_ptr()), 0, KEY_READ, &mut sub) };
            if rc.is_err() {
                return None;
            }
            let mut instances = 0usize;
            let complete = enum_subkeys(sub, |_| instances += 1);
            // SAFETY: `sub` came from a successful RegOpenKeyExW and is closed exactly once.
            unsafe {
                let _ = RegCloseKey(sub);
            }
            if !complete {
                return None;
            }
            if instances > 0 {
                *counts.entry(ids).or_default() += instances;
            }
        }
        Some(counts)
    }

    let mut counts = HashMap::new();
    let mut root = HKEY::default();
    // SAFETY: opening a well-known subtree read-only with a live out-param; the handle is
    // closed below on every path that opened it.
    let opened = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            windows::core::w!("SYSTEM\\CurrentControlSet\\Enum\\PCI"),
            0,
            KEY_READ,
            &mut root,
        )
    };
    if opened.is_err() {
        tracing::debug!("PCI enum tree unavailable — GPU inventory left undeduped");
        return counts;
    }
    match collect_counts(root) {
        Some(complete) => counts = complete,
        // Fail closed: an incomplete tree cannot be told apart from a machine that genuinely has
        // fewer cards, and trusting it would silently hide a real GPU from selection.
        None => {
            tracing::warn!(
                "PCI device tree could not be read completely — GPU inventory left undeduped"
            );
        }
    }
    // SAFETY: `root` came from a successful RegOpenKeyExW above; closed exactly once.
    unsafe {
        let _ = RegCloseKey(root);
    }
    counts
}

#[cfg(not(target_os = "windows"))]
fn physical_device_counts() -> HashMap<(u32, u32), usize> {
    HashMap::new()
}

/// Enumerate GPUs on hosts with no backend yet: empty inventory (software path).
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn enumerate() -> Vec<crate::types::GpuInfo> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{GpuHandle, GpuInfo, VENDOR_INTEL, VENDOR_NVIDIA};

    fn adapter(vendor: u32, device: u32, name: &str, luid: u64) -> GpuInfo {
        GpuInfo {
            id: String::new(),
            name: name.into(),
            vendor_id: vendor,
            device_id: device,
            occurrence: 0,
            vram_bytes: 24 * 1024 * 1024 * 1024,
            handle: GpuHandle {
                render_node: None,
                dxgi_luid: Some(luid),
            },
            pci_identity: None,
        }
    }

    fn counts(entries: &[(u32, u32, usize)]) -> HashMap<(u32, u32), usize> {
        entries.iter().map(|&(v, d, n)| ((v, d), n)).collect()
    }

    /// The live Windows shape: one physical RTX 3090 Ti that DXGI reports three times (three
    /// LUIDs, identical descriptors). PnP knows one PCI device, so the inventory must hold one.
    #[test]
    fn one_card_reported_by_several_adapters_collapses_to_one() {
        let gpus = vec![
            adapter(VENDOR_NVIDIA, 0x2203, "NVIDIA GeForce RTX 3090 Ti", 123143),
            adapter(VENDOR_NVIDIA, 0x2203, "NVIDIA GeForce RTX 3090 Ti", 197186),
            adapter(VENDOR_NVIDIA, 0x2203, "NVIDIA GeForce RTX 3090 Ti", 187063),
        ];
        let out = dedupe_to_physical_cards(gpus, &counts(&[(VENDOR_NVIDIA, 0x2203, 1)]));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].handle.dxgi_luid, Some(123143));
    }

    /// The guard against over-deduping: a hybrid box (two different cards) keeps both, and two
    /// physically identical cards (same vendor/device, two PCI devices) keep both entries.
    #[test]
    fn genuinely_distinct_cards_are_all_kept() {
        let hybrid = vec![
            adapter(VENDOR_NVIDIA, 0x2203, "NVIDIA GeForce RTX 3090 Ti", 11),
            adapter(VENDOR_INTEL, 0x7d55, "Intel(R) Arc(TM) Graphics", 22),
        ];
        let out = dedupe_to_physical_cards(
            hybrid,
            &counts(&[(VENDOR_NVIDIA, 0x2203, 1), (VENDOR_INTEL, 0x7d55, 1)]),
        );
        assert_eq!(out.len(), 2);

        // Two real 3090 Tis: identical descriptors, but the OS reports two PCI devices.
        let twins = vec![
            adapter(VENDOR_NVIDIA, 0x2203, "NVIDIA GeForce RTX 3090 Ti", 11),
            adapter(VENDOR_NVIDIA, 0x2203, "NVIDIA GeForce RTX 3090 Ti", 22),
        ];
        let out = dedupe_to_physical_cards(twins, &counts(&[(VENDOR_NVIDIA, 0x2203, 2)]));
        assert_eq!(out.len(), 2);
    }

    /// A model the PCI tree does not explain is never collapsed: keeping a phantom is better
    /// than hiding a real adapter when the registry answer is missing.
    #[test]
    fn unknown_model_is_never_deduped() {
        let gpus = vec![
            adapter(VENDOR_NVIDIA, 0x1234, "Mystery GPU", 11),
            adapter(VENDOR_NVIDIA, 0x1234, "Mystery GPU", 22),
        ];
        assert_eq!(dedupe_to_physical_cards(gpus, &counts(&[])).len(), 2);
    }

    /// Card A appears three times (same PCI slot) and card B once (different slot). PCI identity
    /// must decide: A's duplicates cannot consume B's allowance even with a partial count map.
    #[test]
    fn pci_identity_dedups_per_slot_not_by_enumeration_position() {
        let mut a1 = adapter(VENDOR_NVIDIA, 0x2203, "Twin A", 11);
        let mut a2 = adapter(VENDOR_NVIDIA, 0x2203, "Twin A", 22);
        let mut a3 = adapter(VENDOR_NVIDIA, 0x2203, "Twin A", 33);
        let mut b1 = adapter(VENDOR_NVIDIA, 0x2203, "Twin B", 44);
        for (gpu, slot) in [
            (&mut a1, "0000:01:00.0"),
            (&mut a2, "0000:01:00.0"),
            (&mut a3, "0000:01:00.0"),
            (&mut b1, "0000:02:00.0"),
        ] {
            gpu.pci_identity = Some(slot.into());
        }
        // Partial PCI count: only one device counted for this model, but B is a real second card.
        let out =
            dedupe_to_physical_cards(vec![a1, a2, a3, b1], &counts(&[(VENDOR_NVIDIA, 0x2203, 1)]));
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].handle.dxgi_luid, Some(11));
        assert_eq!(out[1].handle.dxgi_luid, Some(44));
    }

    /// A card without physical identity must survive when PCI enumeration failed entirely: a
    /// missing registry answer must never hide a valid adapter.
    #[test]
    fn missing_pci_count_keeps_valid_card() {
        let gpus = vec![
            adapter(VENDOR_INTEL, 0x7d55, "Intel Graphics", 11),
            adapter(VENDOR_INTEL, 0x7d55, "Intel Graphics", 22),
        ];
        let out = dedupe_to_physical_cards(gpus, &counts(&[]));
        assert_eq!(
            out.len(),
            2,
            "a missing PCI count must not disable a valid card"
        );
    }
}
