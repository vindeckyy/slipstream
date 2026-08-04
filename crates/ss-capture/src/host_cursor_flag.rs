//! Process-wide flag: the host OS cursor is hidden for streaming (see slipstream-host
//! `host_cursor` + ss-inject platform hide). Capture/encode paths use this to keep publishing
//! a stream cursor overlay even when Win32 `CURSOR_SHOWING` is clear or Mutter's theme-blank
//! sprite would otherwise empty `SPA_META_Cursor`.

use std::sync::atomic::{AtomicBool, Ordering};

static HIDDEN_FOR_STREAM: AtomicBool = AtomicBool::new(false);

/// Mark that the host's local OS cursor is hidden for the duration of live streams.
pub fn set_hidden_for_stream(hidden: bool) {
    HIDDEN_FOR_STREAM.store(hidden, Ordering::Relaxed);
}

/// True while a stream session is holding the host-cursor hide guard.
pub fn is_hidden_for_stream() -> bool {
    HIDDEN_FOR_STREAM.load(Ordering::Relaxed)
}
