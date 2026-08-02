//! A virtual Linux touchscreen for compositors whose EIS session has no touch device.
//!
//! This uses the kernel's multi-touch Protocol B event model. The device is deliberately
//! independent of a compositor protocol: Mutter sees it through its normal libinput seat, while
//! the libei sender remains responsible for pointer and keyboard events.
//!
//! The event shape follows the Linux input documentation and the MIT-licensed inputtino touchscreen
//! design used by Solar-Flare. This is an independent Rust implementation.

use anyhow::{bail, Result};
use slipstream_core::input::{InputEvent, InputKind};
use std::os::fd::{AsRawFd, OwnedFd};

const UI_DEV_CREATE: libc::c_ulong = 0x5501;
const UI_DEV_DESTROY: libc::c_ulong = 0x5502;
const UI_DEV_SETUP: libc::c_ulong = 0x405c_5503;
const UI_ABS_SETUP: libc::c_ulong = 0x401c_5504;
const UI_SET_EVBIT: libc::c_ulong = 0x4004_5564;
const UI_SET_KEYBIT: libc::c_ulong = 0x4004_5565;
const UI_SET_PROPBIT: libc::c_ulong = 0x4004_556e;

const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const SYN_REPORT: u16 = 0;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_PRESSURE: u16 = 0x18;
const ABS_MT_SLOT: u16 = 0x2f;
const ABS_MT_POSITION_X: u16 = 0x35;
const ABS_MT_POSITION_Y: u16 = 0x36;
const ABS_MT_TRACKING_ID: u16 = 0x39;
const ABS_MT_PRESSURE: u16 = 0x3a;
const ABS_MT_ORIENTATION: u16 = 0x34;
const BTN_TOUCH: u16 = 0x14a;
const INPUT_PROP_DIRECT: libc::c_int = 0x01;

const ABS_MAX: i32 = 65535;
const PRESSURE_MAX: i32 = 253;
const MAX_CONTACTS: usize = 16;

#[repr(C)]
struct InputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

