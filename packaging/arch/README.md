# slipstream on Arch Linux / SteamOS

Packaging for slipstream on Arch and Arch-derived immutable distros. The `PKGBUILD` is a **split
package** producing **`slipstream-host`** (the gaming-rig host) and **`slipstream-client`** (the native
GTK4/libadwaita Linux client) — mirrors the rpm subpackages (`packaging/rpm/slipstream.spec`) and the
deb build scripts. On a **Steam Deck used as a client you want `slipstream-client`** (it's what the
[Decky plugin](../../clients/decky/) launches); on a gaming rig, `slipstream-host`.

> **Steam Deck as a HOST:** don't use this PKGBUILD — SteamOS's read-only root makes `makepkg`/sysext
> awkward, and a prebuilt binary breaks on OS library bumps. Use the on-device build script instead:
> **[`scripts/steamdeck/install.sh`](../../scripts/steamdeck/)** (it builds in a Debian-trixie distrobox
> ABI-matched to SteamOS and uses **VAAPI** on the Deck's AMD GPU). The Deck host path is the one
> exception to "host encode is NVENC-only" below.

A third member, **`slipstream-web`** (the browser management console — pairing + status), is
**opt-in**: build it by setting `PF_WITH_WEB=1`, which requires **`bun`** at build time (`bun-bin`
from the AUR if it isn't in your repos). bun is also the **runtime** — the console serves HTTPS
(HTTP/1.1 over TLS) via `Bun.serve`, so the package vendors the bun binary (no `nodejs` dependency). A
default `makepkg` builds only host+client with no JS tooling — mirroring the RPM spec's `%bcond_with web`.

> **Host encode: NVENC on NVIDIA, VAAPI on AMD/Intel** (`SLIPSTREAM_ENCODER=auto` picks one). The host
> now has a VAAPI encoder + zero-copy dmabuf path alongside NVENC/CUDA, so `slipstream-host` works on
> Arch + NVIDIA **and** AMD/Intel (incl. the Steam Deck — see the on-device path above). The client
> decodes via VAAPI on AMD/Intel with a software fallback.

## Install from the binary repo (recommended)

CI (`.github/workflows/arch.yml`) builds this PKGBUILD in an `archlinux:base-devel` container on
every push and publishes the packages to the **GitHub Arch package registry** — a plain pacman
repo, so an Arch box installs and updates slipstream with `pacman -Syu` like everything else.
Two repos mirror the deb/rpm channels: `slipstream` (release tags) and `slipstream-canary`
(rolling main-branch builds, versioned `X.Y.Z-0.<run#>` so a later release always outranks
them). Enable exactly one.

The registry **signs the repo database and every package**, so first import its key into
pacman's keyring (a one-time step — after this, packages install signature-verified):

```sh
# 1. Trust the registry signing key.
curl -fsS https://github.com/vindeckyy/slipstream/api/packages/unom/arch/repository.key \
  | sudo pacman-key --add -
sudo pacman-key --lsign-key E0CA04465C99C936E0B0C6510A317015A34DDD69

# 2. Add the repo (pick ONE channel — slipstream for releases, slipstream-canary for main builds).
#    printf, not a heredoc, so this works in fish too (CachyOS's default shell has no `<<EOF`).
printf '\n[slipstream]\nServer = https://github.com/vindeckyy/slipstream/api/packages/unom/arch/$repo/$arch\n' \
  | sudo tee -a /etc/pacman.conf >/dev/null

# 3. Sync + install.
sudo pacman -Sy slipstream-host        # gaming rig
sudo pacman -Sy slipstream-client      # the native GTK4 Linux client
sudo pacman -Sy slipstream-web         # optional browser management console
```

(No `SigLevel` line needed — pacman's default `Required DatabaseOptional` verifies the signed
packages against the key you just trusted. Arch is rolling, so the packages are built against
current Arch sonames — keep the box itself updated too.)

Then the same first-run steps as a source build (printed by the install scriptlet): `input`
group, `host.env`, `systemctl --user enable --now slipstream-host` — see the next section.

## Build from source — Arch Linux (mutable)

```sh
cd packaging/arch
# Build the working tree (CI / dev) — no git fetch:
PF_SRCDIR="$(git rev-parse --show-toplevel)" makepkg -f --holdver
# …or build the tagged release the AUR way:
makepkg -si
# …add the web console too (needs bun / bun-bin):
PF_WITH_WEB=1 PF_SRCDIR="$(git rev-parse --show-toplevel)" makepkg -f --holdver
```
Then the standard first-run (printed by the install scriptlet):
```sh
sudo usermod -aG input "$USER"          # virtual gamepads; re-login after
mkdir -p ~/.config/slipstream
cp /usr/share/slipstream/host.env.bazzite ~/.config/slipstream/host.env   # gamescope backend
systemctl --user enable --now slipstream-host
# Web console (if you installed the slipstream-web package): enable it + read the login password.
systemctl --user enable --now slipstream-web
journalctl --user -u slipstream-web-init | sed -n 's/.*password generated: //p'   # open https://<host-ip>:47992
```
NVENC/EGL come from the NVIDIA driver: `sudo pacman -S --needed nvidia-utils`. Arch's stock
`ffmpeg` already has NVENC built in — no RPM-Fusion-style swap needed (unlike Fedora).

### Runtime dependency map (Fedora/Debian → Arch)

| Need | Arch package |
|------|--------------|
| FFmpeg + NVENC | `ffmpeg` (NVENC built in) |
| PipeWire + Pulse + session mgr | `pipewire` `pipewire-pulse` `wireplumber` |
| Opus / input injection | `opus` `libei` |
| GL/EGL + gbm + xkb + wayland | `libglvnd` `mesa` `libxkbcommon` `wayland` |
| NVIDIA driver (NVENC/EGL/CUDA) | `nvidia-utils` *(optdepend — never a hard dep)* |
| Compositor backends | `gamescope` (≥3.16.22) / `kwin` / `mutter` / `sway` *(optdepends)* |

## SteamOS 3 (immutable) — use a systemd-sysext

SteamOS has a **read-only `/usr` on A/B partitions**, and every OS update reimages the rootfs —
so `steamos-readonly disable` + `pacman` (and flatpak/distrobox) are fragile or unusable for a
host that needs `/dev/uinput`, `/dev/uhid`, the host PipeWire socket, the GPU render node, and the
right to spawn a compositor. The update-survivable, SteamOS-blessed mechanism is a
**systemd-sysext**: an overlay image merged read-only over `/usr` at boot, living in the writable
`/var/lib/extensions/` (so it persists across A/B updates, no readonly-disable).

Build the package, then wrap its `/usr` payload into a sysext image:
```sh
# 1. build the pacman package (needs an Arch environment / container)
cd packaging/arch && PF_SRCDIR="$(git rev-parse --show-toplevel)" makepkg -f --holdver
# 2. turn it into a sysext .raw (extracts the package's /usr into an image + extension-release)
bash build-sysext.sh slipstream-host-*.pkg.tar.zst
# 3. on the SteamOS box:
sudo cp slipstream-host.raw /var/lib/extensions/
sudo systemctl enable --now systemd-sysext      # merges it; survives OS updates
systemctl --user enable --now slipstream-host     # the user unit is now under /usr/lib
```
The udev rule, sysctl, and systemd **user** unit all live under `/usr/lib`, so the merged sysext
exposes them. `systemd-sysext refresh` re-merges after a reboot.

## Steam Deck — the client (what the Decky plugin launches)

To stream *to* a Deck, you install **`slipstream-client`** there — same sysext mechanism, but
wrapping the client package instead. The split `makepkg` produces both `.pkg.tar.zst` files; on the
Deck use the client one:
```sh
cd packaging/arch && PF_SRCDIR="$(git rev-parse --show-toplevel)" makepkg -f --holdver
bash build-sysext.sh slipstream-client-*.pkg.tar.zst        # → slipstream-client.raw
# on the Deck:
sudo cp slipstream-client.raw /var/lib/extensions/
sudo systemctl enable --now systemd-sysext
sudo pacman -S --needed libva-mesa-driver                  # VAAPI hw decode on the Deck's AMD APU
```
Now `slipstream-client` is on `PATH`, so the **[Decky plugin](../../clients/decky/)** finds and
launches it (`slipstream-client --connect host:port`) — gamescope composites its video like a game.
The client needs no `/dev/uinput` or compositor-spawning rights (it captures input and decodes),
so it's a much lighter sysext than the host.

## Firewall

**Stock Arch ships no firewall** — every port is open by default, so there is nothing to do.
Spins that enable one **do not** get their ports opened for you: an Arch package never touches the
admin's running firewall. **CachyOS is the common case** — it ships `ufw` enabled by default (not
firewalld), so out of the box the host is unreachable until you allow it. Some other spins (e.g.
EndeavourOS) enable `firewalld` instead.

The `slipstream-host` package ships openers for **both** — a ufw application profile
(`/etc/ufw/applications.d/slipstream`) and firewalld service definitions
(`/usr/lib/firewalld/services/`) — so enabling is one command whichever you run:

```sh
# ufw (CachyOS, and Ubuntu once you enable ufw) — reads the profile at once, no reload needed:
sudo ufw allow slipstream-native        # the native-only host (the default)
sudo ufw allow slipstream-gamestream    # …or add this for the Moonlight/GameStream host

# firewalld (EndeavourOS and other Fedora-like spins):
sudo firewall-cmd --reload                                        # pick up the installed def
sudo firewall-cmd --permanent --add-service=slipstream-native
#                              --add-service=slipstream-gamestream  # …for the Moonlight host
sudo firewall-cmd --reload
```

`slipstream-gamestream` opens the fixed Moonlight ports + mDNS; `slipstream-native` opens the QUIC
control port (UDP 9777) + mDNS. Enable both if the host runs `serve --gamestream` (which serves
both planes). The **data plane is an *ephemeral* UDP port** the client opens with a hole-punch, so
there is no fixed data port in either service — the host streams back out through the path the
client opened, which any firewall that allows outbound UDP (the default) passes. The mgmt REST API
(TCP 47990) binds to loopback by default — leave it closed unless you move it off loopback with
`--mgmt-bind IP:PORT` (which then requires `--mgmt-token`).

If you installed the **web console** (`slipstream-web`) and want it reachable from another device,
open its port with the matching one-liner — `sudo ufw allow slipstream-web` or `sudo firewall-cmd
--permanent --add-service=slipstream-web && sudo firewall-cmd --reload` — which opens **TCP 47992**
(HTTPS, login-gated). The mgmt API (47990) stays loopback-only.

Prefer explicit rules (or a firewall the shipped profiles don't cover)? Open the ports directly.
The **native `slipstream/1`** plane:

- **QUIC control plane: UDP 9777** (`serve --native-port N` to change).
- **Data plane: an *ephemeral* UDP port** the client hole-punches — nothing to open inbound as long
  as outbound UDP is allowed (the host streams back out through the client-opened path).

And the **GameStream / Moonlight** ports (fixed) — only needed if you run the host with
`serve --gamestream` (opt-in, trusted LAN only); bare `serve` is native-only and doesn't open these:

| Port | Proto | Purpose |
|---|---|---|
| 47984 | TCP | HTTPS nvhttp (paired, mutual-TLS) |
| 47989 | TCP | HTTP nvhttp (`/serverinfo`, `/pair` PIN flow) |
| 48010 | TCP | RTSP handshake |
| 47998–48010 | UDP | Video RTP (+ FEC), ENet control (47999), audio (48000) |
| 5353 | UDP | mDNS auto-discovery |

The mgmt API (TCP 47990) binds to loopback by default — leave it closed unless you move it off
loopback with `--mgmt-bind IP:PORT` (which then requires `--mgmt-token`).

With `ufw` (explicit ports, instead of the shipped `slipstream-native`/`slipstream-gamestream` profile):

```sh
sudo ufw allow 9777/udp                                 # slipstream/1 control plane
sudo ufw allow 47984/tcp && sudo ufw allow 47989/tcp && sudo ufw allow 48010/tcp
sudo ufw allow 47998,47999,48000/udp                    # GameStream video/control/audio
sudo ufw allow 5353/udp                                 # mDNS discovery
# The slipstream/1 data plane is an ephemeral UDP port the host hole-punches — nothing to open here.
```

With raw `nftables` (add to your `inet filter input` chain):

```
udp dport 9777 accept                  # slipstream/1 control plane
tcp dport { 47984, 47989, 48010 } accept
udp dport { 47998-48000, 5353 } accept # GameStream video/control/audio + mDNS
# The slipstream/1 data plane is an ephemeral UDP port the host hole-punches — a stateful chain that
# accepts ct state established,related (as this one should) passes the return with nothing extra.
```

## Files
- `PKGBUILD` — split package: `slipstream-host` + `slipstream-client` (builds the working tree via
  `PF_SRCDIR`, or a git tag for AUR).
- `slipstream-host.install` / `slipstream-client.install` — pacman scriptlets (udev reload + sysctl +
  first-run hint, incl. the ufw/firewalld enable command for whichever is present), mirror the RPM
  `%post` / deb postinst.
- The firewall openers are shared across all Linux packaging and live in [`../linux/`](../linux/):
  the ufw application profile (`slipstream.ufw` → `/etc/ufw/applications.d/slipstream`) and the
  firewalld service definitions (`slipstream-native.xml` / `slipstream-gamestream.xml` /
  `slipstream-web.xml` → `/usr/lib/firewalld/services/`). None auto-enabled; see Firewall above.
- `build-sysext.sh` — wraps either built `.pkg.tar.zst` into a `systemd-sysext` `.raw` for SteamOS
  (derives the name from the package, so it works for host or client).
