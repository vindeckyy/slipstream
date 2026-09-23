//! Indirect-display plug for a headless Windows session.
//!
//! Mirror stays the default. `SLIPSTREAM_VIRTUAL_DISPLAY=idd` (also `1`/`on`/`virtual`)
//! asks this backend to plug a monitor through the userspace IOCTL protocol shipped by
//! the RustDesk indirect display driver
//! (`GUID_DEVINTERFACE_IDD_DRIVER_DEVICE` = `{781EF630-72B2-11d2-B852-00C04EAF5272}`):
//! plug a connector, publish the client's mode, and unplug on drop when this session
//! created the head. A missing driver is an error that names the install, not a silent
//! fall back onto someone else's monitor.
//!
//! The driver itself is not vendored. Windows will not load an unsigned indirect-display
//! driver, and the IddCx sample is a KMDF package with its own installer. This module is
//! the host side of that installed driver.

use super::{list_monitors, set_mode};
use crate::{DisplayOwnership, Mode, VirtualDisplay, VirtualOutput};
use anyhow::{bail, Context, Result};
use std::collections::HashSet;
use windows::{
    core::{GUID, PCWSTR},
    Win32::{
        Devices::DeviceAndDriverInstallation::{
            SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW,
            SetupDiGetDeviceInterfaceDetailW, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, HDEVINFO,
            SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W,
        },
        Foundation::{CloseHandle, GetLastError, HANDLE, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        System::IO::DeviceIoControl,
    },
};

/// `{781EF630-72B2-11d2-B852-00C04EAF5272}` — RustDeskIddDriver's device interface.
const IDD_INTERFACE: GUID = GUID::from_u128(0x781E_F630_72B2_11D2_B852_00C0_4EAF_5272);

/// `CTL_CODE(FILE_DEVICE_CHANGER, 0x1001, METHOD_BUFFERED, FILE_READ|FILE_WRITE)`.
const IOCTL_PLUG_IN: u32 = 0x0031_0004;
/// `CTL_CODE(FILE_DEVICE_CHANGER, 0x1002, METHOD_BUFFERED, FILE_READ|FILE_WRITE)`.
const IOCTL_PLUG_OUT: u32 = 0x0031_0008;
/// `CTL_CODE(FILE_DEVICE_CHANGER, 0x1003, METHOD_BUFFERED, FILE_READ|FILE_WRITE)`.
const IOCTL_UPDATE_MODES: u32 = 0x0031_000C;

const GENERIC_READ: u32 = 0x8000_0000;
const GENERIC_WRITE: u32 = 0x4000_0000;

/// Dell S2719DGF EDID profile baked into the RustDesk driver (`MONITOR_EDID_MOD_DELL_S2719DGF`).
const EDID_DELL: u32 = 0;

/// Whether the operator asked for a virtual head instead of mirroring.
pub fn use_virtual_display() -> bool {
    match parse_request(std::env::var("SLIPSTREAM_VIRTUAL_DISPLAY").ok().as_deref()) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "SLIPSTREAM_VIRTUAL_DISPLAY ignored — mirroring a physical head");
            false
        }
    }
}

/// `Ok(true)` = plug an indirect display. `Ok(false)` = mirror. `Err` = the value is unknown.
pub(super) fn parse_request(value: Option<&str>) -> std::result::Result<bool, String> {
    match value.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(false),
        Some(v)
            if matches!(
                v.to_ascii_lowercase().as_str(),
                "0" | "off" | "false" | "no" | "mirror"
            ) =>
        {
            Ok(false)
        }
        Some(v)
            if matches!(
                v.to_ascii_lowercase().as_str(),
                "1" | "on" | "true" | "yes" | "idd" | "virtual"
            ) =>
        {
            Ok(true)
        }
        Some(v) => Err(format!(
            "unknown SLIPSTREAM_VIRTUAL_DISPLAY '{v}' (mirror|idd)"
        )),
    }
}

