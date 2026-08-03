---
title: Arch Linux
description: Install a Slipstream host on Arch (and Arch-derived distros) with makepkg from this repo.
---

Set up a Slipstream host on **Arch Linux** (or an Arch-derived distro like CachyOS/EndeavourOS).
There is no public pacman binary repo. Build with the split `PKGBUILD` in `packaging/arch/`, or
install packages from [GitHub Releases](https://github.com/vindeckyy/slipstream/releases) when
attached. Host encode is **NVENC on NVIDIA**; on **AMD/Intel** HEVC and AV1 go through **Vulkan
Video**, with **VAAPI** for H.264 and as the fallback (`SLIPSTREAM_ENCODER=auto` picks per GPU).

> New here? Read [Security & Safe Use](/docs/security) first, a streaming host is remote control of
> the machine, so keep it on a trusted LAN or VPN and require pairing.

## 1. GPU prerequisites

- **NVIDIA:** `sudo pacman -S --needed nvidia-utils` (provides NVENC + the EGL/CUDA zero-copy path).
  Arch's stock `ffmpeg` already has NVENC built in, no RPM-Fusion-style swap like Fedora needs.
- **AMD / Intel:** the Mesa stack. HEVC/AV1 encode goes through **Vulkan Video** by default, so
  install the Vulkan driver, `vulkan-radeon` (AMD) or `vulkan-intel` (Intel), alongside the VAAPI
  drivers (`libva-mesa-driver` for AMD, `intel-media-driver` for Intel), which carry H.264 and the
  fallback path. Both are usually already installed on a desktop.

## 2. Build and install

```sh
git clone https://github.com/vindeckyy/slipstream.git && cd slipstream/packaging/arch
PF_SRCDIR="$(git rev-parse --show-toplevel)" makepkg -si
# Optional web console (needs bun / bun-bin):
# PF_WITH_WEB=1 PF_SRCDIR="$(git rev-parse --show-toplevel)" makepkg -si
```

That installs `slipstream-host` (and `slipstream-client` from the same split package). Add
`slipstream-web` when you build with `PF_WITH_WEB=1`. Full PKGBUILD notes:
[`packaging/arch/README.md`](https://github.com/vindeckyy/slipstream/blob/main/packaging/arch/README.md).

```sh
sudo usermod -aG input "$USER"       # /dev/uinput access for virtual gamepads (re-login to apply)
```

`slipstream-scripting` is the runner behind [Plugins](/docs/plugins); enable it when you want it, 
`systemctl --user enable --now slipstream-scripting`. The host package ships the systemd **user**
units, the udev rule, the UDP socket-buffer sysctl tuning, and example configs.

Updates later: rebuild from a newer checkout (or install a newer package from GitHub Releases), then
`systemctl --user restart slipstream-host` so the running host picks up the new binary. Restart
`slipstream-web` the same way if you run the console. Channel notes for when you publish your own
feeds: [Release Channels](/docs/channels). The web console update card is covered in
[Updating the Host](/docs/updating).

## 3. Configure and run

The host runs as a systemd **`--user`** service, it needs your session's PipeWire and D-Bus. Copy a
starting config:

```sh
mkdir -p ~/.config/slipstream
cp /usr/share/slipstream/host.env.example ~/.config/slipstream/host.env
```

How the host creates its virtual display and injects input depends on your desktop, not your distro, 
edit `host.env` for the desktop you run, following its page for the exact settings and any quirks:

- [KDE Plasma (KWin)](/docs/kde)
- [GNOME (Mutter)](/docs/gnome)
- [Steam / gamescope](/docs/gamescope)
- [Hyprland](/docs/hyprland)
- [Sway / wlroots](/docs/sway)

Then enable the service and turn on linger so it starts at boot without a login:

```sh
systemctl --user daemon-reload
systemctl --user enable --now slipstream-host
sudo loginctl enable-linger "$USER"
```

Check it came up:

```sh
systemctl --user status slipstream-host          # active
journalctl --user -u slipstream-host -f          # watch a client connect
```

Enable the browser console, find your login password, and arm PIN pairing from
[The Web Console](/docs/web-console). For a headless KWin appliance that streams at boot with no
graphical login, see [KDE -> Headless session](/docs/kde#headless-session). Full reference:
[Configuration](/docs/configuration) · [Running as a Service](/docs/running-as-a-service).

## 5. Open the firewall (if you have one)

**Stock Arch ships no firewall**, every port is already open, so you can skip this. But **CachyOS
enables `ufw` by default** (firewalld is not installed), and some other spins (e.g. EndeavourOS)
enable **`firewalld`**, an Arch package never opens ports for you, so on those the host is
unreachable until you allow it.

The `slipstream-host` package installs openers for **both**, so it's a one-liner whichever you run.
The unit you enabled in step 4 runs `serve --gamestream`, the package installs it as it ships and
only rewrites the binary path, so that host serves **both** the native `slipstream/1` plane and
stock [Moonlight](/docs/moonlight) clients, and needs **both** openers:

```sh
# ufw, CachyOS (and Ubuntu, once you enable ufw):
sudo ufw allow slipstream-native
sudo ufw allow slipstream-gamestream

# firewalld, Fedora-like spins (EndeavourOS, ...):
sudo firewall-cmd --reload                                        # load the installed definitions
sudo firewall-cmd --permanent --add-service=slipstream-native
sudo firewall-cmd --permanent --add-service=slipstream-gamestream
sudo firewall-cmd --reload
```

Switched the host to **native-only**, dropped `--gamestream` with a
`systemctl --user edit slipstream-host` drop-in, or you run `slipstream-host serve` by hand? Then open
`slipstream-native` alone and leave `slipstream-gamestream` closed. `systemctl --user cat
slipstream-host` shows which one yours is.

`slipstream-native` opens the QUIC control port (UDP 9777), mDNS discovery and the mgmt/library API
(TCP 47990); `slipstream-gamestream` opens the fixed Moonlight ports, TCP 47984, 47989 and 48010,
UDP 47998-48000, plus the same mDNS.
The media **data plane** uses an *ephemeral* UDP port that the client opens with a hole-punch, the
host streams back out through the path the client opened, so there's **nothing fixed to open** as
long as the firewall allows outbound UDP (the default for both ufw and firewalld).

Enabled the **web console** (`slipstream-web`, above) and want to reach it from your phone or another
machine? It's not opened by the streaming rules, open its port too, the same one-liner way:

```sh
sudo ufw allow slipstream-web                                                            # ufw
sudo firewall-cmd --permanent --add-service=slipstream-web && sudo firewall-cmd --reload  # firewalld
```

That opens **TCP 47992** (HTTPS, login-gated). The mgmt API (47990) is opened for paired clients by the
`slipstream-native` profile (game-library browsing over mTLS); off-loopback it serves only read-only
status/library, and every admin action stays loopback-only. Full port lists (`nftables`, explicit ports) are in
[`packaging/arch/README.md`](https://github.com/vindeckyy/slipstream/blob/main/packaging/arch/README.md#firewall).

## 6. Connect a client

From any [client](/docs/clients), `--discover` finds the host on the LAN. On first connect, complete
the **PIN pairing**: arm it from [The Web Console](/docs/web-console#arm-pairing), which displays a
4-digit PIN to type into the client. (Pairing is required by default; pass `serve --open` only if
you deliberately want to disable it.) See [Clients](/docs/clients) for per-platform setup.

## Next steps

- **Keep it current**, [Updating the Host](/docs/updating).
- **Remove it again**, [Uninstalling](/docs/uninstall).
- **Something not working?**, [Troubleshooting](/docs/troubleshooting).

## Appendix, build from source (PKGBUILD)

To build instead of using the binary repo, use the split `PKGBUILD` in `packaging/arch/` (produces
`slipstream-host` + `slipstream-client`; set `PF_WITH_WEB=1` to also build `slipstream-web` and
`PF_WITH_SCRIPTING=1` to also build `slipstream-scripting`, both need `bun`):

```sh
git clone https://github.com/vindeckyy/slipstream.git && cd slipstream/packaging/arch
# Build the working tree (no git fetch):
PF_SRCDIR="$(git rev-parse --show-toplevel)" makepkg -f --holdver
sudo pacman -U slipstream-host-*.pkg.tar.zst
```

NVENC/EGL come from the NVIDIA driver (`nvidia-utils`); on a GPU-less builder, symlink the CUDA
stub into the link path first (the `PKGBUILD` header documents this). Full details, the
Fedora->Arch dependency map, and the systemd-sysext mechanism are in
[`packaging/arch/README.md`](https://github.com/vindeckyy/slipstream/blob/main/packaging/arch/README.md).
(For a **SteamOS host**, use the [on-device installer](/docs/steamos-host) instead, it builds
the host and the HDR gamescope against the running OS.)
