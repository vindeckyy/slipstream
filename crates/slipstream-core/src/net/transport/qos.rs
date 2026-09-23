//! Shared UDP socket tuning for the media planes: send/recv buffer growth + best-effort link-layer
//! QoS.
//!
//! [`grow_socket_buffers`] is the `SO_SNDBUF`/`SO_RCVBUF` growth the native data plane applies; the
//! GameStream video/audio sockets reuse it so they don't go ENOBUFS-bound at high bitrate.
//!
//! [`set_media_qos`] DSCP-tags the latency-sensitive video/audio traffic (+ Linux `SO_PRIORITY`) so a
//! QoS-aware path (Wi-Fi WMM access categories, a managed switch, a shaped uplink) can prioritize it
//! over bulk flows. Mirrors what Apollo/Sunshine tag, DSCP **CS5** for video, **CS6** for audio. It
//! is **opt-in** (`SLIPSTREAM_DSCP=1`, or [`set_dscp_default`] from an embedder — the Android client
//! ties it to its experimental low-latency mode): DSCP can interact badly with some consumer
//! ISPs/routers, so the caller opts in explicitly and holds the returned [`QosFlow`] guard for as
//! long as the socket sends media.

use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};

/// Target kernel socket-buffer size (`SO_SNDBUF`/`SO_RCVBUF`). A high-resolution frame is a burst (a
/// 5120×1440 keyframe is ~130 packets the send thread hands to `sendmmsg` at once); the default UDP
/// buffer (~208 KB on Linux) overflows on it, which EAGAINs the host send (dropping packets) or drops
/// on the client recv — and with infinite-GOP a single lost frame freezes the decode until the next
/// RFI refresh. Requested large; the OS clamps to `net.core.{wmem,rmem}_max` (Linux) /
/// `kern.ipc.maxsockbuf` (macOS).
///
/// Sized for 1 Gbps+: at ~1.2 Gbps on the wire an 8 MB buffer is only ~49 ms of steady state, and a
/// single multi-MB IDR keyframe (~4 MB ≈ 3300 packets) instantly fills most of it. 32 MB gives ~200 ms
/// of headroom and absorbs a keyframe burst without EAGAIN/ENOBUFS drops. (Paced sending —
/// `native.rs::paced_submit` — spreads a big frame's overflow, so this buffer mostly absorbs the
/// immediate microburst rather than a whole unpaced frame.)
pub(crate) const TARGET_SOCKBUF: usize = 32 * 1024 * 1024;

/// Best-effort grow of `SO_SNDBUF`/`SO_RCVBUF` to [`TARGET_SOCKBUF`]. A failure isn't fatal (the
/// stream just runs lossier); a grant far below the request means the OS cap is too low for clean
/// 4K/5K streaming, so warn with the knob to raise.
pub fn grow_socket_buffers(socket: &UdpSocket) {
    set_socket_buffers(socket, TARGET_SOCKBUF, TARGET_SOCKBUF);
}

/// The `LatencyProfile::LowLatency` send-buffer target: approximately TWO frame payloads,
/// bounded between 256 KiB and 4 MiB — the opposite of the 32 MiB balanced queue, which lets a
/// WAN burst hide behind the socket.
pub fn low_latency_send_target(frame_bytes: usize) -> usize {
    frame_bytes
        .saturating_mul(2)
        .clamp(256 * 1024, 4 * 1024 * 1024)
}

/// Apply a send-buffer target and report the granted `SO_SNDBUF` (post kernel clamping). The
/// receive buffer keeps the balanced target (recv buffering is the client's problem, and
/// clamping it here would only add host-side loss).
pub fn set_send_buffer(socket: &UdpSocket, send_target: usize) -> u64 {
    set_socket_buffers(socket, send_target, TARGET_SOCKBUF);
    socket2::SockRef::from(socket)
        .send_buffer_size()
        .unwrap_or(0) as u64
}

