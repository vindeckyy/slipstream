---
title: Install
description: Install the Slipstream host on Linux.
---

Build packages from this repo, or install artifacts from
[GitHub Releases](https://github.com/vindeckyy/slipstream/releases) when attached. There is no
public apt/rpm registry.

Read [Network](/docs/network-and-vpn) before exposing the host beyond your LAN.

## Requirements

- **Linux** host (x86_64). Supported packaging: Ubuntu, Fedora, Arch, Bazzite, SteamOS, NixOS.
- GPU: NVIDIA, AMD, or Intel preferred. Software H.264 encode works without a GPU encoder.
- Desktop session for the user who runs the host (systemd **user** units).

Capability detail: [Support matrix](/docs/support-matrix).

## Packages

| Package | Role |
|---------|------|
| `slipstream-host` | Streaming host |
| `slipstream-web` | Browser console (`https://<host>:47992`) |
| `slipstream-client` | GTK4 client (Steam Deck Flatpak path) |
| `slipstream-scripting` | Optional plugin runner (disabled until you enable it) |

## Distro install

| Distro | Happy path | Detail |
|--------|------------|--------|
| **Ubuntu** | Build with `packaging/debian`, then `sudo apt install ./dist/slipstream-host_*.deb` | [packaging/debian](https://github.com/vindeckyy/slipstream/blob/main/packaging/debian/README.md) |
| **Fedora** | Build with `packaging/rpm`, then `sudo dnf install ./dist/slipstream-*.rpm` | [packaging/rpm](https://github.com/vindeckyy/slipstream/blob/main/packaging/rpm/README.md) |
| **Arch** | `cd packaging/arch && PF_SRCDIR="$(git rev-parse --show-toplevel)" makepkg -si` | [packaging/arch](https://github.com/vindeckyy/slipstream/blob/main/packaging/arch/README.md) |
| **Bazzite** | Build a sysext image, then run `sudo bash slipstream-sysext.sh install --from-file <image.raw>` | [packaging/bazzite](https://github.com/vindeckyy/slipstream/blob/main/packaging/bazzite/README.md) |
| **SteamOS (host)** | Clone the repo, then `bash scripts/steamdeck/install.sh` | Script builds on-device |
| **NixOS** | `nix run github:vindeckyy/slipstream#slipstream-host -- serve` or the flake module | [packaging/nix](https://github.com/vindeckyy/slipstream/blob/main/packaging/nix/README.md) |

On Arch, install `slipstream-web` yourself (`pacman` does not pull optional deps).

## After install

1. Add your user to the `input` group (needed for virtual gamepads and pen), then re-login:

   ```sh
   sudo usermod -aG input "$USER"
   ```

2. Create config and start services:

   ```sh
   mkdir -p ~/.config/slipstream
   cp /usr/share/slipstream/host.env.example ~/.config/slipstream/host.env
   # Ubuntu may use /usr/share/slipstream-host/host.env.example
   # Bazzite template: host.env.bazzite

   systemctl --user enable --now slipstream-host
   systemctl --user enable --now slipstream-web
   ```

3. Open `https://<host-ip>:47992` and set the console password. See [Console](/docs/web-console).

Packaged units run the secure native-only `serve` command. To enable Moonlight compatibility:

```ini
# systemctl --user edit slipstream-host
[Service]
ExecStart=
ExecStart=/usr/bin/slipstream-host serve --gamestream
```

## Update and uninstall

- Rebuild or reinstall the package for your distro. Bazzite: `sudo slipstream-sysext update` when you publish a feed.
- Stop and disable user units, then remove the packages with your package manager.
- Config lives under `~/.config/slipstream/`; remove it only if you want a clean slate.

## Competing hosts

Do not run Slipstream alongside Sunshine, Apollo, or other GameStream hosts on the same machine
(shared ports and often shared display drivers). `slipstream-host detect-conflicts` reports active conflicts.
