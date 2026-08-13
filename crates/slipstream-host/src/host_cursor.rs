//! Session-scoped host-local cursor hide: while at least one client is streaming, hide the
//! host OS cursor (restore when the last session ends). Mirrors [`crate::sleep_inhibit`]:
//! refcounted across native + GameStream planes. Best-effort - platforms that cannot hide
//! log once and stream on. Off when `SLIPSTREAM_HIDE_HOST_CURSOR=0`.

use std::sync::{Mutex, OnceLock};

/// RAII share of the host-wide cursor hide - hold one per live session/stream.
pub struct StreamHold(());

struct State {
    count: u32,
    /// The platform hide held for the whole 1..N refcount window (dropped on 1→0).
    platform: Option<ss_inject::host_cursor::PlatformHide>,
}

fn state() -> &'static Mutex<State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(|| {
        Mutex::new(State {
            count: 0,
            platform: None,
        })
    })
}

/// Take a share; the underlying OS hide is acquired on the 0→1 edge when the config allows it.
pub fn hold() -> StreamHold {
    if !ss_host_config::config().hide_host_cursor {
        return StreamHold(());
    }
    let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
    st.count += 1;
    if st.count == 1 && st.platform.is_none() {
        st.platform = ss_inject::host_cursor::PlatformHide::acquire();
    }
    StreamHold(())
}

impl Drop for StreamHold {
    fn drop(&mut self) {
        if !ss_host_config::config().hide_host_cursor {
            return;
        }
        let mut st = state().lock().unwrap_or_else(|e| e.into_inner());
        st.count = st.count.saturating_sub(1);
        if st.count == 0 && st.platform.take().is_some() {
            tracing::info!("restored the host OS cursor (no live sessions)");
        }
    }
}