/// Set `SO_SNDBUF`/`SO_RCVBUF` to `send_target`/`recv_target` and warn when the grant falls far
/// short of the send target (the OS cap is the usual cause — `net.core.wmem_max`).
fn set_socket_buffers(socket: &UdpSocket, send_target: usize, recv_target: usize) {
    let sock = socket2::SockRef::from(socket);
    let _ = sock.set_send_buffer_size(send_target);
    let _ = sock.set_recv_buffer_size(recv_target);
    // The kernel reports back the (possibly clamped, Linux-doubled) granted size.
    let granted = sock.send_buffer_size().unwrap_or(0);
    if granted < send_target / 4 {
        #[cfg(target_os = "linux")]
        let knob = "raise net.core.wmem_max / net.core.rmem_max";
        #[cfg(target_os = "windows")]
        let knob = "Windows granted less than requested; large IDR bursts may drop";
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        let knob = "raise the OS UDP socket-buffer cap";
        tracing::warn!(
            granted_kb = granted / 1024,
            target_kb = send_target / 1024,
            "{knob} — high-resolution streaming may drop frames"
        );
    }
}

/// Media class of a socket — selects the DSCP code point (and Linux `SO_PRIORITY`), matching Apollo's
/// mapping: video = CS5, audio = CS6.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaClass {
    Video,
    Audio,
}

impl MediaClass {
    /// DSCP code point (the high 6 bits of the IPv4 TOS / IPv6 traffic-class byte).
    pub(super) const fn dscp(self) -> u32 {
        match self {
            MediaClass::Video => 40, // CS5
            MediaClass::Audio => 48, // CS6
        }
    }
}

/// Runtime default for DSCP marking when `SLIPSTREAM_DSCP` is unset (see [`set_dscp_default`]).
/// Off unless an embedder opts in — on Wi-Fi, access points commonly map DSCP to WMM access
/// categories (a real airtime-priority win), but wired paths rarely honour it and some bleach or
/// reject marked packets, so it never turns on by itself.
static DSCP_DEFAULT: AtomicBool = AtomicBool::new(false);

/// Opt in to (or back out of) DSCP marking for sockets created from now on. Must be called BEFORE
/// connecting — the tag is applied at socket creation. The Android client ties this to its
/// experimental low-latency mode; `SLIPSTREAM_DSCP` still overrides in either direction.
pub fn set_dscp_default(enabled: bool) {
    DSCP_DEFAULT.store(enabled, Ordering::Relaxed);
}

/// Whether DSCP/QoS marking is enabled: `SLIPSTREAM_DSCP` when set (`1`/`true`/`on` forces it on,
/// `0`/`false`/`off` forces it off — e.g. to rule QoS out while debugging a flaky AP), else the
/// [`set_dscp_default`] runtime default.
pub(crate) fn dscp_enabled() -> bool {
    match std::env::var("SLIPSTREAM_DSCP").as_deref() {
        Ok("1") | Ok("true") | Ok("on") => true,
        Ok("0") | Ok("false") | Ok("off") => false,
        _ => DSCP_DEFAULT.load(Ordering::Relaxed),
    }
}

/// RAII token retained by the transport while a media socket is active. Linux applies DSCP through
/// the socket option itself, so this token stays unconstructed and [`set_media_qos`] returns
/// `None`. Windows holds the qWAVE flow open for the socket's life — closing the handle removes
/// the DSCP marking.
pub struct QosFlow {
    #[cfg(target_os = "windows")]
    _keep: qwave::Session,
    #[cfg(not(target_os = "windows"))]
    _never: std::convert::Infallible,
}

/// Best-effort: tag `socket`'s outgoing packets for prioritized delivery of its media class. A no-op
/// unless `SLIPSTREAM_DSCP=1`. Every step is best-effort (failures logged at debug, never fatal) — QoS
/// is a nicety, not required for correctness.
///
/// The socket must already be `connect`ed. IPv4 only (all current media sockets bind `0.0.0.0`);
/// a v6 socket simply isn't tagged. Returns `None` whenever a step refused.
pub fn set_media_qos(socket: &UdpSocket, class: MediaClass) -> Option<QosFlow> {
    if !dscp_enabled() {
        return None;
    }
    apply_media_qos(socket, class);
    #[cfg(target_os = "windows")]
    if let Some(session) = qwave::attach(socket, class) {
        return Some(QosFlow { _keep: session });
    }
    None
}

