//! Win32 window input capture → probe/`ss-inject` wire events.
//!
//! The window procedure in [`crate::window`] turns keyboard and mouse messages into
//! [`InputEvent`]s. This module owns the other half: focus/admission gating and the QUIC datagram
//! pump that carries those events to the host. The event constructors are pure so the wire mapping
//! is unit-testable without a window or a network.
//!
//! Focus is a hard gate: while the presentation window is inactive, keyboard and mouse events are
//! dropped. The one exception is the forced release we send when focus is lost, so a key or button
//! held at the moment of Alt-Tab cannot stay stuck down on the host.

use anyhow::Result;
use quinn::Connection;
use slipstream_core::input::{InputEvent, InputKind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

/// GameStream button ids used by the host injector: 1=left, 2=middle, 3=right, 4=X1, 5=X2.
const MOUSE_LEFT: u32 = 1;
const MOUSE_MIDDLE: u32 = 2;
const MOUSE_RIGHT: u32 = 3;
const MOUSE_X1: u32 = 4;
const MOUSE_X2: u32 = 5;
const SCROLL_HORIZONTAL: u32 = 1;

/// One client's input route. Clone it into the window procedure; it stays inert until the session
/// is admitted and the window reports focus.
#[derive(Debug, Clone)]
pub struct InputSender {
    tx: UnboundedSender<InputEvent>,
    enabled: Arc<AtomicBool>,
    focused: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
}

impl InputSender {
    /// Create the route and its matching receiver. `run_stream` hands the receiver to the QUIC
    /// datagram pump; the sender is carried in [`SessionConfig`](crate::session::SessionConfig).
    pub fn channel() -> (Self, UnboundedReceiver<InputEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                tx,
                enabled: Arc::new(AtomicBool::new(false)),
                focused: Arc::new(AtomicBool::new(false)),
                shutdown: Arc::new(AtomicBool::new(false)),
            },
            rx,
        )
    }

    /// Input is legal only after admission. Set once when the host has accepted the session.
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// The window procedure updates this from `WM_ACTIVATE`, `WM_SETFOCUS` and `WM_KILLFOCUS`.
    pub fn set_focused(&self, focused: bool) {
        self.focused.store(focused, Ordering::Relaxed);
    }

    /// Submit one captured event. Focus and admission are checked here, not in the window
    /// procedure, so a message-handling mistake cannot leak local input to the host.
    pub fn submit(&self, event: InputEvent) {
        if !self.enabled.load(Ordering::Relaxed) || !self.focused.load(Ordering::Relaxed) {
            return;
        }
        let _ = self.tx.send(event);
    }

    /// Submit a forced key-up/button-up during focus loss. Admission still applies, but focus does
    /// not: the release must reach the host even though the window just became inactive.
    pub fn submit_release(&self, event: InputEvent) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        let _ = self.tx.send(event);
    }

    /// Stop the QUIC pump. Used as a drop guard so an early session error cannot leave a task
    /// holding the connection open.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Forward queued events as plain QUIC datagrams, exactly like the probe's input test.
    pub async fn pump(self, mut rx: UnboundedReceiver<InputEvent>, conn: Connection) -> Result<()> {
        while !self.shutdown.load(Ordering::Relaxed) {
            match tokio::time::timeout(Duration::from_millis(100), rx.recv()).await {
                Ok(Some(event)) => {
                    if let Err(e) = conn.send_datagram(event.encode().to_vec().into()) {
                        // Input is lossy by design (QUIC datagrams); log and keep the route alive.
                        tracing::debug!(error = %format!("{e:?}"), "input datagram send failed");
                    }
                }
                Ok(None) => break,
                Err(_) => {}
            }
        }
        Ok(())
    }
}

/// Drops the input sender's shutdown flag when a session returns, including error paths.
pub struct InputSenderGuard(InputSender);

impl InputSenderGuard {
    pub fn new(sender: InputSender) -> Self {
        Self(sender)
    }
}

impl Drop for InputSenderGuard {
    fn drop(&mut self) {
        self.0.shutdown();
    }
}

/// Virtual-key down/up → `KeyDown`/`KeyUp`. The host injects `code` as a Win32 VK.
pub fn key(down: bool, vk: u32) -> Option<InputEvent> {
    if vk == 0 || vk > 0xff {
        return None;
    }
    Some(InputEvent {
        kind: if down {
            InputKind::KeyDown
        } else {
            InputKind::KeyUp
        },
        _pad: [0; 3],
        code: vk,
        x: 0,
        y: 0,
        flags: 0,
    })
}

