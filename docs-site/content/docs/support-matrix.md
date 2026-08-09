---
title: Support matrix
description: What actually works where, Linux host compositors, encoders, client decoders and per-client features, each cell taken from the code that decides it rather than from a feature name.
---

This page is the one place that says **what works where**. Find your row (your host desktop, your
GPU, your client app) and read across; every cell is taken from the code path that makes the
decision, not from a feature's name.

Support in Slipstream is **non-uniform by design**. The host does not ship one capture path and one
input path and hope your desktop cooperates. It carries a separate backend for each compositor and
each GPU vendor, and it adapts to whatever that combination actually offers. A capability that is
excellent on KDE can be impossible on gamescope. That is why the honest answer to "does Slipstream
do X?" is usually "on which compositor, with which GPU, from which client".

## Legend

| | Meaning |
|---|---|
| ✅ | Works. |
| ⚠️ | Works, with a named caveat. The numbered note under the table says which. |
| ❌ | Not supported. Nothing to configure. |
| ❓ | Untested or unverifiable from this repository. The note says what would settle it. |

A ⚠️ is never left unexplained. If you see one, its number is directly below the table.

## Four different kinds of "yes"

The distinction below is the whole value of this page, because it tells you *when* a cell can turn
into a "no" on your machine:

- **Compiled in**, decided when the binary was built (`#[cfg]`, or a cargo feature). It is either
  there or it isn't, and no setting changes it. A hand-built host has no NVENC unless you enabled
  the feature.
- **Probed at runtime**, the host or client asks your driver, GPU or compositor and believes the
  answer. The backend supports it; **your device may still refuse it**. Most codec, 10-bit and
  4:4:4 cells are this kind.
- **Negotiated**, both ends must advertise it before it happens. The clipboard, pen input, 4:4:4
  and client-drawn cursors all die quietly if either side says no.
- **Default on / opt-in / operator-gated**, HDR and 10-bit are attempted by default; the game
  library is off by default on some clients; the shared clipboard is off on the host until an
  operator turns it on.

One more thing a table cannot show: a few capabilities **latch off for the rest of the host
process** after a failure, a lost HDR negotiation, a repeatedly dying zero-copy import worker, a
gamescope that came up without the flags we asked for. Those are called out where they apply. If a
capability worked yesterday and not today, restart the host before believing the table is wrong.

## Host platform

Slipstream hosts on **Linux only**. Android and Steam Deck are client platforms. There is
no configuration that turns a phone or Deck into a host.

### Display and capture

By default every session gets its own display at your client's exact resolution and refresh rate.
How that is built differs per compositor, and [Virtual displays](/docs/virtual-displays) covers the
mechanics. The one exception is a gamescope the host only *attaches* to, which keeps its own mode
(note 4).

| Host | Own virtual display | Mirror a real monitor | Headless box |
|---|---|---|---|
| KDE Plasma (KWin) | ✅ | ✅ | ✅ |
| GNOME (Mutter) | ✅ ¹ | ✅ | ✅ |
| gamescope (SteamOS · Bazzite) | ✅ ² | ⚠️ ³ | ✅ ⁴ |
| sway ⁷ | ✅ | ✅ | ⚠️ ⁵ |
| Hyprland | ⚠️ ⁶ | ✅ | ⚠️ ⁵ |
| anything else | ❌ | ❌ | ❌ |

The physical-monitor mirror path has five Linux capture adapters. Portal/PipeWire is the general
Wayland path and carries dmabuf negotiation when the compositor offers it. The wlroots screencopy
adapter uses the compositor's direct SHM protocol. `kms` reads the active DRM primary plane and
exports its framebuffer as a dma-buf, which avoids a host copy but depends on DRM-card access, an
active primary plane, and an encoder that can import that format/modifier. `x11` is a synchronous
X11 `GetImage` fallback that copies the framebuffer through the X socket. `nvfbc` is the NVIDIA X11
path: the host loads NvFBC and CUDA at runtime, uses the driver's push model, and copies the
driver-owned CUDA surface into a pooled encoder buffer. Every adapter is runtime-probed, so a
missing driver or protocol is reported as unavailable instead of becoming a selectable but doomed
option. Hermes-KMS is not part of Slipstream.

