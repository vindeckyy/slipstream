---
title: Install the Host
description: Install the Slipstream host, on Linux from local packaging or GitHub Releases, or on Windows from a signed installer.
---

On Linux, build packages from this repo (or install artifacts from
[GitHub Releases](https://github.com/vindeckyy/slipstream/releases) when attached). There is no
public apt/rpm/npm package host. Pick your distro and follow the guide; each row links to the full
per-distro walkthrough. On **Windows**, the host ships as a signed installer instead, see
[Windows](#windows).

> **First, read [Security & Safe Use](/docs/security).** A streaming host is remote control of the
> machine. It's built for trusted local networks, don't expose it to the internet, and be thoughtful
> about which machine you host on (especially on Windows).

## Pick your distro

| Distro | Package manager | Happy path | Guide |
|--------|-----------------|------------|-------|
| **Ubuntu** | apt (local `.deb`) | build with `packaging/debian`, then `sudo apt install ./dist/slipstream-host_*.deb` | [Ubuntu](/docs/ubuntu) · [packaging/debian](https://github.com/vindeckyy/slipstream/blob/main/packaging/debian/README.md) |
| **Bazzite / Fedora Atomic** | systemd-sysext | `curl -fsSLO https://raw.githubusercontent.com/vindeckyy/slipstream/main/packaging/bazzite/slipstream-sysext.sh && sudo bash slipstream-sysext.sh install` (or build the image locally) | [Bazzite](/docs/bazzite) · [packaging/bazzite](https://github.com/vindeckyy/slipstream/blob/main/packaging/bazzite/README.md) |
| **Fedora (dnf)** | dnf / rpm-ostree | build with `packaging/rpm`, then `sudo dnf install ./dist/slipstream-*.rpm` | [Fedora](/docs/fedora) · [packaging/rpm](https://github.com/vindeckyy/slipstream/blob/main/packaging/rpm/README.md) |
| **Arch** | pacman / makepkg | `cd packaging/arch && PF_SRCDIR="$(git rev-parse --show-toplevel)" makepkg -si` | [Arch Linux](/docs/arch) · [packaging/arch](https://github.com/vindeckyy/slipstream/blob/main/packaging/arch/README.md) |
| **SteamOS (host)** | on-device script | clone the repo, then `bash ~/slipstream/scripts/steamdeck/install.sh` (builds on-device) | [SteamOS (Host)](/docs/steamos-host) |
| **NixOS / Nix** | nix flake | `nix run github:vindeckyy/slipstream#slipstream-host -- serve --gamestream` | [NixOS](#nixos) · [packaging/nix](https://github.com/vindeckyy/slipstream/blob/main/packaging/nix/README.md) |

After you install a package once, rebuild or re-download for updates (or use
`sudo slipstream-sysext update` on Bazzite when you publish a feed). On **NixOS** add the flake as
an input and enable its module, see [NixOS](#nixos).

> **Stable vs canary.** When you publish your own package feeds, keep stable and canary separate so
> a release never traps rolling builds. See [Release Channels](/docs/channels).

## Windows

Slipstream also runs as a native host on **Windows 11 22H2+ (x64)**, shipped as a signed
installer, see [Windows Host](/docs/windows-host) for what it includes and its limitations.

For hardware encode you need a GPU, NVIDIA (NVENC), AMD (AMF), or Intel (QSV); there's a software
fallback without one. More detail, including the CLI `slipstream-host service install` path, is in
[Running as a Service -> Windows](/docs/running-as-a-service#windows).

### winget (optional)

If you host a private winget REST source (see `packaging/winget/`), register it once in an **admin**
PowerShell, then install. There is no public Slipstream winget source:

```powershell
# winget source add -n slipstream https://<your-winget-host> -t Microsoft.Rest
winget install vindeckyy.SlipstreamHost
```

Otherwise use the manual installer below. winget carries **stable** releases only when you publish
them that way.

### Manual download

Download `slipstream-host-setup-<ver>.exe` from
[GitHub Releases](https://github.com/vindeckyy/slipstream/releases) (when attached) and run it
elevated. The full procedure, everything the installer puts on the machine, its optional tasks, the
console password, and the `/VERYSILENT` unattended switch, lives on one page:
[Windows Host -> Install](/docs/windows-host#install). This is also the path for **canary** builds.
You can also build the installer locally from `packaging/windows/`.

> **About the Unknown Publisher prompt.** The installer is signed with a self-signed certificate, so
> Windows warns before it runs, accepting the prompt is enough, nothing else is required. If you'd
> rather silence it, the matching **`slipstream-host-windows_<ver>.cer`** is published next to the
> installer when available, and it's the **same certificate for every release**, so this is one-time.
> A self-signed certificate is its own root, so it has to go in both stores. In an **admin**
> PowerShell:
>
> ```powershell
> Import-Certificate -FilePath .\slipstream-host-windows_<ver>.cer `
>   -CertStoreLocation Cert:\LocalMachine\Root
> Import-Certificate -FilePath .\slipstream-host-windows_<ver>.cer `
>   -CertStoreLocation Cert:\LocalMachine\TrustedPublisher
> ```
>
> This is a different certificate from the one the bundled **drivers** are signed with, the
> installer imports that one for you.

## NixOS

The repo's `flake.nix` is a supported install path: it builds `slipstream-host`, `slipstream-client`,
`slipstream-web` and `slipstream-scripting`, and ships a NixOS module. **`x86_64-linux` only**, and
NixOS **24.11 or newer**.

You can run it straight from the flake without NixOS (on other distros, wrap it in
[nixGL](https://github.com/nix-community/nixGL) so the GPU drivers resolve):

```sh
nix run github:vindeckyy/slipstream#slipstream-host -- serve --gamestream
```

On NixOS, add the flake as an input, add `slipstream.nixosModules.default` to your system's modules,
and enable the host:

```nix
services.slipstream.host = {
  enable = true;
  users = [ "alice" ];     # added to the `input` group, for virtual gamepads
  openFirewall = true;
  settings = { RUST_LOG = "info"; };   # these become host.env
};
```

The module does declaratively what the deb/RPM scriptlets do, the systemd user service, udev rules,
kernel modules, sysctl tuning, the firewall ports and `input` group membership, and brings in the
web console alongside the host. Because `settings` writes the environment file for you, skip the
`host.env` step in [After installing](#after-installing). The user services are defined but not
started, so from your graphical session enable the host and the console:

```sh
systemctl --user enable --now slipstream-host slipstream-web
```

The full option reference (client, console and scripting options, GPU driver notes, headless
appliance setup) is in
[packaging/nix](https://github.com/vindeckyy/slipstream/blob/main/packaging/nix/README.md). To
update, run `nix flake update slipstream` in your flake directory, then `sudo nixos-rebuild switch`.

## What the packages are

- **`slipstream-host`**, the streaming host. Install this on the Linux machine you want to stream from (gaming PC, workstation, or streaming box).
- **`slipstream-web`**, the browser management console (pairing + status). Recommended alongside the
  host. On apt and RPM the host package *recommends* it, so your package manager pulls it in by
  default when both packages are available, and the Bazzite sysext image already contains it when
  you build one. On **Arch** it's an optional dependency, so name it yourself when you makepkg.
- **`slipstream-client`**, the GTK4 desktop client, for streaming *to* a Linux box (build via
  apt / RPM / Arch packaging, or Flatpak). On a **Steam Deck** prefer the Flatpak, SteamOS's
  `/usr` is read-only, so the native package isn't the path there:

  ```sh
  # Build locally (packaging/flatpak) or install a .flatpak from GitHub Releases when attached:
  # flatpak install --user --bundle /path/to/slipstream-client.flatpak
  ```

  For Gaming Mode, add the [Decky plugin](/docs/steam-deck) on top of it. Full client instructions
  for every device: [Install a Client](/docs/install-client).

- **`slipstream-scripting`**, the plugin/script runner. Install it if you want
  [plugins](/docs/plugins) or [automation](/docs/automation). It's inert until you add something to
  run, so its user unit ships **disabled**, enable it once you have:

  ```sh
  systemctl --user enable --now slipstream-scripting
  ```

## After installing

These three steps are for the **Linux packages**. On Windows the installer does the equivalent for
you; on NixOS the module does steps 1 and 2, and [NixOS](#nixos) above has the units to enable.

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