/// The indirect-display device interface is present (the driver is installed and started).
pub fn driver_present() -> bool {
    device_path().is_some()
}

/// One plugged connector. `owned` is true when this session's plug-in created the head,
/// so drop unplugs it. Adopting a head a previous session left behind does not unplug it.
pub struct WindowsIddDisplay {
    connector: u32,
    device: String,
    owned: bool,
}

impl WindowsIddDisplay {
    pub fn new() -> Result<Self> {
        if !use_virtual_display() {
            bail!("internal: WindowsIddDisplay opened without SLIPSTREAM_VIRTUAL_DISPLAY=idd");
        }
        let path = device_path().context(
            "no indirect display driver (install RustDeskIddDriver and reboot; the host looks \
             for device interface {781EF630-72B2-11d2-B852-00C04EAF5272})",
        )?;
        let before: HashSet<String> = list_monitors()
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.connector)
            .collect();
        // Connectors 0..3 match the sample driver's monitor array. The first plug that
        // produces a new GDI head wins; "already plugged" adopts that head without
        // taking ownership of its lifetime.
        let mut last_err = String::from("driver did not plug a monitor");
        for connector in 0..4u32 {
            let plugged = ioctl_plug_in(&path, connector);
            std::thread::sleep(std::time::Duration::from_millis(200));
            let heads = list_monitors().unwrap_or_default();
            let fresh = heads.iter().find(|m| !before.contains(&m.connector));
            if let Some(head) = fresh {
                tracing::info!(
                    connector,
                    device = %head.connector,
                    "idd: plugged a virtual monitor"
                );
                return Ok(Self {
                    connector,
                    device: head.connector.clone(),
                    owned: true,
                });
            }
            if let Err(e) = plugged {
                last_err = e.to_string();
                continue;
            }
            // Plug-in reported success but GDI has not enumerated a new head yet.
            for _ in 0..10 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if let Some(head) = list_monitors()
                    .unwrap_or_default()
                    .into_iter()
                    .find(|m| !before.contains(&m.connector))
                {
                    return Ok(Self {
                        connector,
                        device: head.connector,
                        owned: true,
                    });
                }
            }
            // This connector is live in the driver but invisible to GDI. Unplug it
            // before trying the next index so a later success does not leave an extra head.
            if let Err(e) = ioctl_plug_out(&path, connector) {
                tracing::warn!(
                    connector,
                    error = %format!("{e:#}"),
                    "idd: could not unplug a connector that never appeared"
                );
            }
            last_err = format!("connector {connector} plugged but no new monitor appeared");
        }
        // Nothing new. If a Dell-EDID head is already attached, adopt it (a previous
        // session leaked the plug). Drop will not unplug a head we did not create.
        if let Some(existing) = list_monitors().unwrap_or_default().into_iter().find(|m| {
            let d = m.description.to_ascii_lowercase();
            d.contains("s2719") || d.contains("dell") || d.contains("rustdesk")
        }) {
            tracing::info!(
                device = %existing.connector,
                "idd: adopting an already-plugged virtual monitor"
            );
            return Ok(Self {
                connector: 0,
                device: existing.connector,
                owned: false,
            });
        }
        bail!("indirect display: {last_err}")
    }
}

