---
title: Picture quality
description: Bitrate, codecs, chroma, soft text, HDR trade-offs, and when PyroWave belongs - recipes for sharp desk work and for high-refresh play, without inventing settings the product does not have.
---

Picture quality in Slipstream is mostly a **client request**. You ask for a mode, a codec, a
bitrate, HDR, and (where the client can) full chroma; the host answers in the handshake with what
it can actually send. A setting the host cannot honor is usually a quiet downgrade rather than an
error. Nothing on this page takes effect mid-stream for codec, HDR, or chroma - reconnect after
you change them. Bitrate and **Match window** behave differently; see
[Client settings](/docs/client-settings).

This page is the tuning guide. The exhaustive per-setting defaults live in
[Client settings](/docs/client-settings#video). Host policy gates (`SLIPSTREAM_10BIT`,
`SLIPSTREAM_444`, PyroWave caps) are in [Configuration](/docs/configuration#video-quality). Which
GPU and client combinations can actually do 10-bit or 4:4:4 is in the
[Support matrix](/docs/support-matrix#encoders).

## What decides what you see

Four levers matter for how sharp and how smooth a stream looks. They interact; turning every
"quality" switch on at once often makes the picture *worse* on a real link.

| Lever | Who sets it | Default | What it buys |
|---|---|---|---|
| **Resolution / refresh** | Client | Native display | Pixel-exact virtual display; nothing is scaled unless you stream a real monitor or use render scale |
| **Bitrate** | Client (host encodes to it) | Automatic | Detail under motion; soft text when too low |
| **Video codec** | Client preference | Automatic → HEVC → AV1 → H.264 | Efficiency and which extras (HDR, 4:4:4, PyroWave) are even possible |
| **10-bit HDR** | Client toggle; host must allow | On | HDR10 (BT.2020 PQ) when the whole chain works |
| **Full chroma (4:4:4)** | Client toggle; host must allow | Off | Crisp coloured text and thin UI lines, at more bandwidth |

**Render scale** sits beside resolution: the host renders and encodes at your mode multiplied by
0.5x-4x, and the client resamples to the window. Above 1x supersamples for sharpness at more
bandwidth and decode work; below 1x is lighter on both. Caps are per-axis 4096 px for H.264 and
8192 px otherwise. Details: [Client settings → Video](/docs/client-settings#video).

There is **no host-side bitrate knob**. The client requests a rate; the host clamps it to
**500 kbps - 8 Gbps** (or uses its Automatic defaults below). Use the client's **Test network
speed...** entry on a host card to measure the link and suggest a value -
[Configuration → Bitrate](/docs/configuration#bitrate).

## Codecs in one paragraph

**Automatic** is a soft preference: the host emits your choice when it can also produce it,
otherwise the best codec you both speak, in the order **HEVC → AV1 → H.264**.
**[PyroWave](/docs/pyrowave) is never auto-picked** - you must choose it explicitly on a client that
offers it. Android and Apple hide AV1 unless the device has a hardware AV1 decoder; Android never
offers PyroWave.

Practical rule of thumb:

- **HEVC** - the usual pick for both desk work and games. Best quality-per-bit for UI among the
  hardware codecs Slipstream uses day to day, and the usual path for HDR (Main10).
- **AV1** - where both ends encode/decode it in hardware. Often efficient; not a requirement for
  sharp text.
- **H.264** - widest compatibility and the GPU-less software path. **Never HDR** on the hardware
  Slipstream targets; pinning H.264 pins the session to SDR.
- **PyroWave** - wired-LAN only, opt-in, hundreds of Mbps. Ultra-low codec latency and every frame
  a keyframe. See [When you are wired](#when-you-are-wired-pyrowave).

## Bitrate: Automatic vs an explicit rate

For **H.264, HEVC and AV1**, **Automatic** means the host's own default, **20 Mbps**, and it turns
on two things an explicit rate switches off:

1. **Adaptive bitrate** - the target can move with network conditions during the session.
2. **A short link-capacity probe** about two seconds in - it measures what your link really
   carries and lets the rate climb **past** 20 Mbps when the path allows.

An **explicit** rate is fixed for the session (still clamped by the host). The stats overlay shows
`(auto)` on the target when Automatic owns it -
[Understanding the stats overlay](/docs/stats).

### When Automatic helps

Leave Automatic on when:

- You move between networks (home Wi-Fi one day, VPN the next) and do not want to retune by hand.
- You are not sure what the link carries - the capacity probe is specifically for "start sane,
  climb if the wire allows."
- The session is mostly quiet desktop with occasional motion: adaptive bitrate can sit under the
  grant when the picture is static and spend it when something moves.

Prefer an **explicit** rate when:

- You already ran **Test network speed...** and want a fixed, known target.
- You are diagnosing soft text or stutter and need the bitrate to stop moving under your feet.
- You are comparing two settings (codec, HDR, chroma) and want one controlled variable.

### PyroWave is the exception

PyroWave has **no useful low-rate regime**. Its Automatic rate is a fixed **per-pixel budget** for
the negotiated mode (hundreds of Mbps at typical desktop sizes), and both adaptive bitrate and the
capacity probe stay **off** for the whole session. An explicit bitrate is honored if you set one,
but under sustained loss the right move is switching back to HEVC, not waiting for Automatic to
"settle." Cap an over-ambitious Automatic pin on the host with
`SLIPSTREAM_PYROWAVE_MAX_MBPS` - [Configuration](/docs/configuration#video-quality).

## Chroma: why text goes soft

Hardware codecs usually send **4:2:0** - chroma (colour) at half the resolution of luma
(brightness). Fine for photographs and most game frames. Bad for **small coloured text**, thin
UI chrome, and red/blue edges: the colour channel is literally subsampled, so glyphs look soft or
fringed even when the bitrate is high.

**Full chroma (4:4:4)** keeps chroma at full resolution. It costs more bandwidth. It needs **HEVC
or PyroWave**, the host's 4:4:4 policy left on (`SLIPSTREAM_444`, **on by default**), a capture
path that delivers full chroma, and a GPU that can encode it. If any gate fails, the host says
**4:2:0** before your decoder is built.

Honest gates today (do not skip these when diagnosing):

- **Only HEVC and PyroWave** can carry 4:4:4. AV1 and H.264 sessions stay 4:2:0.
- **Only NVENC and PyroWave** produce it on the host. AMD (VCN / AMF) and Intel (QSV / VAAPI /
  Vulkan Video) decline HEVC 4:4:4 - that is a hardware or backend limit, not a missing toggle.
  See the [encoder matrix](/docs/support-matrix#encoders).
- **Today only the Apple app actually advertises 4:4:4**, and only when its hardware decode probe
  passes. The Linux and Windows apps store the toggle but their session does not advertise the
  capability yet, so **it has no effect there**. Android, Decky, and the console home do not offer
  it. Moonlight / GameStream sessions are always 4:2:0.
- On Detailed stats, asking for full chroma prints `4:4:4` when granted or `4:4:4→4:2:0` when the
  host could not - [Stats](/docs/stats).

## HDR and picture quality

**10-bit HDR** is **on by default**. Off means "never send me 10-bit," and the host then never
upgrades. On, the stream goes 10-bit BT.2020 PQ only when the host has HDR content **and** the
encoder can do 10-bit. Full chain: [HDR](/docs/hdr).

HDR is not free picture quality for desk work:

- Office UIs and most SDR panels want **HDR off**. A PQ stream tone-mapped onto an SDR laptop, or
  software-decoded HDR without a proper swapchain, looks washed or clipped. The Linux/Windows
  overlay says `HDR` versus `HDR→SDR`.
- **HDR and 4:4:4 fight each other** on HEVC/AV1 in ways that differ by host:
  - **Windows:** for HEVC and AV1 there is no 10-bit full-chroma capture source, so an HDR session
    **drops to 4:2:0**. If you want full chroma with those codecs, turn HDR off for that profile.
    **PyroWave on Windows** is the exception: it can carry HDR and 4:4:4 together.
  - **Linux:** a host encodes 4:4:4 at **8 bits**, so a session that negotiates both resolves back
    down to **SDR** before the stream starts. On Linux **4:4:4 wins**; on Windows **HDR does**.
- H.264 never does HDR. PyroWave HDR needs a **Windows** host today; Linux-hosted PyroWave is SDR
  - stay on HEVC or AV1 for HDR from Linux.

Use two [profiles](/docs/profiles-and-links): Work with HDR off (and 4:4:4 when you can get it),
Play with HDR as you like it for games and films.

## Recipe: Work (sharp UI and text)

Goal: IDEs, browsers, terminals, and documents that look like a local panel - not a soft video of
one. Pair this with [Desktop at work](/docs/desktop-at-work) (Desktop mouse, clipboard, VPN,
Workstation / Hot-desk presets). Picture alone does not fix absolute mouse.

Create a **Work** settings profile and bind it to the office host:

| Setting | Suggestion | Why |
|---|---|---|
| **Video codec** | **HEVC** when the host supports it | Better quality at the same bitrate for UI than H.264 |
| **Full chroma / 4:4:4** | **On** when the client advertises it, the host GPU can encode it, and the link can carry it | Sharper coloured text and thin lines |
| **10-bit HDR** | **Off** | Avoids washed / clipped office UI on typical SDR laptop panels; frees HEVC/AV1 sessions to keep 4:4:4 on Windows |
| **Bitrate** | Higher than couch defaults if the VPN or LAN can carry it; or Automatic with a speed-test check | Soft text is usually bitrate or chroma, not a "broken" host |
| **Resolution / refresh** | Match the laptop panel (Native) | Host builds a virtual display at your client mode - no local upscale of a smaller stream |
| **Render scale** | Native (1x) first; try >1x only if you have spare bitrate and decode headroom | Supersampling helps sharpness but spends bandwidth |

On an Apple client talking to an **NVIDIA** host (NVENC) over a capable path, HEVC + 4:4:4 + HDR
off is the sharpest shipping desk-work picture Slipstream offers today. On Linux or Windows
*clients*, turn the 4:4:4 toggle on if you want it ready when advertising lands, but expect
**4:2:0** until the session advertises the capability - raise bitrate and keep HEVC meanwhile.

Over a **VPN**, raise bitrate until text stops looking muddy, then stop. If the VPN cannot carry
more, drop refresh or resolution before chasing settings the product does not have. Over a **wired
LAN** to the same workstation, you can spend much more - including PyroWave with 4:4:4 when you
want maximum chroma sharpness and minimum codec latency (still not a WAN codec).

## Recipe: Play (motion and high refresh)

Goal: smooth motion, high refresh when the whole pipeline keeps up, HDR for titles that need it.
Pair with Capture mouse and a game-oriented host session
([gamescope](/docs/gamescope) / library launch as you prefer).

| Setting | Suggestion | Why |
|---|---|---|
| **Video codec** | Automatic or HEVC; AV1 where both ends have hardware; **PyroWave** only on wired LAN | Efficiency on Wi-Fi; PyroWave when you can afford hundreds of Mbps for latency |
| **Refresh rate** | Native / match the TV or monitor | 120 Hz+ only helps if capture, encode, network, and decode keep up |
| **Bitrate** | Automatic on Wi-Fi and mixed networks; explicit after a speed test for a fixed couch setup | Undersized bitrate stutters under motion before the encoder is the bottleneck |
| **10-bit HDR** | On when the game / panel / host chain supports it | Default path; turn off if you see washed SDR presentation |
| **Full chroma / 4:4:4** | Usually leave off for games | Games tolerate 4:2:0; 4:4:4 spends bitrate you often want for motion |
| **Render scale** | Native, or slightly under 1x on a weak client / link | Lower scale is lighter on host and wire |

High refresh is not a free lunch. A saturated Wi-Fi hop or an undersized bitrate will stutter
before the encoder is the limit. Read the [stats overlay](/docs/stats): network stage vs decode
stage tells you which knob to turn. For shooters and racing on a **docked Deck or 2.5GbE+ LAN**,
see PyroWave below.

Save a **Play** (or Couch) profile next to Work on the same host so you are not retuning HDR and
mouse mode every time you switch jobs -
[Profiles and links](/docs/profiles-and-links).

## Soft text: diagnosis

When the desktop looks soft, work the list in order. Most cases are bitrate or chroma, not a
corrupt host.

1. **Confirm the negotiated stream.** Detailed stats: codec, bit depth, HDR tag, and chroma tag
   (`4:4:4` vs `4:4:4→4:2:0` vs no chroma line). Android prints
   `HEVC · 10-bit · HDR (BT.2020 PQ) · 4:2:0` outright. You are diagnosing what the host *sent*,
   not what the settings screen still shows from last week.
2. **Bitrate too low for the mode.** Raise bitrate, or leave Automatic and let the capacity probe
   climb. Run **Test network speed...**. Soft glyphs on a 1080p IDE at 10 Mbps on HEVC are
   expected; soft glyphs at 50 Mbps on a quiet LAN usually point elsewhere.
3. **Chroma (4:2:0).** Coloured text and red/blue edges look fringed while grayscale looks
   acceptable - classic 4:2:0. Enable full chroma **when your client advertises it and the host
   GPU can encode it**; otherwise raise bitrate and stay on HEVC. Remember Linux/Windows clients
   do not advertise yet.
4. **HDR fighting the panel or chroma.** Office UI on an SDR laptop with HDR left on → turn HDR
   off. Wanted 4:4:4 on Windows HEVC but left HDR on → session dropped to 4:2:0; turn HDR off for
   that Work profile.
5. **Wrong resolution / upscale.** You asked for a smaller mode than the panel, or the host is
   pinned to a real monitor and the client scales. Prefer Native / Match window for desk work -
   [Virtual displays](/docs/virtual-displays).
6. **Render scale below 1x.** Deliberately softer and cheaper; set Native if you did not mean to.
7. **H.264 instead of HEVC.** Same bitrate, worse UI. Pin HEVC when the host can encode it.
8. **Decoder / presentation path.** Software decode of an HDR stream on Linux/Windows can look
   washed (no HDR10 swapchain path). Turn client HDR off there -
   [HDR → Per client](/docs/hdr#per-client).

If text is sharp but the pointer fights you, that is **mouse mode**, not picture quality -
[Input → Mouse modes](/docs/input#mouse-modes).

## When you are wired: PyroWave

[PyroWave](/docs/pyrowave) is an **opt-in** wavelet codec for links that can afford real bandwidth:
wired Ethernet, a docked Steam Deck, a 2.5GbE LAN. It runs as Vulkan compute on both ends, every
frame a keyframe, encode/decode latency far below the hardware H.26x pipelines. **Do not run it
over Wi-Fi** - that is what HEVC/AV1 are for. It is never selected automatically.

Rough Automatic bitrate ballpark (~1.6 bits per pixel, 4:2:0 SDR):

| Mode | Bitrate |
|---|---|
| 1280×800 @ 60 (Deck) | ~100 Mbps |
| 1920×1080 @ 60 | ~200 Mbps |
| 1920×1080 @ 120 | ~400 Mbps |
| 2560×1440 @ 60 | ~355 Mbps |
| 2560×1440 @ 120 | ~710 Mbps |
| 3840×2160 @ 60 | ~800 Mbps |
| 3840×2160 @ 120 | ~1.6 Gbps |

**4:4:4** multiplies by ~1.6; **HDR** adds ~15 %; both together ~1.9× the 4:2:0 SDR rate. Gigabit
Ethernet tops out around 940 Mbps of payload, so big 4K / 4:4:4 / HDR / high-refresh modes want
2.5GbE, 5GbE, or 10GbE. The docs page has an interactive estimator.

Turn-on summary:

1. Host: Linux and Windows hosts advertise it in default builds (Linux: any GPU vendor via Vulkan
   import; no special `host.env` line).
2. Client: set **Video codec → PyroWave (wired LAN)** on Linux, Windows (x64), or Apple devices
   whose decode probe passes. Or `SLIPSTREAM_PREFER_PYROWAVE=1` where the UI is hard to reach.
3. Leave bitrate on Automatic unless you need a lower explicit pin or a host
   `SLIPSTREAM_PYROWAVE_MAX_MBPS` cap.

PyroWave carries 4:4:4 on Linux and Windows hosts. PyroWave **HDR** needs a **Windows** host
today. For sharp desk work on a wired LAN to an NVIDIA box, PyroWave + 4:4:4 (HDR off) is the
latency-and-chroma extreme; for HDR games from Linux, stay on HEVC or AV1.

## Honest limits

Call these out so expectations stay accurate:

- **4:4:4 advertising is Apple-only today.** Linux and Windows client toggles do not affect the
  session yet. AMD and Intel hosts cannot encode HEVC 4:4:4; use NVIDIA NVENC or PyroWave.
- **Moonlight stays 4:2:0.** Full chroma is native `slipstream/1` only (`SLIPSTREAM_444` has no
  effect on GameStream).
- **Automatic bitrate starts at 20 Mbps** for the hardware codecs, then may climb. It is not a
  guarantee of "as much as the LAN can take" on every hop; use the speed test when you need a
  measured suggestion.
- **HDR is not "more sharpness."** It is a colour/brightness pipeline with real gates and real
  trade-offs against 4:4:4. Desk work usually wants it off.
- **PyroWave is not a WAN / Wi-Fi codec.** Hundreds of Mbps, no adaptive low-rate mode. Under loss,
  switch to HEVC.
- **Web console cannot set bitrate or run a speed test** yet - client apps can. The console can
  show what a live session is using -
  [Support matrix / roadmap notes](/docs/support-matrix).
- **Multi-monitor as separate client windows** for one session is still on the
  [roadmap](/docs/roadmap). Picture quality settings do not unlock that.

## Cross-links

- [Client settings → Video](/docs/client-settings#video) - every default and which clients offer
  which row
- [Configuration → Video quality](/docs/configuration#video-quality) - `SLIPSTREAM_444`,
  `SLIPSTREAM_10BIT`, `SLIPSTREAM_PYROWAVE_MAX_MBPS`
- [Configuration → Bitrate](/docs/configuration#bitrate) - speed test; no host bitrate knob
- [HDR](/docs/hdr) - four-link chain, Windows vs Linux, codec rules, HDR vs 4:4:4
- [PyroWave](/docs/pyrowave) - wired codec, bitrate table, 4:4:4 and HDR
- [Support matrix → Encoders](/docs/support-matrix#encoders) - which GPUs encode 10-bit / 4:4:4
- [Desktop at work](/docs/desktop-at-work) - Work path (mouse, clipboard, VPN, presets)
- [Profiles and links](/docs/profiles-and-links) - Work vs Play profiles on one host
- [Understanding the stats overlay](/docs/stats) - `(auto)`, chroma tags, HDR→SDR
- [How it works](/docs/how-it-works) - soft text vs motion-to-photon
- [Network & VPN](/docs/network-and-vpn) - what LAN vs VPN can realistically carry
- [Troubleshooting](/docs/troubleshooting) - stutter, loss, and when bitrate is the wrong lever
