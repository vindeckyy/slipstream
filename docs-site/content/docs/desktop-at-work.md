---
title: Desktop at work
description: Use your real home or workstation desktop from an office laptop over a trusted VPN — recommended settings, display presets, mouse, clipboard, and honest limits.
---

Slipstream is not only for games. A lot of people run the host on a powerful machine at home (or on
a desk they left behind) and connect from a work laptop so they can use **their real desktop**: the
same apps, same files, same GPU, same resolution as the client asks for.

This page is the Work path. It assumes you can already install a host and pair a client; if you
cannot, finish [Quick Start](/docs/quickstart) on a trusted LAN first, then come back here for the
office setup.

> **Security first.** A streaming host is remote control of the machine. Use a **private VPN**
> (Tailscale, WireGuard, or your router's VPN). **Do not port-forward** Slipstream to the public
> internet. Read [Security & Safe Use](/docs/security) and [Network & VPN](/docs/network-and-vpn).

## Who this is for

- You have a **desktop or workstation** you want to keep running (or wake) at home.
- You sit at an **office laptop** (or another machine on another network) and need that desktop for
  real work: IDEs, browsers, design tools, terminals, documents.
- You already accept that this is **full desktop control**, not a sandboxed remote app.

It is a poor fit if you need enterprise RDP features Slipstream does not claim (domain SSO, MDM
policy packs, audited session recording). Slipstream is a private low-latency stream with PIN
pairing on a network you trust.

## The short checklist

Do these once on the host and once on the office client:

1. **Host stays on a trusted network.** Prefer a dedicated or workstation PC over a machine that
   holds your most sensitive personal data ([Security](/docs/security)).
2. **VPN between office and home.** Same private network as the host from Slipstream's point of
   view. Details: [Network & VPN](/docs/network-and-vpn).
3. **Pair with a native Slipstream client.** Prefer native over Moonlight for work; turn
   **GameStream off** on the host if you do not need Moonlight
   ([Security → GameStream](/docs/security#gamestream--moonlight-compatibility-is-the-weak-crypto-path)).
4. **Desktop (absolute) mouse.** Capture mode is the default on desktop clients and fights remote
   desktop work. Switch to Desktop — see [Mouse modes](/docs/input#mouse-modes).
5. **Clipboard on** (host + client), usually `text-only` on the host — [Clipboard](/docs/clipboard).
6. **Pick a display preset.** **Workstation** or **Hot-desk** for most office cases —
   [Virtual displays](/docs/virtual-displays#pick-a-preset).
7. **Picture for text.** Prefer HEVC where available; turn on full chroma **4:4:4** when the host
   and client support it and your link can carry it —
   [Client settings](/docs/client-settings), [PyroWave](/docs/pyrowave) only on a wired LAN.

## Recommended host setup for work

### Always available when you need it

- Run the host as a service so it comes back at login or boot:
  [Running as a Service](/docs/running-as-a-service).
- On Linux, enable lingering if the host should stay up after you leave a local session, and prefer
  a real desktop session (KDE / GNOME / Hyprland / Sway) over **gamescope** for absolute mouse and
  normal desktop apps.
- On Windows, the installer already installs the `SlipstreamHost` service.

### Display policy

| Situation | Preset to start with |
|---|---|
| This PC is **your** multi-monitor daily driver; you reconnect from several devices | **Workstation** |
| One person at a time; you roam laptop ↔ tablet and want fast reconnect | **Hot-desk** |
| Family or shared PC you also sit at in person | **Shared desktop** |
| Box with no monitor that only exists to be streamed | **Headless box** |

Full policy reference: [Virtual displays](/docs/virtual-displays).

For “I left for the office and might reconnect hours later,” raise **keep-alive** (or use a preset
that already keeps the display) so reconnects do not reshuffle the desktop. **Forever** keep-alive
is powerful — release the display from the console when you are done for the day if you share the
machine.

### GameStream / Moonlight

For a work-oriented host:

- Prefer the **native** protocol and Slipstream clients.
- If you never use Moonlight, run the host **without** `--gamestream` (Windows ships that way;
  Linux packages often enable it — turn it off). GameStream pairing uses legacy plain HTTP and
  belongs on a trusted LAN only.

### Clipboard

Enable on the host with an explicit policy:

```ini
SLIPSTREAM_CLIPBOARD=text-only
```

Restart the host after editing `host.env`. Then enable **Share clipboard with this host** on the
office client's host edit sheet. Both switches must be on — [Clipboard](/docs/clipboard).

## Recommended client settings for work

Create a **Work** [settings profile](/docs/profiles-and-links) on your office laptop and bind it to
the home host:

| Setting | Suggestion | Why |
|---|---|---|
| **Mouse input** | Desktop (absolute) | Point-and-click, window chrome, text selection |
| **Video codec** | HEVC when the host supports it | Better quality at the same bitrate for UI |
| **Full chroma / 4:4:4** | On when supported and the link allows | Sharper text and fine UI; costs bandwidth |
| **Bitrate** | Higher than couch defaults if the VPN can carry it | Soft text is usually bitrate or chroma, not “broken” |
| **Resolution / refresh** | Match the laptop panel | Host creates a virtual display at your client mode |
| **HDR** | Off for most office UIs | Avoids washed or clipped text on SDR panels |
| **Stream microphone** | Off unless you need it | Less background noise into the host |

Clipboard is **not** part of a profile — it stays a per-host trust toggle on the host record.

Save a second **Couch** or **Play** profile with Capture mouse, HDR as you like it, and game-oriented
bitrate for the same host when you play at home.

## Day-in-the-life flow

1. At home, leave the workstation logged in (or configure headless / linger so the desktop session
   exists). Confirm the host service is running.
2. At the office, join your VPN. Confirm the host's VPN address (Tailscale IP, WireGuard peer, etc.).
3. Open the Slipstream client. If mDNS does not cross the VPN, **add the host by IP** —
   [Network & VPN](/docs/network-and-vpn#discovery-across-a-vpn).
4. Connect. Switch to **Desktop** mouse if the stream starts in Capture (`Ctrl+Alt+Shift+M`).
5. Work. Copy/paste across machines if clipboard is enabled. Release input with
   `Ctrl+Alt+Shift+Q` when you need your laptop's own desktop.
6. Disconnect. Your display policy decides whether apps stay arranged for a fast reconnect.

## Honest limits (today)

Call these out so Work expectations stay accurate:

- **No multi-monitor client windows yet** for one session (several clients can each get a display;
  one laptop showing several host monitors as separate windows is still on the
  [roadmap](/docs/roadmap)).
- **Webcam / camera uplink** for video calls on the host is not a finished product story yet.
- **File transfer over clipboard** is policy-ready on the host but no shipping client asks for it
  yet — use your VPN file share, `scp`, or cloud storage for large files.
- **gamescope / Gaming Mode** hosts are excellent for games and poor for absolute desktop mouse —
  use a full desktop session for office work ([gamescope](/docs/gamescope), [input](/docs/input)).
- **Wake-on-LAN** usually does **not** work across a VPN — wake the machine before you leave, leave
  it on, or use another wake path ([Wake-on-LAN](/docs/wake-on-lan)).

## Related pages

- [Network & VPN](/docs/network-and-vpn) — how to reach the host from the office
- [Virtual displays](/docs/virtual-displays) — Workstation, Hot-desk, keep-alive
- [Mouse, touch and pen](/docs/input) — Desktop vs Capture mouse
- [Shared clipboard](/docs/clipboard)
- [Profiles and links](/docs/profiles-and-links) — Work vs Play profiles
- [Security & Safe Use](/docs/security)
- [Running as a Service](/docs/running-as-a-service)
- [Troubleshooting → Office / VPN](/docs/troubleshooting#office--vpn)
