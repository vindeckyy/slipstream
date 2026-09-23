//! Windows service entry: run the host under the Service Control Manager.
//!
//! The MSI registers `slipstream-host` (see `tray::status::SERVICE_NAME` — the two must
//! agree); the SCM then starts this binary with no arguments and `dispatch()` below
//! routes into the service path instead of the CLI. Console runs are unaffected:
//! `StartServiceCtrlDispatcherW` fails with `ERROR_FAILED_SERVICE_CONTROLLER_CONNECT`
//! outside SCM and `dispatch` returns `false` so `real_main` continues normally.
//!
//! Lifecycle: `ServiceMain` reports `START_PENDING`, runs the default `serve` plane
//! set inline (mgmt API + native slipstream/1, GameStream off — the secure default,
//! same as bare `slipstream-host serve`), then reports `RUNNING`. `SERVICE_CONTROL_STOP`
//! reports `STOP_PENDING`, hands the box's session back (the same restore the Unix
//! `SIGTERM` path runs), and exits. Sessions in flight see a disconnect and reconnect
//! to the restarted service — there is no graceful drain yet; a stop during a stream
//! is a reconnect, not data loss.

use std::sync::{
    atomic::{AtomicU32, Ordering},
    OnceLock,
};
use windows::{core::w, Win32::System::Services::*};

/// Current service state, mirrored to the SCM on every transition.
static CURRENT_STATE: AtomicU32 = AtomicU32::new(SERVICE_STOPPED.0);
/// Status handle from `RegisterServiceCtrlHandlerExW`, set once in `ServiceMain`.
/// Stored raw: the handle type is neither `Send` nor `Sync`, and every use rebuilds it
/// on the thread that owns the service lifecycle (same pattern as the tray window).
static STATUS_HANDLE: OnceLock<usize> = OnceLock::new();

/// Rebuild the status handle stored by `ServiceMain`.
fn status_handle() -> Option<SERVICE_STATUS_HANDLE> {
    STATUS_HANDLE
        .get()
        .map(|raw| SERVICE_STATUS_HANDLE(*raw as *mut core::ffi::c_void))
}

/// Whether `e` is `ERROR_FAILED_SERVICE_CONTROLLER_CONNECT` (not running under SCM).
fn is_not_service_process(e: &windows::core::Error) -> bool {
    e.code().0 as u32 & 0xFFFF
        == windows::Win32::Foundation::ERROR_FAILED_SERVICE_CONTROLLER_CONNECT.0
}

/// Run under the SCM when started as a service; return `false` for console runs.
///
/// Blocks until the service stops (which exits the process) — a `true` return only
/// happens if `serve` itself returns, and then the service is reported stopped.
pub fn dispatch() -> anyhow::Result<bool> {
    // SAFETY: null-terminated two-entry table (our main + the required null entry).
    // The name buffer is leaked process-lifetime (the SCM may read it until the
    // dispatcher returns); the proc is a fn item. Fails fast with
    // `ERROR_FAILED_SERVICE_CONTROLLER_CONNECT` for console runs.
    unsafe {
        let name: &'static mut [u16] = Box::leak(
            w!("slipstream-host")
                .as_wide()
                .iter()
                .copied()
                .chain(std::iter::once(0))
                .collect::<Vec<u16>>()
                .into_boxed_slice(),
        );
        let table = [
            SERVICE_TABLE_ENTRYW {
                lpServiceName: windows::core::PWSTR(name.as_mut_ptr()),
                lpServiceProc: Some(service_main),
            },
            SERVICE_TABLE_ENTRYW::default(),
        ];
        match StartServiceCtrlDispatcherW(table.as_ptr()) {
            Ok(()) => Ok(true),
            Err(e) if is_not_service_process(&e) => Ok(false),
            Err(e) => Err(e).context_service_dispatch(),
        }
    }
}

/// `anyhow::Context`-shaped helper without importing it into this leaf (the crate root
/// already depends on anyhow; this keeps the import list to windows + std).
trait ContextServiceDispatch<T> {
    fn context_service_dispatch(self) -> anyhow::Result<T>;
}

impl<T> ContextServiceDispatch<T> for windows::core::Result<T> {
    fn context_service_dispatch(self) -> anyhow::Result<T> {
        self.map_err(|e| anyhow::anyhow!("service dispatcher: {e}"))
    }
}

/// The SCM entry point: register the control handler, run the default serve set.
unsafe extern "system" fn service_main(_argc: u32, _argv: *mut windows::core::PWSTR) {
    // SAFETY: handler registration on the SCM-owned ServiceMain thread; the handle is
    // stored process-wide for the handler below. A failed registration leaves no
    // service to report to — log to the event log is a follow-up; stderr for now.
    let handle = unsafe {
        match RegisterServiceCtrlHandlerExW(w!("slipstream-host"), Some(handler_ex), None) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("slipstream-host service: handler registration failed: {e}");
                return;
            }
        }
    };
    let _ = STATUS_HANDLE.set(handle.0 as usize);
    report_state(SERVICE_START_PENDING, 0);
    match crate::run_default_serve() {
        Ok(()) => {
            report_state(SERVICE_STOPPED, 0);
        }
        Err(e) => {
            eprintln!("slipstream-host service: serve failed: {e:#}");
            report_state(SERVICE_STOPPED, 1);
        }
    }
}

/// The SCM control handler: stop (restore + exit) and interrogate (re-report).
unsafe extern "system" fn handler_ex(
    control: u32,
    _event_type: u32,
    _event_data: *mut core::ffi::c_void,
    _context: *mut core::ffi::c_void,
) -> u32 {
    match control {
        SERVICE_CONTROL_STOP => {
            report_state(SERVICE_STOP_PENDING, 0);
            // The handler must not block: restore + exit on a fresh thread, under the
            // same grace the Unix SIGTERM path honors.
            std::thread::spawn(|| {
                crate::vdisplay::restore_takeover_now();
                std::process::exit(0);
            });
            0 // NO_ERROR
        }
        SERVICE_CONTROL_INTERROGATE => {
            report_state(
                SERVICE_STATUS_CURRENT_STATE(CURRENT_STATE.load(Ordering::SeqCst)),
                0,
            );
            0
        }
        _ => 1, // ERROR_CALL_NOT_IMPLEMENTED
    }
}

/// Report `state` to the SCM (no-op before the handle exists).
fn report_state(state: SERVICE_STATUS_CURRENT_STATE, exit_code: u32) {
    CURRENT_STATE.store(state.0, Ordering::SeqCst);
    if let Some(handle) = status_handle() {
        // SAFETY: `handle` is the live registration from `ServiceMain`; the status is
        // a plain value struct.
        unsafe {
            let _ = SetServiceStatus(
                handle,
                &SERVICE_STATUS {
                    dwServiceType: SERVICE_WIN32_OWN_PROCESS,
                    dwCurrentState: state,
                    dwControlsAccepted: if state.0 == SERVICE_RUNNING.0 {
                        SERVICE_ACCEPT_STOP
                    } else {
                        Default::default()
                    },
                    dwWin32ExitCode: exit_code,
                    dwServiceSpecificExitCode: 0,
                    dwCheckPoint: 0,
                    dwWaitHint: 0,
                },
            );
        }
    }
}
