---
title: Play
description: Game streaming hub - host setup, Capture mouse, game library, display presets, HDR, controllers, Moonlight vs native, bitrate for 4K and high refresh, and honest limits.
---

Slipstream is built for games as much as for desk work. A lot of people run the host on a powerful
PC (or a Steam Deck / Bazzite couch box) and connect from a TV, a phone, or a Deck so
they can play **on the machine that has the GPU**, with a virtual display sized to the client and
controllers flowing back into the host session.

This page is the Play path. It assumes you can already install a host and pair a client; if you
cannot, finish [Quick Start](/docs/quickstart) on a trusted LAN first, then come back here for the
couch setup.

If you also need that same host as a remote office desktop, use a separate **Work** settings
profile and follow [Desktop at work](/docs/desktop-at-work) - same host process, different mouse,
display, and picture choices.

> **Security first.** A streaming host is remote control of the machine. Prefer a **trusted LAN**
> for Play. If you reach the host from another network, use a **private VPN** and **do not
> port-forward** Slipstream to the public internet. Read [Security & Safe Use](/docs/security).

## Who this is for

- You have a **gaming PC, HTPC, or handheld host** you want to stream from.
- You sit on a **couch client** (TV, phone, Steam Deck) and want low-latency play with
  mouse-look, pads, and often HDR.
- You already accept that this is **full desktop / game control** of the host, not a cloud game
  rental or a sandboxed remote app.

It is a poor fit if you need enterprise RDP features Slipstream does not claim, or if you expect a
finished webcam / camera uplink for video calls on the host (that is not a shipping product story
yet). Slipstream is a private low-latency stream with PIN pairing on a network you trust.

## The short checklist

Do these once on the host and once on the couch client:

1. **Install and start the host** on a machine with a working GPU encode path -
   [Install the Host](/docs/install), then [Quick Start](/docs/quickstart).
