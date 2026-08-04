---
title: Install the Host
description: Install the Slipstream host on Linux from local packaging or GitHub Releases.
---

Build packages from this repo (or install artifacts from
[GitHub Releases](https://github.com/vindeckyy/slipstream/releases) when attached). There is no
public apt/rpm/npm package host. Pick your distro and follow the guide; each row links to the full
per-distro walkthrough.

> **First, read [Security & Safe Use](/docs/security).** A streaming host is remote control of the
> machine. Designed for trusted local networks, don't expose it to the internet, and be thoughtful
> about which machine you host on.

## Pick your distro

| Distro | Package manager | Happy path | Guide |
|--------|-----------------|------------|-------|
| **Ubuntu** | apt (local `.deb`) | build with `packaging/debian`, then `sudo apt install ./dist/slipstream-host_*.deb` | [Ubuntu](/docs/ubuntu) · [packaging/debian](https://github.com/vindeckyy/slipstream/blob/main/packaging/debian/README.md) |
| **Bazzite / Fedora Atomic** | systemd-sysext | `curl -fsSLO https://raw.githubusercontent.com/vindeckyy/slipstream/main/packaging/bazzite/slipstream-sysext.sh && sudo bash slipstream-sysext.sh install` (or build the image locally) | [Bazzite](/docs/bazzite) · [packaging/bazzite](https://github.com/vindeckyy/slipstream/blob/main/packaging/bazzite/README.md) |
| **Fedora (dnf)** | dnf / rpm-ostree | build with `packaging/rpm`, then `sudo dnf install ./dist/slipstream-*.rpm` | [Fedora](/docs/fedora) · [packaging/rpm](https://github.com/vindeckyy/slipstream/blob/main/packaging/rpm/README.md) |
| **Arch** | pacman / makepkg | `cd packaging/arch && PF_SRCDIR="$(git rev-parse --show-toplevel)" makepkg -si` | [Arch Linux](/docs/arch) · [packaging/arch](https://github.com/vindeckyy/slipstream/blob/main/packaging/arch/README.md) |
| **SteamOS (host)** | on-device script | clone the repo, then `bash ~/slipstream/scripts/steamdeck/install.sh` (builds on-device) | [SteamOS (Host)](/docs/steamos-host) |
| **NixOS / Nix** | nix flake | `nix run github:vindeckyy/slipstream#slipstream-host -- serve --gamestream` | [NixOS](/docs/nixos) · [packaging/nix](https://github.com/vindeckyy/slipstream/blob/main/packaging/nix/README.md) |

After you install a package once, rebuild or re-download for updates (or use
`sudo slipstream-sysext update` on Bazzite when you publish a feed). On **NixOS** add the flake as
an input and enable its module, see [NixOS](/docs/nixos).

> **Stable vs canary.** When you publish your own package feeds, keep stable and canary separate so
> a release never traps rolling builds. See [Release Channels](/docs/channels).

## NixOS

The repo's `flake.nix` is a supported install path (**`x86_64-linux`**, NixOS **24.11+**): `nix run`
for a quick try, or the NixOS module for a declarative host (systemd user units, udev, firewall,
`input` group, web console). Full walkthrough - module options, GameStream toggle, GPU/nixGL notes,
updates, and headless appliance setup - is on [NixOS](/docs/nixos). Packaging reference:
[packaging/nix](https://github.com/vindeckyy/slipstream/blob/main/packaging/nix/README.md).

## What the packages are

- **`slipstream-host`**, the streaming host. Install this on the Linux machine you want to stream from (gaming PC, workstation, or streaming box).
- **`slipstream-web`**, the browser management console (pairing + status). Recommended alongside the
  host. On apt and RPM the host package *recommends* it, so your package manager pulls it in by
  default when both packages are available, and the Bazzite sysext image already contains it when
  you build one. On **Arch** it's an optional dependency, so name it yourself when you makepkg.
- **`slipstream-client`**, the GTK4 client used under the hood on **Steam Deck** (Flatpak /
  Decky). SteamOS's `/usr` is read-only, so the Flatpak is the path there:

  ```sh
  # Build locally (packaging/flatpak) or install a .flatpak from GitHub Releases when attached:
  # flatpak install --user --bundle /path/to/slipstream-client.flatpak
  ```

  For Gaming Mode, add the [Decky plugin](/docs/steam-deck) on top of it. Full client instructions:
  [Install a Client](/docs/install-client).

- **`slipstream-scripting`**, the plugin/script runner. Install it if you want
  [plugins](/docs/plugins) or [automation](/docs/automation). It's inert until you add something to
  run, so its user unit ships **disabled**, enable it once you have:

  ```sh
  systemctl --user enable --now slipstream-scripting
  ```

## After installing

These three steps are for the **Linux packages**. On NixOS the module does steps 1 and 2 (and writes
`settings` instead of a hand-copied `host.env`), and [NixOS](/docs/nixos) has the units to enable.

1. Add yourself to the `input` group, virtual gamepads and [pen
   input](/docs/input#pen-and-stylus) both need `/dev/uinput`, then re-login. The exact
   command differs per distro, see your guide (`usermod -aG input "$USER"`, or `ujust
   add-user-to-input-group` on Bazzite).
2. Put your `host.env` in place, then start the host. Every Linux package ships a systemd **user**
   unit, so you don't run the host by hand, but that unit reads `~/.config/slipstream/host.env` and
   won't start until the file exists. Each package ships a template to copy; your distro and desktop
   guides say which one to pick (on Bazzite it's `host.env.bazzite`):

   ```sh
   mkdir -p ~/.config/slipstream
   # /usr/share/slipstream/ on Fedora/Arch/Bazzite, /usr/share/slipstream-host/ on Ubuntu
   cp /usr/share/slipstream/host.env.example ~/.config/slipstream/host.env
   systemctl --user enable --now slipstream-host
   ```

   The shipped unit runs `serve --gamestream`, the native `slipstream/1` plane **plus** the
   GameStream/Moonlight-compatible planes, so stock [Moonlight](/docs/moonlight) clients work out of
   the box. Those extra planes are only appropriate on a trusted LAN. To run native-only, drop the
   flag with a drop-in (`systemctl --user edit slipstream-host`):

   ```ini
   [Service]
   ExecStart=
   ExecStart=/usr/bin/slipstream-host serve
   ```

   The empty `ExecStart=` is required, without it systemd adds a second command instead of
   replacing the first, and the binary path has to match your install (`systemctl --user cat
   slipstream-host` shows it; the distro packages use `/usr/bin`). Save the drop-in, then
   `systemctl --user restart slipstream-host`. For what each mode starts, see
   [Host CLI -> `serve`](/docs/host-cli#serve).

3. Enable the web console:

   ```sh
   systemctl --user enable --now slipstream-web
   ```

   Then open `https://<host-ip>:47992`. Reading its [login password](/docs/web-console#login-password)
   and [arming PIN pairing](/docs/web-console#arm-pairing) are covered in
   [The Web Console](/docs/web-console).

### Configure your desktop

How the virtual display and input work depends on your desktop, see [KDE](/docs/kde),
[GNOME](/docs/gnome), [Steam / gamescope](/docs/gamescope), [Hyprland](/docs/hyprland), or
[Sway](/docs/sway) for the compositor-specific setup.

From there, follow the [Quick Start](/docs/quickstart) to pair your first client. To run the host
automatically at boot, see [Running as a Service](/docs/running-as-a-service). If something doesn't
come up, [Troubleshooting](/docs/troubleshooting) starts from the symptom.

## Updating and removing

The web console's **Host -> Updates** card tells you when a newer host is out and shows the exact
command for the way you installed, the full list, plus one-click updating and how to turn the check
off, is on [Updating the Host](/docs/updating).

To take it back off, see [Uninstall](/docs/uninstall), it covers every install method and what is
deliberately left behind (your `~/.config/slipstream`, identity certificate, paired devices, console
password, survives package removal).

## Building from source

If no package exists for your platform, build from source, see the repository README. Source builds
and the packaging scripts under `packaging/` are the supported path.
