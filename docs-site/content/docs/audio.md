---
title: Audio & microphone
description: Host speakers to your device, optional microphone uplink, client Audio settings, mute shortcut, echo loops, and Play vs Work defaults.
---

Audio in Slipstream runs both ways when you ask for it. **Downlink** is always there: the host
captures what the session is playing and your client renders it on your speakers or headphones.
**Uplink** is optional: turn on **Stream microphone** (or **Microphone**) and your device's mic
appears as a virtual microphone on the host for Discord, party chat, or a call on the far machine.

This page is the map. The deep echo FAQ is
[Why do I hear myself](/docs/echo); mute mid-stream is under
[Muting your microphone](/docs/input#muting-your-microphone); every Audio row is in
[Client settings → Audio](/docs/client-settings#audio).

## What you hear (host → client)

The host captures desktop / session audio and sends it with the stream. Your client builds its
decoder from what the **handshake** says the host will really send, never from what you asked for
alone.

### Channels

**Audio channels**, *default: Stereo.* You can ask for **5.1** or **7.1**; anything else is read as
stereo. The host normalizes to 2, 6, or 8 channels and tells your client the real count before the
first frame.

On Linux the host claims a sink advertising exactly that many channels, so applications on the host
can produce real surround.

Offered everywhere except the Decky plugin. Details:
[Client settings → Audio](/docs/client-settings#audio).

### Speaker device

iPhone, Android, Decky, and the console home use the system default endpoints. Speaker and
microphone pickers (where a client offers them) are facts about this device and **cannot** live in a
[settings profile](/docs/profiles-and-links#what-a-profile-cant-change).

On Linux, desktop audio rides PipeWire. Wrong `XDG_RUNTIME_DIR` / uid anchors in `host.env`
point the host at another user's PipeWire and produce `pw audio connect ... Creation failed`;
delete bogus session anchors rather than chasing mixer settings
([Troubleshooting](/docs/troubleshooting#session-fails-right-after-editing-hostenv)).

## What you say (client → host)

### Stream microphone

**Microphone** / **Stream microphone**, *defaults:*

| Client | Default |
|---|---|
| Android, console home, Decky | **Off** |
| iPhone | **On** |

When on, this device's microphone is sent to the host's virtual mic. Games and chat apps on the
host pick that device the way they would any other input.

On Linux, the host presents a PipeWire `Audio/Source` for the uplink.

Moonlight / GameStream sessions do **not** carry a Slipstream microphone uplink; use a native
client when you need mic.

### Muting mid-stream

Where the client supports it, **Ctrl+Alt+Shift+V** stops sending your microphone to the host; press
again to resume. The uplink keeps running underneath, so unmuting is instant rather than waiting for
the device to warm up.

While muted, a **Microphone muted** badge sits in the top-right of the stream. It is separate
from the [stats overlay](/docs/stats): it shows even with stats off, because "am I still muted?" is
a question you ask ten minutes later.

The mute lasts for **that stream only**. The next session starts unmuted, and nothing is written
to settings. If **Stream microphone** is off, the shortcut does nothing and no badge appears.

iPhone, Android, and Decky have no mute shortcut yet; turn **Stream microphone** off in their
settings instead.

Full context: [Muting your microphone](/docs/input#muting-your-microphone).

### Echo cancellation

**Echo cancellation**, *default: on.* Stops the host's audio, playing out of this device's
speakers, from being picked up by the microphone and sent straight back. It hands the mic to the
**system's own canceller**, it does not invent a second DSP stack:

| Platform | What "on" means |
|---|---|
| **iPhone / Android** | Platform voice-processing mode |

Turn it off if your microphone already runs its own processing, or if the canceller makes your
voice sound thin. The row sits under the microphone toggle and greys out while the microphone is
off. Offered by iPhone, Android, and the console home; Decky has no toggle.

Operators can force cancellation off for a run with `SLIPSTREAM_NO_AEC=1` on the **client**
environment ([Configuration → client-side](/docs/configuration)); that only switches processing
off, never back on. The setting in Preferences is the normal control.

**Echo cancellation is not a substitute for headphones** when the stream plays from the same
device's speakers into an open mic. Work through
[Why do I hear myself](/docs/echo) when you hear yourself a beat later.

## Echo: the loops that matter (summary)

The dedicated page is short and worth reading in full: [Why do I hear myself](/docs/echo). In
order:

1. **Your device's speakers.** Stream audio out of a phone or tablet speakers → mic picks it up.
   **Fix: headphones.** Echo cancellation helps; headphones remain the reliable fix.
2. **App monitoring on the host.** If Discord / OBS monitoring is on, your voice plays into the
   host output and returns in the stream. Turn monitoring off.

While the microphone is in use, the host writes a **`mic uplink health`** line to its log every
30 seconds (web console → **Logs**): buffer depth, network gaps, client cadence. It will not name
an echo loop (routing, not network), but if your voice is also choppy or delayed, include that
line in a bug report. Startup logs also name which devices the host picked for mic and for audio
capture.

## Play vs Work

### Play

Couch gaming usually wants downlink audio loud and clear, often with headphones on the client so
party chat and game mix stay clean. Turn **Stream microphone** **on** when you join Discord or
in-game chat on the host; leave **Echo cancellation** on unless your headset already handles it.
Surround (5.1 / 7.1) is a request worth making on a home-theatre client when the host can supply
it. Pair with Capture mouse and a game-oriented bitrate on a [Play](/docs/play) profile.

### Work

For office remote desktop, [Desktop at work](/docs/desktop-at-work) suggests **Stream microphone:
Off unless you need it**, less background noise into the host. You still hear the host's speakers
through the stream. Prefer headphones on the office device if you do enable the mic for a call
that lives on the host. Absolute mouse and clipboard matter more than surround for desk work.

Keep separate **Work** and **Play** [settings profiles](/docs/profiles-and-links) if the same
device does both jobs against the same host.

## Moonlight and other clients

Native Slipstream clients own the Audio settings described here. Stock
[Moonlight](/docs/moonlight) over GameStream gets host audio on the classic GameStream ports; it
does not get Slipstream's microphone uplink or a mute shortcut. Prefer a native client when mic or
echo-cancellation controls matter.

Decky exposes a smaller settings surface (mic on/off among them).

## Troubleshooting quick checks

| Symptom | Where to look |
|---|---|
| I hear myself a beat later | [Why do I hear myself](/docs/echo), start with headphones |
| Mic mute shortcut does nothing | Is **Stream microphone** on? See [Input](/docs/input#muting-your-microphone) |
| Choppy / delayed voice on host | `mic uplink health` in host Logs; network first, then `SLIPSTREAM_MIC_LEGACY_BUFFER` only as a bug-report escape hatch |
| `pw audio connect ... Creation failed` | Wrong uid / `XDG_RUNTIME_DIR` in `host.env` |

## Related pages

- [Why do I hear myself](/docs/echo) - full echo FAQ
- [Client settings → Audio](/docs/client-settings#audio) - channels, mic, AEC, device pickers
- [Mouse, touch and pen → Muting](/docs/input#muting-your-microphone)
- [Configuration → Audio / microphone](/docs/configuration#audio--microphone) - `host.env` knobs
- [Play](/docs/play) - couch streaming path
- [Desktop at work](/docs/desktop-at-work) - office mic default off
- [Support matrix](/docs/support-matrix) - which clients offer microphone and surround
- [Controllers & gamepads](/docs/controllers) - pads that often sit next to a headset on the couch
