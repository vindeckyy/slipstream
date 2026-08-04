---
title: Running as a Service
description: Start the host at boot, for a desktop you log into, or a fully headless always-on machine.
---

Running `serve` in a terminal is fine for trying Slipstream out. To make a machine an
always-available host, run it as a service. That is the usual setup when you leave a workstation at
home and connect from the office ([Desktop at work](/docs/desktop-at-work)). First, what that service
starts, then the two cases, a desktop you log into and a fully headless box.

## What the unit starts

The bundled unit runs `serve --gamestream`, so it serves both the native `slipstream/1` plane and
stock [Moonlight](/docs/moonlight) clients. Every Linux package installs that unit as it ships and
only rewrites the binary path, so a host you installed from apt, dnf, pacman or the Bazzite sysext
has **GameStream on**. See what yours starts:

```sh
systemctl --user cat slipstream-host
```

For a **native-only** host (no GameStream, its pairing runs over plain HTTP and its legacy
encryption is weaker; see [Security & Safe Use](/docs/security)), drop the flag. The packaged unit is
a package file that an upgrade replaces, so override `ExecStart` with a drop-in rather than editing
it:

```sh
systemctl --user edit slipstream-host
```

```ini
[Service]
ExecStart=
ExecStart=/usr/bin/slipstream-host serve
```

The empty `ExecStart=` is required, without it systemd adds a second command instead of replacing
the first, and the path must match the one `systemctl --user cat` printed (the distro packages use
`/usr/bin`). Save, then `systemctl --user restart slipstream-host`.

