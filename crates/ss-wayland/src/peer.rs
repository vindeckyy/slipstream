//! The check itself: `SO_PEERCRED`, `/proc/<pid>/{exe,comm}`, and `lstat`-before-`connect`.
//!
//! Nothing here trusts a string the peer chose. `comm` and `argv[0]` are only consulted as a
//! fallback for the case where the image genuinely *is* an interpreter (a distribution wrapper
//! script), and only after the kernel's credential and the `/proc` executable link have both been
//! read successfully.

use anyhow::{bail, Context, Result};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use wayland_client::Connection;

/// The `/proc` mount, for callers with no reason to read another tree.
///
/// Tests point [`peer_identity`] at a fixture tree *explicitly* rather than through an environment
/// variable: an override that a process can inherit is an override an attacker can inherit too.
pub const DEFAULT_PROC_ROOT: &str = "/proc";

/// [`DEFAULT_PROC_ROOT`] as a path.
pub fn default_proc_root() -> &'static Path {
    Path::new(DEFAULT_PROC_ROOT)
}

/// Which process may sit behind the socket the caller is about to use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeerRule {
    /// KWin itself. For protocols only KWin implements (`zkde_screencast_unstable_v1`,
    /// `org_kde_kwin_fake_input`): anything else answering there is an impostor, because the host
    /// is about to hand it a virtual output or raw input.
    KWin,
    /// Any process of our own uid with a real executable image. For protocol families several
    /// compositors implement (`ext_data_control_v1`, `zwlr_virtual_pointer_v1`,
    /// `zwlr_screencopy_manager_v1`), where an executable-name allowlist would refuse a legitimate
    /// compositor — and a name allowlist is not a security boundary anyway.
    SameUid,
}

/// What the kernel recorded about the peer at `connect(2)` time (`SO_PEERCRED`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerCredentials {
    /// Peer's process id as the kernel recorded it.
    pub pid: i32,
    /// Peer's effective uid.
    pub uid: u32,
    /// Peer's effective gid.
    pub gid: u32,
}

/// What `/proc/<pid>` says the peer process is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerIdentity {
    /// The pid the entry was read for.
    pub pid: i32,
    /// Owner of the `/proc/<pid>` directory — the process's real uid, from the kernel.
    pub uid: u32,
    /// Resolved `/proc/<pid>/exe`: the executable image the process is running.
    pub exe: PathBuf,
    /// `/proc/<pid>/comm`, capped by the kernel at 15 bytes. Advisory only (see module docs).
    pub comm: String,
}

impl PeerIdentity {
    /// A short diagnostic name. The full path can be a caller-chosen filesystem layout detail, so
    /// logs use the file name.
    pub fn exe_name(&self) -> String {
        self.exe
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "<unknown>".into())
    }
}

/// This process's real uid.
fn own_uid() -> u32 {
    // SAFETY: `getuid` takes no arguments, reads no memory and cannot fail; it only reports this
    // process's real uid. Used to compare against a peer credential, so a wrong value can only
    // cause a refusal, never an acceptance of a foreign process.
    unsafe { libc::getuid() }
}

#[cfg(test)]
/// This process's real gid, for building synthetic credentials in tests.
fn own_gid() -> u32 {
    // SAFETY: `getgid` takes no arguments, reads no memory and cannot fail; it only reports this
    // process's real gid, which the peer checks never consult.
    unsafe { libc::getgid() }
}

/// Peer credentials from `SO_PEERCRED`, or `None` when the socket is not a Unix stream that
/// reports them. A `pid` the kernel cannot attribute (0) is treated as unavailable.
pub fn peer_credentials(stream: &UnixStream) -> Option<PeerCredentials> {
    peer_credentials_fd(stream.as_raw_fd())
}

