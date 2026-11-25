//! Shared authentication for the host's Wayland compositor connections.
//!
//! Every socket the host opens on a compositor's behalf used to be an unauthenticated door: the
//! pinned KWin socket, the environment-named socket, and the runtime-dir fallback were all reached
//! with a bare `UnixStream::connect` (or `wayland_client::Connection::connect_to_env`), so any
//! same-user process that could place a socket at the expected path — a symlink, a leftover file, a
//! squatter in the runtime dir — could answer as the compositor. The host would then create virtual
//! outputs, inject input, or hand over the clipboard into an attacker's session.
//!
//! This leaf crate is the one place that decides whether a Wayland peer is the process the caller
//! meant. It does that with kernel-supplied facts only:
//!
//! * `SO_PEERCRED` on the connected socket gives the peer's pid/uid/gid as recorded by the kernel at
//!   `connect(2)` time — not something the peer can forge at its end.
//! * `/proc/<pid>/exe` gives the executable the peer actually exec'd. Unlike `comm` (freely
//!   rewritable with `prctl(PR_SET_NAME)`) and `argv[0]` (just the string the caller passed to
//!   `execve`), `exe` is a property of the running image.
//! * The owner uid of `/proc/<pid>` cross-checks the credential against the process table.
//!
//! `lstat` + symlink rejection run before any connect, so a link planted at a socket path is refused
//! instead of followed. Every check fails closed: an unreadable `/proc` entry, a missing executable
//! link, a credential mismatch, or an unexpected process name is a refusal, never a warning that
//! carries on.
//!
//! System note: this crate has no non-Linux surface. The module is compiled only for Linux, so the
//! crate is empty elsewhere and its callers gate on `cfg(target_os = "linux")` too.

#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(target_os = "linux")]
mod peer;

#[cfg(target_os = "linux")]
pub use peer::{
    connect_env, connect_verified, default_proc_root, env_socket_path, peer_allowed,
    peer_credentials, peer_credentials_fd, peer_identity, peer_is, PeerCredentials, PeerIdentity,
    PeerRule, DEFAULT_PROC_ROOT,
};
