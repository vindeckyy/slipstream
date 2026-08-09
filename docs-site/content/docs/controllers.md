---
title: Controllers & gamepads
description: How Slipstream turns a pad on the couch into a virtual gamepad on the host, Linux input-group setup, DualSense fidelity, rumble, VirtualHere for real USB devices, gamescope limits, and Play vs Work.
---

A stream does not "share" your physical controller. The client reads the pad in your hands, and
the host builds a **virtual gamepad** that games and the desktop see as a normal device. Most of
the time that is automatic: connect, press A, the title responds. This page is what sits under
that, the host permission that makes virtual pads work, the type the host emulates, what DualSense
and rumble actually carry, when you need a real USB device instead, and how Play and Work use pads
differently.

Mouse, touch, pen, and the in-stream keyboard chords live on [Mouse, touch and pen](/docs/input).
The Controllers rows in Preferences are listed with every other client setting in
[Client settings](/docs/client-settings#input).

## How a pad reaches the host

1. Your client enumerates whatever SDL (or the platform) sees as a gamepad.
2. At connect it declares a **type** per pad, Automatic matches the physical controller, or you
   pin one in **Gamepad type** / **Controller type**.
3. The host creates a matching virtual pad and injects buttons, sticks, triggers, and (where the
   backends allow it) motion, touchpad, rumble, and DualSense HID feedback.
4. Each connected pad gets its own player slot. Pads can arrive and leave independently during a
   session.

What the host can actually build:

| Host | Virtual pads |
|---|---|
| **Linux** | uinput Xbox 360, plus UHID DualSense / DualShock 4 (and related rich types). Needs `/dev/uinput` access, see [Linux: the `input` group](#linux-the-input-group) below. |

A type the host has no backend for **degrades to an Xbox 360 pad** rather than failing the
session. A Sony pad on a host that cannot open `/dev/uhid` is the usual example. That rule is also
what [Client settings → when the client and the host disagree](/docs/client-settings#when-the-client-and-the-host-disagree)
lists for a gamepad-type request.

Operators who want to **force** a type for every session can set `SLIPSTREAM_GAMEPAD` in
`host.env` (see [Configuration → Gamepads](/docs/configuration#gamepads)). Leave it alone unless
you have a reason; Automatic from the client is the normal path.

## Linux: the `input` group

Virtual gamepads inject through `/dev/uinput`. That node is gated by the **`input` group**. If the
host user is not in it, clients still "see" your controller and the stream looks fine, but **games
on the host never get a pad**. The same membership is what [pen
input](/docs/input#pen-and-stylus) and GNOME's virtual touchscreen path need.

After installing the host package, add yourself and **log out and back in** so the new group
takes effect:

| Distro | Command |
|---|---|
| **Ubuntu**, **Fedora**, **Arch**, and most others | `sudo usermod -aG input "$USER"` |
| **Bazzite** | `ujust add-user-to-input-group` (do **not** use `usermod`; the base is immutable and the group is managed by a recipe) |

The shared [After installing](/docs/install#after-install) checklist names this as step 1 for
every Linux package. Distro pages repeat it in their own voice:
[Ubuntu](/docs/ubuntu), [Fedora](/docs/fedora), [Arch](/docs/arch), [Bazzite](/docs/bazzite).

On **SteamOS** the host installer adds you to `input`, installs the gamepad udev rule, and loads
`vhci-hcd` for native Steam Deck controller passthrough. That membership only applies **after
the first-install reboot**. Until then the pad degrades to a generic Xbox 360 controller (still
playable). See [SteamOS (Host)](/docs/steamos-host).

## Client settings that matter

Under **Controllers** (wording varies a little by app):

- **Gamepad type** (*Controller type* on iPhone, Android, and the console home), *default:
  Automatic*. Pickers offer Xbox 360, Xbox One, DualSense, and DualShock 4 everywhere, plus Steam
  Deck on Android, the console home, and Decky. Automatic declares what that controller
  really is; an explicit choice declares yours.
- **Forwarded controller** (*Use controller* on iPhone and the console home), *default:
  Automatic*, which forwards **every** connected controller as its own player. Pinning one
  restricts the session to that pad alone. Android has no such picker.

**Forwarded controller** is a fact about the device in your hands: it cannot live in a
[settings profile](/docs/profiles-and-links#what-a-profile-cant-change). The *type* the host creates
can.

Full rows: [Client settings → Input](/docs/client-settings#input). What each client actually
forwards (rumble, gyro, DualSense HID) is tabulated in
[Support matrix → Input while you stream](/docs/support-matrix).

## DualSense, rumble, and rich input

In short:

- **iPhone.** Rich capture is gated to the DualSense / DualShock 4 family; other pads get rumble
  only.
- **Android.** Rumble uses the controller's own motor where the kernel exposes it (many phones do
  not). An opt-in setting can also play player 1's rumble on the phone's own motor for clip-on
  pads. Motion / touchpad / adaptive triggers need the pad claimed over **USB**; over Bluetooth
  those paths are missing or dropped.
- **Steam Deck.** The Deck's own pad and attached controllers forward through the Decky / session
  client path; trackpads ride the touchpad surface where supported.
- **Moonlight / GameStream.** Classic multi-controller events reach the host; the extension
  packets for motion, touchpad, and trigger effects are **not** implemented on Slipstream's
  GameStream plane, so none of that rich input arrives there.

Rumble and HID feedback are host → client planes: the game on the host writes effects into the
virtual pad, and your client replays them on the physical one. If rumble is silent, check that
the emulated type supports it, that the physical pad has motors the OS exposes, and that you are
not on a GameStream-only path expecting DualSense HID.

## When the virtual pad is not enough: VirtualHere

Some devices only make sense as **themselves**: a racing wheel, HOTAS, pedals, an arcade stick, or
any controller whose value is that it is not emulated. For those, install the first-party
**VirtualHere** plugin. It hands a real USB device on the couch machine to the host while you
play, and gives it back afterwards.

VirtualHere itself is a commercial USB-over-IP product **sold separately**; Slipstream does not
bundle or download it. You need the USB Server on the couch and the USB Client on the host. There
is no VirtualHere server for iOS, so iPhones cannot pass devices through.

Install and configure from the [Plugins → VirtualHere](/docs/plugins#virtualhere-usb-passthrough)
page. The console's **Diagnostics** tab (and `slipstream-plugin-virtualhere doctor`) walks the
two-sided setup when nothing moves.

Ordinary Xbox / DualSense / Deck pads should stay on the built-in virtual-gamepad path. Reach for
VirtualHere when the game needs the real USB identity.

## Leaving a stream with a controller

Every client reserves one controller chord: **L1 + R1 + Start + Select** (LB + RB + Start + Back
on an Xbox pad), held on any connected pad.

- **Steam Deck**, a press releases capture only (Decky always launches fullscreen). Holding
  disconnects.
- **iPhone**, holding about 1.5 seconds disconnects; there is no quick-press step.
- **Android**, holding about a second disconnects; a quick press does nothing, and a **Hold to
  quit...** cue appears when the chord completes.

Full shortcut table, including keyboard release and microphone mute:
[Getting your input back](/docs/input#getting-your-input-back).

## gamescope / Gaming Mode

A [gamescope](/docs/gamescope) Gaming Mode host is excellent for couch play. Controllers still
inject; the limits that matter for pads-and-pointer are on the compositor side:

- **Desktop (absolute) mouse is unavailable.** Ask for it and the session quietly stays captured;
  the mouse-mode chord has nothing to switch to. Fine for games; wrong for office chrome.
- **Touch** from a phone or tablet arrives as a **single-finger absolute pointer**, not full
  multi-touch. Trackpad and direct-pointer touch modes still send ordinary mouse events and are
  unaffected.
- **No clipboard** in Gaming Mode, even with both clipboard switches on.

None of those stop a DualSense or Xbox pad from driving a game. They do mean a Bazzite or SteamOS
box in Gaming Mode is a **Play** host, not a **Work** one. Flip to the Plasma desktop (or run a
full desktop distro) when you need absolute mouse and clipboard; Bazzite's host follows that
switch mid-stream ([Bazzite](/docs/bazzite)).

## Play vs Work

Same host process, different pad expectations:

### Play (game streaming)

Host on a powerful PC, Steam Deck, or Bazzite box; client on a TV, phone, or Deck.
Prefer **Capture (games)** mouse, game-oriented bitrate, and often gamescope or a dedicated game
session so a library launch boots straight into the title. Forward every pad you care about
(Automatic), or pin one for single-player. DualSense fidelity and rumble matter here; VirtualHere
matters for wheels and HOTAS. Pair once on the LAN and stream from the couch. Shape of use:
[How it works → Play](/docs/how-it-works#play-game-streaming), and the Play guide at
[Play](/docs/play).

### Work (remote desktop)

Host on the workstation you left at home; client on an office device over a **private VPN**. You
usually want **Desktop (absolute)** mouse, clipboard on, and **Stream microphone** off unless you
need it. A gamepad is optional: useful for media remotes or the odd game at lunch, not required
for IDEs and browsers. Prefer a full desktop session over gamescope. Step-by-step:
[Desktop at work](/docs/desktop-at-work). Keep a separate **Play** settings profile with Capture
mouse and game-oriented bitrate for the same host when you are home on the couch
([Profiles and links](/docs/profiles-and-links)).

## Troubleshooting

### A controller is detected but games don't see it

The host user needs the `input` group. On Bazzite run `ujust add-user-to-input-group`, then log
out and back in. Elsewhere: `sudo usermod -aG input $USER` and re-login. See
[Troubleshooting](/docs/troubleshooting#a-controller-is-detected-but-games-dont-see-it) and
[Bazzite → Allow controller input](/docs/bazzite#allow-controller-input).

### The pad works as Xbox 360 but not as DualSense / Deck

DualSense / DualShock 4 need UHID; Steam Deck native passthrough on SteamOS needs `input` plus
`vhci-hcd` after the first reboot. An unsupported type folds to Xbox 360 by design. Check
`SLIPSTREAM_GAMEPAD` is not forcing a type the host cannot build, and that Automatic is declaring
what you expect in [Client settings](/docs/client-settings#input).

### Rumble or adaptive triggers are missing

Confirm you are on a **native** Slipstream session (not expecting DualSense HID over Moonlight),
that the emulated type is DualSense-family when you want HID effects, and that the physical link
(USB vs Bluetooth on Android) actually exposes those features. See the
[support matrix](/docs/support-matrix) footnotes for your client.

### Still stuck?

Start from [Troubleshooting](/docs/troubleshooting). For USB-passthrough failures, use the
VirtualHere plugin's Diagnostics tab. For "my mouse and keyboard are stuck in the stream," that
is capture working as designed: **Ctrl+Alt+Shift+Q** or **L1+R1+Start+Select** releases them
([Input](/docs/input#getting-your-input-back)).

## Related pages

- [Mouse, touch and pen](/docs/input) - chords, mouse modes, touch, pen
- [Client settings](/docs/client-settings) - Gamepad type, Forwarded controller, Audio
- [Play](/docs/play) - couch / game streaming path
- [Desktop at work](/docs/desktop-at-work) - office remote desktop path
- [Plugins → VirtualHere](/docs/plugins#virtualhere-usb-passthrough) - real USB devices
- [gamescope](/docs/gamescope) - Gaming Mode limits
- [Configuration → Gamepads](/docs/configuration#gamepads) - `SLIPSTREAM_GAMEPAD`
- [Support matrix](/docs/support-matrix) - per-client rumble / gyro / DualSense truth
- [Troubleshooting](/docs/troubleshooting#a-controller-is-detected-but-games-dont-see-it)