/// Peer credentials for a raw descriptor, without taking ownership of it.
///
/// The inherited `WAYLAND_SOCKET` descriptor is checked this way *before* it is wrapped in a
/// `UnixStream`: a malformed value must not become an `OwnedFd` that closes some unrelated
/// descriptor when it is dropped.
pub fn peer_credentials_fd(fd: RawFd) -> Option<PeerCredentials> {
    // SAFETY: `libc::ucred` is plain-old-data (integer fields), so all-zero is a valid initial
    // value; `getsockopt` below overwrites it on success and the fields are only read when the
    // call reports success and a full-sized struct was written.
    let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: `cred` is a live, correctly aligned `libc::ucred`; `len` is its exact byte size, and
    // `getsockopt` writes only within that object. `SO_PEERCRED` reads no user-supplied option
    // buffer, and a descriptor that is not a connected Unix socket simply reports failure (it is
    // not closed or otherwise modified here).
    let rc = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut cred as *mut libc::ucred).cast(),
            &mut len,
        )
    };
    if rc != 0 || (len as usize) < std::mem::size_of::<libc::ucred>() || cred.pid <= 0 {
        return None;
    }
    Some(PeerCredentials {
        pid: cred.pid,
        uid: cred.uid,
        gid: cred.gid,
    })
}

/// Read `/proc/<pid>/{exe,comm}` and the directory owner from `proc_root`.
///
/// `None` means the entry could not be read (no such process, `hidepid`, a race with exit). Callers
/// must treat that as a refusal rather than as "nothing to check".
pub fn peer_identity(pid: i32, proc_root: &Path) -> Option<PeerIdentity> {
    let dir = proc_root.join(pid.to_string());
    let uid = std::fs::metadata(&dir).ok()?.uid();
    let exe = resolve_exe(std::fs::read_link(dir.join("exe")).ok()?)?;
    let comm = std::fs::read_to_string(dir.join("comm"))
        .map(|c| c.trim().to_string())
        .unwrap_or_default();
    Some(PeerIdentity {
        pid,
        uid,
        exe,
        comm,
    })
}

/// `/proc/<pid>/exe` on a process whose binary was replaced on disk resolves to
/// `<path> (deleted)`. The process is still the same image at the same path, so drop the marker
/// rather than refusing the whole session after a routine package upgrade.
fn resolve_exe(link: PathBuf) -> Option<PathBuf> {
    let text = link.to_string_lossy();
    let stripped = text.strip_suffix(" (deleted)").unwrap_or(text.as_ref());
    if stripped.is_empty() {
        return None;
    }
    Some(PathBuf::from(stripped))
}

/// `true` when `name` is a KWin executable name.
///
/// Covers `kwin_wayland`, distribution wrappers such as `kwin_wayland_wrapper`, the
/// `.kwin_wayland-wrapped` form KDE hides from process listings, the `--wayland` compatibility
/// entry point, and versioned builds.
fn is_kwin_name(name: &str) -> bool {
    let name = name.trim_start_matches('.').to_ascii_lowercase();
    name == "kwin_wayland"
        || name.starts_with("kwin_wayland-")
        || name == "kwin_wayland_wrapper"
        || name == "kwin_x11"
}

/// Interpreters a KDE wrapper script can hand the compositor role to.
fn is_interpreter(name: &str) -> bool {
    matches!(
        name,
        "sh" | "bash"
            | "dash"
            | "ksh"
            | "zsh"
            | "fish"
            | "python"
            | "python3"
            | "perl"
            | "env"
            | "toybox"
            | "busybox"
    ) || name.starts_with("python3.")
}

/// Whether the peer's *image* is KWin.
///
/// The executable link is the decisive fact. Only when the image is an interpreter — a distribution
/// wrapper script, whose `exe` is `/bin/sh` or `/usr/bin/python3` by construction — does the
/// kernel-recorded `comm` decide, and then only as a name match on top of an already-verified
/// credential, executable and uid.
fn image_is_kwin(identity: &PeerIdentity) -> bool {
    let exe_name = identity.exe_name();
    if is_kwin_name(&exe_name) {
        return true;
    }
    is_interpreter(&exe_name.to_ascii_lowercase()) && is_kwin_name(&identity.comm)
}

/// Apply the caller's rule to a peer, or explain why it is refused.
///
/// Credential and process-table uid must agree with each other *and* with this process; that is the
/// part no peer can forge. The image check then decides which same-uid process is acceptable.
pub fn peer_allowed(
    credentials: &PeerCredentials,
    identity: &PeerIdentity,
    rule: PeerRule,
) -> Result<()> {
    if credentials.pid != identity.pid {
        bail!(
            "credential pid {} does not match the /proc entry read for {}",
            credentials.pid,
            identity.pid
        );
    }
    let us = own_uid();
    if credentials.uid != us {
        bail!(
            "peer uid {} is not this process's uid {}",
            credentials.uid,
            us
        );
    }
    if identity.uid != credentials.uid {
        bail!(
            "peer credential uid {} disagrees with /proc/<pid> owner {}",
            credentials.uid,
            identity.uid
        );
    }
    match rule {
        PeerRule::SameUid => Ok(()),
        PeerRule::KWin if image_is_kwin(identity) => Ok(()),
        PeerRule::KWin => bail!(
            "peer executable {} is not a KWin image (comm {:?})",
            identity.exe.display(),
            identity.comm
        ),
    }
}

