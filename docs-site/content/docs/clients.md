---
title: Clients
description: The ways to connect to a slipstream host — the Apple app, Moonlight, or the Linux client.
---

A slipstream host accepts clients over its own `slipstream/1` protocol (the Apple and Linux apps) and
over GameStream (Moonlight). Pick whichever fits the device you're streaming *to*. Ready to install?
**[Install a Client](/docs/install-client)** has the step-by-step for every device.

## Apple app (Mac, iPhone, iPad, Apple TV)

The native app for Apple devices speaks slipstream's own [`slipstream/1`](/docs/how-it-works#two-protocols)
protocol — the lowest-latency, most resilient path, with the full feature set:

- **Automatic host discovery** — hosts on your network appear under *On this network*; no IP typing.
- **PIN pairing** built in, and pinned reconnects after that.
- **Controllers**, including DualSense — rumble, adaptive triggers, lightbar, motion, and touchpad.
- A live **stats overlay** (resolution, fps, bitrate, latency) and a built-in **network speed test**
  to pick a bitrate for your link.

Open the app, pick your host, [pair](/docs/pairing) once, and stream. It builds from the
`clients/apple` directory in the repo (Swift / VideoToolbox / Metal).

## Moonlight (anything else)

slipstream also speaks the **GameStream** protocol, so any [Moonlight](https://moonlight-stream.org/)
client — Windows, Android, Steam Deck, a browser, an old phone — connects with no slipstream-specific
software. See [Connect with Moonlight](/docs/moonlight).

This is the broadest-compatibility option and great for couch gaming. It doesn't use the native
protocol's FEC/encryption extensions, but for a healthy LAN that rarely matters.

## Linux desktop client (GTK4)

`slipstream-client` is the native graphical Linux client — a GTK4 / libadwaita app that speaks
`slipstream/1` directly, with hardware decode (VAAPI → dmabuf on Intel/AMD, software fallback),
PipeWire audio, and SDL3 controllers (rumble, lightbar, DualSense touchpad/motion). Like the Apple
app it discovers hosts on your network automatically, does PIN pairing, and pins reconnects.

It ships as a real package, not just a source build — full steps in
[Install a Client](/docs/install-client#linux-desktop-flatpak):

- **Any Flatpak distro (recommended)** — `flatpak install https://flatpak.unom.io/io.unom.Slipstream.flatpakref`
  from the hosted [`flatpak.unom.io`](/docs/install-client#linux-desktop-flatpak) repo, then
  `flatpak update`; this is also what the Decky plugin launches.
- **Ubuntu / Debian** — `apt install slipstream-client` from the slipstream apt registry.
- **Fedora / Bazzite** — `rpm-ostree install slipstream-client` from the GitHub RPM registry.
- **Arch / SteamOS** — the `slipstream-client` split package from the `PKGBUILD`.

Launch it, pick your host from the list, and stream. For scripting you can skip the host list and
connect straight away:

```sh
slipstream-client --connect <host>:9777   # skip the picker, start a session immediately
```

## Windows desktop client (in development)

`slipstream-client` for Windows (`clients/windows`) is the native graphical client
for Windows — pure Rust, the same `slipstream/1` core as the Apple and Linux apps, with a **WinUI 3**
UI (host list, settings, PIN pairing) and the video on a `SwapChainPanel`, plus WASAPI audio, FFmpeg
decode, SDL3 controllers, network discovery, and PIN pairing. Launch it and pick a host from the
list, just like the Apple and Linux apps. It builds on `x86_64-pc-windows-msvc`; hardware (D3D11VA)
decode, 10-bit/HDR present, and packaging are in progress, so it is not yet shipped. A headless CLI
path exists for scripting/measurement:

```sh
slipstream-client                                   # open the WinUI 3 window (host list / settings)
slipstream-client --discover                        # list hosts on the network
slipstream-client --headless --connect <host>:9777  # no window: connect, count frames, print stats
```

Until it ships, **Moonlight** remains the recommended way to stream to Windows (see below).

## Linux reference client (headless)

`slipstream-probe` (in the repo) is a command-line client for the native protocol, used for
testing, development, and latency measurement — not an everyday client. It connects, streams to a
file, runs the speed test, and can discover hosts:

```sh
slipstream-probe --discover                        # list hosts on the network
slipstream-probe --connect <host>:9777 --pin <fp>  # connect to one
```

## Which should I use?

| You're streaming to… | Use |
|---|---|
| A Mac, iPhone, iPad, or Apple TV | The **Apple app** |
| A Linux desktop or laptop, or a Steam Deck | **`slipstream-client`** (GTK4) |
| Windows, Android, a browser, a TV | **Moonlight** |
| Automated tests / latency measurement | **`slipstream-probe`** (headless) |

Whichever you choose, the first connection needs a one-time [pairing](/docs/pairing).
