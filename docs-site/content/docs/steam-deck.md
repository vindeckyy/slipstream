---
title: Steam Deck (Decky)
description: Install the Slipstream Decky plugin to discover, pair, and stream from the Steam Deck's Gaming Mode, no drop to Desktop.
---

The **Decky plugin** adds a **Slipstream** panel to the Steam Deck's Quick Access Menu (the `...`
button), so you can find a host, pair, and start streaming **without leaving Gaming Mode**. It's the
couch-friendly front end for the Steam Deck, built from real Steam UI, gamepad-navigable end to end.

Under the hood the plugin doesn't decode video itself: it discovers hosts, runs the PIN pairing, and
**launches the Flatpak client** (`io.slipstream`) the way gamescope needs so it fullscreens
correctly. So the Deck has two ways to stream, and they share one client + one paired identity:

- **Gaming Mode** -> the **Decky plugin** (this page).
- **Desktop Mode** -> run the [Flatpak](/docs/install-client#steam-deck) directly.

## Before you start

You need three things on the Deck:

1. **Decky Loader**, the plugin loader. Install it from [decky.xyz](https://decky.xyz/) if you
   haven't already.
2. **A Slipstream client on the Deck**, the plugin doesn't decode video itself, it launches a
   client. On a normal Deck that's the Flatpak, installed once in **Desktop Mode**:

   ```sh
   # Build locally or install a .flatpak from GitHub Releases when attached:
   # flatpak install --user --bundle /path/to/slipstream-client.flatpak
   ```

   (Full options: [Install a Client -> Steam Deck](/docs/install-client#steam-deck).) If you have
   no Flatpak but a native `slipstream-client`, a sysext, a distro package, a nix profile, your own
   build, the plugin launches that instead; with both installed the Flatpak wins, unless
   `PF_DECKY_CLIENT=native` (or `flatpak`) is set in the plugin backend's environment. But
   **pairing, Wake-on-LAN and the host game library still go through the Flatpak**, so install it
   on the Deck even then. Both kinds share `~/.config/slipstream`, so your identity, known hosts
   and settings are the same either way.
3. **A Slipstream host** running on your LAN, see [Install the Host](/docs/install). The Deck finds
   it automatically over mDNS, so nothing to configure here.

## Install the plugin

The plugin is published as a ready-to-install zip on every build. You don't need the Decky CLI or a
developer toolchain, just paste a URL into Decky:

1. On the Deck, open the **Quick Access Menu** (`...`) -> the **plug** icon (Decky) -> the **gear**
   (Settings) -> enable **Developer Mode**.
2. Open the new **Developer** tab and choose **Install Plugin from URL**.
3. Paste a URL to a `slipstream.zip` you built (`clients/decky` -> `pnpm run package`) or downloaded
   from [GitHub Releases](https://github.com/vindeckyy/slipstream/releases) when attached, then
   confirm.

The **Slipstream** panel appears in the Quick Access Menu right away, no Deck restart needed.

> **Channels.** Publish separate stable and canary zips if you want two tracks, see
> [Release Channels](/docs/channels). For local development, sideload with `pnpm run deploy` from
> `clients/decky`.

## Use it

Open the **Slipstream** panel from the Quick Access Menu, or **Open Slipstream** for the full-screen
page (host list + stream settings).

- **Discover**, hosts on your network appear automatically (mDNS). Tap **Refresh** to rescan. A
  lock icon means the host requires [pairing](/docs/pairing).
- **Add a host by hand**, if mDNS can't reach it (another subnet, a VPN), tap **+** on the Hosts
  tab and enter its address; the port defaults to **9777**. Saved hosts can be renamed, re-pointed
  at a new address, or forgotten from the same row.
- **Sleeping host?** Streaming sends a [Wake-on-LAN](/docs/wake-on-lan) packet first, and when one
  actually went out the Deck waits far longer than usual for the host to answer, so a stream
  survives a resume from sleep. Nothing to enable, it's a no-op until the plugin has learned that
  host's MAC address, and the packet only lands if the host machine is armed to wake in its
  BIOS and its network card.
- **Pair**, for a locked host, [arm pairing on the host](/docs/pairing) (its console or web
  console shows a 4-digit PIN), then enter that PIN on the Deck's keypad. Pairing persists, so the
  next connection is silent.
- **Stream**, pick a host and the stream launches fullscreen in Gaming Mode. The plugin drives a
  hidden Steam shortcut behind the scenes so gamescope focuses and fullscreens it.
- **Library entry**, a visible, branded **Slipstream** app also appears in your Steam library.
  Launching it opens the client's console home (host picker, pairing, settings), gamepad-navigable
, it does not resume a stream. If it ever disappears, the Quick Access Menu panel has a button to
  put it back.
- **Games**, tap **Games** on a host row to browse that host's [library](/docs/game-library), and
  **Pin** the ones you play. Pinned games show up on the full page *and* in the Quick Access Menu
  as one-tap streams that launch straight into the game.
- **Settings**, resolution, refresh rate, **render scale**, bitrate, **video codec**, gamepad type,
  **host compositor**, and mic, written to the client the plugin launches. Leave **Resolution** /
  **Refresh** on *Native* to get the Deck's own mode, **Render scale** at 1x unless you want to
  trade bandwidth for sharpness (>1x) or sharpness for bandwidth (<1x), and **Video codec** /
  **Host compositor** on *Automatic*, that suits almost every host, so change them only when
  you're troubleshooting. With **Gamepad type** on *Automatic* the Deck's built-in controller is
  forwarded as a **Steam Deck** pad (paddles, both trackpads, gyro), that needs Steam Input set to
  **Off** for Slipstream (game page -> ⚙ -> Controller Settings), else Steam keeps those controls and
  only sticks + buttons reach the host.

> **Steam Input off is a trade-off, not a free win.** The plugin installs a Steam Input layout
> called **Slipstream** and points its shortcuts at it, and that layout's whole job is making the
> Deck's touchscreen arrive at the stream as *real touch*. Leaving Steam Input **On** with that
> layout gives you native touch plus a standard gamepad; setting it **Off** gives you the full Steam
> Deck pad, paddles, both trackpads, gyro, but the touchscreen stops working as touch. Pick per
> game, on the game page -> ⚙ -> **Controller Settings**.

To **leave a stream**: **hold [L1 + R1 + Start + Select](/docs/input#leaving-with-a-controller)**
for about a second and a half, or close the "game" from the Steam overlay. Either ends the session
and drops you straight back to Gaming Mode. A quick press of the same four only releases captured
input, so it is safe to hit by accident.

## Updating

The plugin **checks for updates itself**, no Decky store needed. It covers **both** the plugin *and*
the streaming client (they version independently), so when either has a newer build the panel shows an
**Update** button (in the Quick Access Menu and on the full page). Tap it: the client updates in
place, and if the plugin itself changed it downloads, verifies, replaces itself, and reloads, all
without leaving Gaming Mode.

One exception: if your client isn't one the plugin can install for you (a sysext, a nix profile, a
source build), the panel shows you the update **command** instead of a button, tap-to-install would
only fail. A pending plugin update still gets its button.

The plugin check follows the [channel](/docs/channels) you installed from: a plugin installed from the
**stable** link tracks stable releases; one installed from the **canary** link tracks `main` builds.

> **Updating the client from the terminal?** The Flatpak client is installed **per-user**, so run
> `flatpak update --user io.slipstream`, **without `sudo`**. `sudo flatpak update` only touches
> the *system* installation and silently skips the client. (Un-sudo'd `flatpak update` updates both
> scopes, so it's the safe default.)

> If the plugin **Update** button never appears (an older Decky Loader, or no network), update the
> plugin manually: Decky -> **Developer** -> **Install Plugin from URL**, and paste the same channel
> link again. Decky replaces the installed copy in place.

## Troubleshooting

| Symptom | Fix |
|---|---|
| The stream never starts, **Pair** reports `flatpak-not-found`, or **Games** says the client isn't installed | Install the client Flatpak in Desktop Mode (see [Before you start](#before-you-start)). |
| No hosts listed | Make sure the host is running and on the **same LAN**; the Deck needs `avahi` (shipped on SteamOS). Tap **Refresh**. |
| Pairing fails / "not armed" | The PIN is shown only after you **arm pairing on the host**. Arm it, then enter the PIN within the window. |
| Stream launches but doesn't focus | Start it from the panel (not by launching the Flatpak by hand) so Steam/gamescope focuses it. |
| The stream wedges, black, or won't close | Open the full page -> **About** tab -> **Force-stop**, then start it again. |
| The **Slipstream** library entry disappeared | Quick Access Menu -> **Recreate library shortcut**; it puts the entry back in place. |
| You want a clean slate | **About** tab -> **Reset Slipstream**, clears saved hosts, stream settings and pinned games on this Deck, and keeps your paired identity. |

Nothing here matching? The problem is probably on the host side, start at
[Troubleshooting](/docs/troubleshooting), which is organised by symptom (host not found, pairing
rejected, black picture).

## Uninstalling

Removing the plugin through Decky removes the plugin and nothing else, so do these in order:

1. **Remove the plugin.** Quick Access Menu (`...`) -> the **plug** icon (Decky) -> the **gear**
   (Settings) -> **Plugins** -> **Slipstream** -> **Uninstall**.
2. **Remove the Steam shortcuts it created.** The plugin adds two non-Steam entries, both named
   **Slipstream**, the one you see in your library, and a second one it keeps hidden to carry the
   stream. Decky removes neither. In your library, right-click a **Slipstream** entry ->
   **Manage -> Remove non-Steam game from your library**, and repeat for the hidden one once you've
   let the library show hidden games.
3. **Remove the client**, if you're done streaming on this Deck. In Desktop Mode:

   ```sh
   flatpak uninstall --user --delete-data io.slipstream
   ```

   Your identity and saved hosts live in `~/.config/slipstream` and survive that, delete the
   directory too for a clean slate. See
   [Removing a client](/docs/install-client#removing-a-client).
4. **Revoke the pairing on the host.** The host still trusts this Deck until you remove it from its
   [web console](/docs/web-console), see
   [Managing paired devices](/docs/pairing#managing-paired-devices).

The Steam Input layout the plugin installed also stays behind as a selectable template named
*Slipstream* (`~/.local/share/Steam/controller_base/templates/slipstream.vdf`), along with the
per-account configset entry pointing at it. Leave them, with the shortcuts gone they apply to
nothing, or delete the file if you'd rather not see it offered as a template.

The plugin source lives in
[`clients/decky`](https://github.com/vindeckyy/slipstream/blob/main/clients/decky/README.md).
