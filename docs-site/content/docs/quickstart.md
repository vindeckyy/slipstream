---
title: Quick Start
description: From nothing to streaming, set up a Linux host and connect your first client.
---

This is the shortest path to a working stream. Each step links to the details.

> A streaming host is remote control of the machine, so it's built for **trusted local networks**, keep
> it on your LAN or a VPN and don't expose it to the internet. Two minutes on
> [Security & Safe Use](/docs/security) before you start is worth it.

## Pick your goal early

Before you install, decide which success story you are chasing tonight. Same product, different
first checklist. You can always add the other path later with a second
[settings profile](/docs/profiles-and-links).

### Playing tonight

**Goal:** a game (or Desktop) streaming on your LAN to a couch client, Steam Deck, or phone.

**Do first**

1. Install and start the host on the Linux gaming PC (steps 1-2 below).
2. Open the [web console](/docs/web-console), set the password, arm pairing (steps 3-4).
3. Install a [native client](/docs/clients) for iPhone, Android, or Steam Deck; use
   [Moonlight](/docs/moonlight) only if you need a device without a named app.
4. Pair on the home LAN, start a stream, leave mouse in **Capture** for mouse-look titles.

**You are done when**

- The client lists the host (or you added it by LAN IP) and pairing succeeds once.
- A stream shows a picture at roughly the mode you asked for.
- A keyboard/mouse or [controller](/docs/controllers) moves something on the host.
- Optional: a library title launches into the stream ([Game library](/docs/game-library)).

**Next depth:** [Play](/docs/play) (presets, HDR, bitrate, Headless box),
[Picture quality](/docs/picture-quality), [Controllers](/docs/controllers), [Audio](/docs/audio).

### Office tomorrow

**Goal:** your real home/workstation desktop from another device over a **private VPN** - not a
public port-forward.

**Do first**

1. Complete **Playing tonight** style setup **once on the home LAN** so you know the host, console,
   and native client work before the VPN is in the picture (steps 1-5 below).
