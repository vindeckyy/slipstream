---
title: Connect with Moonlight
description: Stream from a Slipstream host using any Moonlight client.
---

Slipstream speaks the **GameStream** protocol, so [Moonlight](https://moonlight-stream.org/) connects
to it like it would to any GameStream host, no slipstream-specific app needed. It's a great option for
a browser, a smart TV, or any device without a native client.

> Many platforms also have a **native Slipstream client** with lower latency and built-in
> discovery/pairing, including **Windows** and **Android** (phone and Android TV). See
> [Clients](/docs/clients) before reaching for Moonlight.

## Feature delta vs native

Use Moonlight when you need a client Slipstream does not ship. Prefer a **native** app when one
exists for your device — especially for [Play](/docs/play) on a lossy link and for
[Desktop at work](/docs/desktop-at-work) over a VPN.

| Topic | **Native Slipstream client** | **Moonlight (GameStream)** |
|---|---|---|
| Availability | macOS, iOS/iPadOS, tvOS, Linux, Windows, Android ([Clients](/docs/clients)) | Any Moonlight build (TVs, browsers, odd platforms) |
| Host flag | Always on (`slipstream/1`) | Opt-in `--gamestream` (often on for Linux packages; off by default on Windows installer) |
| Discovery | `_slipstream._udp` mDNS | `_nvstream._tcp` mDNS |
| Pairing PIN | Host shows PIN → type into client | Client shows PIN → type into [web console](/docs/web-console) |
| Pairing transport | Native SPAKE2 ceremony | Legacy GameStream over **plain HTTP** |
| Session crypto / FEC | Native plane extensions | GameStream legacy control encryption; no Slipstream-native FEC/encryption extensions |
| Library | Host game library + Desktop | Same library catalog the host exposes (Desktop + detected launchers) |
| Mouse / pen | Desktop vs Capture modes; pen with pressure/tilt where the client sends it | Mouse/keyboard/controllers; pen when Moonlight sends pen events (e.g. Apple Pencil on iPad) |
| Clipboard | Shared clipboard when host + client enable it | Not the native clipboard product path — use native for [Clipboard](/docs/clipboard) office setups |
| Work / VPN fit | **Preferred** | Keep off on Work hosts unless you need Moonlight |
| LAN play on a TV without a native app | Use native if available | **Good fit** |
| Lossy Wi‑Fi / high bitrate | Native FEC path holds up better | Fine on a solid LAN; prefer native when the link is messy |
| Stats overlay | Slipstream [stats](/docs/stats) HUD | Moonlight's own overlay — numbers measure different pipeline slices; do not compare line-for-line without the stats matrix |

Security detail for the GameStream plane:
[Security → GameStream](/docs/security#gamestream--moonlight-compatibility-is-the-weak-crypto-path).
Pairing ceremonies side by side: [Pairing & Trust](/docs/pairing).

## When not to enable GameStream on Work hosts

Turn GameStream **off** when:

- The host's job is **office remote desktop** over a VPN, and every client you care about has a
  native Slipstream app ([Desktop at work](/docs/desktop-at-work)).
- The machine sits on a network segment you do not fully control (shared lab, corp VLAN, housemates
  you would not hand a PIN to casually). GameStream pairing is plain HTTP on the LAN trust model.
- You are hardening a host that should only speak the native plane
  ([Security checklist](/docs/security#a-short-hardening-checklist)).

Leave GameStream **on** when:

- You need a **smart TV**, browser Moonlight, or another device with no native client ([Play](/docs/play)).
- You stay on a **trusted home LAN** and accept the GameStream crypto trade-off for compatibility.

Windows ships GameStream **off** unless you ticked the installer box. Most Linux packages ship it
**on** — override the unit if this is a Work-only box
([Running as a Service → What the unit starts](/docs/running-as-a-service#what-the-unit-starts)).

## 1. Make sure the host is running with GameStream enabled

Moonlight needs the GameStream planes, which are **opt-in** on the host, but most installs already
enable them:

- **Linux packages, NixOS, SteamOS / Steam Deck**, already on: the shipped `slipstream-host` user
  unit runs `serve --gamestream`. To turn it *off*, override `ExecStart` with a drop-in so a package
  upgrade doesn't undo it, see
  [What the unit starts](/docs/running-as-a-service#what-the-unit-starts), or set
  `services.slipstream.host.gamestream = false` on NixOS, or re-run the Deck installer with
  `--no-gamestream`.
- **Windows**, off unless you ticked the installer's *Enable GameStream (Moonlight) compatibility*
  checkbox. To turn it on afterwards, run this from an **elevated** prompt (`=off` puts it back), 
  see [Windows Host](/docs/windows-host):

  ```powershell
  slipstream-host service install --gamestream=on
  slipstream-host service restart
  ```

- **Running `serve` by hand**, add the flag yourself:

  ```sh
  slipstream-host serve --gamestream
  ```

(Bare `serve` is the secure native-only default and stock Moonlight clients can't connect to it; the
native plane is always on, and `--gamestream` adds the Moonlight-compat surface.) GameStream pairs over
plain HTTP and its legacy control encryption is weaker than the native plane's, so only enable it on a
**trusted LAN**. See [Running as a Service](/docs/running-as-a-service) for the bundled unit. The host
advertises itself on the network, so Moonlight usually finds it on its own.

## 2. Add the host in Moonlight

Open Moonlight. Your host should appear automatically on the same network. If it doesn't, use **Add
Host manually** and enter the host machine's IP address.

Still nothing? Two causes account for almost all of it:

- **The GameStream ports aren't open on the host.** They are TCP **47984 / 47989 / 48010** and UDP
  **47998 / 47999 / 48000**, plus mDNS on UDP **5353**. On Linux the packages ship a ready-made rule
  for your distro's firewall but never enable it:

  ```sh
  sudo ufw allow slipstream-gamestream                               # ufw
  sudo firewall-cmd --permanent --add-service=slipstream-gamestream  # firewalld
  sudo firewall-cmd --reload
  ```

  On Windows the installer adds these rules itself, but only for **Private** and **Domain**
  networks, unless you ticked *Allow connections on Public networks* during setup.
- **Another GameStream host is running on the same machine.** Sunshine, Apollo and their forks bind
  the same fixed ports and advertise the same mDNS name, so the wrong host answers (or neither
  does). Run `slipstream-host detect-conflicts` on the host machine, it lists any it finds.

Both are covered in more detail in [Troubleshooting](/docs/troubleshooting) and
[Network & VPN](/docs/network-and-vpn).

## 3. Pair

Moonlight's PIN is typed in on the **host** side, so you need the host's
[web console](/docs/web-console) running, it's the only UI where a Moonlight PIN can be entered.

1. Open the console at `https://<host-ip>:47992` and go to **Pairing**.
2. In Moonlight, select the host and choose **Pair**, it shows a 4-digit PIN.
3. The console's **Moonlight (GameStream) pairing** card now says a client is waiting. Type the PIN
   there and press **Submit PIN**.

The device then appears under **Paired devices**, and Moonlight remembers the host. See
[Pairing & Trust](/docs/pairing) for the full picture, including when the Moonlight card is missing
(GameStream off) and how revoke / re-pair works.

## 4. Stream

Moonlight lists **Desktop** plus the games the host found installed (Steam, Epic, GOG, Xbox), with
cover art, the same [library](/docs/game-library) the native clients show. Pick one and start
streaming. The host creates a virtual display at the resolution and frame rate Moonlight requests
(set these in Moonlight's settings), encodes it on the GPU, and streams it. Mouse, keyboard, and
controllers flow back to the host, and a Moonlight client that sends pen events, an iPad's Apple
Pencil included, drives the same host-side tablet a native client would, with pressure and tilt
intact (see [Pen and stylus](/docs/input#pen-and-stylus)).

That **Desktop** entry is the operator base list. An `apps.json` in the host's config directory
*replaces* it (it doesn't add to it), so put every entry you want in that file if you write one.

For couch play settings (Capture mouse on native, HDR, bitrate), see [Play](/docs/play) and
[Picture quality](/docs/picture-quality). Controllers:
[Controllers](/docs/controllers). Audio behaviour in Moonlight can differ from native mic/clipboard
paths — [Audio](/docs/audio).

## Security summary

- GameStream is a **second plane** beside native. Turning it on does not weaken native sessions;
  it **adds** a legacy surface on the same machine.
- Pairing is **plain HTTP** on your LAN trust model. Do not pair Moonlight on a network you share
  with strangers; do not port-forward GameStream ports to the WAN
  ([Network & VPN](/docs/network-and-vpn), [Security](/docs/security)).
- Prefer native clients on VPN / office paths. If you only enable GameStream for a living-room TV,
  keep that host on the home LAN and use native from the office laptop.
- Revoke a Moonlight device from the console **Paired devices** list the same way you revoke a
  native client ([Pairing → Revoke](/docs/pairing#revoke-and-re-pair)).

## Play vs Work with Moonlight

**Play (trusted LAN).** Moonlight on a TV or living-room box is a common fit: enable GameStream,
open `slipstream-gamestream` on the firewall, pair once on the home Wi‑Fi, set resolution/refresh
in Moonlight before connect. Controllers plugged into the Moonlight device reach the host like
any GameStream pad path — see [Controllers](/docs/controllers) and [Play](/docs/play).

**Work (office / VPN).** Use a **native** Slipstream client. Absolute mouse, shared clipboard, and
Work picture profiles are native product paths. If GameStream is still on for a couch TV at home,
that is fine — just do not rely on Moonlight as your office remote, and do not expose those ports
beyond the private network ([Desktop at work](/docs/desktop-at-work),
[Network & VPN](/docs/network-and-vpn)).

## Tips

- **Set your resolution and frame rate in Moonlight's settings** before connecting, the host matches
  whatever Moonlight asks for, creating the virtual display at that exact mode.
- **Codec:** HEVC (H.265) is a good default; AV1 is available if your client supports it.
- **HDR:** Moonlight only offers its HDR toggle when the host advertises a 10-bit codec, which it
  does only when [its capture path *and* its encoder](/docs/hdr#the-chain) can really deliver
  10-bit BT.2020 PQ. If the toggle is there, turn it on and pick HEVC or AV1, H.264 stays SDR.
  Setting `SLIPSTREAM_10BIT=0` in [`host.env`](/docs/configuration) withdraws the offer entirely; it
  is on by default.
- **Bitrate:** start moderate and raise it. For very high bitrates, the [native
  clients](/docs/clients) have a built-in speed test; with Moonlight, set the bitrate manually.
- Moonlight uses the GameStream protocol, not Slipstream's native FEC/encryption extensions. On a
  solid LAN this is fine; on a lossy link a [native client](/docs/clients) holds up better.
- Comparing Moonlight's performance overlay with a Slipstream client's stats HUD? The numbers
  measure different slices of the pipeline, see [Understanding the Stats Overlay](/docs/stats)
  for a line-by-line comparison matrix before drawing conclusions.

## Related pages

- [Clients](/docs/clients)
- [Pairing & Trust](/docs/pairing)
- [Play](/docs/play)
- [Desktop at work](/docs/desktop-at-work)
- [Network & VPN](/docs/network-and-vpn)
- [Picture quality](/docs/picture-quality)
- [Security & Safe Use](/docs/security)
- [Troubleshooting](/docs/troubleshooting)