impl VirtualDisplay for WindowsIddDisplay {
    fn name(&self) -> &'static str {
        "windows-idd"
    }

    fn create(&mut self, mode: Mode) -> Result<VirtualOutput> {
        let path = device_path().context("indirect display driver disappeared")?;
        let modes = [
            (mode.width, mode.height, mode.refresh_hz),
            (1920, 1080, 60),
            (2560, 1440, 60),
            (3840, 2160, 60),
        ];
        ioctl_modes(&path, self.connector, &modes).context("publish indirect-display modes")?;
        // The driver needs a moment to advertise the mode before GDI will accept it.
        std::thread::sleep(std::time::Duration::from_millis(150));
        set_mode(&self.device, mode.width, mode.height, mode.refresh_hz).with_context(|| {
            format!(
                "set {}x{}@{} on {}",
                mode.width, mode.height, mode.refresh_hz, self.device
            )
        })?;
        let owned = self.owned;
        let connector = self.connector;
        let device = self.device.clone();
        Ok(VirtualOutput {
            node_id: 0,
            preferred_mode: Some((mode.width, mode.height, mode.refresh_hz)),
            keepalive: Box::new(IddKeepalive {
                path,
                connector,
                owned,
            }),
            ownership: DisplayOwnership::External,
            reused_gen: None,
            pool_gen: None,
            windows_head: Some(device),
        })
    }
}

/// Unplugs the connector when this session created it.
struct IddKeepalive {
    path: Vec<u16>,
    connector: u32,
    owned: bool,
}

impl Drop for IddKeepalive {
    fn drop(&mut self) {
        if !self.owned {
            return;
        }
        if let Err(e) = ioctl_plug_out(&self.path, self.connector) {
            tracing::warn!(
                connector = self.connector,
                error = %format!("{e:#}"),
                "idd: could not unplug the virtual monitor"
            );
        } else {
            tracing::info!(
                connector = self.connector,
                "idd: unplugged the virtual monitor"
            );
        }
    }
}

fn device_path() -> Option<Vec<u16>> {
    // SAFETY: SetupAPI enumeration. `info` is destroyed before return. The detail buffer
    // is a live local; `cb_size` is 8, which is what SetupAPI requires for this struct
    // on 64-bit (offset of the flexible path). The path is copied out before the buffer
    // drops.
    unsafe {
        let info = setup_info_set()?;
        let mut iface = SP_DEVICE_INTERFACE_DATA {
            cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };
        if SetupDiEnumDeviceInterfaces(info.0, None, &IDD_INTERFACE, 0, &mut iface).is_err() {
            return None;
        }
        let mut detail = DetailPath {
            cb_size: 8,
            path: [0; 512],
        };
        if SetupDiGetDeviceInterfaceDetailW(
            info.0,
            &iface,
            Some(std::ptr::from_mut(&mut detail).cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>()),
            std::mem::size_of::<DetailPath>() as u32,
            None,
            None,
        )
        .is_err()
        {
            return None;
        }
        let end = detail
            .path
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(detail.path.len());
        if end == 0 {
            return None;
        }
        let mut path = detail.path[..end].to_vec();
        path.push(0);
        Some(path)
    }
}