2. Prefer a **native** client; turn **GameStream off** on a Work-oriented host if you do not need
   Moonlight ([Moonlight](/docs/moonlight), [Security](/docs/security#gamestream--moonlight-compatibility-is-the-weak-crypto-path)).
3. Put host and client on the same VPN ([Network & VPN](/docs/network-and-vpn)); add the host **by
   VPN IP** when discovery is empty.
4. Switch the client to **Desktop (absolute)** mouse; enable clipboard; pick **Workstation** or
   **Hot-desk**; tune picture for text.

**You are done when**

- From the office network, the native client connects using the VPN address (empty mDNS list is OK).
- Desktop mouse lets you click window chrome and select text without capture-lock fighting you.
- Text in an IDE or browser is readable after bitrate / HEVC / chroma tuning -
  [Picture quality](/docs/picture-quality).
- Clipboard text crosses when both host and client switches are on ([Clipboard](/docs/clipboard)),
  or you have an agreed file-share alternative for large files.

**Next depth:** [Desktop at work](/docs/desktop-at-work), [Network & VPN](/docs/network-and-vpn),
[Virtual displays](/docs/virtual-displays), [Input](/docs/input).

If you need both, finish the LAN stream first, then layer the VPN and a **Work** profile on the
same host - do not debug soft text and Tailscale ACLs at the same time.

| | Playing tonight | Office tomorrow |
|---|---|---|
| Network | Home LAN | Private VPN after a proven LAN stream |
| Client | Native (iPhone / Android / Steam Deck); Moonlight OK for other devices | Native preferred; GameStream off if unused |
| Mouse | Capture (default) for games | Desktop (absolute) |
| Display preset | Headless box / Shared desktop | Workstation / Hot-desk |
| Picture | HDR / high refresh as the chain allows | HEVC, HDR off, bitrate for sharp text |
| Success | Picture + input on the couch | VPN connect + readable desktop + clipboard or file share |

## 1. Set up the host

On the Linux machine you want to stream from - a gaming PC, a workstation, or any supported host
with an NVIDIA, AMD, or Intel GPU - follow the install guide for your system:

- [Ubuntu](/docs/ubuntu)
- [Fedora](/docs/fedora)
- [Arch](/docs/arch)
- [Bazzite](/docs/bazzite)
- [SteamOS](/docs/steamos-host)
- [NixOS](/docs/nixos)

Each one covers the GPU driver, the dependencies, and how to install and run the host. After
installing, configure for your desktop ([KDE](/docs/kde) / [GNOME](/docs/gnome) /
[gamescope](/docs/gamescope) / [Hyprland](/docs/hyprland) / [Sway](/docs/sway)). Check the
[Requirements](/docs/requirements) first if you're not sure your machine is a fit.

**Office note:** for remote desktop, prefer a full desktop session over gamescope / Gaming Mode -
absolute mouse needs a real desktop ([Desktop at work](/docs/desktop-at-work)).

## 2. Start the host

You don't run the host by hand. Every Linux package ships a systemd **user** unit. That unit reads
`~/.config/slipstream/host.env` and won't start until the file exists, so put it in place first,
your distro and desktop pages (step 1) have the template to copy and what to put in it. Then enable
the unit once, from a terminal **inside your desktop session**, and it comes back at every login:

```sh
systemctl --user enable --now slipstream-host
journalctl --user -u slipstream-host -f   # watch it come up and print its identity fingerprint
```

Once up, the host advertises itself on your local network, so clients find it by name. It works out
where your compositor is by itself, so there is nothing to export.

That unit runs `serve --gamestream`: the native `slipstream/1` plane **plus** the
GameStream/Moonlight-compatible planes, so stock [Moonlight](/docs/moonlight) clients work too. Those
extra planes pair over plain HTTP and belong on a trusted LAN only, for a native-only host, see
[What the unit starts](/docs/running-as-a-service#what-the-unit-starts).

If the host runs a firewall (Fedora enables firewalld, CachyOS enables ufw), open its ports, the
firewall step in your distro guide has the exact commands, for **both** `slipstream-native` and
`slipstream-gamestream`, because the packaged unit serves both planes. Copy-paste also lives on
[Network & VPN -> Firewalls](/docs/network-and-vpn#firewalls--copy-paste). For a Work-only host you
can open `slipstream-native` alone and leave GameStream closed.

On **SteamOS** even that is done for you, the install script wrote its own `slipstream-host` user
service and started it (GameStream on by default there too; pass `--no-gamestream` to the install
script for a native-only host). Check it with `systemctl --user status slipstream-host`.

## 3. Open the web console

The console is a **separate** process from the host, and you need it in the next step, arming PIN
pairing is done there.

- On Ubuntu, Fedora and Bazzite the `slipstream-web` package comes in with the host but isn't
  enabled for you. On Arch it's an optional dependency, so install it first
  (`sudo pacman -Syu slipstream-web`, a full `-Syu`, never a bare `-S`). Either way, start it as
  your desktop user, then open `https://<host-ip>:47992`:

  ```sh
  systemctl --user enable --now slipstream-web
  ```

  Choose the login password when the setup screen appears.
- **SteamOS:** the install script already started the console and printed its URL when it finished.
  Choose the login password when the setup screen appears.

The console password protects this browser console. It is separate from client pairing, which happens
after you sign in. On the Dashboard, follow **Getting started** to check host readiness, pair a
device, then open the library. You can dismiss that checklist if you already know the setup path.

The certificate is the host's own self-signed one, so your browser warns once, trust it and
continue. Full details and a page-by-page tour: [The Web Console](/docs/web-console).

## 4. Connect and pair a client

On the device you want to stream to, use a [native Slipstream client](/docs/clients) for the lowest
latency, or any Moonlight client:

- **Named clients (iPhone, Android, Steam Deck):** install first,
  [Install a Client](/docs/install-client) has the download for each (Steam Deck: the
  [Decky plugin](/docs/steam-deck)). Then open the Slipstream app, your host appears in the list of
  hosts found on your network. Select it, and when prompted, **pair**.
- **Anything with Moonlight:** add the host (it should be discovered automatically), then pair.

To pair, the host needs to show a PIN. [Arm pairing](/docs/web-console#arm-pairing) from the web
console you opened in step 3, the host displays a 4-digit PIN, you type it into the client, and they
trust each other from then on. Pairing is required by default. Full details:
[Pairing & Trust](/docs/pairing).

**Office / VPN:** discovery often fails across the tunnel - **Add host** by VPN IP, then pair the
same way ([Network & VPN](/docs/network-and-vpn#discovery-across-a-vpn)).

## 5. Stream

Once paired, select the host and start streaming. The host creates a virtual display at your device's
resolution and refresh, and the picture comes up. Mouse, keyboard, and controllers flow back to the
host.

Worth knowing before you need it: when a client **captures** input for games, you need a way to
release it. On clients that support desktop shortcuts, **Ctrl+Alt+Shift+Q** hands mouse and keyboard
back. For remote desktop / office work, switch to **Desktop (absolute)** mouse so the pointer is not
locked - Capture mode is the default and is meant for games. The other in-stream shortcuts, and the
mouse, touch and pen modes, are on [Mouse, touch and pen](/docs/input).

**Playing tonight check:** Capture mouse + a launched game or Desktop feels responsive on LAN.
**Office tomorrow check:** Desktop mouse + readable UI over the VPN path you will actually use.

## Now that it works

Pick the path that matches what you hired Slipstream for.

### Play

- Full couch checklist and presets: [Play](/docs/play).
- Browse the host's installed games and launch one straight into the stream,
  [Game library](/docs/game-library).
- Use Capture (game) mouse for mouse-look titles, [Mouse, touch and pen](/docs/input).
- Controllers and pads: [Controllers](/docs/controllers).
- Get a 10-bit HDR picture where the whole chain allows it, [HDR](/docs/hdr).
- Bitrate, chroma, soft vs sharp picture: [Picture quality](/docs/picture-quality).
- Connect to a host that's asleep, [Wake-on-LAN](/docs/wake-on-lan) (LAN only for magic packets).

### Work

- Follow the office checklist: VPN, Desktop mouse, clipboard, Workstation / Hot-desk,
  [Desktop at work](/docs/desktop-at-work).
- Reach the host from another network safely, [Network & VPN](/docs/network-and-vpn).
- Soft text and Work picture recipe: [Picture quality](/docs/picture-quality).
- Copy on one machine and paste on the other, [Shared clipboard](/docs/clipboard).
- Audio mute / mic defaults for desk sessions: [Audio](/docs/audio).
- Save a Work settings profile separate from your couch profile,
  [Profiles and links](/docs/profiles-and-links).

### Either way

- Save named stream settings, bind them to a host, and start a session from a shortcut or a script,
  [Profiles and links](/docs/profiles-and-links).
- Tune resolution, refresh, bitrate, codec and HDR in
  [client settings](/docs/client-settings); the host's own knobs are in
  [Configuration](/docs/configuration).

## Keep it running

- Make it always-on, no login, no monitor: [Running as a Service](/docs/running-as-a-service).
- Keep it current with [Updating](/docs/updating); changed your mind? [Uninstall](/docs/uninstall).
- Hit a snag? See [Troubleshooting](/docs/troubleshooting), including
  [Office / VPN](/docs/troubleshooting#office--vpn) and
  [Host isn't found](/docs/troubleshooting#the-host-isnt-found-on-the-network).
