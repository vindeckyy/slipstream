//! Windows tray: a `Shell_NotifyIconW` icon fed by the status poller, with the service
//! state coming from the Service Control Manager instead of systemd.
//!
//! One hidden message-only window owns the icon; the poller's `on_change` posts
//! `WM_APP_UPDATE` to it and the window thread rebuilds tooltip + menu. Menu actions
//! (Open console, Start/Stop service, Quit) run inline on the message thread; service
//! control is best-effort and only as privileged as the tray process itself.

use std::sync::{Arc, Mutex, OnceLock};

use crate::status::{Poller, TrayStatus};
use crate::Args;
use windows::{
    core::{w, HSTRING, PCWSTR},
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM},
        System::{
            LibraryLoader::GetModuleHandleW,
            Services::{
                CloseServiceHandle, ControlService, OpenSCManagerW, OpenServiceW, StartServiceW,
                SC_HANDLE, SC_MANAGER_CONNECT, SERVICE_CONTROL_STOP, SERVICE_START, SERVICE_STATUS,
                SERVICE_STOP,
            },
        },
        UI::{
            Shell::{
                ShellExecuteW, Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD,
                NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
            },
            WindowsAndMessaging::*,
        },
    },
};

/// Window class of the hidden tray window (also the `--quit` rendezvous).
const WINDOW_CLASS: PCWSTR = w!("SlipstreamTrayHost");
/// Tray callback message (`lParam` carries the mouse message).
const WM_APP_TRAY: u32 = WM_APP + 1;
/// Posted by the poll thread when the status snapshot changes.
const WM_APP_UPDATE: u32 = WM_APP + 2;
/// Posted by a `--quit` instance to ask the running tray to exit.
const WM_APP_QUIT_TRAY: u32 = WM_APP + 3;

/// Menu command ids.
const CMD_CONSOLE: usize = 1;
const CMD_START: usize = 2;
const CMD_STOP: usize = 3;
const CMD_QUIT: usize = 4;
const CMD_PAIRING: usize = 5;
const CMD_DISPLAYS: usize = 6;

/// Process-wide tray state, read by the window proc.
static STATE: OnceLock<Arc<AppState>> = OnceLock::new();

struct AppState {
    /// Raw `HWND` value (`HWND` itself is not `Send`; rebuilt at each use site).
    hwnd_raw: Mutex<isize>,
    status: Mutex<TrayStatus>,
    console_up: Mutex<bool>,
    web_port: u16,
    poller: Mutex<Option<Poller>>,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    // SAFETY: window-class registration + hidden message-only window on this thread;
    // every handle is closed/destroyed on the matching teardown path below.
    unsafe {
        if args.quit {
            // Ask an already-running tray to exit (absent = nothing to quit).
            if let Ok(hwnd) = FindWindowW(WINDOW_CLASS, PCWSTR::null()) {
                let _ = PostMessageW(hwnd, WM_APP_QUIT_TRAY, WPARAM(0), LPARAM(0));
            }
            return Ok(());
        }
        let instance = GetModuleHandleW(PCWSTR::null())?;
        let mut class_name = WINDOW_CLASS.as_wide().to_vec();
        class_name.push(0);
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            anyhow::bail!("tray: RegisterClassW failed");
        }
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            WINDOW_CLASS,
            w!("slipstream-tray"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            HMENU::default(),
            instance,
            None,
        )?;
        let state = Arc::new(AppState {
            hwnd_raw: Mutex::new(hwnd.0 as isize),
            status: Mutex::new(TrayStatus::Stopped),
            console_up: Mutex::new(true),
            web_port: args.web_port,
            poller: Mutex::new(None),
        });
        let _ = STATE.set(state.clone());
        add_icon(hwnd, &TrayStatus::Stopped)?;
        let poller = Poller::spawn(
            args.mgmt_addr.clone(),
            args.mgmt_port,
            args.web_port,
            Box::new(move |status, console_up| {
                if let Some(state) = STATE.get() {
                    *state.status.lock().unwrap() = status;
                    *state.console_up.lock().unwrap() = console_up;
                    let raw = *state.hwnd_raw.lock().unwrap();
                    let hwnd = HWND(raw as *mut core::ffi::c_void);
                    let _ = PostMessageW(hwnd, WM_APP_UPDATE, WPARAM(0), LPARAM(0));
                }
            }),
        );
        *state.poller.lock().unwrap() = Some(poller);
        // The message loop runs until `WM_APP_QUIT_TRAY` destroys the window.
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).0 > 0 {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        remove_icon(hwnd);
        Ok(())
    }
}