1. Sizing works the other way round here: Mutter sizes the virtual monitor from the video format
   negotiation rather than from the create call. Also, all virtual-monitor changes are serialized
   host-wide on purpose; concurrent ones have crashed gnome-shell.
2. gamescope is not a protocol the host talks to, and it resolves one of three sub-modes. It
   **spawns** a bare nested gamescope at your client's exact size and refresh (the plain-distro
   default); it **manages** a `gamescope-session-plus` / SteamOS session, relaunching it at your
   client's mode (Bazzite and SteamOS, which have that infrastructure); or it **attaches** to a
   gamescope someone else started. An attach is the only one that is not sized for you: a box
   driving a physical panel is mirrored at *its* mode, because re-moding it would flip the screen
   someone is sitting in front of. In every mode the host captures gamescope's built-in video node.
   See [gamescope](/docs/gamescope).
3. Only when gamescope is driving a real connector (a Game Mode session on a TV or handheld).
   A nested or headless gamescope reports no heads and the picker is legitimately empty.
4. The host injects a small splash client on purpose: a fresh gamescope only produces frames once
   something paints, and a game's own bootstrap paints nothing for several seconds.
5. Yes, but it needs managed configuration rather than being dialog-free by nature. The host
   writes and merges the portal's chooser configuration so a headless box can answer the
   "which output?" question that otherwise requires a GUI.
