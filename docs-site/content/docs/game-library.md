---
title: Your game library
description: How Slipstream finds your installed games, how to add one by hand, and how to launch a title from a client, from Moonlight, or from the command line.
---

Every Slipstream host keeps one **game library**, a single list of titles that every surface reads
from. It has three sources: the launchers the host scans on disk, entries you add by hand in the
[web console](/docs/web-console), and titles a [plugin](/docs/plugins) syncs in.

Whichever source a title came from, it looks the same everywhere: a poster, a name, and a stable id
like `steam:570` or `custom:9f2a1c...`. Pick one on a client and the host launches it into the stream.

## Where your games come from

The host reads your launchers' **own local files**. There are no accounts to connect and no API
keys, nothing leaves the machine to build the list. Each scanner is best-effort: a launcher that
isn't installed simply contributes nothing. Cover art is the one exception, and it needs no account
either, see [Cover art](#cover-art).

Which scanners exist on the Linux host:

| Source | What it reads |
|---|---|
| **Steam** | Installed titles from `appmanifest_<appid>.acf` in every Steam library folder, plus your own **non-Steam shortcuts** |
| **Lutris** | The local Lutris database (`pga.db`) |
| **Heroic (Epic / GOG / Amazon)** | Heroic Games Launcher's local library cache, all three of its backends |

Every scanner is **on by default**.

A few things are deliberately left out. Steam's tooling, Proton, the Steam Linux Runtimes, Steamworks
Common Redistributables, SteamVR, is filtered out, so your grid holds games rather than plumbing. A
non-Steam shortcut you have hidden inside Steam stays hidden here too.

To see exactly what the host resolved, run [`slipstream-host library`](/docs/host-cli) on the host: it
prints the whole library as JSON. That answers "does the host see my games?" without involving a
client.

## Turning a source off

The console's **Library** page has a **Game sources** card with one chip per scanner this host
supports. A chip is highlighted when the host scans that launcher; click it to turn the scanner off.

Turning a source off hides its titles from **everywhere at once**, the console grid, every native
client, the Moonlight app list, and launching. Nothing is deleted, the change needs no restart, and
turning the source back on brings the titles straight back on the next read.

Your hand-added entries are not a scanner and have no chip, they are always shown.

The choice is stored per host in `library-scanners.json`, next to the rest of the host config
(`~/.config/slipstream/`). Only the sources you turned *off* are written down, so a scanner added by
a future release starts enabled.

## Adding a game by hand

Anything your launchers don't know about, an emulator, a ROM, a DRM-free build, a tool you want on
the couch, goes in by hand. On the console's **Library** page, click **Add custom game**.

**Title** is the only required field. **Launch command** is the command the host runs for this title;
leave it empty and the entry is a poster the host has nothing to launch from.

Under **Details (optional)** a title can carry:

| Field | Notes |
|---|---|
| Platform | The system it runs on, `PS2`, `Xbox 360`, `SNES`, `PC`. Scanned titles are all stamped `PC` |
| Description | A short blurb |
| Developer / Publisher | Free text |
| Release year | Shown next to the title on the poster tile |
| Players | Maximum local players |
| Region | `NTSC-U`, `PAL`, `NTSC-J` |
| Genres | Comma-separated |
| Tags | Comma-separated labels of your own, `co-op`, `kids`, `finished` |

Every field is optional and free-form; the host doesn't normalize the values. A poster tile shows the
platform badge only when it isn't `PC`, since that would be true of everything scanned.

Manual entries live in `library.json` in the host config directory. That file drives commands the host
runs, so it is locked down to the host user (0600), treat what you type there as operator-level
configuration.

> **Editing replaces the whole entry.** The console form re-sends every field it knows about, so
> nothing you can see is lost. Fields the form has no input for, prep/undo steps in particular, are
> **cleared** when you save an entry through the form.

### Cover art

The form takes four artwork URLs: **Portrait art URL** (the 2:3 poster, best for a grid), **Hero**,
**Header** and **Logo**. The console grid falls back from portrait to header, and to a plain text tile
when a title has neither.

Use a full `http://` or `https://` URL. A plain path like `/home/me/cover.jpg` is **not** recognized
this way.

Scanned titles need no art. Steam covers come from your local Steam cache, falling back to Steam's
public CDN.

## Games from a plugin

A [plugin](/docs/plugins) can own a slice of the library and keep it in sync, this is how the ROM
Manager plugin gets your collection into the grid, box art and all.

Entries a plugin owns are read-only to you. The host refuses a hand edit or a delete of one, because
the next sync would overwrite it anyway, change the title at its source and let the plugin sync
again. Only the plugin can remove its own entries, and it removes every one of them at once. Your
hand-added entries are never touched by a sync.

The console grid can't tell you which entries those are: a plugin's titles carry the same **Custom**
badge as your own and still show **Edit** and **Delete** on hover. The form and the delete
confirmation open as usual, but the host refuses the change and the entry stays exactly as it was.

## Launching a game

Whatever the surface, the client sends only an **id**. The host looks that id up in its own library
and runs what it already knows about the title, so a client can never hand the host a command to run.

- **Native clients**, the browser is a per-device setting in **Settings -> Library**, and it needs a
  **paired** host. It is **on by default on Android**. Turn it on and a paired host's card
  offers **Browse library...**; pick a title and the stream starts with the host launching it. See
  [Client settings](/docs/client-settings).
- **Android**, the library lives only in the controller-optimized home, which a TV always uses and a
  phone or tablet switches to when a controller is connected. Press **Y** on a saved host, or open its
  options and choose **Library**.
- **Steam Deck (Decky)**, the plugin's per-host **Games** picker lists the library and lets you
  **Pin** titles; a pinned game becomes a one-tap row under **Pinned Games** in the Quick Access Menu.
  The picker itself doesn't launch anything, either tap a pinned row, or use **Open library on
  screen** to browse the host's games full-screen on the Deck and launch from there. See
  [Steam Deck](/docs/steam-deck).
- **Moonlight**, when the host runs with `--gamestream`, your library appears in Moonlight's app
  list beside `Desktop`, with covers served by the host. A title keeps the same app id across host
  restarts, so Moonlight's cached tiles stay correct. Titles with no launch recipe are left out.
  See [Moonlight](/docs/moonlight).
- **A link**, a [`slipstream://` link](/docs/profiles-and-links) carries the id in a `launch=`
  parameter, so a desktop shortcut, a browser bookmark or a home-automation rule starts the stream
  with the title already launching: `slipstream://connect/couch-pc?launch=steam:570`.
- **The command line**, the client's own [`slipstream`](/docs/host-cli#slipstream-on-the-client-machine)
  command where packaged. `slipstream library <host-ref>` prints `id`,
  `store` and `title` as tab-separated lines, then a count (`--json` for tools);
  `slipstream launch <host-ref> --game <id>` starts a stream that launches the title. A `<host-ref>` is
  a saved host's name, id or address, and both commands need it paired.

```bash
slipstream library couch-pc
slipstream launch couch-pc --game steam:570
```

How the game is actually started: the host runs the resolved command, `steam steam://rungameid/...`,
`lutris lutris:rungameid/...`, `heroic://launch...`, or your own command line. A title with no
runnable command can't be launched from a client at all.

Where the game *lands*, your live desktop, an existing gamescope session, or a dedicated headless
one, is display policy, covered in
[Dedicated game sessions](/docs/virtual-displays#dedicated-game-sessions).

## When a game or the session ends

The host tracks the game it launched, so quitting the game can end the session and stopping the
session can close the game. Both switches live on the console's **Virtual displays** page, see
[When a game ends, and when a session does](/docs/virtual-displays#when-a-game-ends-and-when-a-session-does).

## Prep and undo steps for one title

A custom entry can carry `prep` steps that run before it launches and undo steps that run when the
session ends, an HDR toggle, an audio-sink switch, a VRR tweak. They are documented with the rest of
the automation surface in [Events & hooks](/docs/automation#per-app-prepundo).