struct InfoSet(HDEVINFO);
impl Drop for InfoSet {
    fn drop(&mut self) {
        // SAFETY: `0` is the set `SetupDiGetClassDevsW` returned, destroyed once.
        unsafe {
            let _ = SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}

fn setup_info_set() -> Option<InfoSet> {
    let flags = DIGCF_PRESENT | DIGCF_DEVICEINTERFACE;
    // SAFETY: no borrowed memory. The returned `HDEVINFO` is owned by `InfoSet`, which
    // destroys it on drop. A null enumerator lists every device exposing the interface.
    let info = unsafe { SetupDiGetClassDevsW(Some(&IDD_INTERFACE), None, None, flags).ok()? };
    Some(InfoSet(info))
}

#[repr(C)]
struct DetailPath {
    cb_size: u32,
    path: [u16; 512],
}

fn ioctl_plug_in(path: &[u16], connector: u32) -> Result<()> {
    #[repr(C)]
    struct PlugIn {
        connector_index: u32,
        monitor_edid: u32,
        container_id: GUID,
    }
    let body = PlugIn {
        connector_index: connector,
        monitor_edid: EDID_DELL,
        container_id: GUID::new().context("CoCreateGuid")?,
    };
    device_io(path, IOCTL_PLUG_IN, &body)
}

fn ioctl_plug_out(path: &[u16], connector: u32) -> Result<()> {
    #[repr(C)]
    struct PlugOut {
        connector_index: u32,
    }
    device_io(
        path,
        IOCTL_PLUG_OUT,
        &PlugOut {
            connector_index: connector,
        },
    )
}

fn ioctl_modes(path: &[u16], connector: u32, modes: &[(u32, u32, u32)]) -> Result<()> {
    let mut buf = Vec::with_capacity(8 + modes.len() * 12);
    buf.extend_from_slice(&connector.to_ne_bytes());
    buf.extend_from_slice(&(modes.len() as u32).to_ne_bytes());
    for &(w, h, hz) in modes {
        buf.extend_from_slice(&w.to_ne_bytes());
        buf.extend_from_slice(&h.to_ne_bytes());
        buf.extend_from_slice(&hz.to_ne_bytes());
    }
    device_io_bytes(path, IOCTL_UPDATE_MODES, &buf)
}

fn device_io<T>(path: &[u16], code: u32, body: &T) -> Result<()> {
    // SAFETY: `body` is a live reference for this call. The slice covers exactly `T`'s
    // bytes and is not mutated; `device_io_bytes` only reads it for the ioctl.
    let bytes = unsafe {
        std::slice::from_raw_parts((body as *const T).cast::<u8>(), std::mem::size_of::<T>())
    };
    device_io_bytes(path, code, bytes)
}

fn device_io_bytes(path: &[u16], code: u32, body: &[u8]) -> Result<()> {
    // SAFETY: `path` is NUL-terminated. The file handle is closed before return.
    // `DeviceIoControl` reads `body` for the duration of the synchronous call.
    unsafe {
        let handle = CreateFileW(
            PCWSTR(path.as_ptr()),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
        .with_context(|| format!("CreateFile indirect display ({})", os_err()))?;
        if handle == INVALID_HANDLE_VALUE {
            bail!("CreateFile indirect display returned an invalid handle");
        }
        let _guard = HandleGuard(handle);
        let mut returned = 0u32;
        DeviceIoControl(
            handle,
            code,
            Some(body.as_ptr().cast()),
            body.len() as u32,
            None,
            0,
            Some(&mut returned),
            None,
        )
        .ok()
        .with_context(|| format!("DeviceIoControl {code:#x} ({})", os_err()))?;
        Ok(())
    }
}

fn os_err() -> u32 {
    // SAFETY: reads the calling thread's last-error code; no pointers.
    unsafe { GetLastError().0 }
}

struct HandleGuard(HANDLE);
impl Drop for HandleGuard {
    fn drop(&mut self) {
        // SAFETY: the handle was opened in `device_io_bytes` and is closed once.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_request, IOCTL_PLUG_IN, IOCTL_PLUG_OUT, IOCTL_UPDATE_MODES};

    #[test]
    fn virtual_display_request_parses() {
        assert_eq!(parse_request(None).unwrap(), false);
        assert_eq!(parse_request(Some("")).unwrap(), false);
        assert_eq!(parse_request(Some("mirror")).unwrap(), false);
        assert_eq!(parse_request(Some("0")).unwrap(), false);
        assert_eq!(parse_request(Some("idd")).unwrap(), true);
        assert_eq!(parse_request(Some("VIRTUAL")).unwrap(), true);
        assert!(parse_request(Some("gamescope")).is_err());
    }

    #[test]
    fn idd_ioctl_codes_match_the_rustdesk_driver() {
        // CTL_CODE(FILE_DEVICE_CHANGER=0x30, fn, METHOD_BUFFERED=0, read|write=3).
        assert_eq!(IOCTL_PLUG_IN, 0x0031_0004);
        assert_eq!(IOCTL_PLUG_OUT, 0x0031_0008);
        assert_eq!(IOCTL_UPDATE_MODES, 0x0031_000C);
    }
}