/// Whether `pid` may sit behind a socket, judged only from `/proc/<pid>` — no socket needed.
///
/// Used for diagnostics and tests; on a live connection the credential itself always comes from
/// `SO_PEERCRED` (see [`connect_verified`]).
pub fn peer_is(pid: i32, rule: PeerRule, proc_root: &Path) -> bool {
    let Some(identity) = peer_identity(pid, proc_root) else {
        return false;
    };
    let credentials = PeerCredentials {
        pid: identity.pid,
        uid: identity.uid,
        gid: 0,
    };
    peer_allowed(&credentials, &identity, rule).is_ok()
}

/// Resolve the Wayland socket named by the process environment, without connecting.
///
/// Absolute `WAYLAND_DISPLAY` wins outright; a relative one joins `XDG_RUNTIME_DIR`. `None` when
/// either is missing, the display name is empty, or a *relative* `XDG_RUNTIME_DIR` would have to be
/// joined — the caller decides whether that is fatal.
///
/// The relative-runtime-dir refusal is deliberate: the XDG base-directory spec requires
/// `XDG_RUNTIME_DIR` to be absolute, and joining a relative one would root the socket path at
/// whatever this process's working directory happens to be, making a caller-controlled tree the
/// runtime root.
pub fn env_socket_path() -> Option<PathBuf> {
    resolve_env_socket(
        std::env::var_os("WAYLAND_DISPLAY"),
        std::env::var_os("XDG_RUNTIME_DIR"),
    )
}

/// The pure half of [`env_socket_path`]: same rules, but fed the two variables explicitly so the
/// relative/absolute decisions are unit-testable without mutating the process environment.
fn resolve_env_socket(
    display: Option<std::ffi::OsString>,
    runtime: Option<std::ffi::OsString>,
) -> Option<PathBuf> {
    let display = PathBuf::from(display?);
    if display.as_os_str().is_empty() {
        return None;
    }
    if display.is_absolute() {
        return Some(display);
    }
    let runtime = PathBuf::from(runtime?);
    if !runtime.is_absolute() {
        tracing::debug!("XDG_RUNTIME_DIR is not absolute; ignoring the environment Wayland socket");
        return None;
    }
    Some(runtime.join(display))
}

/// Connect to `path` and hand out the connection only if the peer passes `rule`.
///
/// `Ok(None)` means "this candidate is unusable" and the caller moves on: the path does not exist,
/// is a symlink (a planted link must not be followed), is not a socket, is owned by another uid, or
/// the peer behind it failed authentication. Only a real error from building the Wayland
/// connection is returned as `Err`.
pub fn connect_verified(
    path: &Path,
    rule: PeerRule,
    proc_root: &Path,
    what: &str,
) -> Result<Option<Connection>> {
    // `symlink_metadata` is `lstat`: it reports the link itself, so a link at the socket path is
    // visible here instead of being silently followed into a peer-chosen target.
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) => {
            tracing::warn!(errno = ?error.raw_os_error(), "{what} is not present");
            return Ok(None);
        }
    };
    if meta.file_type().is_symlink() {
        tracing::warn!(socket = %socket_name(path), "{what} is a symlink; refusing to follow it");
        return Ok(None);
    }
    if !meta.file_type().is_socket() {
        tracing::warn!(socket = %socket_name(path), "{what} is not a socket");
        return Ok(None);
    }
    if meta.uid() != own_uid() {
        tracing::warn!(socket = %socket_name(path), "{what} is owned by another uid");
        return Ok(None);
    }
    let stream = match UnixStream::connect(path) {
        Ok(stream) => stream,
        Err(error) => {
            // errno only: the socket name can itself be a caller-chosen temp path.
            tracing::warn!(errno = ?error.raw_os_error(), "{what} connect failed");
            return Ok(None);
        }
    };
    match authenticate(&stream, rule, proc_root, what, path) {
        Ok(()) => Ok(Some(
            Connection::from_socket(stream)
                .with_context(|| format!("Wayland socket {}", path.display()))?,
        )),
        Err(reason) => {
            tracing::warn!(socket = %socket_name(path), "{what} rejected: {reason}");
            Ok(None)
        }
    }
}