2. **No competing GameStream host running.** Stop Sunshine, Apollo, or forks **before** starting
   Slipstream if they are active on the same machine -
   [Troubleshooting](/docs/troubleshooting#another-streaming-host-sunshine-apollo--is-installed).
3. **Pair a client.** Prefer a [native Slipstream client](/docs/clients) when one exists for your
   device; use [Moonlight](/docs/moonlight) when you need a smart TV or a device without a native
   app.
4. **Capture (games) mouse** for mouse-look titles. Capture is the default on desktop clients -
   [Mouse modes](/docs/input#mouse-modes).
5. **Browse the game library** and launch a title into the stream -
   [Your game library](/docs/game-library).
6. **Pick a display preset.** **Headless box** for a dedicated stream / couch box; **Shared
   desktop** for a family PC you also sit at -
   [Virtual displays](/docs/virtual-displays#pick-a-preset).
7. **HDR on** when the whole chain can deliver 10-bit BT.2020 PQ - [HDR](/docs/hdr).
8. **Controllers** plugged into the client (or built into a Deck / phone clip). See
   [Controllers](/docs/controllers).
9. **Bitrate for the mode you asked for.** Automatic starts at a modest floor; raise it (or run
   the native speed test) for 4K and high refresh -
   [Picture quality](/docs/picture-quality), [Client settings](/docs/client-settings).

## Recommended host setup for play

### Always ready when you want to play

- Run the host as a service so it comes back at login or boot:
  [Running as a Service](/docs/running-as-a-service).
- On a Steam Deck or Bazzite couch box, follow the platform guides:
  [SteamOS (Host)](/docs/steamos-host), [Bazzite](/docs/bazzite). Streaming **to** a Deck as a client
  is [Steam Deck (Decky)](/docs/steam-deck).
- Optional: arm [Wake-on-LAN](/docs/wake-on-lan) so a sleeping host can be woken from the client on
  the same LAN (magic packets usually do **not** cross a VPN).

### Competing hosts (Sunshine and forks)

Slipstream speaks GameStream for Moonlight compatibility. So do Sunshine, Apollo, and their forks.
They bind the **same** fixed ports and advertise the **same** mDNS name. Running another host
**at the same time** as Slipstream is unsupported.

- An installed-but-idle Sunshine is not the conflict; a **running** one is.
- Check on demand: `slipstream-host detect-conflicts`.
- Fix: stop (and preferably uninstall) the other host, then start Slipstream.

Details: [Troubleshooting → Another streaming host](/docs/troubleshooting#another-streaming-host-sunshine-apollo--is-installed)
and [Network & VPN → Competing hosts](/docs/network-and-vpn#competing-hosts-on-the-same-ports).

### Display policy

| Situation | Preset to start with |
|---|---|
| Box with no monitor that only exists to be streamed (closet PC, HTPC, Deck couch box) | **Headless box** |
| Family or shared PC you also sit at in person | **Shared desktop** |
| One person roaming laptop ↔ tablet ↔ TV on the same host | **Hot-desk** |
| Multi-monitor daily driver you also use for games from another room | **Workstation** or **Default** |

Full policy reference: [Virtual displays](/docs/virtual-displays).

**Headless box** keeps the display (and on a gamescope game host, often the game itself) alive
across disconnects with keep-alive **forever**, and the next client takes the box. Release the
display from the console when you are done for the day.

**Shared desktop** never blanks the physical monitors and tears the streamed display down when the
session ends - the right pick when someone might walk up to the host mid-stream.

Where a launched game lands (live desktop, existing gamescope session, or a dedicated headless one)
is covered under
[Dedicated game sessions](/docs/virtual-displays#dedicated-game-sessions).

### Game library

Every host keeps one library: scanned launchers (Steam, and on each OS the other stores it can see),
hand-added custom titles, and plugin-synced entries. Clients browse posters and send only an **id**;
the host runs what it already knows.

- Enable **Show game library** on clients that default it off (on by default on iPhone and
  Android).
- Moonlight sees the same library when GameStream is enabled.
- Deck clients can pin titles in the Decky plugin.

Full detail: [Your game library](/docs/game-library).

### GameStream / Moonlight

For a play-oriented host:

- Prefer a **native** Slipstream client when one exists - lower latency path, built-in discovery,
  speed test, and FEC on lossy links ([Clients](/docs/clients)).
- Keep **GameStream** enabled when you want Moonlight on a smart TV, browser, or other device
  without a native app ([Moonlight](/docs/moonlight)).
- GameStream pairing uses legacy plain HTTP; leave it on a **trusted LAN** only
  ([Security](/docs/security)).

You can run both planes: native for the living-room phone / Deck, Moonlight for the TV.

## Recommended client settings for play

Create a **Play** (or **Couch**) [settings profile](/docs/profiles-and-links) and bind it to the
game host:

| Setting | Suggestion | Why |
|---|---|---|
| **Mouse input** | Capture (games) | Relative mouse-look; pointer locks to the stream |
| **Video codec** | HEVC (or AV1 when both ends support it) | Best everyday quality/bitrate on Wi-Fi and LAN |
| **Bitrate** | Explicit, or Automatic + speed test | Automatic's H.264/HEVC/AV1 floor is **20 Mbps** - fine for 1080p60, short for 4K / high refresh |
| **Resolution / refresh** | Match the client panel (or the TV mode you want) | Host creates a virtual display at your client mode |
| **HDR** | On when the chain allows | 10-bit BT.2020 PQ when source, encoder, codec, and client all agree - [HDR](/docs/hdr) |
| **Full chroma / 4:4:4** | Usually off for games | Costs bandwidth; more useful for sharp UI/text than for 3D titles |
| **PyroWave** | Wired LAN only, explicit pick | Ultra-low codec latency at hundreds of Mbps - [PyroWave](/docs/pyrowave) |

Save a second **Work** profile with Desktop mouse, clipboard, and text-oriented picture settings for
the same host when you use it from the office - [Desktop at work](/docs/desktop-at-work).

### Bitrate for 4K and high refresh

Picture softness and stutter at high modes are usually **bitrate or link**, not a broken host.

- **Native clients:** run **Test network speed...** on the host card, then apply the suggestion, or
  set an explicit bitrate in [client settings](/docs/client-settings). Use the
  [stats overlay](/docs/stats) to see whether you are network-bound or decode-bound.
- **Moonlight:** set bitrate in Moonlight's settings; start moderate and raise it.
- **120 Hz+** only helps if capture, encode, network, and decode keep up. A saturated Wi-Fi link
  fails before the encoder does ([How it works](/docs/how-it-works#games-and-interactive-motion)).
- **4K** wants a strong 5 GHz / wired path and a bitrate well above the Automatic 20 Mbps floor.
  If the link cannot carry it, drop resolution or refresh before chasing codecs.
- **Wired Ethernet / 2.5GbE+:** consider [PyroWave](/docs/pyrowave) when you want to spend bandwidth
  to shrink codec time. Do **not** run PyroWave over Wi-Fi.

Deeper tuning: [Picture quality](/docs/picture-quality).

### Controllers

Pads attached to the client are injected on the host as virtual gamepads. Multiple controllers get
stable slots; rumble and (where the client and pad support it) DualSense extras forward on the
native plane. Moonlight's GameStream path carries classic pad events but not the richer motion /
touchpad / adaptive-trigger extensions.

Host-side prerequisites matter too: on Linux the host user usually needs the `input` group
([Bazzite](/docs/bazzite#allow-controller-input), [Troubleshooting](/docs/troubleshooting#a-controller-is-detected-but-games-dont-see-it)).

Full guide: [Controllers](/docs/controllers).

## Day-in-the-life flow

1. Leave the host logged in (or configure headless / linger / Game Mode so a session exists). Confirm
   the host service is running.
2. On the couch client, open a [native app](/docs/clients) or [Moonlight](/docs/moonlight). If the
   host is asleep on the same LAN, wake it - [Wake-on-LAN](/docs/wake-on-lan).
3. Pair once if this device is new - [Pairing & Trust](/docs/pairing).
4. Connect to the desktop, **or** open the [game library](/docs/game-library) and launch a title.
5. Confirm **Capture** mouse for shooters (`Ctrl+Alt+Shift+M` toggles). Plug in a pad if you use one.
6. Play. Release input with `Ctrl+Alt+Shift+Q`, or **L1+R1+Start+Select** on a controller -
   [Getting your input back](/docs/input#getting-your-input-back).
7. Disconnect. Your display preset decides whether the game session stays ready for a fast reconnect
   (Headless box) or tears down cleanly (Shared desktop).

## Honest limits (today)

Call these out so Play expectations stay accurate:

- **Webcam / camera uplink** for video calls on the host is not a finished product story yet.
- **gamescope / Gaming Mode** hosts are excellent for games and cannot take absolute Desktop mouse -
  fine for Play, a poor fit for office UI ([gamescope](/docs/gamescope), [input](/docs/input)).
- **Moonlight** is broad compatibility, not the lowest-latency path; rich DualSense extras travel
  on the native plane ([Clients](/docs/clients), [Moonlight](/docs/moonlight)).
- **Wake-on-LAN** usually does **not** work across a VPN - wake on the LAN, leave the box on, or
  use another wake path ([Wake-on-LAN](/docs/wake-on-lan)).
- **Running Slipstream beside Sunshine** (or another GameStream host) on the same machine is
  unsupported while the other host is active.

## Related pages

- [Quick Start](/docs/quickstart) - first host and first stream
- [Your game library](/docs/game-library) - scan, custom titles, launch from clients
- [Mouse, touch and pen](/docs/input) - Capture vs Desktop mouse
- [Controllers](/docs/controllers) - pads, rumble, host prerequisites
- [HDR](/docs/hdr) - the four gates for a 10-bit stream
- [Picture quality](/docs/picture-quality) - bitrate, codec, chroma, refresh
- [PyroWave](/docs/pyrowave) - wired-LAN ultra-low-latency codec
- [Virtual displays](/docs/virtual-displays) - Headless box, Shared desktop, keep-alive
- [Clients](/docs/clients) / [Moonlight](/docs/moonlight) - how to connect
- [Steam Deck (Decky)](/docs/steam-deck) - stream to the Deck from Gaming Mode
- [Wake-on-LAN](/docs/wake-on-lan)
- [Desktop at work](/docs/desktop-at-work) - office path on the same host
- [Security & Safe Use](/docs/security)
- [Profiles and links](/docs/profiles-and-links) - Play vs Work profiles
