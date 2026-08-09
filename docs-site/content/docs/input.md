---
title: Mouse, touch and pen
description: The in-stream keyboard shortcuts that give your mouse back, the two mouse modes, the three touch modes, and full-fidelity stylus input.
---

A stream takes your mouse and keyboard the moment you click into it. This page starts with how to
get them back, then covers driving the host with a mouse, a touchscreen and a pen. The rows that
pick these modes sit in your client's **Input** settings; the toggles that share that page with
them are in [Client settings](/docs/client-settings#input).

## Getting your input back

Input is **captured** when the stream starts and whenever you tap or click the video: keys and
pointer go to the host instead of your own device. In the default mouse mode the pointer is also
locked to the stream, see [Mouse modes](#mouse-modes) below.

| Shortcut | What it does |
|---|---|
| **Ctrl+Alt+Shift+Q** | Release captured input (press again, or tap/click the stream, to take it back) |
| **Ctrl+Alt+Shift+M** | Switch the mouse mode (capture ⇄ desktop), where the client offers it |
| **Ctrl+Alt+Shift+D** | Disconnect |
| **Ctrl+Alt+Shift+S** | Cycle the [stats overlay](/docs/stats), off · compact · normal · detailed |
| **Ctrl+Alt+Shift+V** | Mute or unmute your microphone (where the client supports it) |

### Muting your microphone

Where the client supports it, **Ctrl+Alt+Shift+V** stops sending your microphone to the host, and
pressing it again resumes. The uplink itself keeps running underneath, so unmuting is instant rather
than a second of the device warming back up.

While you are muted a **Microphone muted** badge sits in the top-right corner of the stream. It
is deliberately separate from the [stats overlay](/docs/stats): it shows even with stats off,
because "am I still muted?" is a question you ask ten minutes later.

The mute lasts for that stream only, the next session starts unmuted, and nothing is written to
your settings. If the stream isn't sending a microphone at all (**Stream microphone** off in
[client settings](/docs/client-settings#audio)) the shortcut does nothing and no badge appears,
rather than pretending to mute something.

Android and Steam Deck have no mute shortcut yet; turn **Stream microphone** off in their
settings instead.

### On each client

- **Android and Android TV** honour **Ctrl+Alt+Shift+Q** only; it toggles pointer capture. The
  system Back button leaves the stream.
- **Steam Deck**, use the controller chord below; Decky always launches fullscreen.

### Leaving with a controller

Every client reserves one controller chord: **L1 + R1 + Start + Select** (LB + RB + Start + Back on
an Xbox pad), held on any connected pad.

- **Steam Deck**, a press releases captured input only. The Decky plugin always launches the client
  fullscreen, and a stream that started fullscreen stays that way. Holding disconnects.
- **Android**, holding it about a second disconnects. A quick press does nothing; the moment the
  chord completes a **Hold to quit...** cue appears so you know it registered.

## Mouse modes

There are two, and they are a per-client setting called **Mouse input**. Pick based on the job:

| Job | Mode |
|---|---|
| Games with mouse-look, shooters, many 3D titles | **Capture (games)** |
| Remote desktop, office apps, browsers, IDEs | **Desktop (absolute)** |

- **Capture (games)**, the pointer locks to the stream and only relative movement is sent. The only
  cursor you see is the host's. This is what mouse-look in a game needs.
- **Desktop (absolute)**, the pointer is not locked. It moves in and out of the stream freely and
  its position is sent as an absolute point, what you want for remote desktop work. Your local
  cursor is hidden over the stream; the one you see there is the host's. This is the mode the
  [Desktop at work](/docs/desktop-at-work) checklist expects.

**Capture is the default** where the client offers mouse modes. **Android defaults to Desktop**,
a phone or TV is more often driven by touch or a pad than by a locked mouse.

Switch live with **Ctrl+Alt+Shift+M** where supported. On
Android, Ctrl+Alt+Shift+Q flips the capture instead.

Two things can override your choice. **gamescope hosts can't take absolute pointer input**: ask for
desktop mode against one and the session quietly stays captured, and the chord has nothing to offer
(see [gamescope](/docs/gamescope)). And against a host that forwards its cursor separately instead of
drawing it into the video, some clients flip to relative motion by themselves when an app on the host
grabs or hides the pointer, then back when it lets go.

## Host cursor while streaming

While at least one client is connected, the host hides its **local** OS cursor on the machine
running Slipstream (so the laptop or desktop pointer is not sitting on the mirrored display). The
stream still gets a pointer: host-composite and the cursor channel keep drawing it for the client.
When the last session ends, the host cursor comes back.

Turn this off with `SLIPSTREAM_HIDE_HOST_CURSOR=0` (or `input.hide_host_cursor` in host-config) if
you want the host pointer visible on-glass during a stream. On GNOME Wayland, if a hardware cursor
still shows after connect, try `MUTTER_DEBUG_DISABLE_HW_CURSORS=1` in the session environment as a
last resort (requires a new gnome-shell session).

## Touch modes

On a touchscreen client the **Touch input** setting picks one of three models. Android exposes all
three; Steam Deck uses its controller and mouse modes.

- **Trackpad** (the default), your finger drives the host cursor like a laptop touchpad. The cursor
  stays put when you touch down and moves by your finger's travel, so you can lift and re-swipe to
  walk it across a screen far larger than your own.
- **Direct pointer**, the cursor jumps to your finger and follows it.
- **Touch passthrough**, every finger is forwarded as a real touch contact, with no gesture
  interpretation at all. Only useful for apps and games that genuinely understand touch.

Trackpad and Direct pointer share one gesture vocabulary: tap = left click, two-finger tap = right
click, two-finger drag = scroll, tap-then-press-and-drag = a held left drag, **three-finger tap =
cycle the stats overlay**. On Android a **three-finger swipe up or down** summons or
dismisses the local on-screen keyboard for typing on the host.

Touch passthrough depends on the host being able to inject touch, and that varies:

| Host | Touch passthrough |
|---|---|
| KDE Plasma (KWin), GNOME | Full multi-touch |
| Sway, Hyprland and other wlroots compositors | Not injected, contacts are dropped |
| gamescope Gaming Mode | Degraded to a single absolute pointer, see [gamescope](/docs/gamescope) |

On GNOME/Mutter, Slipstream uses a virtual Linux touchscreen when Mutter's direct EIS session
does not expose a touchscreen device. This needs the same `/dev/uinput` access used by virtual
gamepads. Without that access, GNOME falls back to the single-pointer behavior described below.

The gamescope row is a rule, not an exception: wherever the compositor offers no touchscreen device
to drive, only the first finger is used, as an absolute pointer. Tapping still clicks; pinches and
other multi-finger gestures do not survive.

The trackpad and pointer models are unaffected by all of this: they send ordinary mouse events.

## Pen and stylus

A stylus is not treated as a finger. Slipstream carries **position, tip pressure, tilt angle and tilt
direction, barrel roll, hover distance, the eraser end, and two barrel buttons** on their own input
plane, so drawing and handwriting behave the way they do locally.

**Clients that send pen input:**

- **Android** phones and tablets with an active stylus, pressure, tilt, hover, the eraser tool and
  both barrel buttons. Android exposes no barrel-roll axis, so roll is not sent from there.
- **[Moonlight](/docs/moonlight) clients** that send pen events reach the same host-side pen.

Steam Deck does not send stylus input.

**What the host presents it as:**

On Linux, a virtual tablet named **Slipstream Pen** appears the first time you use the stylus
and is removed when the session ends. Applications see a real pen through the usual tablet path,
so Krita, GIMP and Xournal++ treat it as a graphics tablet. It is a screen tablet, mapped by your
compositor's own default tablet mapping, correct on a single output; multi-monitor pinning is up
to the compositor.

**Before touch passthrough and pen input can work**, the host needs access to `/dev/uinput`, the
same `input` group step the virtual gamepads need, covered under
[After installing](/docs/install#after-install). Without it, GNOME falls back to a single
pointer and the host never offers pen input.

**If the host is too old, or pen is switched off**, nothing breaks: the client keeps folding the
stylus into its ordinary touch or pointer path. You can still draw, just without pressure and tilt.
Whether pen splits out is decided by the host, not by your touch mode: you can be in Trackpad mode
and still draw with full fidelity.

**Operators** can turn the whole feature off by setting `SLIPSTREAM_PEN=0` in the host's `host.env`
(see [Configuration](/docs/configuration)). The host then stops advertising pen to Slipstream and
Moonlight clients alike, and every client falls back to touch.
