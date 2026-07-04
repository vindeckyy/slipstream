---
title: Arch Linux
description: Install a slipstream host on Arch (and Arch-derived distros) from the signed pacman binary repo.
---

Set up a slipstream host on **Arch Linux** (or an Arch-derived distro like CachyOS/EndeavourOS). The
host installs from a **signed pacman binary repo**, so it updates with `pacman -Syu` like the rest
of your system — no building required. Host encode is **NVENC on NVIDIA** and **VAAPI on
AMD/Intel** (`SLIPSTREAM_ENCODER=auto` picks per GPU).

> New here? Read [Security & Safe Use](/docs/security) first — a streaming host is remote control of
> the machine, so keep it on a trusted LAN or VPN and require pairing.

> Prefer to build it yourself? A split `PKGBUILD` (host + client + optional web console) is in the
> repo at `packaging/arch/` — see the [appendix](#appendix--build-from-source-pkgbuild). The binary
> repo below is the supported path.

## 1. GPU prerequisites

- **NVIDIA:** `sudo pacman -S --needed nvidia-utils` (provides NVENC + the EGL/CUDA zero-copy path).
  Arch's stock `ffmpeg` already has NVENC built in — no RPM-Fusion-style swap like Fedora needs.
- **AMD / Intel:** the Mesa stack (`mesa`, `libva-mesa-driver` for AMD, `intel-media-driver` for
  Intel) provides the VAAPI encoder — usually already installed on a desktop.

## 2. Add the signed repo

The registry **signs its database and every package**, so first trust its key once (after this,
packages install signature-verified):

```sh
# Trust the registry signing key.
curl -fsS https://github.com/vindeckyy/slipstream/api/packages/unom/arch/repository.key \
  | sudo pacman-key --add -
sudo pacman-key --lsign-key E0CA04465C99C936E0B0C6510A317015A34DDD69

# Add the repo (append to /etc/pacman.conf). No SigLevel line needed — pacman's default
# verifies signed packages against the key you just trusted.
sudo tee -a /etc/pacman.conf >/dev/null <<'EOF'

[slipstream]
Server = https://github.com/vindeckyy/slipstream/api/packages/unom/arch/$repo/$arch
EOF
```

> **Stable vs canary.** `[slipstream]` is the **stable** channel — it moves only when a `vX.Y.Z`
> release is cut. For the latest `main` build, use `[slipstream-canary]` instead (same `Server` line,
> just the repo name). Enable exactly one. See [Release Channels](/docs/channels).

## 3. Install the host

```sh
sudo pacman -Sy slipstream-host      # the streaming host
sudo pacman -S  slipstream-web       # optional: the browser management console (pairing + status)
sudo usermod -aG input "$USER"      # /dev/uinput access for virtual gamepads (re-login to apply)
```

`slipstream-client` (the GTK4 couch/Deck client) is in the same repo if this box is also a client.
The host package ships the systemd **user** units, the udev rule, the UDP socket-buffer sysctl
tuning, and example configs. Updates later are just `sudo pacman -Syu`.

## 4. Configure and run

The host runs as a systemd **`--user`** service — it needs your session's PipeWire and D-Bus.
Copy a starting config, enable the service, and enable linger so it starts at boot without a login:

```sh
mkdir -p ~/.config/slipstream
cp /usr/share/slipstream/host.env.example ~/.config/slipstream/host.env   # then edit
systemctl --user daemon-reload
systemctl --user enable --now slipstream-host
sudo loginctl enable-linger "$USER"
```

Which compositor the host captures depends on your desktop — it drives a per-client virtual output
via KWin (Plasma), Mutter (GNOME), or wlroots (Sway), or spawns a headless **gamescope** session
per connect. For a headless appliance, the package also ships `slipstream-kde-session.service`
(a dedicated `kwin --virtual` session, same as the [Fedora KDE](/docs/fedora-kde#3-kwin-streaming-session)
guide — `cp /usr/share/slipstream/host.env.kde ~/.config/slipstream/host.env` and enable it alongside
the host). See [Configuration](/docs/configuration) for every knob and
[Running as a Service](/docs/running-as-a-service) for the service model.

Check it came up:

```sh
systemctl --user status slipstream-host          # active
journalctl --user -u slipstream-host -f          # watch a client connect
```

### Web console

The console (status, paired devices, arm pairing) ships as `slipstream-web` — enable it, then open
`http://<host-ip>:47992`:

```sh
systemctl --user enable --now slipstream-web
```

#### Console login password

On first start `slipstream-web-init` generates a random login password and saves it to
`~/.config/slipstream/web-password` (as `SLIPSTREAM_UI_PASSWORD=…`). Read it back at any time:

```sh
journalctl --user -u slipstream-web-init | sed -n 's/.*password generated: //p'
sed -n 's/^SLIPSTREAM_UI_PASSWORD=//p' ~/.config/slipstream/web-password
```

To set your own, edit that file and `systemctl --user restart slipstream-web`. Forgot it? See
[Forgot your Password?](/docs/forgot-password).

## 5. Connect a client

From any [client](/docs/clients), `--discover` finds the host on the LAN. On first connect, complete
the **PIN pairing** — arm it from the host's web console, which displays a 4-digit PIN to type into
the client. (Pairing is required by default; pass `serve --open` only if you deliberately want to
disable it.) See [Clients](/docs/clients) and [Pairing](/docs/pairing).

## Appendix — build from source (PKGBUILD)

To build instead of using the binary repo, use the split `PKGBUILD` in `packaging/arch/` (produces
`slipstream-host` + `slipstream-client`; set `PF_WITH_WEB=1` to also build `slipstream-web`, which needs
`bun`):

```sh
git clone https://github.com/vindeckyy/slipstream.git && cd slipstream/packaging/arch
# Build the working tree (no git fetch):
PF_SRCDIR="$(git rev-parse --show-toplevel)" makepkg -f --holdver
sudo pacman -U slipstream-host-*.pkg.tar.zst
```

NVENC/EGL come from the NVIDIA driver (`nvidia-utils`); on a GPU-less builder, symlink the CUDA
stub into the link path first (the `PKGBUILD` header documents this). Full details, the
Fedora→Arch dependency map, and the SteamOS systemd-sysext path are in
[`packaging/arch/README.md`](https://github.com/vindeckyy/slipstream.git/src/branch/main/packaging/arch/README.md).