/// Apply the credential + identity checks to a connected stream.
fn authenticate(
    stream: &UnixStream,
    rule: PeerRule,
    proc_root: &Path,
    what: &str,
    path: &Path,
) -> std::result::Result<(), String> {
    let Some(credentials) = peer_credentials(stream) else {
        return Err(format!("{what} peer credentials unavailable"));
    };
    authenticate_with(credentials, rule, proc_root, what)?;
    tracing::debug!(socket = %socket_name(path), "Wayland peer authenticated");
    Ok(())
}

/// Apply the identity half of the check to credentials already read from the descriptor.
fn authenticate_with(
    credentials: PeerCredentials,
    rule: PeerRule,
    proc_root: &Path,
    what: &str,
) -> std::result::Result<(), String> {
    let Some(identity) = peer_identity(credentials.pid, proc_root) else {
        return Err(format!(
            "{what} peer {} has no readable /proc identity",
            credentials.pid
        ));
    };
    peer_allowed(&credentials, &identity, rule).map_err(|error| format!("{error:#}"))
}

/// Connect to the compositor the process environment names, authenticated.
///
/// Both launch styles are covered: `WAYLAND_SOCKET` (an inherited connected fd, which
/// `wayland_client::Connection::connect_to_env` would take with no peer check at all) and
/// `WAYLAND_DISPLAY` + `XDG_RUNTIME_DIR`.
pub fn connect_env(rule: PeerRule, proc_root: &Path) -> Result<Connection> {
    if let Some(fd) = std::env::var_os("WAYLAND_SOCKET") {
        return connect_inherited_fd(&fd, rule, proc_root);
    }
    let path = env_socket_path().context(
        "no Wayland socket in the environment (WAYLAND_DISPLAY / XDG_RUNTIME_DIR are unset)",
    )?;
    match connect_verified(&path, rule, proc_root, "environment socket")? {
        Some(connection) => Ok(connection),
        None => bail!("the environment-named Wayland socket failed authentication"),
    }
}

/// Set by whichever connection attempt adopts the inherited `WAYLAND_SOCKET` descriptor.
///
/// Reading and removing an environment variable is not atomic, so two concurrent [`connect_env`]
/// calls can both observe the value and both wrap the same descriptor number in an owning
/// `UnixStream`. The first drop would then close a descriptor the second still believes it owns —
/// the launcher's compositor socket, out from under a live session. This flag makes exactly one
/// attempt the winner; a loser fails closed instead of taking ownership of a descriptor it must not
/// close (or use) a second time.
static INHERITED_FD_ADOPTED: AtomicBool = AtomicBool::new(false);

/// Claim the right to adopt the inherited descriptor, or report that another attempt already did.
fn claim_inherited_fd() -> Result<()> {
    if INHERITED_FD_ADOPTED.swap(true, Ordering::SeqCst) {
        bail!("WAYLAND_SOCKET was already adopted by an earlier connection attempt");
    }
    Ok(())
}

/// Mark `fd` close-on-exec.
///
/// The adopted compositor socket is a live, authenticated connection to KWin (or the session
/// compositor) for the rest of the process's life; without `FD_CLOEXEC` every child this process
/// `exec`s inherits it and can speak the host's Wayland session (`exit(2)`-style fd leak). Rust's
/// `UnixStream::connect` sets this on sockets it opens itself; a descriptor that arrives from the
/// launcher does not.
fn set_cloexec(fd: RawFd) -> std::io::Result<()> {
    // SAFETY: `fcntl` with `F_GETFD`/`F_SETFD` reads and writes only this descriptor's own flag
    // word; `fd` is a live descriptor supplied by the caller and no user memory is involved.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    if flags < 0 {
        return Err(std::io::Error::last_os_error());
    }
    if flags & libc::FD_CLOEXEC != 0 {
        return Ok(());
    }
    // SAFETY: as above; `F_SETFD` only stores `arg` into the descriptor's flag word, and
    // `flags | FD_CLOEXEC` preserves every flag that was already set.
    let rc = unsafe { libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) };
    if rc < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// Authenticate and adopt the connected socket passed in `WAYLAND_SOCKET`.
