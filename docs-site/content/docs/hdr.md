---
title: HDR
description: How an HDR10 stream is decided end to end, the four things that must all be true, what each host and client can actually do, and how to check which link is missing.
---

An HDR session carries a **10-bit BT.2020 PQ (HDR10)** picture from the host's display to your
screen. It is on by default wherever it works, and a session that can't be HDR streams 8-bit BT.709
SDR instead. Which one you get is decided **before the first frame**: the host resolves every gate
below, then tells the client what it is really going to send. Nothing on this page takes effect
mid-stream, so reconnect after changing any of it.

## The chain

Four things must all be true. If your stream is SDR when you expected HDR, one of these is why.

1. **The source.** What the host captures must hand it 10-bit PQ pixels. This is the link that fails
   most often, and it is entirely a host-side question, see [Per host](#per-host).
2. **The encoder.** The host GPU must encode 10-bit for the codec the session picked. The host
   probes this by opening a tiny real encoder once per GPU and codec, and believes the answer.
   (PyroWave skips the probe, it has its own rule, below.)
3. **The codec.** Only HEVC, AV1 and PyroWave have a 10-bit path, see [Codec rules](#codec-rules).
4. **The client.** Your client must advertise 10-bit and HDR (its **HDR** setting), and be able to
   present or tone-map PQ.

The host allows 10-bit by default (`SLIPSTREAM_10BIT`). It only ever *allows*, the client's setting
is the real per-session switch.

## Per host

### Linux + gamescope

A stock gamescope tone-maps its composite down to 8 bits before handing it over, so its capture
output is SDR no matter what the game rendered. Real HDR needs **`slipstream-gamescope`**, a build
carrying a patch that adds the 10-bit PQ formats to its PipeWire node. It installs beside your system
gamescope rather than replacing it; [HDR on gamescope](/docs/gamescope#hdr-on-gamescope) has the
package for each distro.

The host settles two facts before spawning anything: the gamescope binary it will run carries the
patch (its `--version` banner contains `+pfhdr`), and this host is the one **starting** the session
rather than attaching to a node someone else started.

**Attach mode is the trap.** The patched build only reaches sessions the host spawns itself,
managed, `SLIPSTREAM_GAMESCOPE_SESSION`, or a bare spawn. A session started by your display manager
runs the distro's own gamescope, which offers neither the 10-bit formats nor the in-node cursor. The
host cannot tell that from the outside unless you pinned `SLIPSTREAM_GAMESCOPE_NODE`: with
`SLIPSTREAM_GAMESCOPE_ATTACH=1` and the patched build installed it reads the binary, believes HDR is
available, and offers it. The attached session can't answer that negotiation, so the connect fails
with no picture, the host latches an SDR downgrade for the rest of its life, and the next connect
streams, in SDR.

That is exactly what the [Bazzite](/docs/bazzite) template ships: it pins attach *and* the sysext
installs `slipstream-gamescope`. Either comment `SLIPSTREAM_GAMESCOPE_ATTACH=1` out and let the managed
default take over (you get HDR and the compositor-drawn cursor), or stay on attach and set
`SLIPSTREAM_GAMESCOPE_HDR=0` so the failed attempt never happens. Staying on attach also leaves the
stream with no cursor; [HDR on gamescope](/docs/gamescope#hdr-on-gamescope) has the fix for that
half.

SDR content rides the same PQ container, the desktop, the Steam overlay, an SDR game, mapped in at
`SLIPSTREAM_GAMESCOPE_SDR_NITS` (gamescope's own default is 400). That is the knob when white looks
too bright or too dim on your TV.

### Linux + GNOME

A Slipstream host serves [two protocols](/docs/how-it-works#two-protocols): its own `slipstream/1`,
which the Android and Steam Deck apps speak, and GameStream, which
[Moonlight](/docs/moonlight) speaks. GNOME HDR is available on the GameStream side only.

GNOME 50 added HDR screencast for **real monitors** only, so this route mirrors a monitor instead of
creating a virtual display: set `SLIPSTREAM_VIDEO_SOURCE=portal`, put the monitor in HDR mode in
**Settings -> Displays**, and connect an HDR-capable client. `SLIPSTREAM_CAPTURE_MONITOR=<connector>`
pins which head, and when it is set the host checks *that* monitor's colour mode rather than asking
whether any monitor is in HDR. If none is, the session degrades to 8-bit SDR and says so in the log.

A Slipstream app connecting to a GNOME host over `slipstream/1` gets SDR. On that protocol the only
Linux HDR source is the gamescope virtual output.

### Linux virtual displays on KWin, Mutter and wlroots

**These are SDR.** Mutter's `RecordVirtual` streams and the KWin and wlroots virtual outputs are
8-bit upstream, so there is nothing for the host to capture in 10 bits, no setting changes this.
Streaming a *physical* monitor with the [Streamed screen](/docs/virtual-displays) setting is SDR to
a Slipstream app too, HDR panel or not; the GNOME/GameStream route above is the only Linux monitor
mirror that can be HDR.

## Per client

| Client | HDR10 present | Advertises HDR when |
|---|---|---|
| **Android** (phone + TV) | Yes, HDR10 via the Surface dataspace plus static metadata | the setting is on **and** the panel reports HDR10 or HDR10+. On an SDR panel the toggle is disabled |
| **Steam Deck** | Yes, through the same presenter path the Deck session client uses | the setting is on. Look for `HDR` versus `HDR->SDR` on the stats overlay |
| **Moonlight** | Its own HDR toggle, which appears only when the host advertises a 10-bit codec |, |

One exception: frames from **software decode** never take the HDR10 swapchain, whatever the surface
offers. On a client with no hardware HEVC decode an HDR stream is therefore presented on the SDR
swapchain without a tone-map, which looks washed out. Turn the client's HDR setting off there. The
[Steam Deck plugin](/docs/steam-deck) streams through this same client path.

## Codec rules

- **HEVC**, Main10. The usual HDR codec.
- **AV1**, 10-bit, where the GPU encodes it. Advertised separately from HEVC, so a box that does
  one and not the other tells the truth about each.
- **H.264**, never. High10 is not an encode mode on the hardware Slipstream targets, so negotiation
  never even asks. Pinning H.264 in your client settings pins the session to SDR.
- **[PyroWave](/docs/pyrowave)**, a Linux-hosted PyroWave session is SDR today. The Linux PyroWave
  capture path has no HDR colour conversion. Use HEVC or AV1 for HDR from Linux.

One more rule if you also use full chroma: the host encodes 4:4:4 at 8 bits, so a session that
negotiates both resolves back down to SDR before the stream starts. On Linux 4:4:4 wins. Full chroma
is off until you turn it on, so this only bites if you did.

## Check it

One subcommand answers every link in the chain:

```bash
slipstream-host hdr-probe
```

It prints, line by line: whether a monitor is in BT.2100 colour mode, whether the resolved gamescope
offers 10-bit PQ capture, the state of `SLIPSTREAM_GAMESCOPE_HDR`, whether gamescope will paint the
cursor into the capture node, the encoder's Main10 answer for HEVC and for AV1, whether the resolved
compositor can do HDR on Slipstream's own plane, and the combined GameStream capability.

**Run it with the environment the host service has.** The service loads `host.env` itself; your shell
does not, and `PATH` decides which gamescope binary gets probed:

```bash
set -a; . ~/.config/slipstream/host.env; set +a
slipstream-host hdr-probe
```

From the client side, set the [stats overlay](/docs/stats) to **Detailed**. Look for an `HDR` tag,
or `HDR->SDR` when a PQ stream is arriving but the local surface can't present it. Android prints
the negotiated depth and colour outright at the same tier
(`HEVC · 10-bit · HDR (BT.2020 PQ) · 4:2:0`).

## The switches

Host, in [`host.env`](/docs/configuration):

| Setting | Default | Effect |
|---|---|---|
| `SLIPSTREAM_10BIT` | **on** | Allow 10-bit (HEVC Main10 / AV1 10-bit) at all. `0`, `false`, `off` or `no` forces every session to 8-bit SDR. |
| `SLIPSTREAM_GAMESCOPE_HDR` | **on** | Allow HDR on the gamescope backend. It only decides whether HDR is *attempted*, a host without `slipstream-gamescope` stays SDR either way. `0` is the escape hatch that puts the gamescope backend back on the old SDR path, spawn flags included. |
| `SLIPSTREAM_GAMESCOPE_SDR_NITS` | gamescope's own (400) | How bright SDR content is inside the PQ container of an HDR gamescope session. |
| `SLIPSTREAM_VIDEO_SOURCE=portal` | unset | Required for the GNOME 50+ monitor-mirror route. GameStream/Moonlight only, it has no effect on `slipstream/1` sessions. |

Client: one toggle, in Settings under **Quality** with
[the rest of the video settings](/docs/client-settings#video), **HDR** on Android. It is **on by
default**. Turning it off means "never send me 10-bit", and the host then
never upgrades the session. Like the other video settings it can be set per
[profile](/docs/profiles-and-links), so a Work profile can prefer 4:4:4 while a Couch profile
prefers HDR.
