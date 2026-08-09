---
title: Why do I hear myself
description: Echo while streaming, the places it can come from (client speakers, host app monitoring) and how to stop each one.
---

You talk into your device's microphone and hear your own voice come back a beat later. The echo
is almost never "in the stream" itself, it's a loop in a well-known place. Work through them in
order; the first covers nearly every report.

## Your device's speakers

If the stream's audio plays out of the **speakers of the device you're streaming on**, a
phone or tablet without headphones, its microphone picks that sound back up and sends
it to the host along with your voice. Everyone in your voice chat hears the game twice, and you
hear yourself whenever anything routes the mic back.

**Fix: use headphones on the device you're streaming on.** Clients also try to cancel it for
you: **Echo cancellation** in [client settings](/docs/client-settings#audio) is on by default on
Android and the console home, and hands the microphone to the system's own canceller. How
much it removes depends on the device, so headphones remain the reliable fix everywhere.

If you just need to stop talking for a moment, turn **Stream microphone** off in client settings,
or use the mute shortcut where your client offers one; see
[Input](/docs/input#getting-your-input-back).

## App monitoring on the host

The same loop hides in apps on the host: **Discord's** *Mic Test* / input monitoring, **OBS's**
*Monitor audio* on a mic source, and similar monitoring features in other tools all play your mic
into the host's output. Turn the monitoring off rather than the mic.

## What the host log can tell you

While your microphone is in use, the host writes a **`mic uplink health`** line to its log every
30 seconds (web console -> **Logs**). It shows how much of your voice is buffered on the host
(`depth_ms` vs `target_ms`), how much the network lost (`gaps`, `concealed`), and how steadily
your client is delivering audio (`cadence_ms`). It won't point at an echo loop directly, echo
is a routing problem, not a network one, but if your voice also sounds choppy or delayed,
include that line in a bug report. The startup log also names exactly which devices the host
picked for the microphone and for audio capture, which is the quickest way to spot a monitoring
loop like the ones above.
