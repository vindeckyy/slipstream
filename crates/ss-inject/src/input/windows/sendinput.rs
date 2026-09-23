//! Windows `SendInput` pointer/keyboard injection — the primary Windows backend.
//!
//! Client [`InputEvent`]s become Win32 [`SendInput`] batches: relative mouse motion,
//! absolute motion normalized to the 0..65535 virtual-desktop range (honoring the
//! [`AbsoluteAnchor`] head origin when set), buttons, 120-scaled wheel, VK key
//! down/up (with `EXTENDEDKEY` where the controller expects it), committed text via
//! `KEYEVENTF_UNICODE` (UTF-16 units, surrogate pairs included — no host-layout
//! dependence), and single-touch mapped onto the mouse (down/move/up of the first
//! finger; extra fingers are ignored, not merged).
//!
//! Gamepad *button/axis/state* events never reach here — they route to the virtual-pad
//! managers, like on Linux. ViGEmBus (the Windows pad transport) lands separately.

use super::{InputEvent, InputInjector};
use anyhow::{Context, Result};
use slipstream_core::input::InputKind;
use windows::Win32::UI::{
    Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS,
        KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MOUSEEVENTF_ABSOLUTE,
        MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
        MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
        MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT,
        MOUSE_EVENT_FLAGS, VIRTUAL_KEY,
    },
    WindowsAndMessaging::{
        GetSystemMetrics, SM_CXSCREEN, SM_CXVIRTUALSCREEN, SM_CYSCREEN, SM_CYVIRTUALSCREEN,
        SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, XBUTTON1, XBUTTON2,
    },
};

/// GameStream horizontal-scroll marker (mirrors the Linux injectors).
const SCROLL_HORIZONTAL: u32 = 1;

/// GameStream mouse button ids (see `gs_button_to_evdev`): 1=left … 5=X2.
const GS_LEFT: u32 = 1;
const GS_MIDDLE: u32 = 2;
const GS_RIGHT: u32 = 3;
const GS_X1: u32 = 4;
const GS_X2: u32 = 5;

/// VK codes that need `KEYEVENTF_EXTENDEDKEY`: navigation cluster, ins/del, the right
/// modifiers, and numpad divide (the controller distinguishes them from their
/// non-extended twins).
fn vk_needs_extended(vk: u8) -> bool {
    matches!(
        vk,
        0x21..=0x28 | // PRIOR/NEXT/END/HOME/LEFT/UP/RIGHT/DOWN
        0x2D | 0x2E | // INSERT/DELETE
        0x6F | // VK_DIVIDE (numpad /)
        0xA3 | 0xA5 // VK_RCONTROL/VK_RMENU
    )
}

/// The Windows `SendInput` injector. Stateless across events except the single-touch
/// finger being tracked; lives on the injector-service thread like every backend.
pub struct WindowsSendInput {
    /// Touch id currently driving the mouse (`None` = no finger down).
    active_touch: Option<u32>,
}

impl WindowsSendInput {
    pub fn open() -> Result<Self> {
        Ok(WindowsSendInput { active_touch: None })
    }

    /// Submit one batch; `0` injected means the input was lost (locked desktop, UIPI
    /// block) — surfaced loudly, since input is lossy by design but silence is not.
    fn send(inputs: &[INPUT], what: &str) -> Result<()> {
        // SAFETY: `inputs` is a live slice of plain `INPUT` structs; `SendInput`
        // copies them synchronously and touches no caller memory afterwards. A `0`
        // return (nothing injected — locked/secure desktop, UIPI integrity block) is
        // reported, never silently swallowed.
        let n = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
        if n == 0 {
            anyhow::bail!("SendInput({what}) injected 0/{} events", inputs.len());
        }
        if (n as usize) != inputs.len() {
            tracing::debug!(
                what,
                injected = n,
                total = inputs.len(),
                "SendInput partial batch"
            );
        }
        Ok(())
    }

