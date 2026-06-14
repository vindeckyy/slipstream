# slipstream on Arch Linux / SteamOS

Packaging for slipstream on Arch and Arch-derived immutable distros (SteamOS 3, etc.). The
`PKGBUILD` is a **split package** producing both **`slipstream-host`** (the gaming-rig host) and
**`slipstream-client`** (the GTK4 couch/Deck client) — mirrors the rpm subpackages
(`packaging/rpm/slipstream.spec`) and the two deb build scripts. On a **Steam Deck you want
`slipstream-client`** (it's what the [Decky plugin](../../clients/decky/) launches); on a gaming
rig, `slipstream-host`.

> ⚠️ **Host encode is NVENC-only today.** `crates/slipstream-host/src/encode/linux.rs` implements
> `hevc_nvenc`/`av1_nvenc`/`h264_nvenc` + a CUDA zero-copy path — there is **no VAAPI encoder**. So
> `slipstream-host` works on **Arch + NVIDIA** (incl. `bazzite-deck-nvidia`); an **AMD Deck-as-host**
> can't encode until a `hevc_vaapi` backend is added (a code change, not packaging). The **client
> is unaffected** — `slipstream-client` decodes via **VAAPI on AMD/Intel** (the Deck) with a software
> fallback, so streaming *to* a Deck works today.

## Arch Linux (mutable)

```sh
cd packaging/arch
# Build the working tree (CI / dev) — no git fetch:
PF_SRCDIR="$(git rev-parse --show-toplevel)" makepkg -f --holdver
# …or build the tagged release the AUR way:
makepkg -si
```
Then the standard first-run (printed by the install scriptlet):
```sh
sudo usermod -aG input "$USER"          # virtual gamepads; re-login after
mkdir -p ~/.config/slipstream
cp /usr/share/slipstream/host.env.bazzite ~/.config/slipstream/host.env   # gamescope backend
systemctl --user enable --now slipstream-host
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

## Files
- `PKGBUILD` — split package: `slipstream-host` + `slipstream-client` (builds the working tree via
  `PF_SRCDIR`, or a git tag for AUR).
- `slipstream-host.install` / `slipstream-client.install` — pacman scriptlets (udev reload + sysctl +
  first-run hint), mirror the RPM `%post` / deb postinst.
- `build-sysext.sh` — wraps either built `.pkg.tar.zst` into a `systemd-sysext` `.raw` for SteamOS
  (derives the name from the package, so it works for host or client).