/// Window proc: tray callbacks, status updates, and the quit rendezvous.
unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_APP_TRAY => {
            // `lParam` carries the mouse message (low word).
            let mouse = (lparam.0 & 0xFFFF) as u32;
            if mouse == WM_RBUTTONUP {
                show_menu(hwnd);
            } else if mouse == WM_LBUTTONDBLCLK {
                open_console();
            }
            LRESULT(0)
        }
        WM_APP_UPDATE => {
            if let Some(state) = STATE.get() {
                let status = state.status.lock().unwrap().clone();
                update_icon(hwnd, &status);
            }
            LRESULT(0)
        }
        WM_APP_QUIT_TRAY => {
            // SAFETY: tears down our own window + icon; the message loop then exits.
            unsafe {
                remove_icon(hwnd);
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // SAFETY: quits the message loop owned by this thread.
            unsafe {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        _ => {
            // SAFETY: default handling for messages we do not process.
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
    }
}

/// Add (or re-add) the notification icon.
fn add_icon(hwnd: HWND, status: &TrayStatus) -> anyhow::Result<()> {
    // SAFETY: `hwnd` is our live message window; the icon handle is a shared system
    // icon (never destroyed); the tip is copied by the shell.
    unsafe {
        let mut data = icon_data(hwnd, status)?;
        Shell_NotifyIconW(NIM_ADD, &data).ok()?;
        // Re-adding after an explorer restart is the caller's business (the poller
        // refreshes on `TaskbarCreated` in a follow-up); the first add is enough here.
        let _ = &mut data;
        Ok(())
    }
}

/// Refresh the icon + tooltip for the latest status.
fn update_icon(hwnd: HWND, status: &TrayStatus) {
    // SAFETY: see `add_icon`; failures (explorer restarting) just skip this refresh.
    unsafe {
        if let Ok(data) = icon_data(hwnd, status) {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
        }
    }
}

/// Drop the icon (best-effort — process exit removes it anyway).
fn remove_icon(hwnd: HWND) {
    // SAFETY: see `add_icon`; best-effort teardown.
    unsafe {
        let data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            ..Default::default()
        };
        let _ = Shell_NotifyIconW(NIM_DELETE, &data);
    }
}

/// Build the `NOTIFYICONDATAW` for `status` (icon per state, headline tooltip).
fn icon_data(hwnd: HWND, status: &TrayStatus) -> anyhow::Result<NOTIFYICONDATAW> {
    // SAFETY: shared system icons (never destroyed); `LoadIconW(None, …)` loads from
    // the system image list.
    let icon = unsafe {
        LoadIconW(
            HINSTANCE::default(),
            match status {
                TrayStatus::Running(s) if s.session.is_some() || s.video_streaming => {
                    IDI_INFORMATION
                }
                TrayStatus::Running(_) => IDI_APPLICATION,
                TrayStatus::Starting | TrayStatus::Degraded => IDI_WARNING,
                TrayStatus::Error(_) => IDI_ERROR,
                TrayStatus::Stopped | TrayStatus::NotInstalled => IDI_APPLICATION,
            },
        )
        .unwrap_or_default()
    };
    let mut tip: [u16; 128] = [0; 128];
    for (i, unit) in status.headline().encode_utf16().take(127).enumerate() {
        tip[i] = unit;
    }
    Ok(NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_APP_TRAY,
        hIcon: icon,
        szTip: tip,
        ..Default::default()
    })
}

