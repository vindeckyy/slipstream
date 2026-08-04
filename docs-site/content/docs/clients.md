---
title: Clients
description: Connect to a Slipstream host from iPhone, Android, Steam Deck, or Moonlight.
---

A Slipstream host accepts clients over its own `slipstream/1` protocol (the iPhone, Android, and
Steam Deck apps) and over GameStream (Moonlight). Pick whichever fits the device you're streaming
*to*. Ready to install?
**[Install a Client](/docs/install-client)** has the step-by-step for each named client, plus how to
[update](/docs/install-client#keeping-a-client-up-to-date) and
[remove](/docs/install-client#removing-a-client) each one.

Two things apply to every named app:
[profiles and `slipstream://` links](#profiles-and-links-every-app), and the keys and chords that
work [while you're streaming](#while-youre-streaming). What each one lets you change, resolution,
bitrate, codec, HDR, audio, controllers, is catalogued in
[Client settings](/docs/client-settings).

## iPhone

The native app for iPhone speaks Slipstream's own [`slipstream/1`](/docs/how-it-works#two-protocols)
protocol, the lowest-latency, most resilient path, with the full feature set:

- **Automatic host discovery**, hosts on your network appear under *On this network*; no IP typing.
- **PIN pairing** built in, and pinned reconnects after that.
- **Controllers**, including DualSense, rumble, adaptive triggers, lightbar, motion, and touchpad.
- A **[game library](/docs/game-library)**, browse the host's installed games with cover art and
  launch one straight into the stream.
- A live **stats overlay** (resolution, fps, bitrate, latency) and a built-in **network speed test**
  to pick a bitrate for your link.
- **Widgets, Live Activities and Shortcuts**, a hosts widget for the home screen, a Live Activity
  while a session runs, and App Intents so Siri and the Shortcuts app can start a stream.

Open the app, pick your host, [pair](/docs/pairing) once, and stream. It builds from the
`clients/apple` directory in the repo (Swift / VideoToolbox / Metal). Install via
[TestFlight](/docs/install-client#iphone).

## Android

The native Android app speaks `slipstream/1` directly, on both phones and Android TV. It does hardware
HEVC decode (including [HDR10](/docs/hdr#per-client)), Opus audio with a mic uplink, game
controllers with rumble and DualSense feedback, automatic host discovery, PIN pairing with pinned
reconnects, the host's **game library** with cover art, and a live stats overlay, with D-pad and
game-controller focus navigation for the couch. It builds from the `clients/android` directory
(Kotlin + a shared Rust core).

**Controllers.** Plug a **DualSense**, **DualSense Edge** or **DualShock 4** into the phone or tablet
by USB and grant the USB permission Android asks for when it attaches, Slipstream then drives the pad
itself instead of taking what Android's gamepad layer exposes, so the host gets rumble, adaptive
triggers, the lightbar and gyro. The app's **Controllers** screen lists attached pads and their
capture state, and the switch that turns this off is *DualSense / DualShock passthrough (USB)* in
Settings. Over **Bluetooth** the pad still works as an ordinary gamepad, but adaptive triggers and
the lightbar need the USB connection.

The app is on Google Play as a **test track** (closed testing for stable, internal testing for
canary), request a tester invite on our [**Discord**](https://discord.gg/kaPNvzMuGU) and we'll add
you, or sideload the public APK instead (see
[Install a Client](/docs/install-client#android)). Then open the app, pick your host,
[pair](/docs/pairing) once, and stream.

## Steam Deck

On Steam Deck, Gaming Mode is the named product path: the **[Decky plugin](/docs/steam-deck)** adds a
Slipstream panel to the Quick Access Menu. Under the hood the plugin launches the Flatpak
`slipstream-client` (GTK4) so gamescope can fullscreen the stream. Desktop Mode can run that same
Flatpak directly.

- **Gaming Mode** -> [Decky plugin](/docs/steam-deck) (install Decky Loader, the plugin, and the
  Flatpak once).
- **Desktop Mode** -> Flatpak, see [Install a Client -> Steam Deck](/docs/install-client#steam-deck).

The Deck's built-in controls forward as a Steam Deck pad (paddles, trackpads, gyro) when Steam Input
is set correctly; the Decky guide covers that.

## Moonlight (optional)

Slipstream also speaks the **GameStream** protocol, so any [Moonlight](https://moonlight-stream.org/)
client, a browser, a smart TV, an old phone, a games console, connects with no slipstream-specific
software. Moonlight is the catch-all for devices without a named Slipstream app. See
[Connect with Moonlight](/docs/moonlight).

This is the broadest-compatibility option and great for couch gaming. It doesn't use the native
protocol's FEC/encryption extensions, but for a healthy LAN that rarely matters.

## Profiles and links (every app)

Two things work the same in the iPhone, Android, and Steam Deck apps. **Settings profiles** are
named sets of stream overrides, bitrate, resolution, codec, HDR and the rest, that you bind to a
host or pick for a single connect, with every field you didn't touch still following your defaults.
And a **`slipstream://` link** starts a stream from a shortcut or automation, carrying only
*references* to things that already exist on your device, never a setting, and never a trust
decision.

[Profiles and links](/docs/profiles-and-links) has both in full: the link grammar, where each app
puts **Copy link**, and what a link is refused for.

## While you're streaming

When a client **captures** input for games, you need a way to release it. On clients that support
desktop shortcuts, **Ctrl+Alt+Shift+Q** gives mouse and keyboard back. On Steam Deck, hold
**[L1 + R1 + Start + Select](/docs/input#leaving-with-a-controller)** to leave a stream.

That chord, the others a stream reserves, the two mouse modes, the three touch modes and stylus
input are all on [Mouse, touch and pen](/docs/input#getting-your-input-back).

Copying between the two machines is a separate opt-in: the host operator allows it in `host.env`
and you turn it on for that one host in your client. See [Shared clipboard](/docs/clipboard).

## Which should I use?

| You're streaming to... | Use |
|---|---|
| An **iPhone** | The **[iPhone app](#iphone)** (TestFlight) |
| An **Android** phone or TV | The **[Android app](#android)** |
| A **Steam Deck** | The **[Decky plugin](/docs/steam-deck)** in Gaming Mode, or the Flatpak in Desktop Mode |
| A browser, smart TV, or any other device | **[Moonlight](/docs/moonlight)** (optional catch-all) |

Whichever you choose, the first connection needs a one-time [pairing](/docs/pairing), and
[Install a Client](/docs/install-client) covers installing, updating and removing it.