/// qWAVE (`qwave.dll`) flow for a connected UDP socket. Windows ignores `IP_TOS` unless a QoS
/// flow is attached; the DLL is resolved at runtime so a missing system component is a skip,
/// not a link failure. The session is `Send` because the socket and the flow live on the send
/// thread that holds the [`QosFlow`] guard.
#[cfg(target_os = "windows")]
mod qwave {
    use super::MediaClass;
    use std::ffi::c_void;
    use std::net::UdpSocket;
    use std::os::windows::io::AsRawSocket;

    #[repr(C)]
    struct QosVersion {
        major: u16,
        minor: u16,
    }

    struct Api {
        create: unsafe extern "system" fn(*const QosVersion, *mut *mut c_void) -> i32,
        add:
            unsafe extern "system" fn(*mut c_void, usize, *const c_void, u32, u32, *mut u32) -> i32,
        set_flow: unsafe extern "system" fn(
            *mut c_void,
            u32,
            u32,
            u32,
            *const u32,
            u32,
            *mut c_void,
        ) -> i32,
        remove: unsafe extern "system" fn(*mut c_void, usize, u32, u32) -> i32,
        close: unsafe extern "system" fn(*mut c_void) -> i32,
    }

    pub(super) struct Session {
        handle: *mut c_void,
        flow: u32,
        socket: usize,
        api: &'static Api,
    }

    // SAFETY: the handle is an opaque qWAVE object. After `attach` returns, the only use is
    // `Drop`, which takes `&mut self` and therefore cannot race a shared reference. The
    // transport stores the session behind the socket that is itself `Sync`; no `&Session`
    // method touches the pointer, so sharing the guard across threads does not share a qWAVE
    // call. Drop still runs exactly once.
    unsafe impl Send for Session {}
    // SAFETY: see the `Send` impl above — no `&Session` method touches the qWAVE pointer,
    // so a shared reference cannot race a qWAVE call. Drop still runs exactly once.
    unsafe impl Sync for Session {}

    impl Drop for Session {
        fn drop(&mut self) {
            // SAFETY: `handle` was returned by `QOSCreateHandle` and the flow id by
            // `QOSAddSocketToFlow` on this socket. Both are released once, here.
            unsafe {
                let _ = (self.api.remove)(self.handle, self.socket, self.flow, 0);
                let _ = (self.api.close)(self.handle);
            }
        }
    }

    fn api() -> Option<&'static Api> {
        static TABLE: std::sync::OnceLock<Option<Api>> = std::sync::OnceLock::new();
        TABLE
            .get_or_init(|| {
                // SAFETY: `qwave.dll` is a Windows system library. Symbols are the documented
                // qWAVE exports; each pointer is copied out before the library is leaked for
                // the process lifetime.
                unsafe {
                    let lib = libloading::Library::new("qwave.dll").ok()?;
                    let create = lib
                        .get::<unsafe extern "system" fn(*const QosVersion, *mut *mut c_void) -> i32>(
                            b"QOSCreateHandle\0",
                        )
                        .ok()?;
                    let add = lib
                        .get::<unsafe extern "system" fn(
                            *mut c_void,
                            usize,
                            *const c_void,
                            u32,
                            u32,
                            *mut u32,
                        ) -> i32>(b"QOSAddSocketToFlow\0")
                        .ok()?;
                    let set_flow = lib
                        .get::<unsafe extern "system" fn(
                            *mut c_void,
                            u32,
                            u32,
                            u32,
                            *const u32,
                            u32,
                            *mut c_void,
                        ) -> i32>(b"QOSSetFlow\0")
                        .ok()?;
                    let remove = lib
                        .get::<unsafe extern "system" fn(*mut c_void, usize, u32, u32) -> i32>(
                            b"QOSRemoveSocketFromFlow\0",
                        )
                        .ok()?;
                    let close = lib
                        .get::<unsafe extern "system" fn(*mut c_void) -> i32>(b"QOSCloseHandle\0")
                        .ok()?;
                    let api = Api {
                        create: *create,
                        add: *add,
                        set_flow: *set_flow,
                        remove: *remove,
                        close: *close,
                    };
                    std::mem::forget(lib);
                    Some(api)
                }
            })
            .as_ref()
    }

    pub(super) fn attach(socket: &UdpSocket, class: MediaClass) -> Option<Session> {
        let api = api()?;
        let raw = socket.as_raw_socket() as usize;
        let version = QosVersion { major: 1, minor: 0 };
        let mut handle: *mut c_void = std::ptr::null_mut();
        // SAFETY: `version`/`handle` are live locals. `QOSCreateHandle` writes the handle on
        // success. The socket value is the connected UDP socket's `SOCKET`. A null destination
        // marks every datagram on that socket. `QOS_NON_ADAPTIVE_FLOW` (0x2) keeps the flow
        // from throttling the media rate. DSCP is the same code point `apply_media_qos` put
        // in `IP_TOS` (the high 6 bits, unshifted — qWAVE takes the 6-bit value).
        unsafe {
            if (api.create)(&version, &mut handle) == 0 || handle.is_null() {
                return None;
            }
            let mut flow = 0u32;
            let traffic = match class {
                MediaClass::Video => 3, // QOSTrafficTypeAudioVideo
                MediaClass::Audio => 4, // QOSTrafficTypeVoice
            };
            if (api.add)(handle, raw, std::ptr::null(), traffic, 0x2, &mut flow) == 0 {
                let _ = (api.close)(handle);
                return None;
            }
            let dscp = class.dscp();
            if (api.set_flow)(handle, flow, 2, 4, &dscp, 0, std::ptr::null_mut()) == 0 {
                tracing::debug!(
                    ?class,
                    "QOSSetFlow DSCP failed — flow stays at the traffic type"
                );
            }
            Some(Session {
                handle,
                flow,
                socket: raw,
                api,
            })
        }
    }
}

