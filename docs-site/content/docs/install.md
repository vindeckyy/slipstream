---
title: Install the Host
description: Pick your distro and install the slipstream host from its package registry.
---

The package registries are the real distribution channel. Pick your distro, add the repo, and install
with your native package manager. Each row links to the full per-distro guide (add the repo, first-run
steps, the web console) — those are the source of truth, so this page doesn't duplicate them.

## Pick your distro

| Distro | Package manager | One-command happy path | Guide |
|--------|-----------------|------------------------|-------|
| **Ubuntu / Debian** | apt | `sudo apt install slipstream-host` | [Ubuntu — GNOME](/docs/ubuntu-gnome) · [Ubuntu — KDE](/docs/ubuntu-kde) · [packaging/debian](https://github.com/vindeckyy/slipstream.git/src/branch/main/packaging/debian/README.md) |
| **Fedora / Bazzite** | rpm-ostree | `rpm-ostree install slipstream slipstream-web` | [Fedora — KDE](/docs/fedora-kde) · [Bazzite](/docs/bazzite) · [packaging/rpm](https://github.com/vindeckyy/slipstream.git/src/branch/main/packaging/rpm/README.md) |
| **Arch / Steam Deck** | PKGBUILD / sysext | `makepkg -si` (Arch) · sysext `.raw` (SteamOS/Deck) | [packaging/arch](https://github.com/vindeckyy/slipstream.git/src/branch/main/packaging/arch/README.md) |

Each registry is public — no auth, you just trust the repo's signing key. Adding the repo is a
one-time step covered in the linked guide; after that, normal `apt upgrade` / `rpm-ostree upgrade`
tracks new builds automatically.

## What the packages are

- **`slipstream-host`** — the streaming host. Install this on your Linux + NVIDIA gaming machine.
- **`slipstream-web`** — the browser management console (pairing + status). Recommended alongside the
  host; on RPM list it explicitly (`rpm-ostree install slipstream slipstream-web`).
- **`slipstream-client`** — the GTK4 desktop client, for streaming *to* a Linux box (also shipped via
  apt / RPM / Arch / Flatpak). On a Steam Deck, this is the package you want.

## After installing

1. Add yourself to the `input` group (virtual gamepads need `/dev/uinput`), then re-login. The exact
   command differs per distro — see your guide (`usermod -aG input "$USER"`, or `ujust
   add-user-to-input-group` on Bazzite).
2. Start the host inside your desktop session:

   ```sh
   slipstream-host serve --native
   ```

3. Enable the web console and read its login password, then open `http://<host-ip>:3000`:

   ```sh
   systemctl --user enable --now slipstream-web
   journalctl --user -u slipstream-web-init | sed -n 's/.*password generated: //p'
   ```

From there, follow the [Quick Start](/docs/quickstart) to pair your first client. To run the host
automatically at boot, see [Running as a Service](/docs/running-as-a-service).

## Building from source

If no package exists for your platform, you can build from source — see the repository README. Source
builds are a fallback; the registries are the supported path.