#[repr(C)]
struct UinputSetup {
    id: InputId,
    name: [u8; 80],
    ff_effects_max: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct AbsInfo {
    value: i32,
    minimum: i32,
    maximum: i32,
    fuzz: i32,
    flat: i32,
    resolution: i32,
}

#[repr(C)]
struct UinputAbsSetup {
    code: u16,
    _pad: u16,
    absinfo: AbsInfo,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct InputEventRaw {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

fn ioctl_int(fd: i32, req: libc::c_ulong, arg: libc::c_int, what: &str) -> Result<()> {
    // SAFETY: the request takes a plain integer and `fd` is the live uinput descriptor owned by
    // the caller. No pointer is passed to the kernel.
    if unsafe { libc::ioctl(fd, req, arg) } < 0 {
        bail!("{what}: {}", std::io::Error::last_os_error());
    }
    Ok(())
}

fn ioctl_ptr<T>(fd: i32, req: libc::c_ulong, arg: *mut T, what: &str) -> Result<()> {
    // SAFETY: callers pass a live repr(C) value matching UI_DEV_SETUP or UI_ABS_SETUP. The kernel
    // reads it during the ioctl and retains no pointer.
    if unsafe { libc::ioctl(fd, req, arg) } < 0 {
        bail!("{what}: {}", std::io::Error::last_os_error());
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Default)]
struct TouchSlots {
    ids: [Option<u32>; MAX_CONTACTS],
}

impl TouchSlots {
    fn new() -> Self {
        Self::default()
    }

    fn down(&mut self, id: u32) -> Option<usize> {
        if let Some(slot) = self.motion(id) {
            return Some(slot);
        }
        let slot = self.ids.iter().position(Option::is_none)?;
        self.ids[slot] = Some(id);
        Some(slot)
    }

    fn motion(&self, id: u32) -> Option<usize> {
        self.ids.iter().position(|current| *current == Some(id))
    }

    fn up(&mut self, id: u32) -> Option<usize> {
        let slot = self.motion(id)?;
        self.ids[slot] = None;
        Some(slot)
    }

    fn active_count(&self) -> usize {
        self.ids.iter().filter(|id| id.is_some()).count()
    }
}

/// Convert a client coordinate and extent to the virtual device's 0..65535 range.
fn scale_coordinate(value: i32, extent: i32) -> i32 {
    if extent <= 0 {
        return 0;
    }
    let value = i64::from(value).clamp(0, i64::from(extent));
    (value * i64::from(ABS_MAX) / i64::from(extent)) as i32
}

/// A multi-touch Protocol B touchscreen backed by `/dev/uinput`.
pub struct VirtualTouchscreen {
    fd: OwnedFd,
    slots: TouchSlots,
}

impl VirtualTouchscreen {
    /// Create and register the virtual touchscreen. Creation is lazy so hosts without touch
    /// clients do not acquire an extra input device.
    pub fn create() -> Result<Self> {
        use std::os::fd::FromRawFd;

        // SAFETY: the path is a static NUL-terminated string. A successful call returns a fresh
        // descriptor that becomes owned by the OwnedFd below.
        let raw = unsafe {
            libc::open(
                c"/dev/uinput".as_ptr(),
                libc::O_RDWR | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if raw < 0 {
            bail!(
                "open /dev/uinput: {} (install scripts/60-slipstream.rules and grant the user \
                 access to the input group)",
                std::io::Error::last_os_error()
            );
        }
        // SAFETY: `raw` is a fresh descriptor with one owner.
        let fd = unsafe { OwnedFd::from_raw_fd(raw) };

        ioctl_int(raw, UI_SET_EVBIT, EV_KEY as i32, "UI_SET_EVBIT(EV_KEY)")?;
        ioctl_int(raw, UI_SET_EVBIT, EV_ABS as i32, "UI_SET_EVBIT(EV_ABS)")?;
        ioctl_int(
            raw,
            UI_SET_KEYBIT,
            BTN_TOUCH as i32,
            "UI_SET_KEYBIT(BTN_TOUCH)",
        )?;
        ioctl_int(
            raw,
            UI_SET_PROPBIT,
            INPUT_PROP_DIRECT,
            "UI_SET_PROPBIT(INPUT_PROP_DIRECT)",
        )?;

        let position = AbsInfo {
            minimum: 0,
            maximum: ABS_MAX,
            resolution: 100,
            ..Default::default()
        };
        let slot = AbsInfo {
            minimum: 0,
            maximum: (MAX_CONTACTS - 1) as i32,
            ..Default::default()
        };
        let tracking = AbsInfo {
            minimum: 0,
            maximum: ABS_MAX,
            ..Default::default()
        };
        let pressure = AbsInfo {
            minimum: 0,
            maximum: PRESSURE_MAX,
            ..Default::default()
        };
        let orientation = AbsInfo {
            minimum: -90,
            maximum: 90,
            ..Default::default()
        };

        for (code, absinfo) in [
            (ABS_X, position),
            (ABS_Y, position),
            (ABS_MT_SLOT, slot),
            (ABS_MT_POSITION_X, position),
            (ABS_MT_POSITION_Y, position),
            (ABS_MT_TRACKING_ID, tracking),
            (ABS_PRESSURE, pressure),
            (ABS_MT_PRESSURE, pressure),
            (ABS_MT_ORIENTATION, orientation),
        ] {
            let mut setup = UinputAbsSetup {
                code,
                _pad: 0,
                absinfo,
            };
            ioctl_ptr(raw, UI_ABS_SETUP, &mut setup, "UI_ABS_SETUP")?;
        }

        let mut setup = UinputSetup {
            id: InputId {
                bustype: 0x0006,
                vendor: 0x1209,
                product: 0x5354,
                version: 1,
            },
            name: [0; 80],
            ff_effects_max: 0,
        };
        let name = b"Slipstream Touchscreen";
        setup.name[..name.len()].copy_from_slice(name);
        ioctl_ptr(raw, UI_DEV_SETUP, &mut setup, "UI_DEV_SETUP")?;
        ioctl_int(raw, UI_DEV_CREATE, 0, "UI_DEV_CREATE")?;
        tracing::info!(
            contacts = MAX_CONTACTS,
            "virtual touchscreen created (Slipstream Touchscreen, uinput)"
        );

        Ok(Self {
            fd,
            slots: TouchSlots::new(),
        })
    }

    /// Apply one wire touch event as a complete evdev frame.
    pub fn apply(&mut self, event: &InputEvent) {
        match event.kind {
            InputKind::TouchDown => {
                let was_empty = self.slots.active_count() == 0;
                let new_contact = self.slots.motion(event.code).is_none();
                let Some(slot) = self.slots.down(event.code) else {
                    tracing::warn!(
                        id = event.code,
                        contacts = MAX_CONTACTS,
                        "virtual touchscreen contact limit reached"
                    );
                    return;
                };
                self.select_slot(slot);
                if new_contact {
                    self.emit(EV_ABS, ABS_MT_TRACKING_ID, slot as i32 + 1);
                }
                self.emit_position(event);
                if was_empty {
                    self.emit(EV_KEY, BTN_TOUCH, 1);
                }
                self.emit(EV_SYN, SYN_REPORT, 0);
            }
            InputKind::TouchMove => {
                let Some(slot) = self.slots.motion(event.code) else {
                    return;
                };
                self.select_slot(slot);
                self.emit_position(event);
                self.emit(EV_SYN, SYN_REPORT, 0);
            }
            InputKind::TouchUp => {
                let Some(slot) = self.slots.up(event.code) else {
                    return;
                };
                self.select_slot(slot);
                self.emit(EV_ABS, ABS_MT_TRACKING_ID, -1);
                if self.slots.active_count() == 0 {
                    self.emit(EV_KEY, BTN_TOUCH, 0);
                    self.emit(EV_ABS, ABS_PRESSURE, 0);
                }
                self.emit(EV_SYN, SYN_REPORT, 0);
            }
            _ => {}
        }
    }

    /// Release every contact still held by the device.
    pub fn release_all(&mut self) {
        let mut released = false;
        for slot in 0..MAX_CONTACTS {
            if self.slots.ids[slot].take().is_some() {
                self.select_slot(slot);
                self.emit(EV_ABS, ABS_MT_TRACKING_ID, -1);
                released = true;
            }
        }
        if released {
            self.emit(EV_KEY, BTN_TOUCH, 0);
            self.emit(EV_ABS, ABS_PRESSURE, 0);
            self.emit(EV_SYN, SYN_REPORT, 0);
        }
    }

    fn select_slot(&self, slot: usize) {
        self.emit(EV_ABS, ABS_MT_SLOT, slot as i32);
    }

    fn emit_position(&self, event: &InputEvent) {
        let width = ((event.flags >> 16) & 0xffff) as i32;
        let height = (event.flags & 0xffff) as i32;
        let x = scale_coordinate(event.x, width);
        let y = scale_coordinate(event.y, height);
        self.emit(EV_ABS, ABS_X, x);
        self.emit(EV_ABS, ABS_Y, y);
        self.emit(EV_ABS, ABS_MT_POSITION_X, x);
        self.emit(EV_ABS, ABS_MT_POSITION_Y, y);
        self.emit(EV_ABS, ABS_PRESSURE, PRESSURE_MAX);
        self.emit(EV_ABS, ABS_MT_PRESSURE, PRESSURE_MAX);
        self.emit(EV_ABS, ABS_MT_ORIENTATION, 0);
    }

    fn emit(&self, type_: u16, code: u16, value: i32) {
        let event = InputEventRaw {
            time: libc::timeval {
                tv_sec: 0,
                tv_usec: 0,
            },
            type_,
            code,
            value,
        };
        // SAFETY: `event` is a fully initialized repr(C) value. The byte slice is used only for
        // this synchronous write while the local remains alive.
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &event as *const _ as *const u8,
                std::mem::size_of::<InputEventRaw>(),
            )
        };
        // SAFETY: the descriptor remains open for this synchronous call and the kernel only reads
        // the supplied event bytes.
        let _ = unsafe {
            libc::write(
                self.fd.as_raw_fd(),
                bytes.as_ptr() as *const libc::c_void,
                bytes.len(),
            )
        };
    }
}

impl Drop for VirtualTouchscreen {
    fn drop(&mut self) {
        self.release_all();
        // SAFETY: the descriptor remains open until OwnedFd drops after this method, and the
        // destroy ioctl takes no pointer argument.
        let _ = unsafe { libc::ioctl(self.fd.as_raw_fd(), UI_DEV_DESTROY, 0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_ids_keep_their_slots_until_release() {
        let mut slots = TouchSlots::new();

        assert_eq!(slots.down(7), Some(0));
        assert_eq!(slots.down(9), Some(1));
        assert_eq!(slots.motion(7), Some(0));
        assert_eq!(slots.up(7), Some(0));
        assert_eq!(slots.down(11), Some(0));
        assert_eq!(slots.motion(9), Some(1));
    }

    #[test]
    fn unknown_contacts_do_not_create_or_release_slots() {
        let mut slots = TouchSlots::new();

        assert_eq!(slots.motion(4), None);
        assert_eq!(slots.up(4), None);
        assert_eq!(slots.down(4), Some(0));
        assert_eq!(slots.up(4), Some(0));
        assert_eq!(slots.motion(4), None);
    }

    #[test]
    fn coordinates_are_scaled_to_the_device_range() {
        assert_eq!(scale_coordinate(540, 1080), 32767);
        assert_eq!(scale_coordinate(-10, 1080), 0);
        assert_eq!(scale_coordinate(2000, 1080), 65535);
        assert_eq!(scale_coordinate(10, 0), 0);
    }

    #[test]
    #[ignore = "requires access to /dev/uinput"]
    fn uinput_touchscreen_accepts_a_two_contact_sequence() {
        let mut touchscreen = VirtualTouchscreen::create().expect("create virtual touchscreen");
        let event = |kind, id, x, y| InputEvent {
            kind,
            _pad: [0; 3],
            code: id,
            x,
            y,
            flags: (1080 << 16) | 1920,
        };

        touchscreen.apply(&event(InputKind::TouchDown, 7, 100, 200));
        touchscreen.apply(&event(InputKind::TouchDown, 9, 900, 1600));
        touchscreen.apply(&event(InputKind::TouchMove, 7, 120, 220));
        touchscreen.apply(&event(InputKind::TouchUp, 7, 120, 220));
        touchscreen.apply(&event(InputKind::TouchUp, 9, 900, 1600));
    }
}
