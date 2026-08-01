---
title: The Web Console
description: Enable the Slipstream browser console, read or change its login password, arm PIN pairing, and what every page in it does.
---

The web console is the browser UI for a Slipstream host — live status, pairing, display policy, the
game library, logs, plugins and host updates. It ships as the **`slipstream-web`** systemd user unit
on Linux and runs under the **Slipstream Host service** on Windows, and serves on **`https://<host-ip>:47992`**
(HTTPS with the host's own self-signed identity cert — your browser warns once; trust it and
continue). It's the surface you expose on the LAN to administer the host; the host's own management
API (47990) keeps every admin action loopback-only and off-loopback serves only read-only status +
game-library browsing to paired clients.

> New here? Read [Security & Safe Use](/docs/security) first — a streaming host is remote control of
> the machine, so keep it on a trusted LAN or VPN and require pairing.

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
  optional dependencies — so install it yourself from the same repo the host came from (see
  [Arch Linux](/docs/arch)), then enable it exactly as above. Take it as a full `-Syu`, never a bare
  `pacman -S`, so you don't end up on a partial upgrade:

  ```sh
  sudo pacman -Syu slipstream-web
  systemctl --user enable --now slipstream-web
  ```

- **Windows host:** the installer sets up the console and its runtime; the Slipstream Host service
  runs it and automatically brings it back if it ever stops. There is nothing to enable — open
  `https://<this-PC>:47992`.

- **SteamOS host:** the install script builds and starts the console as a user service for you. It
  prints the URL when it finishes.

## Login password

The console is password-protected. Where that password lives and how you change it depends on the
host platform.

**Linux packages (apt / RPM / Bazzite).** On a new install, open the console and choose the password
when the setup screen appears. It is then saved to `~/.config/slipstream/web-password` as
`SLIPSTREAM_UI_PASSWORD=…`.

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

**Windows host.** You choose the password during install — a secure random default is pre-filled and
shown again on the installer's final page. It's stored in `%ProgramData%\slipstream\web-password` (as
`SLIPSTREAM_UI_PASSWORD=…`), readable only by Administrators and SYSTEM. To change it, edit the file
and restart the Slipstream Host service (which runs the console) in an **elevated** PowerShell:

```powershell
notepad "$env:ProgramData\slipstream\web-password"   # set SLIPSTREAM_UI_PASSWORD=<your-password>
slipstream-host service restart
```

Forgot it? See [Forgot your Password?](/docs/forgot-password).

## Arm pairing

The host **requires PIN pairing** by default (secure on a LAN). To connect the first time, open the
console, log in, then open **Pairing** in the sidebar and click **Pair a device**. The host shows a
one-time 4-digit PIN — enter it on your [client](/docs/clients) to pair. If the device already tried
to connect it appears under **Waiting for approval** instead; approving it pairs it immediately, no
PIN needed. See [Pairing & Trust](/docs/pairing) for the full trust model and how to approve or
remove devices later.

## What's in it

Nine destinations in the sidebar (a **More** tab on a phone holds the last five):

- **Dashboard** — live status: whether video and audio are streaming, the active sessions with
  their codec, resolution, frame rate and bitrate, which games are running, and how many clients
  are paired. Buttons here stop a session or ask the encoder for a fresh keyframe.
- **Host** — this host's identity (hostname, OS, local IP, version, unique id), the codecs it
  advertises, its ports, the **Updates** card (see [Updating the Host](/docs/updating)), the
  **GPUs** card — Automatic, or prefer one GPU for capture and encode, applied to the next session
  — and the compositor backends it found.
- **Virtual displays** — the policy for the display each session gets, and the Streamed screen
  picker. See [Virtual displays](/docs/virtual-displays).
- **Library** — the games every client sees: turn a launcher source on or off, add or edit a custom
  title with its own art and launch command. See [Your game library](/docs/game-library).
- **Performance** — arm a capture, run a session, stop it, and read the recording back as
  per-stage latency, throughput and health graphs.
- **Logs** — the host's recent log stream: follow it live, filter by level, search it, and download
  or share it for a bug report.
- **Pairing** — arm a PIN, approve or deny devices waiting for approval, and unpair a device. A
  second PIN box for [Moonlight/GameStream](/docs/moonlight) clients appears only when this host
  runs the GameStream plane.
- **Plugins** — the plugin store's **Browse**, **Installed** and **Sources** tabs plus the plugin
  runner switch; an installed plugin with a UI gets its own entry below. See
  [Plugins](/docs/plugins).
- **Settings** — the console's language, and **Sign out**.
