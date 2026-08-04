---
title: Audio & microphone
description: Host speakers to your device, optional microphone uplink, client Audio settings, mute shortcut, echo loops, Windows VB-CABLE and VoiceMeeter notes, and Play vs Work defaults.
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

What surround means differs by host:

- **Linux.** The host claims a sink advertising exactly that many channels, so applications on the
  host can produce real surround.
- **Windows.** The host loopback-captures your current output endpoint and lets Windows convert it,
  so 5.1 from a stereo endpoint is an **upmix**, not new discrete channels.

Offered everywhere except the Decky plugin. Details:
[Client settings → Audio](/docs/client-settings#audio).

### Speaker device

**Speaker** (and on some apps a matching **Microphone** picker), *default: System default.* Which
endpoint stream audio plays out of. Only the **Linux** app (PipeWire nodes) and the **Mac** app
have these pickers. iPhone, iPad, Apple TV, Android, Decky, and the console home have none. The
Windows app has none and ignores a stored speaker choice; the session follows the system default.

On Linux, a device that has since disappeared keeps a "(not detected)" entry rather than silently
snapping back to default. Speaker and microphone pickers are facts about this device and
**cannot** live in a [settings profile](/docs/profiles-and-links#what-a-profile-cant-change).

### Windows: why the PC goes quiet

While a session is capturing desktop audio, the Windows host parks the default playback device on
a silent sink so sound comes out of the **client** only. That is why the PC in the other room goes
quiet when a stream starts. The default is restored when capture closes.

If you want the stream's sound **also** playing on the host's own speakers (someone in the room
watching along), set in `host.env`:

```ini
SLIPSTREAM_HOST_AUDIO=1
```

Restart the host after editing. Caveat: if you are streaming **from that same room**, your
device's microphone can hear those speakers and you get an echo loop; see
[The host's own speakers](/docs/echo#the-hosts-own-speakers) and remove the setting (or turn the
host volume down) while you stream from nearby.

`SLIPSTREAM_KEEP_DEFAULT` leaves Windows' default playback/recording devices alone entirely; the
mic uplink still picks a target, and you may have to select it yourself in Sound settings. Full
table: [Configuration → Audio / microphone](/docs/configuration#audio--microphone).

On **Linux**, desktop audio rides PipeWire. Wrong `XDG_RUNTIME_DIR` / uid anchors in `host.env`
point the host at another user's PipeWire and produce `pw audio connect ... Creation failed`;
delete bogus session anchors rather than chasing mixer settings
([Troubleshooting](/docs/troubleshooting#session-fails-right-after-editing-hostenv)).

## What you say (client → host)

### Stream microphone

**Microphone** / **Stream microphone**, *defaults:*

| Client | Default |
|---|---|
| Linux, Windows, Android, console home, Decky | **Off** |
| Apple app (Mac, iPhone, iPad) | **On** |
| Apple TV | No microphone input at all (tvOS) |

When on, this device's microphone is sent to the host's virtual mic. Games and chat apps on the
host pick that device the way they would any other input.

On **Windows**, the host installer can install **VB-CABLE** (VB-Audio, donationware) as that
virtual microphone. The checkbox is **ticked by default**; clear it, or pass
`/MERGETASKS="!installaudiocable"`, if you do not want it. VB-CABLE is a third-party **kernel-mode**
driver (the one exception to Slipstream's user-mode driver story); uninstalling Slipstream leaves
it in place because other apps may use it. See [Windows Host](/docs/windows-host) and
[Security](/docs/security). On a headless box with no real sound device, a virtual cable is also
what desktop audio may play into.

On **Linux**, the host presents a PipeWire `Audio/Source` for the uplink. Target a specific
Windows mic endpoint by friendly-name substring with `SLIPSTREAM_MIC_DEVICE` if needed
([Configuration](/docs/configuration#audio--microphone)).

Moonlight / GameStream sessions do **not** carry a Slipstream microphone uplink; use a native
client when you need mic.

### Muting mid-stream

**Ctrl+Alt+Shift+V** on the **Linux and Windows** clients stops sending your microphone to the
host; press again to resume. The uplink keeps running underneath, so unmuting is instant rather
than waiting for the device to warm up.

While muted, a **Microphone muted** badge sits in the top-right of the stream. It is separate
from the [stats overlay](/docs/stats): it shows even with stats off, because "am I still muted?" is
a question you ask ten minutes later.

The mute lasts for **that stream only**. The next session starts unmuted, and nothing is written
to settings. If **Stream microphone** is off, the shortcut does nothing and no badge appears.

Apple, Android, and Decky have no mute shortcut yet; turn **Stream microphone** off in their
settings instead. macOS honours the other Ctrl+Alt+Shift chords but **not** microphone mute.
Look the shortcut up again from **Keyboard Shortcuts** (Linux) or **Shortcuts** (Windows) without
a stream running; the in-stream hint over the video omits mute to stay one readable line.

Full context: [Muting your microphone](/docs/input#muting-your-microphone).

### Echo cancellation

**Echo cancellation**, *default: on.* Stops the host's audio, playing out of this device's
speakers, from being picked up by the microphone and sent straight back. It hands the mic to the
**system's own canceller**, it does not invent a second DSP stack:

| Platform | What "on" means |
|---|---|
| **Linux** | Capture from an echo-cancelled PipeWire source when the desktop provides one |
| **Windows** | Ask WASAPI for the Communications stream category so the endpoint's processing engages |
| **Apple / Android** | Platform voice-processing mode |

Turn it off if your microphone already runs its own processing, or if the canceller makes your
voice sound thin. The row sits under the microphone toggle and greys out while the microphone is
off. Offered by Linux, Windows, Apple, Android, and the console home; Decky has no toggle.

Operators can force cancellation off for a run with `SLIPSTREAM_NO_AEC=1` on the **client**
environment ([Configuration → client-side](/docs/configuration)); that only switches processing
off, never back on. The setting in Preferences is the normal control.

**Echo cancellation is not a substitute for headphones** when the stream plays from the same
device's speakers into an open mic. Work through
[Why do I hear myself](/docs/echo) when you hear yourself a beat later.

## Echo: the four loops (summary)

The dedicated page is short and worth reading in full: [Why do I hear myself](/docs/echo). In
order:

1. **Your device's speakers.** Stream audio out of a phone, tablet, or laptop speakers → mic
   picks it up. **Fix: headphones.** Echo cancellation helps; headphones remain the reliable fix.
2. **"Listen to this device" and app monitoring (Windows hosts).** If *Listen to this device* is
   ticked for the Slipstream mic (usually *CABLE Output*), or Discord / OBS monitoring is on, your
   voice plays into the host output and returns in the stream. Untick Listen; turn monitoring off.
3. **The host's own speakers.** `SLIPSTREAM_HOST_AUDIO` plus streaming from the same room. Remove
   the setting or lower host volume.
4. **Virtual mixers (VoiceMeeter and friends).** Older hosts could pick one VoiceMeeter strip as
   the mic target and another as audio capture, a feedback loop with no acoustics. **Current hosts
   refuse to capture VoiceMeeter or other virtual endpoints for desktop audio**, so this fixes
   itself with an update. If you still route through VoiceMeeter on purpose, make sure no strip
   that hears the Slipstream mic feeds the output being streamed.

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
through the stream. Prefer headphones on the office laptop if you do enable the mic for a call
that lives on the host. Absolute mouse and clipboard matter more than surround for desk work.

Keep separate **Work** and **Play** [settings profiles](/docs/profiles-and-links) if the same
laptop does both jobs against the same host.

## Moonlight and other clients

Native Slipstream clients own the Audio settings described here. Stock
[Moonlight](/docs/moonlight) over GameStream gets host audio on the classic GameStream ports; it
does not get Slipstream's microphone uplink or the Ctrl+Alt+Shift+V mute. Prefer a native client
when mic or echo-cancellation controls matter.

Apple TV has no microphone path at all. Decky exposes a smaller settings surface (mic on/off
among them) and shares the Linux client's settings file, so a change in either place shows up in
the other.

## Troubleshooting quick checks

| Symptom | Where to look |
|---|---|
| I hear myself a beat later | [Why do I hear myself](/docs/echo), start with headphones |
| Mic mute shortcut does nothing | Is **Stream microphone** on? Linux/Windows only; see [Input](/docs/input#muting-your-microphone) |
| Host PC silent while streaming (Windows) | Expected unless `SLIPSTREAM_HOST_AUDIO` is set |
| No mic device on Windows host | Was VB-CABLE installed? Re-run installer tasks or check Sound settings for *CABLE Output* |
| Choppy / delayed voice on host | `mic uplink health` in host Logs; network first, then `SLIPSTREAM_MIC_LEGACY_BUFFER` only as a bug-report escape hatch |
| `pw audio connect ... Creation failed` | Wrong uid / `XDG_RUNTIME_DIR` in `host.env` |
| Surround sounds like stereo upmix | Windows loopback behaviour; Linux can advertise a real multi-channel sink |

## Related pages

- [Why do I hear myself](/docs/echo) - full echo FAQ
- [Client settings → Audio](/docs/client-settings#audio) - channels, mic, AEC, device pickers
- [Mouse, touch and pen → Muting](/docs/input#muting-your-microphone) - Ctrl+Alt+Shift+V
- [Configuration → Audio / microphone](/docs/configuration#audio--microphone) - `host.env` knobs
- [Play](/docs/play) - couch streaming path
- [Desktop at work](/docs/desktop-at-work) - office mic default off
- [Windows Host](/docs/windows-host) - VB-CABLE install
- [Support matrix](/docs/support-matrix) - which clients offer microphone and surround
- [Controllers & gamepads](/docs/controllers) - pads that often sit next to a headset on the couch
