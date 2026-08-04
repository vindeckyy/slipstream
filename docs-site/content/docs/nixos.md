---
title: NixOS
description: Install the Slipstream host on NixOS (or run it with Nix) from the repo flake.
---

Install a Slipstream host on **NixOS** with the repo's `flake.nix`: it builds `slipstream-host`,
`slipstream-client`, `slipstream-web` and `slipstream-scripting`, and ships a NixOS module that does
declaratively what the deb/RPM scriptlets do. You can also `nix run` the host on other Linux
distros without enabling the module. **`x86_64-linux` only**, and NixOS **24.11 or newer** (the
module uses `hardware.graphics`).

> New here? Read [Security & Safe Use](/docs/security) first, a streaming host is remote control of
> the machine, so keep it on a trusted LAN or VPN and require pairing.

Full option reference, GPU notes, and caveats live in
[`packaging/nix/README.md`](https://github.com/vindeckyy/slipstream/blob/main/packaging/nix/README.md).

## 1. Try it without NixOS (`nix run`)

You can run the host straight from the flake on any Nix-capable Linux box:

```sh
nix run github:vindeckyy/slipstream#slipstream-host -- serve --gamestream
```

That starts the native `slipstream/1` plane **plus** the GameStream/Moonlight-compatible planes, so
stock [Moonlight](/docs/moonlight) clients work. Those extra planes are only appropriate on a
trusted LAN; drop `--gamestream` for native-only. See
[Host CLI -> `serve`](/docs/host-cli#serve) and
[Security -> GameStream](/docs/security#gamestream--moonlight-compatibility-is-the-weak-crypto-path).

GPU drivers resolve at runtime from `/run/opengl-driver/lib`. On **NixOS** the module sets
`hardware.graphics.enable = true` for you. On **other distros**, wrap the command in
[nixGL](https://github.com/nix-community/nixGL) so that path is populated (`nixGL nix run ...`).

Other flake packages you can build or run the same way: `slipstream-client`, `slipstream-web`,
`slipstream-scripting`. `packages.x86_64-linux.default` is `slipstream-host`.

## 2. Enable the NixOS module

Add the flake as an input, import `slipstream.nixosModules.default`, and enable the host:

```nix
{
  inputs.slipstream.url = "github:vindeckyy/slipstream";
  # (optional) share your nixpkgs: inputs.slipstream.inputs.nixpkgs.follows = "nixpkgs";

  outputs = { self, nixpkgs, slipstream, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        slipstream.nixosModules.default
        ({ ... }: {
          services.slipstream.host = {
            enable = true;
            users = [ "alice" ];     # added to the `input` group, for virtual gamepads
            openFirewall = true;     # native + GameStream ports (when gamestream is on)
            settings = {
              RUST_LOG = "info";
              # SLIPSTREAM_VIDEO_SOURCE = "virtual";
              # SLIPSTREAM_444 = true;   # booleans render as 1/0
            };
          };
        })
      ];
    };
  };
}
```

Then rebuild:

```sh
sudo nixos-rebuild switch
```

### What the module configures

Everything the RPM/deb `%install` + `%post` do, declaratively:

- **systemd `--user` service** `slipstream-host` → `serve [--gamestream]`, with `EnvironmentFile`
  from `settings` (and an optional secret file)
- **udev rules** for `/dev/uinput`, `/dev/uhid`, and the vhci sysfs perms
- **kernel modules** `uinput`, `uhid`, `vhci-hcd`
- **sysctl** `net.core.{r,w}mem_max = 32 MB` (high-bitrate UDP headroom)
- **`input` group** membership for `users`
- **`hardware.graphics.enable = true`** so `/run/opengl-driver/lib` has the driver libs
- **firewall** when `openFirewall` is set (see [Open the firewall](#4-open-the-firewall))
- **tray autostart** entry
- **web console** (`services.slipstream.web`) on by default whenever the host is enabled
- **scripting runner** (`services.slipstream.scripting`) installed with the host, but not started
  until you enable the unit

Because `settings` writes the environment file for you, you do **not** copy a `host.env` template
by hand the way the apt/RPM/Arch packages expect.

### Host options

| Option | Default | Meaning |
|--------|---------|---------|
| `enable` | `false` | Install the host and wire udev/sysctl/kernel modules/firewall and the user service |
| `gamestream` | `true` | `serve --gamestream` (Moonlight-compatible). `false` = native-only, more secure, and drops the GameStream firewall ports |
| `autoStart` | `false` | Add the user service to `default.target` (appliance mode - pair with lingering) |
| `users` | `[ ]` | Users added to the `input` group (virtual gamepads) |
| `settings` | `{ }` | `host.env` key/values (booleans render as `1`/`0`). Do not put secrets here - use `environmentFile` |
| `environmentFile` | `null` | Extra `EnvironmentFile` for secrets (e.g. `SLIPSTREAM_MGMT_TOKEN`); loaded optionally |
| `openFirewall` | `false` | Open the inbound ports |
| `gamescopeHdr` | `true` | Put the HDR-patched `slipstream-gamescope` on the host service PATH |
| `package` | flake's | Override the package |

### GameStream toggle

The module defaults `services.slipstream.host.gamestream = true`, matching the Linux packages. To
run a secure native-only host (no Moonlight plane, no GameStream firewall ports):

```nix
services.slipstream.host.gamestream = false;
```

See [Security -> GameStream](/docs/security#gamestream--moonlight-compatibility-is-the-weak-crypto-path)
and [Moonlight](/docs/moonlight).

### Web console and scripting

`services.slipstream.web` is **on by default** whenever the host is enabled (mirrors the RPM's
`Recommends: slipstream-web`). It runs as a systemd `--user` service on **TCP 47992 (HTTPS)**,
auto-wired to the host's per-user `~/.config/slipstream/{mgmt-token,cert.pem,key.pem}`. Set
`services.slipstream.web.enable = false` for a console-less host. `openFirewall` and `autoStart`
follow the host's values by default.

`services.slipstream.scripting` is also enabled with the host, but the unit is **not** auto-started
(`autoStart` defaults to `false`) - the runner is inert until you add scripts or plugins. Turn it
on when you have something to run:

```sh
systemctl --user enable --now slipstream-scripting
```

### Client (optional)

To install the GTK4 desktop client on the same or another NixOS box:

```nix
services.slipstream.client = {
  enable = true;
  openFirewall = true;   # UDP 5353 for mDNS discovery
};
```

Under Nix the session binary's optional Skia on-glass stats overlay is off (sandbox cannot fetch
prebuilt Skia); streaming itself is unaffected, and the GTK shell is fully featured. Details in
[`packaging/nix/README.md`](https://github.com/vindeckyy/slipstream/blob/main/packaging/nix/README.md).

## 3. GPU drivers

The module enables `hardware.graphics` but does **not** install a vendor driver - set those yourself:

- **NVIDIA:** `hardware.nvidia` plus `hardware.graphics.enable = true`. NVENC/CUDA come from the
  driver at runtime (nothing pinned in the closure).
- **AMD/Intel:** `hardware.graphics.enable = true` with `extraPackages` for VAAPI encode
  (`vaapiVdpau` / `intel-media-driver` as appropriate); the host's raw Vulkan-Video HEVC path needs
  only Mesa.

One binary covers every vendor: NVENC/CUDA entry points are `dlopen`'d at runtime, so the same
host runs on NVIDIA, AMD/Intel, or software encode.

## 4. Open the firewall

`openFirewall = false` by default. When you set it `true`, the module opens:

- **Native** always: UDP 9777 (QUIC control), UDP 5353 (mDNS), TCP 47990 (mgmt/library API)
- **GameStream** when `gamestream = true`: TCP 47984, 47989, 48010 and UDP 47998-48000
- **Web console** when `services.slipstream.web.openFirewall` is true (follows the host by
  default): TCP 47992

The media data plane uses an ephemeral UDP port the client hole-punches, so there is nothing fixed
to open for video.

## 5. Enable the user services

The module defines the user units but does **not** start them (unless you set `autoStart = true`).
From your graphical session:

```sh
systemctl --user enable --now slipstream-host slipstream-web
```

Confirm they came up:

```sh
slipstream-host --version
slipstream-host detect-conflicts    # exits 1 if Sunshine/Apollo is also installed
systemctl --user status slipstream-host
journalctl --user -u slipstream-host -f
```

If `detect-conflicts` reports another streaming host, remove it before going further - two hosts on
one machine is the most common reason a clean install never streams. See
[Troubleshooting](/docs/troubleshooting#another-streaming-host-sunshine-apollo--is-installed).

Then open `https://<host-ip>:47992`. Reading the
[login password](/docs/web-console#login-password) and
[arming PIN pairing](/docs/web-console#arm-pairing) are covered in
[The Web Console](/docs/web-console).

## 6. Configure your desktop

How the host creates its virtual display and injects input depends on your desktop, not your distro.
Set compositor / video-source knobs in `services.slipstream.host.settings` (or leave them empty for
per-connect auto-detection), then follow the page for the desktop you run:

- [KDE Plasma (KWin)](/docs/kde)
- [GNOME (Mutter)](/docs/gnome)
- [Steam / gamescope](/docs/gamescope)
- [Hyprland](/docs/hyprland)
- [Sway / wlroots](/docs/sway)

For a **headless / appliance** box, set `autoStart = true`, enable lingering for the host user, and
optionally pin a backend in `settings` (pinning `SLIPSTREAM_COMPOSITOR` disables live-session
auto-detection - leave it out on any box that switches between a desktop and Game Mode):

```nix
services.slipstream.host = {
  enable = true;
  autoStart = true;
  users = [ "streamer" ];
  settings = { SLIPSTREAM_COMPOSITOR = "gamescope"; };  # appliance-only; omit to auto-detect
};
users.users.streamer.linger = true;
# For gamescope/KWin backends, extend the service PATH, e.g.:
# systemd.user.services.slipstream-host.path = [ pkgs.gamescope ];
```

More detail: [Running as a Service](/docs/running-as-a-service).

## Updating

In your flake directory:

```sh
nix flake update slipstream
sudo nixos-rebuild switch
```

Then restart the running user units so they pick up the new binaries:

```sh
systemctl --user restart slipstream-web slipstream-host
```

The web console's **Host -> Updates** card shows the same command. Full notes:
[Updating the Host](/docs/updating). To roll back a generation,
`sudo nixos-rebuild switch --rollback`, or pin the flake input to a `v<x.y.z>` tag - see
[Release Channels](/docs/channels).

## Next steps

- **Pair a client**, [Quick Start](/docs/quickstart) · [Install a Client](/docs/install-client).
- **Keep it current**, [Updating the Host](/docs/updating).
- **Remove it again**, [Uninstalling -> NixOS](/docs/uninstall#nixos) (drop the options and module,
  then rebuild).
- **Something not working?**, [Troubleshooting](/docs/troubleshooting).
