---
title: Understanding the Stats Overlay
description: What every number in the Slipstream stats HUD means, and how to compare them fairly with Moonlight/Sunshine.
---

The Android and Steam Deck clients use **the same vocabulary and the same four measurement points**,
so a stage name on your phone means what the same name means on your Deck.

## The four measurement points

Every latency figure is the time between two of these four points in a video frame's
life:

1. **capture**, the host grabs the frame from the (virtual) display. Stamped on the
   host's clock and carried with the frame.
2. **received**, your client has fully received and reassembled the frame from the
   network (after any FEC recovery), before decoding.
3. **decoded**, the video decoder has produced the picture.
4. **displayed**, the picture is handed to the screen (as close to "photons" as the
   platform lets us measure).

## Detail levels

The overlay has four levels, **Off → Compact → Normal → Detailed**, that you cycle live
in-stream:

| Platform | Cycle with |
|---|---|
| Steam Deck | **Ctrl+Alt+Shift+S** |
| Android | a **three-finger tap** |

**Ctrl+Alt+Shift+S** is one of a small set of shortcuts a stream reserves; the others, release
captured input, switch mouse mode, disconnect, mute the microphone, are in
[Getting your input back](/docs/input#getting-your-input-back).

**Compact** is a one-line pill (fps · end-to-end ms · Mb/s, plus a loss flag when frames are being
lost). **Normal** adds the stream line and the p50/p95 headline. **Detailed** adds the per-stage
breakdown everywhere; on Steam Deck it also adds the encoder's target bitrate, the decode path,
an HDR tag and a chroma tag, and on Android the decoder plus the full codec/bit-depth/colour line.
You can also set the level a stream starts at in each client's
[Settings](/docs/client-settings#overlay). The examples below are the **Detailed** view.

The overlay follows your display's scaling, so it should already be readable. To nudge it, set
`SLIPSTREAM_OSD_SCALE` in the **client's** environment (0.5x-4x), see [Configuration ->
Client-side](/docs/configuration#client-side-native-clients).

## Reading the overlay

Every client reports the same measurements, but each family lays them out a little
differently. Steam Deck:

```
1920x1080@120 · 120 fps · 24.3 Mb/s · target 30 Mb/s (auto) · vulkan · HDR
e2e 14.2/19.8 ms (p50/p95) · host 3.1 · net 6.7 · decode 2.1 · display 2.3 ms
host: queue 0.6 · encode 1.8 · xfer 0.2 · pace 0.5 ms
lost 3 (2.4%)
```

Android:

```
1920x1080@120   120 fps   24.3 Mb/s
c2.qti.hevc.decoder · low-latency
HEVC · 10-bit · HDR (BT.2020 PQ) · 4:2:0
end-to-end 14.2 ms p50 · 19.8 p95 · capture→displayed
= host 3.1 + network 6.7 + decode 2.1 + display 2.3
lost 3 (2.4%) · skipped 1 · FEC 12
```

- **Line 1, the stream.** Resolution@refresh, frames received per second, and the
  received video bitrate (goodput, FEC overhead not counted). Steam Deck follows the
  measured rate with `target N Mb/s`, what the host's encoder is currently *allowed* to
  produce, so a quiet desktop under a large grant (measured far below target) reads
  differently from an encoder pinned at its cap (measured hugging the target). `(auto)`
  means the [Automatic bitrate](/docs/client-settings#video) controller owns the target
  and moves it with network conditions; no target at all means an older host that doesn't
  report one. Then the decode path, an [HDR](/docs/hdr) tag (`HDR`, or `HDR→SDR` when a PQ
  stream is tone-mapped onto an SDR screen), and, when you asked for
  [full chroma](/docs/client-settings), the resolved chroma: `4:4:4` when the host
  granted it, `4:4:4→4:2:0` when it couldn't. Android puts its decoder and the negotiated
  codec, bit depth, colour and chroma on rows of their own underneath.
  If the session resolved to a [settings profile](/docs/profiles-and-links), its name closes this
  line. On **Android** a `⚠ panel NN Hz` warning joins it whenever the device's panel is refreshing
  *below* the stream's rate, the tell for a phone or TV governor that ignored the requested mode,
  which otherwise reads as inexplicable judder plus a refresh of extra latency.
- **Line 2, the headline.** `end-to-end` (`e2e` on Steam Deck) is the *directly measured* time from host capture to the
  endpoint named at the end of the line, `capture→on-glass` or `capture→displayed`. Steam Deck
  doesn't spell the endpoint out, because its presenter always measures to the present instant. `p50` = the typical
  frame (median), `p95` = the slow outliers. This is the one number that summarizes your
  stream.
- **Line 3, where the time goes.** The first four stages **tile the end-to-end interval**,
  each starts where the previous one ends, so they add up to the headline. Android may show
  additional presenter counters beneath them; those values explain the display stage and are not
  extra time.
  - `host`, capture → sent: the host's own share (capture read, encode, error
    coding, the paced send), reported by the host itself once per frame.
  - `network` (`net` on Steam Deck), sent → received: the network flight plus
    reassembly on your device.
  - `decode`, received -> decoded, on your device.
  - `display`, decoded -> displayed: waiting for the right screen refresh, rendering,
    and vsync.
  - `display X (pace A + latch B)` and `presents N` *(Android only)*, when the timeline presenter
    is running it splits `display` in two: `pace` is the wait it deliberately holds the frame for
    its target refresh, `latch` is SurfaceFlinger picking it up and scanning it out. `presents`
    counts the frames confirmed on glass this second, well below `fps` means the presenter is
    dropping or serializing frames; an `fps` shortfall with `presents` keeping up is upstream of
    the client.

  Against an **older host** that doesn't report its share yet, the first two terms
  merge into a single `host+network` number (`host+net` on Steam Deck), same total,
  one split fewer. On Steam Deck, Detailed adds one further line, `host: queue ... ·
  encode ... · xfer ... · pace ...`, splitting the host's own share into its stages, when the
  host reports them.

  (Stage values are per-stage medians, so they sum only *approximately* to the
  headline median, percentiles aren't perfectly additive. The headline is measured
  directly, never computed as a sum.)
- **Line 4, reliability** (only shown when something is nonzero). `lost` = frames the
  network dropped beyond FEC's ability to recover, every client reports it. `skipped`
  (frames your client chose not to display because a newer one had already arrived) and
  `FEC` (packet shards the error correction recovered this second, loss you *didn't*
  feel) are reported by the **Android client only**; the other clients show `lost` alone.

All values refresh once per second over the last second of frames.

## Host capture diagnostics

The web console's **Stats** page shows source-side diagnostics alongside its latency charts when
the host reports them. `Backend` identifies the selected adapter, `Newest frame age` measures how
old the newest source frame was when the host recorded the sample, and `Peak sampled age` shows the
largest value in the displayed recording. These values stop before encoding, network transfer,
decode, and display, so they isolate compositor and capture delay from the rest of the path.

`Frames overwritten` counts frames replaced in the one-deep newest-frame slot, and `Buffers
drained` counts older PipeWire buffers discarded while selecting the newest buffer. Rising values
mean the source is producing work faster than the downstream path consumes it. `Age over threshold`
means at least one sample exceeded `SLIPSTREAM_CAPTURE_MAX_AGE_MS`; it is diagnostic only and does
not drop or pace frames. Source size and the negotiated modifier help identify a format or import
fallback when comparing two capture methods.

### Clocks, and the `(same-host clock)` tag

`end-to-end` and `host+network` span two machines, so they need the two clocks to
agree: at connect, the client runs an NTP-style handshake with the host and corrects
for the measured clock offset. If that handshake wasn't possible, the overlay appends
**`(same-host clock)`**, the numbers are then only trustworthy when client and host
run on the same machine. `decode` and `display` are single-machine measurements and
are always exact.

### What each platform can measure

Not every platform exposes a true "displayed" instant, so the point the headline stops at
differs by client, and the clients that have a choice name it on the line rather than
pretending:

| client | headline | why |
|---|---|---|
| Steam Deck | `capture→on-glass` | present instant available (measured right after the Vulkan swapchain present); published raw |
| Android | `capture→displayed` | MediaCodec's per-frame render callback reports SurfaceFlinger's render timestamp; on the rare window where no callback is delivered (the platform may drop them under load) the HUD falls back to `capture->decoded` |

A shorter chain means the number is **smaller because it measures less**, check the endpoint before
comparing two devices.

## Comparing with Moonlight / Sunshine

Moonlight's overlay and Slipstream's measure different slices of the pipeline, and the
single biggest difference is:

> **Moonlight has no end-to-end number.** Its overlay shows separate client-side
> segments (decode time, queue delay, render time) and, on Sunshine hosts, a
> host-side number. Nothing in Moonlight measures capture-to-glass, and nothing
> measures the network flight of video frames. Slipstream's `end-to-end` line has **no
> Moonlight counterpart**, never compare it against any single Moonlight line.

To compare fairly, reconstruct an approximate end-to-end from Moonlight's lines:

```
Moonlight  approx  host processing latency (avg)
          + ½ x average network latency
          + average decoding time
          + average frame queue delay
          + average rendering time
```

...and compare *that* against Slipstream's `end-to-end`. (It's still approximate:
Moonlight's segments are averages over a slightly different window, and the ½·RTT term
stands in for a one-way frame flight that Moonlight doesn't measure.)

### Line-by-line matrix

| Moonlight overlay line | What it actually measures | Slipstream equivalent | Comparable? |
|---|---|---|---|
| `Video stream: WxH FPS` | Received **plus inferred-lost** frames/s (host-rate estimate from frame sequence gaps) | `fps` (line 1) |  approx  equal when loss is near zero; Slipstream counts received frames only |
| `Incoming frame rate from network` | Frames reassembled from the network per second | `fps` (line 1) | **Yes, direct** |
| `Decoding frame rate` (desktop only) | Frames leaving the decoder per second | not shown separately (equals `fps` unless the decoder is falling behind) |, |
| `Rendering frame rate` (desktop only) | Frames actually presented per second | `fps` minus `skipped` (Android only) | Approximately |
| `Host processing latency min/max/avg` (Sunshine hosts) | Host capture -> just-before-send, reported by Sunshine per frame | `host` (line 3), the host reports capture->fully-sent per frame the same way | **Yes, direct** (Slipstream's includes the paced send itself, Sunshine's stops just before it; avg vs p50) |
| `Frames dropped by your network connection` | Frame-sequence gaps ÷ total frames | `lost` (line 4) | **Yes, direct** |
| `Frames dropped due to network jitter` | Decoded frames the *client's pacer* chose to drop ÷ decoded frames | `skipped` (line 4, Android only) | Approximately (both are client-side pacing decisions, despite Moonlight's name) |
| `Average network latency` | The **control connection's round-trip time** (ENet RTT + variance), not video frame latency | `network` (line 3) is the closest concept, but it's the *actual one-way frame path* (flight + reassembly), not an RTT | **No direct comparison.** Roughly, Slipstream's `network`  approx  ½ x an idle RTT plus serialization time of the frame |
| `Average decoding time` | Mean time from decoder enqueue to picture out | `decode` (p50) | Yes (mean vs median; both include decoder queueing) |
| `Average frame queue delay` | Mean time a decoded frame waits for its vsync slot | inside `display` | Sum the two Moonlight lines -> |
| `Average rendering time (incl. V-sync latency)` | Mean duration of the present call | inside `display` | ...and compare against Slipstream's `display` |
| *(no equivalent)* |, | `end-to-end`, true capture->glass, clock-skew-corrected across machines | **Slipstream only** |
| *(no equivalent)* |, | `FEC` recovered shards (loss absorbed invisibly; Android only) | Slipstream only |

Other differences worth knowing when squinting at both overlays side by side:

- **Averages vs percentiles.** Moonlight's time values are means; Slipstream shows
  medians (p50) with a p95 for the headline. Under jitter, a mean sits above the
  median, Moonlight's numbers read slightly "worse" than an equivalent p50.
- **Refresh window.** Both overlays refresh about once per second; Moonlight over a ~1-2 s sliding
  window, Slipstream over the last full second.
- **Host frame rate.** Moonlight's headline FPS estimates what the *host* produced
  (received + lost). Slipstream shows what your client actually received, and reports
  loss separately.

## Recording a capture for a bug report

The overlay only ever shows the last second. To capture a whole run, use the host's own recorder, 
the **Performance** page in the [web console](/docs/web-console):

1. Press **Start capture**. Sampling happens at the host's existing aggregation boundary (about
   every 1-2 s), so arming it costs the stream nothing.
2. Reproduce the problem. The live graphs fill in as it runs.
3. Press **Stop & save**. The recording appears in the list below, and survives a host restart.

A recording carries per-stage p50/p99 pipeline latency, new frames/s versus re-encoded holds/s
(source starvation), the attempted wire bitrate against the target, and frame/packet/send drops plus
FEC recoveries. Its header names the **encoder backend and the GPU** that produced it, without
those, a stage split can't be read at all.
It also records the resolved capture backend, source dimensions and modifier, capture age, and
published, overwritten, and drained source-frame counts, so a rising age or overwrite count points
to the compositor handoff rather than the encoder.

**Download** saves it as a `.json` file you can attach to a report; **Delete** removes it. On disk
they live on the host in `~/.config/slipstream/captures/` until you delete them.