/// The unconditional QoS application, factored out of [`set_media_qos`] so it is directly testable
/// without touching the process-global `SLIPSTREAM_DSCP` env. Best-effort (every step logs-and-continues).
fn apply_media_qos(socket: &UdpSocket, class: MediaClass) {
    let sock = socket2::SockRef::from(socket);
    // DSCP occupies the high 6 bits of the TOS byte → shift left 2.
    if let Err(e) = sock.set_tos_v4(class.dscp() << 2) {
        tracing::debug!(error = %e, ?class, "set IP_TOS (DSCP) failed — QoS marking skipped");
    }
    // SO_PRIORITY must be set AFTER IP_TOS (setting TOS resets SO_PRIORITY to 0 on Linux). Linux-only;
    // 6 is the highest priority allowed without CAP_NET_ADMIN, so video=5 / audio=6 (Apollo's scheme).
    #[cfg(target_os = "linux")]
    {
        let prio = match class {
            MediaClass::Video => 5,
            MediaClass::Audio => 6,
        };
        if let Err(e) = sock.set_priority(prio) {
            tracing::debug!(error = %e, "set SO_PRIORITY failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dscp_code_points_match_apollo() {
        // CS5 video / CS6 audio, shifted into the TOS byte (high 6 bits).
        assert_eq!(MediaClass::Video.dscp(), 40);
        assert_eq!(MediaClass::Audio.dscp(), 48);
        assert_eq!(MediaClass::Video.dscp() << 2, 0xA0);
        assert_eq!(MediaClass::Audio.dscp() << 2, 0xC0);
    }

    #[test]
    fn qos_and_buffer_growth_are_best_effort_and_never_panic() {
        let sock = UdpSocket::bind("127.0.0.1:0").unwrap();
        // No SLIPSTREAM_DSCP in the test env → early return; must not panic regardless.
        assert!(set_media_qos(&sock, MediaClass::Video).is_none());
        assert!(set_media_qos(&sock, MediaClass::Audio).is_none());
        grow_socket_buffers(&sock);
    }

    #[test]
    fn apply_qos_tags_the_socket() {
        // Exercise the enabled path directly (no env), and read the options back where we can.
        let sock = UdpSocket::bind("127.0.0.1:0").unwrap();
        apply_media_qos(&sock, MediaClass::Video);
        #[cfg(target_os = "linux")]
        {
            let s = socket2::SockRef::from(&sock);
            assert_eq!(s.tos_v4().unwrap(), 0xA0, "video → CS5 in the TOS byte");
            assert_eq!(s.priority().unwrap(), 5, "video → SO_PRIORITY 5");
        }
    }
}