///
/// Three things happen before the descriptor is used. A process-wide claim (see
/// [`INHERITED_FD_ADOPTED`]) makes exactly one attempt the owner, because removing the variable is
/// not atomic and cannot do that by itself. `FD_CLOEXEC` is set so the authenticated compositor
/// socket is not inherited across `exec`. Only then is ownership taken with `from_raw_fd`, matching
/// `wayland-client`'s own contract for this variable; an authentication failure drops the stream
/// (closing the fd once) and returns an error.
fn connect_inherited_fd(
    fd: &std::ffi::OsStr,
    rule: PeerRule,
    proc_root: &Path,
) -> Result<Connection> {
    claim_inherited_fd()?;
    let parsed = fd
        .to_str()
        .and_then(|text| text.parse::<i32>().ok())
        .context("WAYLAND_SOCKET is not a numeric file descriptor")?;
    std::env::remove_var("WAYLAND_SOCKET");
    // Read the credentials off the descriptor *before* claiming it: if the launcher's value is not
    // a connected Unix socket (or the number names some unrelated descriptor), this fails without
    // ever constructing the owned stream that would close it.
    let credentials = peer_credentials_fd(parsed)
        .context("WAYLAND_SOCKET is not a connected Unix socket with peer credentials")?;
    if let Err(error) = set_cloexec(parsed) {
        // This attempt is the one claimed owner and the number is proven to be a connected Unix
        // socket, so closing it here cannot touch an unrelated descriptor.
        // SAFETY: as in the `from_raw_fd` below — owned by this attempt, closed exactly once here.
        unsafe {
            libc::close(parsed);
        }
        return Err(anyhow::Error::from(error)).context("mark WAYLAND_SOCKET close-on-exec");
    }
    // SAFETY: `WAYLAND_SOCKET` carries a connected socket the launcher handed this process, and
    // taking ownership of it exactly once is this variable's documented contract. The number is
    // proven to be a connected Unix socket above, the claim above makes this the only owner, and
    // the peer is authenticated below before the connection is used; on that failure the stream is
    // dropped, closing the descriptor once.
    let stream = unsafe { UnixStream::from_raw_fd(parsed) };
    authenticate_with(credentials, rule, proc_root, "WAYLAND_SOCKET")
        .map_err(anyhow::Error::msg)?;
    Connection::from_socket(stream).context("inherited Wayland socket")
}

