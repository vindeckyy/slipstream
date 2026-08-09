---
title: Profiles and links
description: How settings profiles override your client defaults per host or per connect, and how slipstream:// links start a stream from a shortcut, a script or a browser.
---

Two features that landed together in 0.22.0 and work with each other: **settings profiles**, named
bundles of stream settings you can attach to a host, and **`slipstream://` links**, URLs that start a
stream you have already set up.

Both live in the Android app and Steam Deck client (where the controller shell uses a bound profile).
Neither exists in the host's [web console](/docs/web-console).

The controller-driven surfaces are a half-exception: the Android app's console mode and the Steam
Deck console the Decky plugin launches all *use* the profile a host is bound to, but none of them can
create or edit one. Do that on a phone first.

## What a profile is

A profile is a *sparse* set of overrides on top of your normal client settings. Only the rows you
actually touch are stored. Everything else keeps following your defaults **live**, so changing a
default later also moves every profile that never overrode it.

Touching a row records the override even when you pick the value the default already has. That is a
deliberate *pin*: the profile keeps that value when the default later moves. The only way back to
inheriting is the row's explicit **Reset**.

Where the catalog is kept:

| Client | Stored in |
|---|---|
| Android | app-private storage |
| Steam Deck | `~/.config/slipstream/client-profiles.json` (shared with the session client) |

The catalog is per device, and nothing syncs it, so a profile you make on your phone doesn't appear
on your Deck.

## Creating and editing one

Profiles are created and edited in the client's own **Settings** screen, there is no second editor,
so a profile can never drift from the surface it overrides.

1. Open Settings. At the top is a scope switcher listing **Default settings**, your profiles, and a
   **New profile** entry. Android shows the choices as a row of chips.
2. Create a new profile. Android asks for a name (and a colour) first. Names must be
   unique, ignoring case.
3. Change the rows you want. Every row shows the *effective* value, the inherited default until you
   touch it.
4. A row you have overridden grows a marker and a **Reset** control. Reset drops that one override
   and puts the row back to following your defaults.

Each profile can carry a colour from a small preset palette (the exact swatches differ slightly
between apps). It tints the profile's chip on host cards, so a grid of hosts is readable at a
glance.

Renaming, duplicating (overrides and colour included) and deleting sit on the selected profile's own
chip in Android; tap it a second time.

While a stream runs with a profile, the profile's name closes the first line of the
[stats overlay](/docs/stats) from the Normal tier up.

## What a profile can't change

In profile scope, rows that aren't profileable simply don't render. They are facts about *this
device*, the video decoder and GPU it uses, its audio endpoints, which physical controller you
hold, whether it wakes hosts on connect and whether it shows a game library, not about how you
want a stream to look. **Share clipboard** is out for the neighbouring reason: it's a per-host trust
decision stored on the host record rather than a client setting at all, see
[Clipboard](/docs/clipboard).