    fn mouse_input(dx: i32, dy: i32, data: u32, flags: MOUSE_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx,
                    dy,
                    mouseData: data,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    fn key_input(vk: u8, down: bool) -> INPUT {
        let mut flags = if down {
            KEYBD_EVENT_FLAGS(0)
        } else {
            KEYEVENTF_KEYUP
        };
        if vk_needs_extended(vk) {
            flags |= KEYEVENTF_EXTENDEDKEY;
        }
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk as u16),
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    fn unicode_input(unit: u16, down: bool) -> INPUT {
        let flags = if down {
            KEYEVENTF_UNICODE
        } else {
            KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
        };
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: unit,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    /// Map a client absolute position (in the `flags`-packed `w × h` space) onto the
    /// 0..65535 virtual-desktop range, anchored at the streamed head's origin when the
    /// host pin set one ([`AbsoluteAnchor`]) — the Windows analogue of the libei region
    /// ladder. Falls back to the primary monitor's origin/size.
    fn absolute_to_vdesk(x: i32, y: i32, w: u32, h: u32) -> (i32, i32) {
        // SAFETY: nullary metric reads; no pointers, no handles.
        let (vdesk_x, vdesk_y, vdesk_w, vdesk_h) = unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1),
                GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1),
            )
        };
        let (origin_x, origin_y) = super::absolute_anchor()
            .and_then(|a| a.origin)
            .unwrap_or((0, 0));
        // SAFETY: nullary metric reads (see above).
        let (head_w, head_h) = unsafe {
            (
                GetSystemMetrics(SM_CXSCREEN).max(1),
                GetSystemMetrics(SM_CYSCREEN).max(1),
            )
        };
        // Clamp into the client's own space first (a buggy/lying client must not throw
        // the pointer light-years off-screen), then scale into head pixels, then offset
        // by the head origin into virtual-desktop space, then normalize.
        let w = w.max(1) as i64;
        let h = h.max(1) as i64;
        let px = origin_x as i64 + x as i64 * head_w as i64 / w;
        let py = origin_y as i64 + y as i64 * head_h as i64 / h;
        let nx = ((px - vdesk_x as i64) * 65535 / vdesk_w.max(1) as i64).clamp(0, 65535) as i32;
        let ny = ((py - vdesk_y as i64) * 65535 / vdesk_h.max(1) as i64).clamp(0, 65535) as i32;
        (nx, ny)
    }

    fn inject_text(&mut self, scalar: u32) -> Result<()> {
        let Some(ch) = char::from_u32(scalar) else {
            tracing::debug!(scalar, "TextInput scalar is not a char — dropped");
            return Ok(());
        };
        // One down+up per UTF-16 unit (surrogate pairs type astral chars whole).
        let mut units = [0u16; 2];
        let mut batch = Vec::with_capacity(4);
        for unit in ch.encode_utf16(&mut units) {
            batch.push(Self::unicode_input(*unit, true));
            batch.push(Self::unicode_input(*unit, false));
        }
        Self::send(&batch, "text")
    }
}

