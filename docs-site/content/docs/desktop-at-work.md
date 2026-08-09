---
title: Desktop at work
description: Use your real home or workstation desktop from an office laptop over a trusted VPN - recommended settings, display presets, mouse, clipboard, and honest limits.
---

Slipstream is not only for games. A lot of people run the host on a powerful machine at home (or on
a desk they left behind) and connect from a work laptop so they can use **their real desktop**: the
same apps, same files, same GPU, same resolution as the client asks for.

This page is the Work path. It assumes you can already install a host and pair a client; if you
cannot, finish [Quick Start](/docs/quickstart) on a trusted LAN first, then come back here for the
office setup. If the same host is also your game box, keep a separate **Play** settings profile -
[Play](/docs/play) - and switch profiles instead of retuning mouse and picture every time.

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
   ([Security → GameStream](/docs/security#gamestream--moonlight-compatibility-is-the-weak-crypto-path),
   [Moonlight](/docs/moonlight)).
4. **Desktop (absolute) mouse.** Capture mode is the default on desktop clients and fights remote
   desktop work. Switch to Desktop - see [Mouse modes](/docs/input#mouse-modes).
5. **Clipboard on** (host + client), usually `text-only` on the host - [Clipboard](/docs/clipboard).
6. **Pick a display preset.** **Workstation** or **Hot-desk** for most office cases -
   [Virtual displays](/docs/virtual-displays#pick-a-preset).
7. **Picture for text.** Prefer HEVC where available; turn on full chroma **4:4:4** when the host
   and client support it and your link can carry it -
   [Picture quality](/docs/picture-quality), [Client settings](/docs/client-settings).
   [PyroWave](/docs/pyrowave) only on a wired LAN, not over a VPN.

## Recommended host setup for work

### Always available when you need it

- Run the host as a service so it comes back at login or boot:
  [Running as a Service](/docs/running-as-a-service).
- On Linux, enable lingering if the host should stay up after you leave a local session, and prefer
  a real desktop session (KDE / GNOME / Hyprland / Sway) over **gamescope** for absolute mouse and
  normal desktop apps.

### Headless vs logged-in

Office hosts usually fall into one of two shapes. Pick deliberately; the wrong shape is a common
cause of "it worked at home, black screen from the office."

| Shape | What it means | Use when |
|---|---|---|
| **Logged-in desktop** | You leave a real user session running (or log in once after boot). The host process rides that compositor. | Daily-driver PC you also sit at; KDE / GNOME / Hyprland / Sway at home. |
| **Headless / linger** | No one is at the keyboard; a lingering user session or headless compositor keeps the desktop alive after you leave. | Closet workstation, always-on box, or "I lock the door and go." See [Running as a Service → Headless](/docs/running-as-a-service#b-a-headless-always-on-host). |

Practical rules:

- A **locked GNOME** session can block capture ("Session creation inhibited") - for always-on
  office hosts, follow the headless GNOME notes so lock does not kill the stream
  ([Troubleshooting](/docs/troubleshooting#capture-fails-session-creation-inhibited-gnome)).
- **Do not** use a gamescope / Gaming Mode-only host as your office remote - see below.

### Display policy and keep-alive for the office

| Situation | Preset to start with |
|---|---|
| This PC is **your** multi-monitor daily driver; you reconnect from several devices | **Workstation** |
| One person at a time; you roam laptop ↔ tablet and want fast reconnect | **Hot-desk** |
| Family or shared PC you also sit at in person | **Shared desktop** |
| Box with no monitor that only exists to be streamed | **Headless box** |

Full policy reference: [Virtual displays](/docs/virtual-displays).

For "I left for the office and might reconnect hours later," raise **keep-alive** (or use a preset
that already keeps the display) so reconnects do not reshuffle the desktop:

- **Hot-desk** and **Workstation** linger about **five minutes** after a drop - enough for a VPN
  blip or a laptop lid close, not enough to pin the display overnight.
- If you need apps and window layout to survive a long lunch or a commute, raise keep-alive under
  **Custom**, or use **Forever** and **release** the display from the console when you are done
  for the day ([Keep alive](/docs/virtual-displays#keep-alive)).
- **Forever** keep-alive with **Exclusive** topology leaves physical monitors dark until you
  release - fine on a dedicated box, a bad surprise on a shared desk. Prefer **Shared desktop**
  when someone else still uses the screens in person.
- A deliberate **quit** from the client tears the display down immediately (linger skipped). A
  **network drop** uses the keep-alive window. Know which you did when debugging "why did my
  layout vanish?"

### Gamescope / Gaming Mode warning

**gamescope and Steam Deck Gaming Mode are excellent for games and a poor fit for office UI.**

They cannot take **absolute** (Desktop) mouse. If your Work profile asks for Desktop mouse against
a gamescope host, the session quietly stays in Capture mode - the pointer feels "stuck" or
wrong for window chrome, text selection, and IDE tabs
([Mouse modes](/docs/input#mouse-modes), [gamescope](/docs/gamescope),
[Virtual displays → Office work and gamescope](/docs/virtual-displays#office-work-and-gamescope)).

For remote work, run a **full desktop session** on the host (KDE, GNOME, Hyprland, Sway). Keep
gamescope / Game Mode for the [Play](/docs/play) path on the same machine if you want both.

### GameStream / Moonlight

For a work-oriented host:

- Prefer the **native** protocol and Slipstream clients.
- If you never use Moonlight, run the host **without** `--gamestream` (packages often enable it;
  turn it off). GameStream pairing uses legacy plain HTTP and belongs on a trusted LAN only -
  [Moonlight → when not to enable](/docs/moonlight),
  [Security](/docs/security#gamestream--moonlight-compatibility-is-the-weak-crypto-path).

### Clipboard

Enable on the host with an explicit policy:

```ini
SLIPSTREAM_CLIPBOARD=text-only
```

Restart the host after editing `host.env`. Then enable **Share clipboard with this host** on the
office client's host edit sheet. Both switches must be on - [Clipboard](/docs/clipboard).

### Controllers and audio at the desk

Office sessions almost never need a gamepad. Leave controllers alone unless you intentionally use
one; details live on [Controllers](/docs/controllers) (Work section there). For sound:

- Host → client audio works for video calls *on the host* and system sounds; mute mid-stream when
  you need the laptop's own speakers ([Audio](/docs/audio)).
- Stream microphone (client → host) is off in the Work profile table below unless you need it -
  it pipes office ambient noise into the host.

## Recommended client settings for work

Create a **Work** [settings profile](/docs/profiles-and-links) on your office laptop and bind it to
the home host:

| Setting | Suggestion | Why |
|---|---|---|
| **Mouse input** | Desktop (absolute) | Point-and-click, window chrome, text selection |
| **Video codec** | HEVC when the host supports it | Better quality at the same bitrate for UI |
| **Full chroma / 4:4:4** | On when supported and the link allows | Sharper text and fine UI; costs bandwidth |
| **Bitrate** | Higher than couch defaults if the VPN can carry it | Soft text is usually bitrate or chroma, not "broken" |
| **Resolution / refresh** | Match the laptop panel | Host creates a virtual display at your client mode |
| **HDR** | Off for most office UIs | Avoids washed or clipped text on SDR panels |
| **Stream microphone** | Off unless you need it | Less background noise into the host |

Clipboard is **not** part of a profile - it stays a per-host trust toggle on the host record.

Save a second **Couch** or **Play** profile with Capture mouse, HDR as you like it, and game-oriented
bitrate for the same host when you play at home - [Play](/docs/play),
[Picture quality](/docs/picture-quality).

### Soft text from the office

Muddy IDE fonts and fuzzy browser chrome over a VPN are almost always **picture settings or
bandwidth**, not a broken host. Work this short list before changing display presets:

1. Confirm codec / chroma / bitrate on the [stats overlay](/docs/stats) (`Ctrl+Alt+Shift+S`).
2. Raise bitrate until text stops looking muddy, then stop. If the VPN cannot carry more, drop
   refresh or resolution.
3. Prefer **HEVC**; turn **HDR off** for SDR laptop panels; enable **4:4:4** only when the client
   advertises it and the host can encode it (today that advertising path is Apple + capable host
   GPUs - see [Picture quality → Soft text](/docs/picture-quality#soft-text-diagnosis)).
4. If text is sharp but the pointer fights you, that is **mouse mode**, not picture - switch to
   Desktop (`Ctrl+Alt+Shift+M`).

Full recipes: [Picture quality → Work](/docs/picture-quality#recipe-work-sharp-ui-and-text).
Do **not** reach for [PyroWave](/docs/pyrowave) over a VPN; it is a wired-LAN codec.

## Day-in-the-life flow

1. At home, leave the workstation logged in (or configure headless / linger so the desktop session
   exists). Confirm the host service is running.
2. At the office, join your VPN. Confirm the host's VPN address (Tailscale IP, WireGuard peer, etc.).
3. Open the Slipstream client. If mDNS does not cross the VPN, **add the host by IP** -
   [Network & VPN](/docs/network-and-vpn#discovery-across-a-vpn).
4. Connect. Switch to **Desktop** mouse if the stream starts in Capture (`Ctrl+Alt+Shift+M`).
5. Work. Copy/paste across machines if clipboard is enabled. Release input with
   `Ctrl+Alt+Shift+Q` when you need your laptop's own desktop.
6. Disconnect. Your display policy decides whether apps stay arranged for a fast reconnect.

## Day-2 VPN operations

After the first successful office stream, these are the habits that keep the path boring:

### Before you leave home

- Confirm the host service is up (`systemctl --user status slipstream-host`).
- Note the address you will use from the office (Tailscale IP, WireGuard peer, or LAN IP reachable
  through the tunnel). Discovery will often fail over VPN - that is normal.
- If the machine sleeps, **wake it before you leave** or leave it on. [Wake-on-LAN](/docs/wake-on-lan)
  magic packets usually **do not** cross a VPN.
- Optional: open the [web console](/docs/web-console) once on the home LAN and confirm Preflight is
  clean, so you are not debugging capture from the office.

### At the office, every connect

1. Join the VPN and wait until the host IP pings (or Tailscale shows the peer online).
2. Open the client → select the saved host (or **Add host** by VPN IP on first day).
3. If connect stalls with no picture, check UDP on the VPN path - see
   [Network & VPN](/docs/network-and-vpn) and
   [Troubleshooting → Office / VPN](/docs/troubleshooting#office--vpn).
4. Confirm **Desktop** mouse and your **Work** profile are active before blaming the host for
   "bad remote desktop feel."

### Multi-device reconnect

One person, several clients (office laptop, home tablet, phone):

- **Hot-desk** - one session at a time; a second device is told the host is busy. Keep-alive helps
  when you close the laptop and open the tablet within the linger window.
- **Workstation** - extra clients can each get a **separate** virtual display; layout can stick
  per client. Use this when you intentionally want more than one stream into the same desktop
  arrangement.
- Reconnect to a **kept** display resumes the same surface without reshuffling, as long as you
  reconnect inside keep-alive (or under Forever until you release).
- Pair each device once ([Pairing](/docs/pairing)); revoke a lost laptop from the console rather
  than leaving it on the allow-list.

Do not expect one client window to show several host monitors in one session. Several clients can each receive a display.

### File transfer alternatives

Clipboard **file** transfer is policy-ready on the host (`SLIPSTREAM_CLIPBOARD=on`) but **no
shipping client asks for it yet**, and host backends do not offer file formats yet - so `on` and
`text-only` behave the same in practice ([Clipboard](/docs/clipboard)).

For real files between office laptop and home host, use something outside the stream:

- A folder share over the same VPN (SMB, NFS, Syncthing, Nextcloud, ...).
- `scp` / `sftp` / `rsync` to the host's VPN address.
- Cloud storage both machines already trust.

Text, HTML, rich text, and images can still cross on the shared clipboard when both switches are
on - that is the shipping office path today.

## Honest limits (today)

Call these out so Work expectations stay accurate:

- **One client window cannot show several host monitors** in one session. Several clients can each receive a display.
- **Webcam / camera uplink** for video calls on the host is not a finished product story yet.
- **File transfer over clipboard** is policy-ready on the host but no shipping client asks for it
  yet - use your VPN file share, `scp`, or cloud storage for large files.
- **gamescope / Gaming Mode** hosts are excellent for games and poor for absolute desktop mouse -
  use a full desktop session for office work ([gamescope](/docs/gamescope), [input](/docs/input)).
- **Wake-on-LAN** usually does **not** work across a VPN - wake the machine before you leave, leave
  it on, or use another wake path ([Wake-on-LAN](/docs/wake-on-lan)).
- **4:4:4 advertising is iPhone-only today** on the client side; other clients may not negotiate
  full chroma yet ([Picture quality](/docs/picture-quality#honest-limits)).

## Troubleshooting deeper

When the office path misbehaves, jump straight to the matching section rather than reinstalling:

| Symptom | Start here |
|---|---|
| Host list empty over VPN, fine on home Wi‑Fi | [Network → Discovery](/docs/network-and-vpn#discovery-across-a-vpn), [Host isn't found](/docs/troubleshooting#the-host-isnt-found-on-the-network) |
| Connect starts, video stalls or never appears | [Office / VPN](/docs/troubleshooting#office--vpn), [Data plane / slow start](/docs/troubleshooting#video-is-slow-to-start-or-fails-across-subnets), [Network → data ports](/docs/network-and-vpn#data-plane-ports) |
| Soft / muddy text | [Picture quality → Soft text](/docs/picture-quality#soft-text-diagnosis) |
| Mouse feels wrong | [Input → Mouse modes](/docs/input#mouse-modes), gamescope note above |
| Clipboard does nothing | [Clipboard](/docs/clipboard), [Troubleshooting](/docs/troubleshooting#copy-and-paste-between-host-and-client-does-nothing) |
| Pairing rejected after reinstall | [Pairing](/docs/pairing), [Troubleshooting](/docs/troubleshooting#pairing-is-rejected--the-client-cant-connect) |
| Host asleep, will not wake from office | [Wake-on-LAN](/docs/wake-on-lan), [Network → WoL](/docs/network-and-vpn#wake-on-lan-and-vpns) |
| Another Sunshine/Apollo process running | [Competing hosts](/docs/network-and-vpn#competing-hosts-on-the-same-ports) |

## Related pages

- [Play](/docs/play) - same host, game-oriented settings
- [Network & VPN](/docs/network-and-vpn) - how to reach the host from the office
- [Picture quality](/docs/picture-quality) - soft text, bitrate, 4:4:4
- [Virtual displays](/docs/virtual-displays) - Workstation, Hot-desk, keep-alive
- [Mouse, touch and pen](/docs/input) - Desktop vs Capture mouse
- [Shared clipboard](/docs/clipboard)
- [Audio & microphone](/docs/audio) - Work mute and mic defaults
- [Controllers](/docs/controllers) - leave pads alone for desk work
- [Profiles and links](/docs/profiles-and-links) - Work vs Play profiles
- [Security & Safe Use](/docs/security)
- [Running as a Service](/docs/running-as-a-service)
- [The Web Console](/docs/web-console)
- [Troubleshooting → Office / VPN](/docs/troubleshooting#office--vpn)