The row-by-row list, and why each row stays global, is on
[Client settings](/docs/client-settings#settings-that-are-facts-about-your-device).

## Three ways to use a profile

**Bind it to a host.** Open a saved host's edit sheet and set **Profile**. Every plain click on that
host's card now uses it. This is the only sticky choice.

**Use it once.** A card's menu has **Connect with**, pick a profile for this connect only. It never
rebinds the host. **Default settings** in that menu is a real choice: on a bound host it forces your
globals for one session. (Android lists the same choices flat, as *Connect with: ...*.)

**Pin it as its own card.** A pinned profile gets its own card beside the host, one click, no menu.
Pin it in the host's edit sheet on Android, or from the card menu using **Pin as card: ...**. A
pinned card is a shortcut, not a second host: unpinning
changes neither the profile nor the host's binding.

### Work vs Play on the same host

If one host is both your office desktop and your game PC, make two profiles and bind or pin them:

| Profile | Typical overrides |
|---|---|
| **Work** | Desktop (absolute) mouse, HEVC, full chroma / 4:4:4 when the link allows, HDR off, bitrate high enough for sharp text |
| **Play** | Capture (games) mouse, HDR as you like, bitrate / refresh for the title |

Clipboard stays a per-host trust toggle, not a profile row - see [Clipboard](/docs/clipboard). The
full office checklist is on [Desktop at work](/docs/desktop-at-work).

## Deleting a profile

The confirmation tells you what breaks: how many hosts will fall back to **Default settings**, and
how many pinned cards will disappear. Bindings and pins are deliberately left pointing at the gone
profile rather than rewritten; everywhere they are read, a dangling reference resolves as "no
profile", which is exactly your defaults. Nothing errors and no connect is blocked.

## `slipstream://` links

A link starts a stream on a host this device already trusts. The apps register the scheme with the
operating system, so a link works from a browser (behind the browser's own "open this app?"
prompt), a home-automation rule or a script:

```text
slipstream://connect/<host-ref>[?fp=<64-hex>][&host=<addr[:port]>][&launch=<id>][&profile=<ref>][&name=<label>]
```

`<host-ref>` is a saved host's stable record id, its name (unique, ignoring case), or `addr[:port]`.
Resolution goes in that order, and a name matching two saved hosts is refused rather than guessed.

| Parameter | Means |
|---|---|
| `fp` | the host certificate fingerprint the link expects, 64 hex characters |
| `host` | `addr[:port]` to fall back on when the reference no longer resolves; port defaults to `9777` |
| `launch` | a store-qualified [library](/docs/game-library) id such as `steam:570`, launched on arrival |
| `profile` | a settings profile, by id or unique name, for this connect only |
| `name` | a display label, shown as *claimed*, never trusted |

The scheme and the route word are case-insensitive, a trailing slash is fine, a `#fragment` is
dropped, unknown parameters are ignored, an empty value means "not given", and if a parameter
appears twice the first one wins. `pf://` parses as an input alias, but nothing emits it and no app
registers it with the operating system, so write `slipstream://`.

`connect` is the only route any client acts on today; `wake` and `browse` parse, but every client
answers them with a notice. Values are capped (2048 for the whole URL, 128 for the host reference
and `launch`, 64 for `profile` and `name`), and `launch` must be printable ASCII with no spaces,
quotes, backslashes, `$` or backticks.

Worked examples:

```text
slipstream://connect/Living%20Room%20PC
slipstream://connect/Living%20Room%20PC?launch=steam:570
slipstream://connect/Living%20Room%20PC?profile=Work
```

## What a link can and can't do

The rule the grammar exists to keep: **a link may only do what clicking a card you already have
could do, minus every trust decision.**

- It carries *references*, never values. There is no resolution, bitrate, codec or HDR parameter,
  so a web page cannot shape your session beyond choosing among your own configurations.
- There is no `pair` route and never will be. `slipstream://pair/...` is refused outright;
  [pairing](/docs/pairing) stays something you do with the fingerprint on screen.
- A link naming a host you don't know is never connected. When it carries an address, as
  `<host-ref>` or as `host=`, Android opens the app's normal trust prompt, pre-filled with that
  address and any `fp` the link carried, so the first connect is verified rather than blind. A link
  with no address to fall back on, a bare name or a stale record id, is simply refused with a notice.
- If the link's `fp` contradicts the fingerprint already pinned for that host, it is a hard refusal
  with a notice. Nothing connects.
- A `profile=` that names nothing on this device, or two profiles at once, refuses **before**
  anything is dialled, a shortcut that can't honour its profile says so rather than streaming with
  the wrong settings. A link with no `profile=` honours the host's binding, exactly like a click.
- A link never preempts a running session. Clients say "A session is already running, end it first",
  except when the link points at the host you are already streaming, which just brings the app
  forward.

## Getting a link, and making a shortcut

Android has no copy action yet. On Steam Deck, use a pinned card or a `slipstream://` URL from a
script where the CLI is available.

A copied link carries the host's stable record id, plus `host=` and `fp=` (the fingerprint only when
one is pinned). That is what keeps a shortcut written today working after the host changes address
or you reinstall the client.

Any launcher works as long as it hands the URL to the client. From a script, use the `slipstream`
CLI where packaged:

```bash
slipstream profiles list                          # ids, names, how many settings each overrides
slipstream open 'slipstream://connect/Desk?profile=Work'
```

See [Clients](/docs/clients) for the rest of its verbs.
