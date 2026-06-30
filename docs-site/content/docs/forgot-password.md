---
title: Forgot your Password?
description: Where the slipstream web console login password lives — and how to read or reset it — on each host platform.
---

The slipstream **web console** (status, paired devices, PIN pairing) is protected by a login
password. That password is generated — or, on Windows, chosen — when the console is first set up, and
it lives on the **host**. So if you can't get past the login screen, you recover or change it on the
host machine itself, not from the browser.

> This is **only** the web console login. It is **not** your client/device pairing — if a client
> won't connect, that's [Pairing](/docs/pairing), not this password.

## Find your host

Jump to your host platform for exactly where the password lives and how to read or reset it:

| Host | Where the password lives | Section |
|------|--------------------------|---------|
| **Ubuntu — GNOME** | `~/.config/slipstream/web-password` | [Console login password](/docs/ubuntu-gnome#console-login-password) |
| **Ubuntu — KDE Plasma** | `~/.config/slipstream/web-password` | [Console login password](/docs/ubuntu-kde#console-login-password) |
| **Fedora — KDE Plasma** | `~/.config/slipstream/web-password` | [Console login password](/docs/fedora-kde#console-login-password) |
| **Bazzite — gamescope** | `~/.config/slipstream/web-password` | [Console login password](/docs/bazzite#console-login-password) |
| **SteamOS (host)** | `~/.config/slipstream/web.env` | [Console login password](/docs/steamos-host#console-login-password) |
| **Windows host** | `%ProgramData%\slipstream\web-password` | [Console login password](/docs/windows-host#console-login-password) |

## The short version

**Linux packages (apt / RPM / Bazzite).** The password is generated on first start and saved to
`~/.config/slipstream/web-password`. Read it back:

```sh
# from the init service's journal (printed once, when it was generated):
journalctl --user -u slipstream-web-init | sed -n 's/.*password generated: //p'
# …or straight from the file:
sed -n 's/^SLIPSTREAM_UI_PASSWORD=//p' ~/.config/slipstream/web-password
```

Change it by editing that file (`SLIPSTREAM_UI_PASSWORD=<your-password>`) and restarting the console:
`systemctl --user restart slipstream-web`.

**SteamOS / Steam Deck.** Same idea, but the installer writes it to `~/.config/slipstream/web.env`
and prints it at the end of the install run:

```sh
sed -n 's/^SLIPSTREAM_UI_PASSWORD=//p' ~/.config/slipstream/web.env
```

Edit that file and `systemctl --user restart slipstream-web` to change it.

**Windows.** You pick the password during install (a secure random default is pre-filled and shown
on the installer's final page). It lives in `%ProgramData%\slipstream\web-password`. To change it,
edit the file and restart the **SlipstreamWeb** task — in an **elevated** PowerShell:

```powershell
notepad "$env:ProgramData\slipstream\web-password"   # set SLIPSTREAM_UI_PASSWORD=<your-password>
schtasks /End /TN SlipstreamWeb; schtasks /Run /TN SlipstreamWeb
```

Still stuck? See [Troubleshooting](/docs/troubleshooting).
