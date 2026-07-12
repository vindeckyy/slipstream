---
title: The Web Console
description: Enable the slipstream browser console, read or change its login password, and arm PIN pairing.
---

The web console is the browser UI for a slipstream host — live status, paired devices, and the PIN
pairing flow. It ships as the **`slipstream-web`** systemd user unit on Linux and the **`SlipstreamWeb`**
task on Windows, and serves on **`https://<host-ip>:47992`** (HTTPS with the host's own self-signed
identity cert — your browser warns once; trust it and continue). It's the surface you expose on the LAN to
administer the host; the host's own management API (47990) keeps every admin action loopback-only and
off-loopback serves only read-only status + game-library browsing to paired clients.

> New here? Read [Security & Safe Use](/docs/security) first — a streaming host is remote control of
> the machine, so keep it on a trusted LAN or VPN and require pairing.

## Enable the console

- **Linux packages (apt / RPM / Bazzite):** `slipstream-host` recommends `slipstream-web`, so your
  package manager pulls it in. Enable and start it as your desktop user, then open the URL:

  ```sh
  systemctl --user enable --now slipstream-web
  # then browse to https://<host-ip>:47992
  ```

- **Windows host:** the installer sets up the console, its runtime, and the `SlipstreamWeb` task and
  starts it at boot. There is nothing to enable — open `https://<this-PC>:47992`.

- **SteamOS host:** the install script builds and starts the console as a user service for you. It
  prints the URL when it finishes.

## Login password

The console is password-protected. Where that password lives and how you change it depends on the
host platform.

**Linux packages (apt / RPM / Bazzite).** On first start `slipstream-web-init` generates a random
password and saves it to `~/.config/slipstream/web-password` (as `SLIPSTREAM_UI_PASSWORD=…`). Read it
back from the init service's journal or straight from the file:

```sh
journalctl --user -u slipstream-web-init | sed -n 's/.*password generated: //p'
sed -n 's/^SLIPSTREAM_UI_PASSWORD=//p' ~/.config/slipstream/web-password
```

To set your own, edit that file (`SLIPSTREAM_UI_PASSWORD=<your-password>`) and restart the console:
`systemctl --user restart slipstream-web`.

**SteamOS host.** Same idea, but the install script writes the generated password to
`~/.config/slipstream/web.env` and prints it at the end of the install run:

```sh
sed -n 's/^SLIPSTREAM_UI_PASSWORD=//p' ~/.config/slipstream/web.env
```

Edit that file and `systemctl --user restart slipstream-web` to change it.

**Windows host.** You choose the password during install — a secure random default is pre-filled and
shown again on the installer's final page. It's stored in `%ProgramData%\slipstream\web-password` (as
`SLIPSTREAM_UI_PASSWORD=…`), readable only by Administrators and SYSTEM. To change it, edit the file
and restart the task in an **elevated** PowerShell:

```powershell
notepad "$env:ProgramData\slipstream\web-password"   # set SLIPSTREAM_UI_PASSWORD=<your-password>
schtasks /End /TN SlipstreamWeb; schtasks /Run /TN SlipstreamWeb
```

Forgot it? See [Forgot your Password?](/docs/forgot-password).

## Arm pairing

The host **requires PIN pairing** by default (secure on a LAN). To connect the first time, open the
console, log in, and go to **Devices → arm pairing**. The host displays a 4-digit PIN — enter it on
your [client](/docs/clients) to pair. See [Pairing & Trust](/docs/pairing) for the full trust model
and how to approve or remove devices later.