/// GameStream mouse button id → `MouseButtonDown`/`MouseButtonUp`.
pub fn mouse_button(down: bool, button: u32) -> Option<InputEvent> {
    if !matches!(
        button,
        MOUSE_LEFT | MOUSE_MIDDLE | MOUSE_RIGHT | MOUSE_X1 | MOUSE_X2
    ) {
        return None;
    }
    Some(InputEvent {
        kind: if down {
            InputKind::MouseButtonDown
        } else {
            InputKind::MouseButtonUp
        },
        _pad: [0; 3],
        code: button,
        x: 0,
        y: 0,
        flags: 0,
    })
}

/// Client-window cursor delta → `MouseMove`. Zero deltas produce no datagram.
pub fn mouse_move(dx: i32, dy: i32) -> Option<InputEvent> {
    if dx == 0 && dy == 0 {
        return None;
    }
    Some(InputEvent {
        kind: InputKind::MouseMove,
        _pad: [0; 3],
        code: 0,
        x: dx,
        y: dy,
        flags: 0,
    })
}

/// Win32 wheel delta (already in 120-unit detents) → `MouseScroll`. `code` selects the axis: 0
/// vertical, 1 horizontal, matching `SCROLL_HORIZONTAL` in the host injector.
pub fn wheel(delta: i32, horizontal: bool) -> Option<InputEvent> {
    if delta == 0 {
        return None;
    }
    Some(InputEvent {
        kind: InputKind::MouseScroll,
        _pad: [0; 3],
        code: if horizontal { SCROLL_HORIZONTAL } else { 0 },
        x: delta,
        y: 0,
        flags: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc::error::TryRecvError;

    #[test]
    fn virtual_keys_map_to_wire_events() {
        let down = key(true, 0x41).unwrap();
        let up = key(false, 0x41).unwrap();
        assert_eq!(down.kind, InputKind::KeyDown);
        assert_eq!(up.kind, InputKind::KeyUp);
        assert_eq!(down.code, 0x41);
        assert_eq!(down.encode()[0], slipstream_core::input::INPUT_MAGIC);
        assert_eq!(down.encode().len(), slipstream_core::input::INPUT_WIRE_LEN);
        assert!(key(true, 0).is_none());
        assert!(key(true, 0x100).is_none());
    }

    #[test]
    fn mouse_buttons_use_gamestream_ids() {
        for (id, name) in [
            (MOUSE_LEFT, "left"),
            (MOUSE_MIDDLE, "middle"),
            (MOUSE_RIGHT, "right"),
            (MOUSE_X1, "x1"),
            (MOUSE_X2, "x2"),
        ] {
            let ev = mouse_button(true, id).unwrap();
            assert_eq!(ev.kind, InputKind::MouseButtonDown);
            assert_eq!(ev.code, id, "{name}");
            assert_eq!(
                mouse_button(false, id).unwrap().kind,
                InputKind::MouseButtonUp
            );
        }
        assert!(mouse_button(true, 0).is_none());
        assert!(mouse_button(true, 6).is_none());
    }

    #[test]
    fn movement_and_wheel_keep_host_scaling() {
        let mv = mouse_move(7, -3).unwrap();
        assert_eq!(mv.kind, InputKind::MouseMove);
        assert_eq!((mv.x, mv.y), (7, -3));
        assert!(mouse_move(0, 0).is_none());

        let vertical = wheel(120, false).unwrap();
        let horizontal = wheel(-240, true).unwrap();
        assert_eq!(vertical.code, 0);
        assert_eq!(vertical.x, 120);
        assert_eq!(horizontal.code, SCROLL_HORIZONTAL);
        assert_eq!(horizontal.x, -240);
        assert!(wheel(0, false).is_none());
    }

    #[test]
    fn focus_and_admission_gate_captured_events() {
        let (sender, mut rx) = InputSender::channel();
        let event = key(true, 0x41).unwrap();

        sender.submit(event); // not admitted yet
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));

        sender.set_enabled(true);
        sender.submit(event); // admitted but unfocused
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));

        sender.set_focused(true);
        sender.submit(event);
        assert_eq!(rx.try_recv().unwrap(), event);
    }

    #[test]
    fn forced_release_bypasses_focus_but_not_admission() {
        let (sender, mut rx) = InputSender::channel();
        let release = key(false, 0x41).unwrap();

        sender.submit_release(release);
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));

        sender.set_enabled(true);
        sender.submit_release(release);
        assert_eq!(rx.try_recv().unwrap(), release);
    }
}