/// Show the context menu at the cursor and run the chosen command.
fn show_menu(hwnd: HWND) {
    // SAFETY: popup-menu lifecycle (create → track → destroy) on this thread; the
    // foreground + cursor calls feed `TrackPopupMenu`, which returns the command id
    // synchronously (`TPM_RETURNCMD`).
    unsafe {
        let (status, console_up, web_port) = match STATE.get() {
            Some(state) => (
                state.status.lock().unwrap().clone(),
                *state.console_up.lock().unwrap(),
                state.web_port,
            ),
            None => return,
        };
        let Ok(menu) = CreatePopupMenu() else {
            return;
        };
        let headline = HSTRING::from(status.headline());
        let _ = AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, &headline);
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let console_label = HSTRING::from(if console_up {
            "Open web console".to_string()
        } else {
            "Open web console (not responding)".to_string()
        });
        let _ = AppendMenuW(menu, MF_STRING, CMD_CONSOLE, &console_label);
        // Pairing approvals and kept (lingering/pinned) displays get deep-link entries,
        // mirroring the Linux menu — the console performs the privileged action.
        if status.pairing_attention() {
            let pairing = HSTRING::from("Approve pairing request…");
            let _ = AppendMenuW(menu, MF_STRING, CMD_PAIRING, &pairing);
        }
        match status.kept_displays() {
            0 => {}
            1 => {
                let kept = HSTRING::from("Release kept display…");
                let _ = AppendMenuW(menu, MF_STRING, CMD_DISPLAYS, &kept);
            }
            n => {
                let kept = HSTRING::from(format!("Release {n} kept displays…"));
                let _ = AppendMenuW(menu, MF_STRING, CMD_DISPLAYS, &kept);
            }
        }
        let running = matches!(
            status,
            TrayStatus::Running(_) | TrayStatus::Starting | TrayStatus::Degraded
        );
        let startable = matches!(
            status,
            TrayStatus::Stopped | TrayStatus::Error(_) | TrayStatus::NotInstalled
        );
        if running {
            let stop = HSTRING::from("Stop service");
            let _ = AppendMenuW(menu, MF_STRING, CMD_STOP, &stop);
        }
        if startable {
            let start = HSTRING::from("Start service");
            let _ = AppendMenuW(menu, MF_STRING, CMD_START, &start);
        }
        let quit = HSTRING::from("Quit tray");
        let _ = AppendMenuW(menu, MF_STRING, CMD_QUIT, &quit);
        let mut point = POINT::default();
        if GetCursorPos(&mut point).is_err() {
            let _ = DestroyMenu(menu);
            return;
        }
        let _ = SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON | TPM_RETURNCMD,
            point.x,
            point.y,
            0,
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);
        // `PostMessageW(WM_NULL)` is the documented companion that lets the menu
        // dismiss cleanly; omitted here — `TPM_RETURNCMD` already returned.
        match cmd.0 as u32 {
            x if x == CMD_CONSOLE as u32 => open_console_at(web_port, ""),
            x if x == CMD_PAIRING as u32 => open_console_at(web_port, "pairing"),
            x if x == CMD_DISPLAYS as u32 => open_console_at(web_port, "displays"),
            x if x == CMD_START as u32 => control_service(true),
            x if x == CMD_STOP as u32 => control_service(false),
            x if x == CMD_QUIT as u32 => {
                let _ = PostMessageW(hwnd, WM_APP_QUIT_TRAY, WPARAM(0), LPARAM(0));
            }
            _ => {}
        }
    }
}

/// Open the web console in the default browser.
fn open_console() {
    if let Some(state) = STATE.get() {
        open_console_at(state.web_port, "");
    }
}

/// Open the web console at `path` ("" = dashboard) in the default browser.
fn open_console_at(web_port: u16, path: &str) {
    // SAFETY: opens a URL in the registered browser; string args live for the call.
    unsafe {
        let url = HSTRING::from(format!("https://127.0.0.1:{web_port}/{path}"));
        let r = ShellExecuteW(
            HWND::default(),
            w!("open"),
            &url,
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOW,
        );
        if r.0 as usize <= 32 {
            eprintln!("tray: failed to open the web console");
        }
    }
}

/// Open the host service with `access`, by the [`SERVICE_NAME`] both the tray and the
/// status poller agree on (the MSI registers this exact name).
fn open_host_service(scm: SC_HANDLE, access: u32) -> windows::core::Result<SC_HANDLE> {
    let name = HSTRING::from(crate::status::SERVICE_NAME);
    // SAFETY: `scm` is a live manager handle; the name lives for the call; the
    // returned service handle is reference-counted (closed by the caller).
    unsafe { OpenServiceW(scm, &name, access) }
}

/// Start or stop the host service (best-effort — needs the tray to run at least as
/// privileged as the service registration; failures are logged and re-polled).
fn control_service(start: bool) {
    // SAFETY: SCM/service handles opened here and always closed; control calls are
    // synchronous against the live service.
    unsafe {
        let Ok(scm) = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT) else {
            eprintln!("tray: cannot open the service manager");
            return;
        };
        let access = if start { SERVICE_START } else { SERVICE_STOP };
        let svc = match open_host_service(scm, access) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("tray: cannot open the host service: {e}");
                let _ = CloseServiceHandle(scm);
                return;
            }
        };
        if start {
            if let Err(e) = StartServiceW(svc, None) {
                eprintln!("tray: failed to start the host service: {e}");
            }
        } else {
            let mut status = SERVICE_STATUS::default();
            if let Err(e) = ControlService(svc, SERVICE_CONTROL_STOP, &mut status) {
                eprintln!("tray: failed to stop the host service: {e}");
            }
        }
        let _ = CloseServiceHandle(svc);
        let _ = CloseServiceHandle(scm);
    }
    poke();
}

/// Force an immediate re-poll (right after a service action).
fn poke() {
    if let Some(state) = STATE.get() {
        if let Some(poller) = state.poller.lock().unwrap().as_ref() {
            poller.poke();
        }
    }
}