6. Contracts are verified against Hyprland 0.55.4 with xdph 1.3.x, but the virtual output has
   **not been exercised end to end on real display hardware**, see
   [What is not verified](#what-is-not-verified).
7. Sway specifically, not the wlroots family. Creating the headless output, setting its mode and
   listing your monitors all go through sway's IPC (`swaymsg`), so a wlroots compositor without it
   (River, dwl, ...) cannot host; the session fails at `swaymsg get_outputs`. Their input would work
   (they do have the wlroots virtual pointer and keyboard protocols), but with no video there is no
   stream. See [Sway / wlroots](/docs/sway).

### Input, cursor and HDR

| Host | Input backend | Approval dialog | Cursor | HDR10 source |
|---|---|---|---|---|
| KDE Plasma (KWin) | `fake_input` | none ¹ | ✅ | ❌ ² |
| GNOME (Mutter) | libei (direct) | none ⁸ | ⚠️ ³ | ⚠️ ⁴ |
| gamescope | libei (gamescope's own) | none | ⚠️ ⁵ | ⚠️ ⁶ |
| sway / wlroots | virtual pointer + keyboard | none | ⚠️ ⁷ | ❌ ² |
| Hyprland | virtual pointer + keyboard | none | ⚠️ ⁷ | ❌ ² |

1. Authorization comes from the host's installed `.desktop` entry (`X-KDE-Wayland-Interfaces`),
   which is how KWin grants the restricted `zkde_screencast` protocol without a pop-up. A headless
   box needs nobody to click "Allow".
2. Virtual outputs on these compositors are 8-bit. This is a compositor limitation, not a setting.
3. Cursor metadata is used for effectively every session because GNOME's "compositor draws it"
   mode is not real on a virtual stream from Mutter 48 onwards: recorded frames come out without
   a pointer, and pointer-only movement schedules no new frame at all.
4. There is a second, separate Linux HDR route: mirroring a real HDR monitor on **GNOME 50+**. It
   is available on the Moonlight/GameStream plane only, it must be asked for explicitly, and it is
   re-checked live against the monitor's current colour mode. See [HDR](/docs/hdr).
5. gamescope puts the pointer on a hardware plane, so no cursor arrives in the captured video. The
   host reconstructs and draws it, and only skips that blend when three things hold at once: the
   resolved binary is the patched `slipstream-gamescope` build **at patch revision 2 or newer**
   (revision 1 carries HDR only), **this host spawned or manages the session** (an attach to a
   gamescope someone else started can promise nothing), and no earlier spawn was seen coming up
   without the flags we asked for. Note 6 gates HDR on the same two terms.
6. The only Linux virtual output that can be 10-bit, and it needs three things to be true at once:
   HDR attempts are on (they are, by default), the resolved gamescope binary is the patched
   `slipstream-gamescope` build, and **this host spawned or manages the session**. Attaching to a
   gamescope someone else started can never be HDR. See
   [HDR on gamescope](/docs/gamescope#hdr-on-gamescope).
7. Cursor metadata is wired up but has **never been validated on real hardware** on these two
   backends. It should work; nobody has confirmed it. The compositor-draws-it mode is the tested
   path.
8. Not a desktop entry. On GNOME the host talks to Mutter's own `org.gnome.Mutter.RemoteDesktop`
   D-Bus EIS directly rather than the xdg RemoteDesktop portal. That interface asks for no
   interactive approval at all, which is the point: the portal's `Start()` would simply time out
   on a headless box with nobody there to answer it.

### Version floors worth knowing

- **KWin**, the compositor must implement `createVirtualOutput`. The **DRM backend**, which is
  what an ordinary Plasma session runs, does at **any** version; KWin's headless **virtual
  backend** (`kwin_wayland --virtual`, what the headless test path uses) only since **6.5.6**.
  Below that the request fails with "Could not find output". **6.6** adds custom modes on virtual
  outputs, which is how the host reaches refresh rates above 60 Hz. KWin **6.7+** changed its
  output model again; that is handled.
- **sway 1.8**, headless outputs can be removed again on teardown.
- **Hyprland**, no version gate in the host: the `hyprctl` mode-rule path is version-independent
  and the running version is read for the log only. Contracts are verified against **0.55.4 with
  xdph 1.3.x**. On **0.49+**, if you have turned `ecosystem.enforce_permissions` on (it is off by
  default), grant the host screencopy and virtual pointer/keyboard. A denial shows up as silent
  black frames and dropped input, not as an error.
- **gamescope 3.16.22**, below it, headless capture deadlocks against PipeWire 1.6 and newer.
  **3.16.23**, below it, the Steam overlay never reaches the stream.

## Encoders

Every codec cell below is a **runtime probe**: the host opens a tiny real encoder, or asks the
driver's capability list, once per GPU and codec, and believes the answer. Your silicon may decline
what the backend supports. AV1 encode in particular is narrow (Ada and newer on NVIDIA, RDNA3 and
newer on AMD, Arc and newer on Intel).

| Host GPU | Backend | Codecs | 10-bit / HDR | 4:4:4 |
|---|---|---|---|---|
| NVIDIA | NVENC (direct SDK) | probed ¹ | ✅ ² | ⚠️ ³ |
| AMD, Intel | Vulkan Video | HEVC, AV1 ⁴ | ⚠️ probed | ❌ |
| AMD, Intel | VAAPI | probed | ⚠️ probed | ❌ ⁶ |
| any | PyroWave | wavelet ⁵ | ❌ ⁷ | ✅ |
| none | software H.264 ⁸ | H.264 only | ❌ | ❌ |

1. H.264, HEVC and AV1, intersected with what the driver reports. If the probe cannot run (no
   driver, or a build without NVENC), the host advertises the full set rather than nothing, so an
   advertised codec is not always a *confirmed* codec.
2. Requires the direct-SDK NVENC path (which every shipped Linux package builds). Without it the
   frame takes a slower CPU route to reach 10-bit.
3. HEVC only, and only when the GPU's 4:4:4 capability bit says yes. 4:4:4 **and** HDR together is
   refused.
4. H.264 never uses this backend, and it exists only in a build carrying the Vulkan-encode feature
   (every shipped Linux package does). Even then the device is asked, per codec and per bit depth,
   whether it can open that encode profile; a "no" routes to VAAPI before a session is burned. A
   Vulkan open that fails also falls back to VAAPI, with one deliberate exception: if the capture
   already negotiated producer-native NV12 (gamescope), there is no fallback and the session fails
   loudly, because VAAPI would misread that two-plane buffer as packed RGB and stream silent
   garbage.
5. [PyroWave](/docs/pyrowave) is a wavelet codec, not H.26x. It is never picked automatically: your
   client has to ask for it by name in its codec setting.
6. Not a hardware limit. The VAAPI backend simply has no 4:4:4 path yet, so the probe declines
   unconditionally and the session is negotiated as 4:2:0.
7. PyroWave is 8-bit on Linux.
8. Explicit-only. `auto` never resolves here: a box with no usable GPU driver fails the session at
   encoder open rather than quietly encoding on the CPU, so set `SLIPSTREAM_ENCODER=software`
   deliberately if that is what you want.

**4:4:4 across the whole project:** only HEVC and PyroWave can carry it, only NVENC and PyroWave can
produce it when a native client requests it. GameStream sessions are always 4:2:0.

### How the host picks a backend

It is **not** a ladder. The web console's GPU preference outranks the probe. If the console names a
specific adapter, that adapter's vendor decides: AMD or Intel -> the AMD/Intel opener, even on a
box that also has an NVIDIA card; NVIDIA -> NVENC, but still only if the proprietary driver's
`/dev/nvidiactl` or `/dev/nvidia0` is there, so a nouveau card falls to the AMD/Intel opener. With
no preference set, a live CUDA capture or an NVIDIA device node means NVENC; otherwise the
AMD/Intel opener, which tries Vulkan Video first for HEVC and AV1 and falls back to VAAPI. Linux
`auto` **never** lands on the software encoder. You have to ask for it deliberately, so that a box
with a broken driver fails loudly instead of quietly encoding on the CPU.

Codec negotiation itself: your client's list is intersected with the host's, a single explicit
client preference wins if the host can also produce it, and otherwise the order is **HEVC -> AV1 ->
H.264**. PyroWave is outside that order entirely, preference only.

### Zero-copy capture to encode

This is a **GPU and encoder** question, not a compositor one, which is why it is not a column above.

VAAPI takes the captured buffer directly; NVIDIA imports it through an isolated worker process;
PyroWave imports it on any vendor. Two compositor-shaped edges matter: gamescope offers only linear
buffers, and GNOME on NVIDIA allocates tiled-only, which PyroWave can still import, so that
combination keeps zero-copy where H.26x would not. Repeated import failures latch the host onto the
CPU path for the rest of its life.

### What is actually in the build you installed

- **Linux packages** (Arch, RPM, deb, Nix): NVENC and Vulkan Video, plus PyroWave. VAAPI and the
  software encoder are always compiled in.
- **A hand `cargo build` of the host** compiles in only what needs no cargo feature: VAAPI, libav
  NVENC and the software encoder, plus PyroWave (a *default* feature). Direct-SDK NVENC and Vulkan
  Video are opt-in. Without the direct SDK, an NVIDIA box still encodes on the GPU through libav
  NVENC; it loses real loss recovery and the 10-bit zero-copy path, not the GPU. If you built from
  source and your GPU seems unused, check this first.

## Client decode

Product clients are **Android** and **Steam Deck**. **Moonlight** works as a catch-all
over the GameStream plane. Other Moonlight-compatible clients are fine on that plane; their decode
details live in those apps.

| Client | Decode path (in order) | Codecs | 10-bit / HDR | 4:4:4 |
|---|---|---|---|---|
| Android · Android TV | MediaCodec only ¹ | H.264, HEVC, AV1 ² | ⚠️ ³ | ❌ |
| Steam Deck (via Decky) | Vulkan Video -> VAAPI -> software ⁴ | probed ⁵ | ✅ | ❌ |
| Moonlight | your Moonlight app's | negotiated | ⚠️ ⁶ | ❌ |
| LG webOS (`ss-webos`) | ❓ | ❓ | ❓ | ❓ ⁷ |

1. Chosen by name from a ranked device list that prefers hardware, real SoC vendors and low-latency
   decoders, and blocks the known-bad software ones. There is no software rung.
2. H.264 and HEVC are assumed universal on Android hardware; AV1 is probed.
3. Runtime-probed against the actual display. On an SDR panel the client advertises no HDR at all
   so the host sends a correct 8-bit picture instead of PQ your screen would mangle.
4. The Decky plugin does not decode anything; it launches the Linux session binary, so the decode
   path is identical to that binary, including the Mesa `RADV_PERFTEST=video_decode` opt-in the
   session binary sets before any Vulkan call (without it RADV exposes no decode queue and the Deck
   silently falls back to VAAPI, which fringes chroma on that GPU). What differs on the Deck is the
   hardware, not the code. **The order depends on your GPU vendor.** NVIDIA and AMD get Vulkan
   Video first; Intel and unknown vendors get VAAPI first, because FFmpeg's Vulkan path is
   field-broken on Intel Arc even though the driver advertises it. Pick one explicitly in
   Preferences or with `SLIPSTREAM_DECODER`; an explicit choice that fails is a hard error, never a
   silent fallback. Mid-session demotion is laddered, and each rung needs both three consecutive
   decode errors *and* a full second of them: Vulkan Video first demotes to VAAPI, and only that
   backend demotes to software.
5. Enumerated from FFmpeg at startup, plus PyroWave when the GPU passes its compute probe.
6. Whether HDR is offered is decided by the host and layered into what Moonlight is told, so an
   HDR toggle only appears in Moonlight when the host could really do it. See
   [Moonlight](/docs/moonlight).
7. `ss-webos` is a community client in a separate repository. Nothing in this codebase can
   establish its capabilities, so it is honestly blank rather than optimistically filled in.

**Multi-slice frames** are not advertised by Android, because some TV-box decoders wedge
the whole device on them, so those sessions always receive single-slice frames. The Steam Deck
session binary can advertise them. This is decoder truth, not a tuning knob.

## Clients and features

**Steam Deck** streams through the Decky plugin, which launches the shared Linux session binary.
**Android** is one app, with Android TV being the same app in leanback mode. **Moonlight** uses the
GameStream plane.

### Before you connect

| Client | Profiles | `slipstream://` links | Game library | Speed test | Wake-on-LAN | Updates itself |
|---|---|---|---|---|---|---|
| Android · Android TV | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ ² |
| Steam Deck (Decky) | ❌ ¹ | ❌ | ✅ ² | ❌ | ✅ ³ |
| Moonlight | ❌ ⁴ | ❌ ⁴ | ✅ ⁵ | ❓ ⁶ | ❓ ⁶ | ❓ ⁶ |

1. The plugin writes flat values into the shared client settings; it has no profile surface. The
   client it launches still honours whatever profile that settings file names.
2. Including pinned one-tap "Stream *game*" rows in the Quick Access Menu, which follow a host
   across IP changes. Not subject to a desktop opt-in.
3. Both the plugin itself and, where the install kind allows it, the client it launches.
4. [Profiles and links](/docs/profiles-and-links) are Slipstream-app concepts and do not exist on
   the GameStream plane.
5. Moonlight sees the host's titles as ordinary GameStream apps.
6. These live in whichever Moonlight app you use, not in this project. The host does publish the
   information a Wake-on-LAN-capable client needs. See [Wake-on-LAN](/docs/wake-on-lan).

### Input while you stream

| Client | Gamepads | Rumble | Gyro · touchpad · triggers | Pen | Touch modes | Mouse modes |
|---|---|---|---|---|---|---|
| Android · Android TV | ✅ ¹ | ⚠️ ² | ⚠️ ³ | ✅ ⁴ | ✅ ⁵ | ✅ |
| Steam Deck (Decky) | ✅ ¹ | ✅ | ✅ ⁶ | ❌ | ⚠️ ⁷ | ⚠️ ⁸ |
| Moonlight | ✅ | ✅ | ❌ ⁹ | ⚠️ ¹⁰ | ⚠️ ¹¹ |, |

1. Multiple controllers, each on its own stable slot, arriving and leaving independently. The pad
   **type** the host emulates is picked per pad. Android and Decky offer six presets including
   Steam Deck.
2. Uses the controller's own vibration motor where the phone kernel exposes it; many do not. There
   is an opt-in setting that **also** plays player 1's rumble on the phone's own motor, for clip-on
   pads with no motors of their own.
3. Only when the pad is claimed over **USB** (on by default). Over Bluetooth, Android's normal
   controller path carries no motion or touchpad, and adaptive-trigger effects are parsed and
   dropped because Android has no public API for them.
4. Full stylus support including eraser, both barrel buttons and hover. Android exposes no barrel
   roll.
5. Not applicable on Android TV, no touchscreen.
6. DualSense/DualShock 4 touchpad and motion, plus any controller SDL exposes a gyro on (including
   the Deck's own pad). The Deck's trackpads ride the same touchpad surface.
7. All three touch modes exist in the shared session binary and the picker is there. Only
    meaningful on a touchscreen.
8. The desktop (absolute) mouse model is unavailable against a **gamescope** host, which grants
    relative input only. The switch chord silently does nothing there; the session stays captured.
9. Slipstream's GameStream server decodes only the classic multi-controller event and Sunshine's
   `CONTROLLER_ARRIVAL`; the extension packets that would carry pad motion, touchpad contacts and
   trigger effects are not implemented, so none of it reaches the host on this plane.
10. The host understands Moonlight's pen and touch extensions and feeds them through the same
   injection path, so Moonlight on a tablet with a stylus really does draw. Whether your Moonlight
   app sends them is up to that app.
11. Forwarded, but pressure and contact area are dropped on the way in.

Slipstream keyboard chords, and what each host backend can and cannot inject (including committed
text from an IME), are covered in [Input](/docs/input).

### Picture and sound while you stream

| Client | HDR | 4:4:4 | Surround 5.1 / 7.1 | Microphone | Clipboard | Stats overlay |
|---|---|---|---|---|---|---|
| Android · Android TV | ⚠️ ¹ | ❌ | ✅ ² | ✅ | ⚠️ ³ | ✅ |
| Steam Deck (Decky) | ✅ ⁴ | ❌ ⁵ | ✅ ² | ✅ | ❌ ⁶ | ✅ |
| Moonlight | ⚠️ ⁷ | ❌ | ✅ ² | ❌ | ❌ | ❓ ⁸ |

1. Runtime-probed against your actual display, see note 2 under [Client decode](#client-decode).
2. Requested by the client and **resolved by the host**: it clamps to what it can actually capture
   (2, 6 or 8 channels) and tells you the real answer before the first frame.
3. Text only, by design. Android's per-host **Shared clipboard** switch
   starts **on** (other clients default it off). Nothing crosses until the host also enables its
   clipboard, but the client-side consent is pre-granted.
4. Advertised whenever the HDR setting is on, which it is by default. No display probe: the Deck
   asks, and the host decides. Presented on a real HDR10 surface where gamescope offers one, and
   tone-mapped in-shader otherwise. Software-decoded frames never take the HDR surface.
5. The session settings still show a **Full chroma (4:4:4)** switch, and it is a per-profile field,
   but the session binary never advertises the 4:4:4 capability, so the switch has no effect today
   and the stream stays 4:2:0.
6. There is a "Share clipboard" switch, but the Linux side of the bridge is a stub; nothing is
   ever offered or applied. Treat the clipboard as unavailable on Steam Deck until this lands.
7. Decided entirely by the host and layered into what Moonlight is offered.
8. Moonlight has its own overlay; [stats](/docs/stats) here describes Slipstream's.

**File transfer through the clipboard does not exist yet** on any client. The wire format and the
host-side policy for it are in place, but no client offers files, so a copied file never crosses.
See [Clipboard](/docs/clipboard).

### Things both ends have to agree on

These are negotiated, and either side can be the reason it did not happen:

- **Shared clipboard**, off on the host until an operator enables it, and unavailable on a Linux
  host without the clipboard protocol (a gamescope session, for instance). The per-host "Share
  clipboard" switch is edited before you connect, so it is never greyed out; on Android and the
  Android add-host sheet it stays settable against a host that will refuse, and just does nothing.
- **Pen input**, the host advertises it only if it can really inject: a usable `/dev/uinput` on
  Linux. Without it, clients fold pen into touch.
- **Committed text (IME)**, only the sway/wlroots backend can type arbitrary text. On KDE, GNOME
  and gamescope, sessions fall back to key synthesis, which cannot express every character.
- **Client-drawn cursor**, needs the client to be in desktop mouse mode *and* a host capture path
  that carries the cursor separately. gamescope never can. When both agree, the host stops drawing
  the pointer into the video.
- **10-bit, HDR and 4:4:4**, see [HDR](/docs/hdr) for the full four-link chain.

## How finished each part is

The tables above say what a part *can do*. This one says how settled it is: maturity, not
capability.

| Part | Where it stands |
|---|---|
| **Protocol core**, `slipstream-core`, the C ABI, FEC and crypto | Stable. Both the wire format and the embeddable C surface are versioned contracts (see below) and are changed reluctantly. |
| **Linux host** | The product host surface. What differs is not the host but the desktop under it: each compositor gets its own capture, virtual-display and input backend, and they are not equally capable. |
| **GameStream / Moonlight plane** | Works, and whether it is on depends on how you installed. Every Linux package (deb, RPM, Arch, the Bazzite sysext) and the SteamOS installer ship the unit as `serve --gamestream`, so GameStream is **on** there; NixOS defaults it on too. A bare `slipstream-host serve` is off. It pairs over plain HTTP with weaker legacy encryption, trusted LAN only, and worth turning off if you don't use Moonlight (see [Security](/docs/security#gamestream--moonlight-compatibility-is-the-weak-crypto-path)). It is a compatibility surface, so Slipstream-only features (profiles, links, clipboard, microphone) are not on it. |
| **Android client** (phone · TV) | Distributed on Play's **closed (alpha)** track for releases, Internal testing for canary, plus a sideloadable APK. The same app in leanback mode is the TV client. |
| **Decky plugin** (Steam Deck) | Ships through install-from-URL rather than the Decky store, and keeps itself and the client it launches up to date. It launches the Linux session binary rather than streaming itself, and has no settings surface of its own beyond the flat values it writes into the shared client settings. |
| **Web console** | The full management surface: dashboard and sessions, pairing, library, displays, plugins and the plugin store, logs, stats, settings, and host updates. It cannot yet run a speed test or set a bitrate; the client apps can. |
| **Plugins** | First-party ones (ROM Manager, VirtualHere) plus the SDK, installed from the console. See [Plugins](/docs/plugins). |
| **`ss-webos`** (LG TV) | A community client in a separate repository. Nothing here can establish its state; ask that project. |

## Mixing versions

Update a host and its clients whenever it suits you. They interoperate across versions, and
anything already paired stays paired. Separate contracts make that true, each versioned on its own
so that adding a client-side feature never locks a client out of a deployed host:

| Contract | Current | What it governs |
|---|---|---|
| `slipstream/1` wire version | **2** | The `Hello`/`Welcome` handshake and the session planes. Hosts equality-check it, so this is the one that must match. |
| C ABI version | **13** | The embeddable C surface a client links against. It grows far more often than the wire does. |

Everything newer than a peer is **negotiated**: a capability the other end never heard of is simply
not offered, and the session proceeds without it rather than failing. That is why a feature can be
present in both apps and still not appear, see [Things both ends have to agree
on](#things-both-ends-have-to-agree-on).

## Where it has been run

Day-to-day development and validation happen on Linux hosts across the KWin, Mutter, gamescope and
wlroots backends, on NVIDIA and AMD GPUs, with the Android and Steam Deck clients, plus
stock Moonlight over the GameStream path. HDR on gamescope is verified end to end on Bazzite and on
SteamOS.

Cross-machine latency figures are trustworthy because a wall-clock handshake removes the clock
offset between the two machines before anything is measured, so a capture-to-receipt number is valid
across the LAN rather than only on one box. (The remaining term, receipt to actually on screen, is
what the current measurements do not cover is the final display-present step. See
[Understanding the stats overlay](/docs/stats).

## What is not verified

The honest counterpart to the list above. Some of these are ❓ cells; more of them are ⚠️ cells
whose caveat *is* "nobody has run this on real hardware". A wrong ✅ is worse than either.

- **Hyprland's virtual output on real display hardware.** Every contract is verified against
  Hyprland 0.55.4 and the code reports a clear error rather than a black stream, but no one has run
  create-output -> negotiate -> first frame on a physical Hyprland box. One session log settles it.
- **Client-drawn cursor on sway/wlroots and Hyprland.** Wired, never confirmed on glass. A single
  cursor-forwarding session on each, checking the pointer arrives and is not drawn twice, settles
  it.
- **gamescope headless capture on the proprietary NVIDIA driver.** Plausible by architecture, not a
  well-trodden path, and there is no probe that would catch it failing. One spawn-and-capture run on
  an NVIDIA box settles it.
- **The `ss-webos` LG TV client.** A community project in another repository. Its codecs, HDR
  behaviour and feature set cannot be established from here.
- **Everything client-side about Moonlight.** Wake-on-LAN, overlays, updates and which extensions
  it sends differ per Moonlight flavour. The honest answer is "ask your Moonlight app".
- **HDR when mirroring a real monitor on KDE, sway or Hyprland.** By construction it is SDR. Only
  gamescope's virtual output and the GNOME portal mirror carry HDR, but nothing states that as a
  deliberate decision rather than a gap.

Where this page and another page disagree, this one is the one that was checked against the code.
The unverified cases are listed below.
