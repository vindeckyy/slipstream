---
title: How It Works
description: Virtual displays per client, the capture→encode→network→decode path, slipstream/1 vs GameStream, pairing, discovery, and why latency feels different for games and desk work.
---

You don't need to know any of this to use Slipstream, but it helps to understand what's happening
when you connect, whether you're launching a game to the couch or opening your real desktop from an
office laptop.

Slipstream is a **host** on a Linux or Windows machine and one or more **clients** on other devices.
The host creates a display, captures it, encodes frames on the GPU, sends them over the network, and
injects your mouse, keyboard, and controllers back into that session. The same pipeline serves both
**game streaming** and **remote desktop / office** use; what changes is how you configure mouse mode,
bitrate, display policy, and which network path you trust.

## A virtual display, sized to your device

When a client connects, the host asks your desktop to create a **new virtual display** at exactly the
client's resolution and refresh rate, captures that display, and streams it. The virtual display is
real to your desktop, apps can be moved onto it, games open on it, but it isn't tied to any physical
monitor. When the client disconnects, the virtual display goes away (unless you've set
[keep-alive](/docs/virtual-displays#keep-alive) to linger or hold it). That same path is what makes
Slipstream work for remote desktop at the office: you get a full desktop session sized to your
laptop, not a scaled mirror of a physical monitor.

That's why a 1080p60 laptop and a 1440p120 desktop can stream from the same host **at the same time**,
each at its own mode, they each get their own virtual display.

### Why per-client modes matter

Without a per-client virtual display, a streaming host usually mirrors one physical monitor and
scales or letterboxes to fit the client. That costs sharpness for office text, wastes pixels on a
high-refresh gaming client, and fights multi-device use. Slipstream's default is the opposite: the
client announces the mode it wants, the host creates a matching head, and the stream is 1:1 with that
panel.

For **games**, that means the title can run at the TV's or Deck's native resolution and refresh
instead of whatever the desk monitor happens to be. For **desk work**, it means your IDE, browser,
and terminals render at the laptop's pixel grid, so UI chrome and text stay crisp when bitrate and
chroma allow.

How the virtual display is created depends on your host:

| Host | How |
|---|---|
| **GNOME** (Mutter) | A virtual monitor via the screen-cast API |
| **KDE Plasma** (KWin) | A virtual output via KWin's screencast |
| **Bazzite / Steam** (gamescope) | A nested gamescope session launched at the client's mode |
| **[Hyprland](/docs/hyprland)** | A headless output added with `hyprctl`, captured through xdg-desktop-portal-hyprland |
| **Sway** (wlroots) | A headless output added to the running session |
| **Windows** | A virtual-display driver, including Slipstream's own **indirect display driver** the host pushes frames straight into, a real virtual display, no physical monitor, even on the secure desktop |

That last one is the distinctive part on Windows: rather than only capturing an existing screen,
Slipstream has **its own indirect display driver (IDD)**, and the host can push finished frames
**straight into the driver**. You get the same on-the-fly virtual display the Linux compositors give
you, at the client's exact mode, with no physical monitor or dummy HDMI dongle, and even on the
secure desktop (UAC / lock screen). That tight, push-based integration is unusual among Windows
streaming hosts.

### Display policy in brief

Creating the display is only half the story. [Virtual displays](/docs/virtual-displays) covers
presets (Default, Headless box, Shared desktop, Hot-desk, Workstation), keep-alive, topology
(extend / primary / exclusive), and conflict handling when a second client connects. For office use,
**Workstation** and **Hot-desk** are the usual starting points; for a couch-only box,
**Headless box** keeps the game session ready to resume. You rarely need to touch these on day one.

On Linux you can also pin a **physical** monitor and mirror it instead of creating a virtual one
(shop-floor PCs, media boxes). That path is intentional and documented on the same page; Windows
sessions always get a virtual display.

## From screen to GPU to wire to your device

End to end, a frame travels through four stages. Understanding them makes bitrate, codec, and
"feels laggy" conversations concrete.

### 1. Capture

The host grabs frames from the virtual display (or a pinned physical monitor on Linux) as GPU
buffers. On Linux that usually means the compositor's screen-cast / PipeWire dmabuf path. On Windows
the IDD direct-push path copies finished frames into a host-owned shared GPU texture ring, no Desktop
Duplication, no Windows.Graphics.Capture.

Captured frames never touch the CPU on their way to the encoder on the validated zero-copy paths, a
**zero-copy GPU path** that keeps latency low even at high resolutions and frame rates. Which
combinations are zero-copy today is spelled out in the
[support matrix](/docs/support-matrix#zero-copy-capture-to-encode).

### 2. Encode

Which encoder runs depends on your GPU: **NVIDIA** -> NVENC on both platforms; **AMD** -> AMF and
**Intel** -> QSV on Windows. On Linux AMD and Intel share one path, **Vulkan Video** for HEVC and
AV1, with **VAAPI** for H.264 and as the fallback when Vulkan encode isn't available. There's also a
GPU-less software H.264 encoder: on Windows the host picks it when it finds no supported GPU, and on
Linux you turn it on yourself (`SLIPSTREAM_ENCODER=software`, see [Configuration](/docs/configuration)).

Client and host then negotiate the codec: **HEVC** by default, **AV1** where both sides support it,
**H.264** on the software path, and, if you pick it on a wired link, **[PyroWave](/docs/pyrowave)**,
an intra-only wavelet codec that trades bandwidth for a fraction of a millisecond of codec latency.
**HDR** (10-bit BT.2020 PQ) rides the same path where the host's capture, its encoder, the codec and
your client all allow it; [HDR](/docs/hdr) has the four-link chain and what each host and client can
really do.

For desk work, full chroma **4:4:4** (where host and client support it and the link can carry it)
keeps text edges sharper than typical game-oriented 4:2:0 streams. That costs bandwidth; it is a
quality choice, not a separate product mode.

### 3. Network

Encoded packets leave the host over the active protocol plane (native `slipstream/1` or GameStream).
The native path uses a **QUIC** control channel and a **UDP** media data channel with encryption and
**forward error correction** so brief Wi‑Fi loss does not always mean a frozen frame. GameStream uses
the classic Moonlight ports and transport.

Both ends must be on a **trusted private network**: your LAN, or a VPN that makes the client look
local. Slipstream is not a WAN product you port-forward to the public internet. Ports, firewalls, and
VPN patterns live on [Network & VPN](/docs/network-and-vpn).

### 4. Decode and present

The client hardware-decodes the stream (VideoToolbox on Apple, Vulkan Video / VAAPI / vendor paths on
Linux, hardware decode on Windows and Android where available) and presents it fullscreen or in a
window. Input travels the other way: mouse, keyboard, touch, pen, and controllers are injected into
the host session. See [Clients](/docs/clients) and [Mouse, touch and pen](/docs/input).

Audio follows the same session: host output to the client, optional microphone uplink when you enable
it. Clipboard can cross the boundary when both host policy and the client's per-host trust toggle
allow it ([Shared clipboard](/docs/clipboard)).

## Two protocols

Slipstream speaks two protocols over the same host:

- **GameStream**, the protocol Moonlight uses. Start the host with `--gamestream` and any
  [Moonlight](/docs/moonlight) client connects with no special software. This is the most compatible way in.
- **slipstream/1 (native)**, a purpose-built protocol with a QUIC control channel and a UDP data
  channel hardened with forward error correction and encryption. It's lower-latency and more resilient
  on imperfect networks, and it's what the [native clients](/docs/clients) (Apple, Linux, Windows,
  Android) use.

The native `slipstream/1` plane runs by default (the secure default); add `--gamestream` and both planes
serve from a single host process, Moonlight clients use GameStream, the native clients use slipstream/1.

### Which should you use?

| Situation | Prefer |
|---|---|
| Native app available (macOS, iOS/tvOS, Linux, Windows, Android) | **slipstream/1** |
| Couch / TV / device with only Moonlight | **GameStream** (`--gamestream`) |
| Office laptop over VPN, security-sensitive host | **Native only**; turn GameStream off if you do not need Moonlight |
| Maximum compatibility on a trusted home LAN | Both planes on the same host |

GameStream pairing uses legacy plain HTTP and weaker control crypto than the native plane. That is
fine on a trusted LAN for Moonlight convenience; it is a poor default for a work-oriented host you
reach over a wide VPN. Windows ships GameStream **off**; many Linux packages ship it **on**. Details:
[Security](/docs/security#gamestream--moonlight-compatibility-is-the-weak-crypto-path),
[Moonlight](/docs/moonlight).

## Pairing and trust

Slipstream has **no accounts and no cloud**. Trust is device-to-device on your network.

The first time a device connects, you pair it: the host shows a short **PIN**, you type it into the
client, and the two remember each other. (Moonlight runs the PIN the other way: Moonlight shows it,
you submit it in the host console.) After that the device reconnects automatically on a pinned
cryptographic identity, no PIN, no account, no cloud. You can also **Approve** a waiting device from
the [web console](/docs/web-console) without typing a PIN. See [Pairing & Trust](/docs/pairing).

What that means in practice:

- Pairing is **required by default**. A random device on the LAN cannot stream until you admit it.
- After pairing, the client pins the host's certificate fingerprint; a fingerprint change forces
  re-pairing instead of silent trust.
- The host stores an allow-list of paired clients you can revoke from the console.
- `--open` / trust-on-first-use exists for fully trusted single-user networks only; it is not the
  recommended posture for shared or office setups.

A streaming client is remote control of the machine: screen out, input in. Treat pairing the way you
would treat handing someone an unlocked keyboard. [Security & Safe Use](/docs/security) is the full
story.

## Finding hosts

Hosts advertise themselves on your local network, so clients can **discover** them automatically
instead of needing an IP address. The native clients and Moonlight both list hosts they find on the
LAN.

Discovery uses **mDNS**: `_slipstream._udp` for the native plane, and `_nvstream._tcp` when
GameStream is enabled. Multicast usually stops at the subnet boundary, so over many VPNs the host
list is empty even though streaming works fine once you **add the host by IP**. That is normal, not a
broken install. After pairing, the client remembers the host.

You can disable mDNS (`--no-mdns` / `SLIPSTREAM_MDNS=0`) on networks where multicast does not work;
then every client adds the host manually. Full networking guidance:
[Network & VPN](/docs/network-and-vpn#discovery-across-a-vpn).

## Why latency matters differently for games and text UI

"Low latency" is not one number. Different tasks notice different parts of the pipeline.

### Games and interactive motion

For shooters, racing, and anything where aim or timing matters, you feel **motion-to-photon** delay:
input leave the client, arrive on the host, the game simulates, a frame is captured, encoded, sent,
decoded, and shown. Tens of milliseconds are noticeable; hundreds feel broken. That is why Slipstream
pushes zero-copy capture→encode, keeps the native transport UDP-oriented with FEC, and offers
codecs like PyroWave for wired LAN when you want to spend bandwidth to shrink codec time.

High refresh (120 Hz+) only helps if capture, encode, network, and decode can keep up. A saturated
Wi‑Fi link or an undersized bitrate will stutter before the encoder is the bottleneck. Use the
[stats overlay](/docs/stats) to see whether you are network-bound or decode-bound.

### Desk work and text UI

For IDEs, browsers, and documents, a few extra tens of milliseconds of glass-to-glass delay are often
tolerable. What hurts more is **soft text**, chroma blur, and pointer mismatch:

- **Bitrate and chroma** decide whether fine UI looks sharp. Raise bitrate on a capable VPN; enable
  4:4:4 when supported before assuming the host is wrong.
- **Mouse mode** matters more than codec for "can I work?". Capture (relative) mouse is the gaming
  default and fights window chrome; **Desktop (absolute)** mouse is what office use wants. Toggle
  with `Ctrl+Alt+Shift+M` on Linux/Windows clients, or set it in
  [Input](/docs/input#mouse-modes).
- **gamescope / Gaming Mode** hosts are excellent for games and a poor fit for absolute desktop mouse;
  use a full KDE / GNOME / Hyprland / Sway session for office work.

A congested office VPN is still a VPN: drop refresh or resolution before you chase mythical RDP
features Slipstream does not claim. Practical Work setup:
[Desktop at work](/docs/desktop-at-work).

## Multiple devices at once

A host can stream to several clients simultaneously, your laptop and your TV both viewing (and
controlling) the desktop, each at its own resolution. The native `serve` host allows up to **4**
concurrent sessions by default (an encoder bound); further clients wait until a slot frees. Display
policy decides whether a second client gets its own screen, joins, steals, or is rejected. On
Windows today a second concurrent client is rejected even under "separate" conflict handling, two
clients cannot yet share one virtual display's capture there. See
[Multiple devices](/docs/configuration#multiple-devices-at-once) and
[Virtual displays](/docs/virtual-displays).

What is **not** shipping yet: one client opening **multiple host monitors as separate windows**.
Several clients can each become a monitor of one desktop on Linux; a single laptop showing several
host heads as multi-window remote desktop is still on the [roadmap](/docs/roadmap). Webcam / camera
uplink for video calls on the host is likewise not a finished product story.

## Two common shapes of use

### Play (game streaming)

Host on a powerful PC or Steam Deck / Bazzite box, client on TV, phone, another PC, or Deck. Enable
GameStream if you want Moonlight on a smart TV. Prefer Capture mouse, game-oriented bitrate, and
often a gamescope or dedicated game session so a library launch boots straight into the title. Pair
once on the LAN; stream from the couch.

### Work (remote desktop)

Host on the workstation you left at home, client on the office laptop over a **private VPN**. Prefer
native clients, Desktop mouse, clipboard on, Workstation or Hot-desk display presets, and GameStream
off if you never use Moonlight. Add the host by VPN IP when discovery does not cross the tunnel.
Step-by-step: [Desktop at work](/docs/desktop-at-work). Networking and ports:
[Network & VPN](/docs/network-and-vpn).

Neither path requires a cloud account. Both use the same host process, the same virtual-display idea,
and the same pairing model.

## Where to go next

- **[Quick Start](/docs/quickstart)** - from nothing to a first stream
- **[Virtual displays](/docs/virtual-displays)** - presets, keep-alive, topology, multi-client layout
- **[Desktop at work](/docs/desktop-at-work)** - office laptop → home desktop checklist
- **[Network & VPN](/docs/network-and-vpn)** - Tailscale / WireGuard, discovery, ports, firewalls
- **[Pairing & Trust](/docs/pairing)** - PIN, console approve, pinned reconnects
- **[Clients](/docs/clients)** / **[Moonlight](/docs/moonlight)** - how to connect
- **[Security & Safe Use](/docs/security)** - what a streaming host really exposes
- **[Support matrix](/docs/support-matrix)** - what works where, read out of the code that decides it