Windows is the other way round: an install from the setup `.exe` leaves GameStream **off** unless you
tick it, and it is configured differently, see [Windows](#windows) below.

## A. A desktop you log into

If you sit at the machine (or it auto-logs-in to a desktop), run the host as a **systemd user
service** that starts with your session.

**Put your `host.env` in place first.** The unit reads `~/.config/slipstream/host.env` and won't start
until that file exists, no package creates it for you, they only ship a template to copy. The
defaults in it are right for an ordinary desktop; your distro and desktop guides say if yours wants
a different template (on Bazzite it's `host.env.bazzite`):

```sh
mkdir -p ~/.config/slipstream
# /usr/share/slipstream/ on Fedora/Arch/Bazzite, /usr/share/slipstream-host/ on Ubuntu
cp /usr/share/slipstream/host.env.example ~/.config/slipstream/host.env
```

**Installed from a package** (apt, dnf, pacman, or the Bazzite sysext), the unit is already at
`/usr/lib/systemd/user/slipstream-host.service`, with its `ExecStart` pointing at the installed
binary. There's nothing to copy:

```sh
systemctl --user daemon-reload             # the sysext route needs this; harmless elsewhere
systemctl --user enable --now slipstream-host
```

**Built from source**, install the unit from your checkout, and take `host.env` from there too
(`cp scripts/host.env.example ~/.config/slipstream/host.env`). The unit's `ExecStart` points at
`%h/slipstream/target/release/slipstream-host` (`%h` is your home directory), so edit the copy if your
checkout lives somewhere else:

```sh
mkdir -p ~/.config/systemd/user
cp scripts/slipstream-host.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now slipstream-host
```

Don't do the copy on a packaged install: a unit in `~/.config/systemd/user/` shadows the packaged
one, and the source unit points at a build tree you don't have, the service then fails with
`status=203/EXEC`.

The host now starts whenever you log in. Check it with `systemctl --user status slipstream-host`.

**You don't need to export anything for it.** The host finds the live compositor session itself on
every connect and works out where to reach it (`WAYLAND_DISPLAY`, `XDG_RUNTIME_DIR`, the session bus,
sway's `SWAYSOCK`, Hyprland's instance signature) from the running compositor, so `host.env` is for
policy, not session plumbing, and `systemctl --user import-environment` is not a prerequisite.

### Restart the host with your desktop

Add one drop-in so the host follows your session's lifetime:

```sh
mkdir -p ~/.config/systemd/user/slipstream-host.service.d
# /usr/share/slipstream/ on Fedora/Arch/Bazzite, /usr/share/slipstream-host/ on Ubuntu,
# scripts/ in a source checkout
cp /usr/share/slipstream/slipstream-host-desktop-session.conf \
   ~/.config/systemd/user/slipstream-host.service.d/desktop-session.conf
systemctl --user daemon-reload
systemctl --user reenable slipstream-host
systemctl --user restart slipstream-host
```

Without it, restarting Plasma or GNOME, a crash, a log out and back in, "restart the shell", leaves
the host running against a compositor that no longer exists. It keeps listening and answering, and
every session after that fails at capture, which is a confusing way to find out. The drop-in makes a
compositor restart a host restart.

Skip it on the headless/appliance route below (which has its own session unit), and on **Sway or
Hyprland**, which don't hand their session to systemd: they never reach `graphical-session.target`, so
the drop-in is harmless there but does nothing. To make the host come and go with the session on
those, start it from the compositor's own config instead of enabling the unit, add
`exec systemctl --user start slipstream-host` to your sway config, or
`exec-once = systemctl --user start slipstream-host` to Hyprland's, and leave the unit itself
disabled (`systemctl --user disable slipstream-host`), so it isn't also started at login.

## B. A headless, always-on host

To run with **no monitor and no login**, a machine in a closet that's always ready, you need two
things: a desktop session that comes up at boot, and the host service started without a login.

Start by making the host service start at boot even when nobody logs in:

```sh
sudo loginctl enable-linger "$USER"
```

Then bring up a session automatically. How you do that is desktop-specific, auto-login, lock
disable, and the session unit differ per compositor, so each is documented on its own page:

- GNOME: [GNOME -> Headless session](/docs/gnome#headless-session).
- KDE Plasma: [KDE -> Headless session](/docs/kde#headless-session).
- Steam / gamescope: [gamescope](/docs/gamescope), the host launches its own session per client, so
  there's no separate session unit.

Once a session comes up at boot, enable the host user service (section A) and reboot. The host comes up
on that session.

### Headless Bazzite

On Bazzite, the host launches its own gamescope/Steam session per client, so you don't need a separate
session unit, see [Bazzite](/docs/bazzite) and [gamescope](/docs/gamescope).

## Windows

> Slipstream has first-class **Linux and Windows** hosts. On Windows it ships as a signed installer
> with an SCM service and a virtual-display driver, including Slipstream's own **indirect display
> driver** the host pushes frames straight into. The Windows host is newer than the Linux host. (Not
> to be confused with the Windows *client*, which streams *to* a Windows PC.)

On Windows the host runs as a `LocalSystem` service that launches into the interactive session, so it
captures the secure desktop (UAC / lock screen) and survives reboots with nobody logged in, the same
model Sunshine/Apollo use. Because it runs at that privilege level, keep it on a trusted network and be
deliberate about which machine you host on, see [Security & Safe Use](/docs/security).

The easy path is the **signed installer**: download `slipstream-host-setup-<ver>.exe` from
[GitHub Releases](https://github.com/vindeckyy/slipstream/releases) when attached (or build from
`packaging/windows/`) and run it. It drops the host
into `C:\Program Files\slipstream`, installs the bundled **ss-vdisplay** virtual-display driver, and
registers + starts the service for you (`/VERYSILENT` for unattended). Upgrades and uninstall are
handled through Add/Remove Programs.

Prefer the CLI? Run `slipstream-host service install` from an elevated prompt, see
[Windows Host](/docs/windows-host). For hardware encode you need a GPU, NVIDIA (NVENC), AMD (AMF), or
Intel (QSV); the host falls back to software H.264 without one.

**GameStream on Windows.** Unlike the Linux unit, the installer leaves Moonlight compatibility
**off**, it's a checkbox in the wizard (`/MERGETASKS="gamestream"` to select it unattended). There's
no `ExecStart` to edit here: the service launches whatever `SLIPSTREAM_HOST_CMD` in
`%ProgramData%\slipstream\host.env` says, which is also where the rest of the Windows host's
configuration lives. To change it later, from an elevated prompt:

```powershell
slipstream-host service install --gamestream=on   # or --gamestream=off
slipstream-host service restart
```

Registering the service by hand is the exception. A bare `slipstream-host service install` writes a
fresh `host.env` with `SLIPSTREAM_HOST_CMD` left commented out, and with no value set the service
falls back to `serve --gamestream`, so add `--gamestream=off` to that command if you want the
native-only host.

> **Firewall scope.** The installer opens the streaming + console ports on **Private and Domain**
> networks only, not **Public**. If your LAN is (mis)classified Public, clients won't connect until
> you set it to Private (Windows Settings -> Network), and the host logs a warning when it's on a Public
> network. For a trusted network Windows insists is Public, tick **"Allow connections on Public
> networks"** at install (or pass `--allow-public-network` to `service install`). See
> [Security & Safe Use](/docs/security) for the reasoning.

## Verifying

After a reboot, from another machine on the network:

```sh
slipstream reachable 192.168.1.50   # exit 0 = the host answered, 2 = it didn't
slipstream hosts list --probe       # every saved host, online or offline
```

`slipstream` is the headless client CLI, it ships in the Linux client packages (`slipstream-client`)
and with the Windows client. From a source checkout, `slipstream-probe --discover` browses the LAN
instead; it's a dev tool and isn't packaged. Or just open a native client / Moonlight and look for
the host.

If the host answers, it's up. If not, check `journalctl --user -u slipstream-host` on the host, on
a Windows host, run `slipstream-host service status` from an elevated prompt on the machine itself.

## Stopping and removing

After a Linux package update the user service keeps running the old binary until it's restarted, and
a package can't restart another user's `--user` units for you, [Updating](/docs/updating) has the
update command for every install method and the restart that finishes the job. The Windows installer
restarts its own service.

To stop the host for now:

```sh
systemctl --user stop slipstream-host          # Linux
slipstream-host service stop                   # Windows, elevated prompt
```

To stop it for good, so it doesn't come back at login or boot:

```sh
# add slipstream-web (the console) and slipstream-kde-session (the headless KDE route) if you enabled them
systemctl --user disable --now slipstream-host
rm -rf ~/.config/systemd/user/slipstream-host.service.d   # any drop-ins you added
sudo loginctl disable-linger "$USER"                     # only if you enabled lingering
```

On Windows, `slipstream-host service uninstall` from an elevated prompt stops the service, removes it,
and removes the firewall rules it added. To remove the whole install instead, use Add/Remove Programs.

Neither removes `~/.config/slipstream` (Linux) or `%ProgramData%\slipstream` (Windows), your
certificate, pairings and console password stay, so a reinstall picks up where you left off. See
[Uninstall](/docs/uninstall) to clear them out.
