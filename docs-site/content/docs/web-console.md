---
title: The Web Console
description: Enable the Slipstream browser console, read or change its login password, arm PIN pairing, and what every page in it does.
---

The web console is the browser UI for a Slipstream host, live status, pairing, display policy, the
game library, logs, plugins, host readiness, power controls and host updates. It ships as the
**`slipstream-web`** systemd user unit
on Linux and runs under the **Slipstream Host service** on Windows, and serves on **`https://<host-ip>:47992`**
(HTTPS with the host's own self-signed identity cert, your browser warns once; trust it and
continue). It's the surface you expose on the LAN to administer the host; the host's own management
API (47990) keeps every admin action loopback-only and off-loopback serves only read-only status +
game-library browsing to paired clients.

> New here? Read [Security & Safe Use](/docs/security) first, a streaming host is remote control of
> the machine, so keep it on a trusted LAN or VPN and require pairing.

## First-run walkthrough

Do this once after install so the rest of the console has somewhere useful to point.

1. **Enable the console** (Linux) or confirm the Windows / SteamOS install already started it -
   commands in [Enable the console](#enable-the-console) below.
2. From a device on the same trusted network, open `https://<host-ip>:47992`. Accept the
   self-signed certificate warning for this host.
3. **Choose the login password** when the setup screen appears (new Linux / SteamOS installs), or
   log in with the password the Windows installer showed. Details:
   [Login password](#login-password).
4. Open **Host** and skim **Preflight**. Fix any blocked checks (missing `host.env`, encoder,
   competing host) before you chase client issues.
5. Open **Pairing** → **Pair a device**, then complete PIN entry (or approval) on your
   [client](/docs/clients). Full trust model: [Pairing & Trust](/docs/pairing).
6. Open **Virtual displays** and pick a preset that matches how you use the machine -
   **Workstation** / **Hot-desk** for [Desktop at work](/docs/desktop-at-work), **Headless box** /
   **Shared desktop** for [Play](/docs/play).
7. Optional: **Library** to confirm launchers are visible; **Plugins** only if you intend to run
   them ([Plugins](/docs/plugins) - they run code on the host).
8. Start a stream from the client. Return to **Dashboard** to see the live session; use **Logs**
   or **Performance** if something looks wrong.

After first run, day-to-day office reconnects usually never need the console - only pairing a new
device, changing display policy, reading logs, or updating the host.

## Enable the console

- **Linux packages (apt / RPM / Bazzite):** on Ubuntu the host package is `slipstream-host`
  and on Fedora/Bazzite it's `slipstream`; either way it *recommends* `slipstream-web`, so your
  package manager pulls the console in with the host (the Bazzite sysext image already contains
  it). Enable and start it as your desktop user, then open the URL:

  ```sh
  systemctl --user enable --now slipstream-web
  # then browse to https://<host-ip>:47992
  ```

- **Arch / CachyOS (pacman):** the console is an *optional* package here, and pacman never installs
  optional dependencies, so install it yourself from the same repo the host came from (see
  [Arch Linux](/docs/arch)), then enable it exactly as above. Take it as a full `-Syu`, never a bare
  `pacman -S`, so you don't end up on a partial upgrade:

  ```sh
  sudo pacman -Syu slipstream-web
  systemctl --user enable --now slipstream-web
  ```

- **Windows host:** the installer sets up the console and its runtime; the Slipstream Host service
  runs it and automatically brings it back if it ever stops. There is nothing to enable, open
  `https://<this-PC>:47992`.

- **SteamOS host:** the install script builds and starts the console as a user service for you. It
  prints the URL when it finishes.

Reach the console over a VPN the same way you reach the host: use the VPN IP and open TCP
**47992** on the host firewall (`slipstream-web`). See [Network & VPN](/docs/network-and-vpn).

## Login password

The console is password-protected. Where that password lives and how you change it depends on the
host platform.

**Linux packages (apt / RPM / Bazzite).** On a new install, open the console and choose the password
when the setup screen appears. It is then saved to `~/.config/slipstream/web-password` as
`SLIPSTREAM_UI_PASSWORD=...`.

For new Linux and SteamOS consoles, open the page from a trusted device first. Until the password is
saved, the first visitor can complete setup.

```sh
sed -n 's/^SLIPSTREAM_UI_PASSWORD=//p' ~/.config/slipstream/web-password
```

Older Linux installs may already have a generated password in that file. Read it with the command
above, or remove the file before restarting the console if you want the browser setup screen instead.

To set your own, edit that file (`SLIPSTREAM_UI_PASSWORD=<your-password>`) and restart the console:
`systemctl --user restart slipstream-web`.

**SteamOS host.** New installs ask you to choose the password in the browser. The password is saved
to `~/.config/slipstream/web-password`; the session secret remains in `web.env`:

```sh
sed -n 's/^SLIPSTREAM_UI_PASSWORD=//p' ~/.config/slipstream/web-password
```

Edit that file and `systemctl --user restart slipstream-web` to change it.

**Windows host.** You choose the password during install, a secure random default is pre-filled and
shown again on the installer's final page. It's stored in `%ProgramData%\slipstream\web-password` (as
`SLIPSTREAM_UI_PASSWORD=...`), readable only by Administrators and SYSTEM. To change it, edit the file
and restart the Slipstream Host service (which runs the console) in an **elevated** PowerShell:

```powershell
notepad "$env:ProgramData\slipstream\web-password"   # set SLIPSTREAM_UI_PASSWORD=<your-password>
slipstream-host service restart
```

Forgot it? See [Forgot your Password?](/docs/forgot-password).

## First run

After setup, the Dashboard shows a **Getting started** checklist while no client is paired. It keeps
the common path in order:

1. **Check host readiness** opens Host and its Preflight checks.
2. **Pair a device** opens Pairing, where you can arm a PIN or approve a pending native client.
3. **Open the library** takes you to the games available to clients.

The checklist disappears after a client is paired. You can also dismiss it locally from the Dashboard;
that only changes this browser and does not change host pairing or readiness.

## Arm pairing

The host **requires PIN pairing** by default (secure on a LAN). To connect the first time, open the
console, log in, then open **Pairing** in the sidebar and click **Pair a device**. The host shows a
one-time 4-digit PIN, enter it on your [client](/docs/clients) to pair. If the device already tried
to connect it appears under **Waiting for approval** instead; approving it pairs it immediately, no
PIN needed. See [Pairing & Trust](/docs/pairing) for the full trust model and how to approve or
remove devices later.

## Guided tour

The sidebar groups destinations by operator job: **Watch**, **Connect**, **Host**, and **Tools**. A
**More** tab on a phone holds the less frequent destinations, while Dashboard, Library, Host, and
Pairing stay in the bottom navigation:

**Watch** contains Dashboard, Sessions, Performance, and Logs. **Connect** contains Pairing, Library,
and Virtual displays. **Host** contains Host and Automation. **Tools** contains Configuration,
Settings, and Plugins.

### Dashboard

Live status for the host you are administering:

- Whether **video** and **audio** are streaming right now.
- **Active sessions** with codec, resolution, frame rate, and bitrate - useful when diagnosing soft
  text or stutter alongside the client's [stats overlay](/docs/stats) and
  [Picture quality](/docs/picture-quality).
- Which **games** are running and how many clients are **paired**.
- Actions: **stop** a session, or ask the encoder for a **fresh keyframe** if the picture looks
  stuck after a glitch.

Open Dashboard after you connect from a client to confirm the host sees the same mode you asked
for. For office work, you mainly care that a session is up and the resolution matches the laptop
panel ([Desktop at work](/docs/desktop-at-work)).

### Pairing

Trust management for this host:

- **Pair a device** - arms a 4-digit PIN for about **two minutes** (countdown + Cancel in the UI).
- **Waiting for approval** - devices that already knocked; **Approve** (optional label) or
  **Deny** (dismisses; not a permanent blocklist). Requests expire after **10 minutes**.
- **Paired devices** - revoke access for a lost or retired client; re-pair is the same ceremony
  again.
- **Moonlight (GameStream) pairing** - PIN entry box appears **only** when this host runs the
  GameStream plane. Direction is reversed vs native: Moonlight shows the PIN, you type it here.
  See [Moonlight](/docs/moonlight) and [Pairing](/docs/pairing).

Prefer native pairing for Work hosts; leave GameStream off if you do not need Moonlight
([Security](/docs/security#gamestream--moonlight-compatibility-is-the-weak-crypto-path)).

### Virtual displays

Policy for the display each session gets, plus the **Streamed screen** picker when you pin a real
monitor instead of a virtual one. Presets:

| Preset | Typical use |
|---|---|
| **Workstation** / **Hot-desk** | Office remote desktop |
| **Headless box** / **Shared desktop** | Couch / family play |
| **Default** | Leave alone until you have a reason |

Tune **keep-alive**, topology, and custom presets here when reconnects reshuffle the desktop or
you need Forever + Release for a dedicated box. Full reference:
[Virtual displays](/docs/virtual-displays). Office keep-alive habits:
[Desktop at work](/docs/desktop-at-work). Play-oriented presets: [Play](/docs/play).

### Library

What every client sees as launchable titles:

- Turn a **launcher source** on or off (Steam, Epic, GOG, Xbox, ... as the host found them).
- Add or edit a **custom** title with its own art and launch command.

This is the same catalog native clients and Moonlight show as the game list. Details:
[Your game library](/docs/game-library). Desk-only Work users can ignore Library until they also
use the host for [Play](/docs/play).

### Plugins

Plugin store tabs **Browse**, **Installed**, and **Sources**, plus the plugin **runner** switch.
An installed plugin with a UI gets its own sidebar entry below. Plugins run code on your host -
read [Plugins](/docs/plugins) and the security notes in [Security](/docs/security#plugins-run-code-on-your-host)
before enabling the runner on a machine that holds sensitive work data.

### Logs and Performance

**Logs** - follow the host's recent log stream live, filter by level, search, download or share for
a bug report, and create an owner-private **redacted support bundle**. The bundle contains host
state, recent logs, and performance summaries, downloads as JSON, and is also stored under the
host's private config directory. It is **never uploaded** by the console.

Search Logs for strings like `Wake-on-LAN`, `METRONOMIC`, or encoder errors when
[Troubleshooting](/docs/troubleshooting) points you there.

**Performance** - arm a capture, run a session, stop it, and read the recording back as per-stage
latency, throughput, and health graphs. Use it when you need host-side evidence that a VPN or
encode path is the bottleneck, alongside client [stats](/docs/stats).

### Host

Identity and readiness for this machine:

- Hostname, OS, local IP, version, unique id.
- Codecs the host **advertises**, ports it listens on.
- **Preflight** - same checks as `slipstream-host doctor` (configuration, storage, encoder,
  compositor, capture, competing hosts). Blocked checks include an action that can resolve them;
  **Refresh** after you change setup.
- **Updates** - [Updating the Host](/docs/updating).
- **GPUs** - Automatic or a preferred GPU for capture and encode (applies to the **next**
  session), plus compositor backends found.
- **Host power** - **Restart** restarts the Slipstream host process and waits for the console to
  reconnect; **Shutdown** stops the process **without** powering off the computer. Both end
  active sessions and require confirmation. Start the host service again on the machine after a
  shutdown.

### Settings

Console language, and **Sign out**. This page is about the browser UI session, not the host's
stream encoder knobs - those live in [Configuration](/docs/configuration) / `host.env` and in
client [profiles](/docs/profiles-and-links).

## Preflight and host power (detail)

The **Preflight** card runs read-only checks for configuration, storage, the encoder, the compositor,
capture, and competing hosts. A blocked check includes an action that can resolve it. Use **Refresh**
after changing the host setup.

**Restart** restarts the Slipstream host process and waits for the console to reconnect. **Shutdown**
stops the process without powering off the computer. Both actions end active sessions and require
confirmation. Start the host service again on the machine after a shutdown.

## Related pages

- [Quick Start](/docs/quickstart)
- [Pairing & Trust](/docs/pairing)
- [Virtual displays](/docs/virtual-displays)
- [Your game library](/docs/game-library)
- [Plugins](/docs/plugins)
- [Desktop at work](/docs/desktop-at-work)
- [Network & VPN](/docs/network-and-vpn)
- [Forgot your Password?](/docs/forgot-password)
- [Troubleshooting](/docs/troubleshooting)
