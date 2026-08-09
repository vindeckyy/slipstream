//! `ss-update`, the root helper behind triggered Linux package updates.
//!
//! Two verbs, one per product:
//!
//! * `ss-update apply` — the HOST, via `slipstream-update.service` (triggered from the web
//!   console).
//! * `ss-update apply-client` — the CLIENT, via `slipstream-client-update.service` (triggered
//!   by `slipstream-client --apply-update`, which is what the Decky plugin's one-tap runs).
//!
//! Both are started by an unprivileged process through polkit, authorised for members of the
//! `slipstream-update` group. **The helper takes zero attacker-influenceable parameters**: no
//! versions, no URLs, no package names from the caller — the verb comes from a root-owned
//! unit's fixed `ExecStart`, the install kind from root-owned markers, the package list from
//! the local package database, and every payload from the distro package manager's own signed
//! repositories. Compromising a trigger yields "run the system's normal update for the
//! slipstream packages", nothing more.
//!
//! Both verbs upgrade every installed `slipstream*` package — a box with both gets both,
//! whichever unit ran. What the verb changes is which marker is read (the two packages cannot
//! own one marker path: that is a hard conflict in deb, rpm and pacman alike) and which binary
//! the **run-the-binary gate** executes afterwards, requiring a clean exit — the
//! CI-green-on-the-wrong-program class (the 0.22.0 clobber) dies there for one binary run's
//! worth of cost. The outcome is written to `/var/lib/slipstream/{,client-}update-result.json`
//! (root-written, world-readable) for the unprivileged caller to read; stdout/stderr land in
//! the unit's journal.

#[cfg(target_os = "linux")]
mod apply;
#[cfg(target_os = "linux")]
mod detect;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
mod mode;
#[cfg(target_os = "linux")]
mod result;
#[cfg(target_os = "linux")]
mod runutil;

#[cfg(target_os = "linux")]
fn main() {
    linux::main();
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("ss-update is a Linux-only root helper");
    std::process::exit(2);
}