impl InputInjector for WindowsSendInput {
    fn inject(&mut self, event: &InputEvent) -> Result<()> {
        match event.kind {
            InputKind::MouseMove => Self::send(
                &[Self::mouse_input(event.x, event.y, 0, MOUSEEVENTF_MOVE)],
                "move",
            )
            .context("relative mouse move"),
            InputKind::MouseMoveAbs => {
                let w = (event.flags >> 16) & 0xffff;
                let h = event.flags & 0xffff;
                if w == 0 || h == 0 {
                    tracing::debug!("absolute move with zero extent — dropped");
                    return Ok(());
                }
                let (nx, ny) = Self::absolute_to_vdesk(event.x, event.y, w, h);
                Self::send(
                    &[Self::mouse_input(
                        nx,
                        ny,
                        0,
                        MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                    )],
                    "move-abs",
                )
                .context("absolute mouse move")
            }
            InputKind::MouseButtonDown | InputKind::MouseButtonUp => {
                let down = event.kind == InputKind::MouseButtonDown;
                let input = match event.code {
                    GS_LEFT => Self::mouse_input(
                        0,
                        0,
                        0,
                        if down {
                            MOUSEEVENTF_LEFTDOWN
                        } else {
                            MOUSEEVENTF_LEFTUP
                        },
                    ),
                    GS_MIDDLE => Self::mouse_input(
                        0,
                        0,
                        0,
                        if down {
                            MOUSEEVENTF_MIDDLEDOWN
                        } else {
                            MOUSEEVENTF_MIDDLEUP
                        },
                    ),
                    GS_RIGHT => Self::mouse_input(
                        0,
                        0,
                        0,
                        if down {
                            MOUSEEVENTF_RIGHTDOWN
                        } else {
                            MOUSEEVENTF_RIGHTUP
                        },
                    ),
                    GS_X1 => Self::mouse_input(
                        0,
                        0,
                        XBUTTON1 as u32,
                        if down {
                            MOUSEEVENTF_XDOWN
                        } else {
                            MOUSEEVENTF_XUP
                        },
                    ),
                    GS_X2 => Self::mouse_input(
                        0,
                        0,
                        XBUTTON2 as u32,
                        if down {
                            MOUSEEVENTF_XDOWN
                        } else {
                            MOUSEEVENTF_XUP
                        },
                    ),
                    other => {
                        tracing::debug!(button = other, "unknown mouse button — dropped");
                        return Ok(());
                    }
                };
                Self::send(&[input], "button").context("mouse button")
            }
            InputKind::MouseScroll => {
                // GameStream sends 120-scaled deltas; Win32 takes them as-is (one WHEEL_DELTA
                // notch per detent). Positive = up/right on both sides — no sign flip.
                let (data, flags) = if event.code == SCROLL_HORIZONTAL {
                    (event.x as u32, MOUSEEVENTF_HWHEEL)
                } else {
                    (event.x as u32, MOUSEEVENTF_WHEEL)
                };
                Self::send(&[Self::mouse_input(0, 0, data, flags)], "wheel").context("mouse wheel")
            }
            InputKind::KeyDown | InputKind::KeyUp => {
                let down = event.kind == InputKind::KeyDown;
                if event.code > 0xFF {
                    anyhow::bail!("VK code {:#x} out of range — dropped", event.code);
                }
                Self::send(&[Self::key_input(event.code as u8, down)], "key")
                    .context("keyboard event")
            }
            InputKind::TextInput => self.inject_text(event.code).context("committed text"),
            // Single-touch drives the mouse: first finger down moves + presses, its moves
            // track, its release lets go. Extra fingers are ignored (never merged).
            InputKind::TouchDown => {
                if self.active_touch.is_some() {
                    tracing::debug!("second touch down — ignored (single-pointer only)");
                    return Ok(());
                }
                let w = (event.flags >> 16) & 0xffff;
                let h = event.flags & 0xffff;
                if w == 0 || h == 0 {
                    return Ok(());
                }
                let (nx, ny) = Self::absolute_to_vdesk(event.x, event.y, w, h);
                Self::send(
                    &[
                        Self::mouse_input(
                            nx,
                            ny,
                            0,
                            MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                        ),
                        Self::mouse_input(0, 0, 0, MOUSEEVENTF_LEFTDOWN),
                    ],
                    "touch-down",
                )
                .context("touch down")?;
                self.active_touch = Some(event.code);
                Ok(())
            }
            InputKind::TouchMove => {
                if self.active_touch != Some(event.code) {
                    return Ok(());
                }
                let w = (event.flags >> 16) & 0xffff;
                let h = event.flags & 0xffff;
                if w == 0 || h == 0 {
                    return Ok(());
                }
                let (nx, ny) = Self::absolute_to_vdesk(event.x, event.y, w, h);
                Self::send(
                    &[Self::mouse_input(
                        nx,
                        ny,
                        0,
                        MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                    )],
                    "touch-move",
                )
                .context("touch move")
            }
            InputKind::TouchUp => {
                if self.active_touch != Some(event.code) {
                    return Ok(());
                }
                self.active_touch = None;
                Self::send(
                    &[Self::mouse_input(0, 0, 0, MOUSEEVENTF_LEFTUP)],
                    "touch-up",
                )
                .context("touch up")
            }
            InputKind::GamepadState
            | InputKind::GamepadButton
            | InputKind::GamepadAxis
            | InputKind::GamepadRemove
            | InputKind::GamepadArrival => {
                // Pad traffic routes to the virtual-pad managers, never the pointer/keyboard
                // injector (ViGEmBus lands separately).
                Ok(())
            }
        }
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn extended_flag_covers_navigation_and_right_modifiers() {
        for vk in [0x21u8, 0x25, 0x28, 0x2D, 0x2E, 0x6F, 0xA3, 0xA5] {
            assert!(vk_needs_extended(vk), "{vk:#x} should be extended");
        }
        for vk in [0x0Du8, 0x20, 0x41, 0x10, 0x5B] {
            assert!(!vk_needs_extended(vk), "{vk:#x} should not be extended");
        }
    }

    #[test]
    fn absolute_mapping_clamps_and_scales() {
        // Degenerate extents never divide by zero and stay in range.
        let (x, y) = WindowsSendInput::absolute_to_vdesk(0, 0, 0, 0);
        assert!((0..=65535).contains(&x) && (0..=65535).contains(&y));
    }
}
