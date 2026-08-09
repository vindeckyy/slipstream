---
title: Shared clipboard
description: Copy on one machine and paste on the other, the two switches that have to be on, what actually crosses, and why the toggle does nothing when only one of them is flipped.
---

Slipstream can share the clipboard between the machine you are sitting at and the host you are
streaming. Copy a URL on your phone, paste it into a browser on the host. Copy an error message on
the host, paste it into a chat app on your phone. For office remote-desktop setups this is one of
the first features to turn on - see [Desktop at work](/docs/desktop-at-work).

**Two separate switches have to be on:**

1. The **host** operator has to allow it, with a line in `host.env` and a host restart. This one is
   off by default.
2. **You** have to turn it on for that one host, in that host's edit sheet on your client (where the
   client offers one). This one is **on by default on Android**.

Flipping one and not the other looks exactly like the feature not existing. So check both.

## 1. Allow it on the host

Add a `SLIPSTREAM_CLIPBOARD` line to the host's `host.env`, `~/.config/slipstream/host.env`.

```ini
SLIPSTREAM_CLIPBOARD=on
```

The accepted values:

| Value | Effect |
|---|---|
| unset, empty, `0`, `off`, `false` | **Off (the default).** The host never advertises the clipboard capability and never accepts a clipboard transfer. |
| `text-only`, `no-files`, `text` | On for text, HTML, rich text and images. File transfer is refused. |
| `on`, `1` | On, and file transfer is permitted by policy. |

Values are trimmed and compared case-insensitively. **Anything the host doesn't recognise is
treated as `on`**, a typo like `SLIPSTREAM_CLIPBOARD=yes` or `no-file` enables the permissive
policy rather than failing, so check the spelling if you meant `text-only`.

The file is only read at startup, so restart the host:

```bash
systemctl --user restart slipstream-host
```

See [Configuration](/docs/configuration) for the rest of `host.env`.

> **About the file mode.** No client shipping today asks for file transfer, and no host clipboard
> backend offers file formats yet. `on` and `text-only` therefore behave the same in practice,
> `text-only` is how you make that explicit and keep it that way.

## 2. Turn it on for that host, in your client

The client switch is **per saved host**, not global: handing a machine your clipboard is a decision
about *that* machine. You set it in the host's edit sheet, and it is deliberately not something a
[settings profile can carry](/docs/profiles-and-links#what-a-profile-cant-change).

| Client | Where the switch is | Label | Default |
|---|---|---|---|
| Android (touch) | Host card menu -> **Edit...** | **Shared clipboard** | **On** |

On Android the switch is only in the touch edit dialog. The controller/TV interface, what you get
on Android TV, and on a phone when a controller is attached, has its own **Edit Host** screen with
no clipboard row, so there is nowhere to change it there. It stays on, which is the Android default.

The setting is read when a session starts, so if you change it while streaming, reconnect.

The Steam Deck Decky plugin has no clipboard switch, see
[what each client does](#which-hosts-and-clients-support-it) below.

## Nothing crosses until something pastes

A copy costs nothing. When you copy, your machine announces only the **list of formats** it now
holds, no bytes. The bytes are pulled across on a separate transfer, and only when an application
on the other end actually pastes. Copying a large image and never pasting it transfers nothing.

That holds for everything you copy on your own machine, and for both directions on the host. It
does **not** hold for a host copy arriving at an Android client: Android fetches the content straight
away and puts it on your local clipboard, whether or not you ever paste (there is no way to satisfy
a paste from the network at all).

A single transfer is capped at 64 MiB. Nothing else limits size, so a very large host-side copy can
cross to an Android client for a paste that never happens.

The Android client has no secret-content filter, and neither does the host, so a password copied
**on the host** is announced to your client like anything else.

The clipboard rides the native Slipstream protocol's control channel, so it only exists in sessions
from a Slipstream client. A Moonlight client has no clipboard.

## Which hosts and clients support it

**Hosts.** The host needs one of two mechanisms in the session it is streaming:

- `ext-data-control-v1`, KWin, wlroots/Sway and Hyprland. Tried first.
- GNOME's own `org.gnome.Mutter.RemoteDesktop.Session` clipboard, used directly. Tried second.

The older `zwlr-data-control-unstable-v1` is **not** implemented, so a compositor that offers only
that has no backend. Neither does a [gamescope](/docs/gamescope) session.

**Clients**, and what each one actually moves:

| Client | What crosses |
|---|---|
| Android, Android TV | **Plain text only** |
| Steam Deck | Not implemented |

When the client has no clipboard bridge, turning a switch on (if one exists) is harmless but has no
effect today. The Decky plugin on the Steam Deck has no switch at all.

The host side is richer than any client: it can offer and accept text, HTML, RTF, PNG, JPEG and
GIF. What you get is therefore whatever your client supports.

## Why the toggle does nothing (or is greyed out)

On clients that offer a per-host switch, the switch always looks available, and a host that can't do
it simply does nothing. Work through these in order:

- **The host has it off.** The default. Nothing was added to `host.env`, or the value is `off`,
  `0`, `false` or empty. Fix it with step 1 above.
- **`host.env` was edited but the host wasn't restarted.** The file is read once, at startup.
- **The switch is off for this host in your client.** It is per saved host. Check the host's
  **Edit...** sheet, step 2 above.
- **The host's session has no supported backend.** The host allows the clipboard, so it still
  advertises the capability, but it has nothing to read the desktop's clipboard with. This is a
  gamescope session, a compositor with only the old `zwlr-data-control-unstable-v1`, or a GNOME
  session whose Mutter doesn't expose the direct RemoteDesktop clipboard. Nothing on screen tells
  you this apart, the host log does.
- **The host is older than the feature.** A host from before clipboard sync never advertises it.
- **Your client doesn't implement it**, Steam Deck. Nothing crosses regardless of what the host
  allows.
- **You changed the switch while connected.** Reconnect.

Still stuck? The host log records what it decided on each session, a `clipboard control` line with
the resolved state, and a `clipboard backend unavailable` line when the session had nothing to bind
to. That is the fastest way to tell "off by policy" from "no backend", see
[Troubleshooting](/docs/troubleshooting).
