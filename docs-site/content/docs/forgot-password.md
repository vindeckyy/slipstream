---
title: Forgot your Password?
description: Where the Slipstream web console login password lives — and how to read or reset it — on each host platform.
---

The Slipstream **web console** (status, paired devices, PIN pairing) is protected by a login
password. New Linux and SteamOS installs ask you to choose it in the browser; Windows asks during
installation. It lives on the **host**, so if you can't get past the login screen, recover or change
it on the host machine itself.

New to the console? See [The Web Console](/docs/web-console) to enable it and arm pairing.

> This is **only** the web console login. It is **not** your client/device pairing — if a client
> won't connect, that's [Pairing](/docs/pairing), not this password.

## Find your host

Find your host platform for exactly where the password lives, then read it back or change it below:

| Host | Where the password lives | Section |
|------|--------------------------|---------|
| **Linux packages (apt / RPM / Arch / Bazzite / NixOS)** | `~/.config/slipstream/web-password` | [Login password](/docs/web-console#login-password) |
| **SteamOS (host)** | `~/.config/slipstream/web-password` | [Login password](/docs/web-console#login-password) |
| **Windows host** | `%ProgramData%\slipstream\web-password` | [Login password](/docs/web-console#login-password) · [Windows Host](/docs/windows-host) |

## Read it back, or set your own

The password is stored on the host as a `SLIPSTREAM_UI_PASSWORD=…` line, so you can read it straight
out of the file. On the **Linux packages** and the **SteamOS host**:

```sh
sed -n 's/^SLIPSTREAM_UI_PASSWORD=//p' ~/.config/slipstream/web-password   # Linux packages
sed -n 's/^SLIPSTREAM_UI_PASSWORD=//p' ~/.config/slipstream/web.env        # SteamOS host
```

On a **Windows host**, from an **elevated** PowerShell (the file is readable only by Administrators
and SYSTEM):

```powershell
Get-Content "$env:ProgramData\slipstream\web-password"
```

To replace it with one you pick, follow [Login password](/docs/web-console#login-password). It has
the exact edit-and-restart steps for each of the three platforms above, and it's the one place that
procedure is kept up to date.

## The password is right and it still won't let you in

The login screen says **"Wrong password."** for every failure, including two that have nothing to do
with the password you typed.

- **Too many attempts.** Five wrong guesses from the same device are free; every one after that
  arms a lockout that doubles — a second, two, four — up to **five minutes**. While it holds, even
  the correct password is refused. Wait it out, or clear it at once by restarting the console (the
  lockout is only kept in the console's memory):

  ```sh
  systemctl --user restart slipstream-web
  ```

  ```powershell
  slipstream-host service restart
  ```

  (The PowerShell one is Windows, from an **elevated** prompt — the console runs under the
  Slipstream Host service there.)
- **No password is configured at all.** If the file is missing or empty, the console shows the
  first-run setup screen. Choose a password there. On older Linux installs you can intentionally
  **delete** `~/.config/slipstream/web-password` and restart the console to return to setup:

  ```sh
  systemctl --user restart slipstream-web
  ```

  If the setup screen cannot save the password, restore the line
  `SLIPSTREAM_UI_PASSWORD=<your-password>` in the file and restart again.

Still stuck? See [Troubleshooting](/docs/troubleshooting).