/// The socket's file name, for logs that must not leak local filesystem layout.
fn socket_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "<socket>".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// A `/proc` fixture: one pid whose `exe` link and `comm` the test controls.
    struct ProcFixture {
        root: PathBuf,
        pid: i32,
    }

    impl ProcFixture {
        fn new(tag: &str, exe: &str, comm: &str) -> Self {
            let pid = std::process::id() as i32;
            let root = std::env::temp_dir().join(format!("ss-wayland-{tag}-{pid}"));
            let _ = std::fs::remove_dir_all(&root);
            let dir = root.join(pid.to_string());
            std::fs::create_dir_all(&dir).unwrap();
            std::os::unix::fs::symlink(exe, dir.join("exe")).unwrap();
            std::fs::write(dir.join("comm"), format!("{comm}\n")).unwrap();
            Self { root, pid }
        }
    }

    impl Drop for ProcFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn own_credentials() -> PeerCredentials {
        PeerCredentials {
            pid: std::process::id() as i32,
            uid: own_uid(),
            gid: own_gid(),
        }
    }

    /// A socket `connect` can reach, in a temp dir this process owns.
    struct Listener {
        dir: PathBuf,
        path: PathBuf,
        _listener: std::os::unix::net::UnixListener,
    }

    impl Listener {
        fn new(tag: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("ss-wayland-sock-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            let path = dir.join("wayland-0");
            let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
            listener.set_nonblocking(true).unwrap();
            Self {
                dir,
                path,
                _listener: listener,
            }
        }
    }

    impl Drop for Listener {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn a_credentialled_real_socket_is_authenticated() {
        let listener = Listener::new("ok");
        let fixture = ProcFixture::new("ok", "/usr/bin/kwin_wayland", "kwin_wayland");
        let connection = connect_verified(&listener.path, PeerRule::KWin, &fixture.root, "test")
            .unwrap()
            .expect("an authenticated KWin peer must be accepted");
        drop(connection);
        let identity = peer_identity(fixture.pid, &fixture.root).unwrap();
        assert_eq!(identity.uid, own_uid());
    }

    #[test]
    fn a_symlink_at_the_socket_path_is_refused() {
        let listener = Listener::new("link");
        let fixture = ProcFixture::new("link", "/usr/bin/kwin_wayland", "kwin_wayland");
        let link = listener.dir.join("wayland-planted");
        std::os::unix::fs::symlink(&listener.path, &link).unwrap();
        assert!(
            connect_verified(&link, PeerRule::KWin, &fixture.root, "test")
                .unwrap()
                .is_none(),
            "a symlinked socket path must be refused, not followed"
        );
    }

    #[test]
    fn a_regular_file_at_the_socket_path_is_refused() {
        let listener = Listener::new("file");
        let fixture = ProcFixture::new("file", "/usr/bin/kwin_wayland", "kwin_wayland");
        let file = listener.dir.join("not-a-socket");
        let mut handle = std::fs::File::create(&file).unwrap();
        handle.write_all(b"planted").unwrap();
        drop(handle);
        assert!(
            connect_verified(&file, PeerRule::KWin, &fixture.root, "test")
                .unwrap()
                .is_none()
        );
    }

    /// Blocker 2's regression: a process that names itself `kwin_wayland` in `comm` but runs some
    /// other executable is not KWin, and `comm` alone must never be enough.
    #[test]
    fn comm_alone_is_not_identity() {
        let fixture = ProcFixture::new("squat", "/usr/bin/squatter", "kwin_wayland");
        let identity = peer_identity(fixture.pid, &fixture.root).unwrap();
        let refused = peer_allowed(&own_credentials(), &identity, PeerRule::KWin);
        assert!(
            refused.is_err(),
            "comm 'kwin_wayland' over a non-KWin executable must be refused"
        );
    }

    /// The distribution-wrapper case: the image really is an interpreter, so the kernel-recorded
    /// name is the only remaining signal — and it is accepted only because exe/uid already passed.
    #[test]
    fn interpreter_wrapper_is_recognised_by_image_and_name() {
        let fixture = ProcFixture::new("wrap", "/usr/bin/bash", ".kwin_wayland-w");
        let identity = peer_identity(fixture.pid, &fixture.root).unwrap();
        assert!(peer_allowed(&own_credentials(), &identity, PeerRule::KWin).is_ok());
    }

    #[test]
    fn a_foreign_uid_is_refused() {
        let fixture = ProcFixture::new("uid", "/usr/bin/kwin_wayland", "kwin_wayland");
        let identity = peer_identity(fixture.pid, &fixture.root).unwrap();
        let mut credentials = own_credentials();
        credentials.uid += 1;
        assert!(peer_allowed(&credentials, &identity, PeerRule::KWin).is_err());
        assert!(peer_allowed(&credentials, &identity, PeerRule::SameUid).is_err());
    }

    #[test]
    fn a_proc_uid_mismatch_is_refused() {
        let fixture = ProcFixture::new("proc-uid", "/usr/bin/kwin_wayland", "kwin_wayland");
        let mut identity = peer_identity(fixture.pid, &fixture.root).unwrap();
        identity.uid = identity.uid.wrapping_add(1);
        assert!(peer_allowed(&own_credentials(), &identity, PeerRule::SameUid).is_err());
    }

    #[test]
    fn a_missing_executable_link_fails_closed() {
        let pid = std::process::id() as i32;
        let root = std::env::temp_dir().join(format!("ss-wayland-noexe-{pid}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(pid.to_string())).unwrap();
        std::fs::write(root.join(pid.to_string()).join("comm"), "kwin_wayland\n").unwrap();
        assert!(
            peer_identity(pid, &root).is_none(),
            "no /proc/<pid>/exe means no identity, and therefore no acceptance"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_deleted_binary_keeps_its_path() {
        assert_eq!(
            resolve_exe(PathBuf::from("/usr/bin/kwin_wayland (deleted)")).unwrap(),
            PathBuf::from("/usr/bin/kwin_wayland")
        );
    }

    /// `WAYLAND_SOCKET` is a number from the environment, so the check must be able to look at a
    /// descriptor without owning it: a pipe (or any non-socket) reports no credentials and is left
    /// open, which is what stops a malformed value from closing an unrelated descriptor.
    #[test]
    fn a_non_socket_fd_reports_no_credentials_and_stays_open() {
        let mut fds = [0 as libc::c_int; 2];
        // SAFETY: `pipe(2)` writes exactly two descriptors into the two-element array supplied.
        let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert_eq!(rc, 0);
        assert!(peer_credentials_fd(fds[0]).is_none());
        // The read end is still open, so closing it succeeds; the write end still accepts a byte.
        // SAFETY: both descriptors were created by the `pipe(2)` above and are owned by this test,
        // which closes each exactly once.
        let closed = unsafe { libc::close(fds[0]) };
        assert_eq!(
            closed, 0,
            "the check must not close a descriptor it did not own"
        );
        let byte = [1u8];
        // SAFETY: `fds[1]` is live and `byte` is a one-byte stack array, so the write stays in
        // bounds.
        let wrote = unsafe { libc::write(fds[1], byte.as_ptr().cast(), 1) };
        assert_eq!(wrote, 1);
        // SAFETY: `fds[1]` is still owned by this test and is closed exactly once here.
        assert_eq!(unsafe { libc::close(fds[1]) }, 0);
    }

    /// A relative `XDG_RUNTIME_DIR` must never be joined: the join would root the socket at this
    /// process's working directory, so a caller-controlled tree would become the runtime root.
    #[test]
    fn a_relative_runtime_dir_is_refused() {
        assert_eq!(
            resolve_env_socket(
                Some(std::ffi::OsString::from("wayland-0")),
                Some(std::ffi::OsString::from("run/user/1000"))
            ),
            None,
            "a relative XDG_RUNTIME_DIR must not be used as the socket root"
        );
    }

    #[test]
    fn an_absolute_display_ignores_a_relative_runtime_dir() {
        assert_eq!(
            resolve_env_socket(
                Some(std::ffi::OsString::from("/tmp/wayland-9")),
                Some(std::ffi::OsString::from("run/user/1000"))
            ),
            Some(PathBuf::from("/tmp/wayland-9"))
        );
    }

    #[test]
    fn a_relative_display_joins_an_absolute_runtime_dir() {
        assert_eq!(
            resolve_env_socket(
                Some(std::ffi::OsString::from("wayland-0")),
                Some(std::ffi::OsString::from("/run/user/1000"))
            ),
            Some(PathBuf::from("/run/user/1000/wayland-0"))
        );
    }

    /// The adopted descriptor outlives the session and must not cross `exec`: `FD_CLOEXEC` is set
    /// before the socket is wrapped, so a child of this process cannot speak the compositor session.
    #[test]
    fn an_adopted_descriptor_is_close_on_exec() {
        let mut fds = [0 as libc::c_int; 2];
        // SAFETY: `socketpair(2)` writes exactly two descriptors into the two-element array; both
        // ends are owned by this test and closed exactly once at the end.
        let rc = unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) };
        assert_eq!(rc, 0);
        // The same pre-flight check `connect_inherited_fd` runs: a connected Unix socket.
        assert!(peer_credentials_fd(fds[0]).is_some());
        set_cloexec(fds[0]).unwrap();
        // SAFETY: `fcntl` reads the descriptor flags of `fds[0]`, a live descriptor owned here.
        let flags = unsafe { libc::fcntl(fds[0], libc::F_GETFD) };
        assert_ne!(
            flags & libc::FD_CLOEXEC,
            0,
            "an adopted compositor socket must be close-on-exec"
        );
        // SAFETY: both ends were created by the `socketpair(2)` above and are owned by this test,
        // which closes each exactly once.
        unsafe {
            assert_eq!(libc::close(fds[0]), 0);
            assert_eq!(libc::close(fds[1]), 0);
        }
    }

    /// Reading and removing `WAYLAND_SOCKET` is not atomic, so exactly one attempt may adopt the
    /// descriptor; a second must fail closed rather than double-own (and double-close) the number.
    #[test]
    fn the_inherited_fd_has_a_single_winner() {
        assert!(claim_inherited_fd().is_ok(), "the first attempt claims it");
        let second = claim_inherited_fd();
        assert!(second.is_err(), "a second attempt must not adopt it again");
        assert!(
            format!("{:#}", second.unwrap_err()).contains("already adopted"),
            "the refusal explains itself"
        );
    }
}
