# Embedding slipstream-core (the C ABI)

This guide is for developers who want to build their **own** slipstream client — on a platform we
don't ship an app for — by linking `slipstream-core` through its stable C ABI. It covers what the
core does (and, importantly, what it doesn't), how to build and link it, the full client lifecycle,
and worked integration blueprints for **webOS** and **Tizen**.

The authoritative header is [`include/slipstream_core.h`](../include/slipstream_core.h) — it is
generated from Rust by cbindgen and every symbol carries a doc comment. This guide is the narrative;
the header is the contract.

---

## 1. What the core gives you — and what it doesn't

`slipstream-core` is the *one* implementation of the wire format, shared by the host and every
first-party client. When you embed it you get the entire network side of a client for free:

**The core does:**

- The `slipstream/1` handshake (Hello/Welcome/Start) over a **QUIC control plane**.
- The **UDP video data plane**: packetization, reassembly, pacing, socket tuning.
- **Forward error correction** (GF(2⁸) Reed–Solomon and GF(2¹⁶) Leopard-RS) — loss recovery.
- **AES-128-GCM** session crypto, **cert pinning / TOFU**, and **SPAKE2 PIN pairing**.
- **Clock synchronization** (host↔client offset for glass-to-glass latency math).
- Delivery of every plane: reassembled **video access units**, **Opus audio** (raw or decoded to
  PCM in-core), **rumble**, **DualSense HID output**, **HDR metadata**, **host timing**.
- The uplink: **input events**, **mic**, **rich input** (touchpad/motion).
- Loss-recovery signalling: keyframe requests and **reference-frame invalidation** (RFI).

**The core does NOT do (this is your job):**

- **Video decoding.** The core hands you encoded access units (H.264, HEVC, or AV1 NAL units /
  OBUs). You feed them to the platform's hardware decoder.
- **Rendering / presentation.** You own the swapchain / surface and put decoded frames on glass.
- **Audio output.** You own the audio sink; the core only delivers Opus (or f32 PCM).
- **Input capture / controller enumeration.** You read the platform's input and translate it into
  `SlipstreamInputEvent`s.
- **UI, discovery UX, settings.** (mDNS discovery is not part of the C ABI; see §4.)

Think of it as: **the core is the modem and the codec-agnostic protocol brain; you are the TV and
the remote.**

```
                          ┌─────────────────────────── slipstream-core (C ABI) ───────────────────────────┐
   your platform          │                                                                              │      slipstream host
  ┌───────────┐  input    │  slipstream_connection_send_input / _send_mic / _send_rich_input  ───────────────▶
  │  remote / │──────────▶│                                                                              │
  │  gamepad  │           │                          QUIC control + UDP data + FEC + AES-GCM             │
  └───────────┘           │                                                                              │
  ┌───────────┐  AUs      │  ◀── slipstream_connection_next_au        (H.264 / HEVC / AV1 access units)   │◀─── encoder
  │ HW decoder│◀──────────│  ◀── slipstream_connection_next_audio(_pcm)(Opus / f32 PCM)                   │
  │ + display │           │  ◀── slipstream_connection_next_rumble2 / _next_hidout / _next_hdr_meta        │
  └───────────┘           └──────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Building and linking the library

### 2.1 The `quic` feature is mandatory for clients

The entire connection API (`slipstream_connect*`, `slipstream_connection_*`, pairing, probe) is
gated, in both Rust and the C header, behind the **`quic`** Cargo feature. Everything under
`#if defined(SLIPSTREAM_FEATURE_QUIC)` in the header only exists when the core was built with it.

> **Two things must line up or nothing links:**
> 1. Build the library with `--features quic`.
> 2. Compile your C/C++ with `-DSLIPSTREAM_FEATURE_QUIC` so the header declares those prototypes.

The non-QUIC surface (`slipstream_session_new`, `slipstream_host_submit_frame`,
`slipstream_client_poll_frame`, the loopback test pair) is the **raw transport** used by the host and
by tests. **Client embedders do not use it** — you use the `slipstream_connect*` family.

### 2.2 Build outputs

```sh
# Native (host machine) — good for a desktop prototype
cargo build -p slipstream-core --features quic --release
```

`crate-type = ["lib", "cdylib", "staticlib"]`, so one build produces all three:

| Output      | File (per platform)                                             | Use when |
|-------------|----------------------------------------------------------------|----------|
| `cdylib`    | `libslipstream_core.so` / `.dylib`                                 | dynamic linking (most TV apps, sandboxed apps) |
| `staticlib` | `libslipstream_core.a`                                             | static embedding into a single binary |

The header lands at `include/slipstream_core.h` (regenerated on build; also checked in).

### 2.3 Cross-compiling for your device

Every target below is a standard Rust target. Add it and point Cargo at the platform sysroot/linker.

```sh
rustup target add aarch64-unknown-linux-gnu    # most ARM Smart TVs (webOS, Tizen)
rustup target add armv7-unknown-linux-gnueabihf # older 32-bit TV SoCs

# Tell Cargo which cross-linker + sysroot to use (example: aarch64 TV NDK)
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
cargo build -p slipstream-core --features quic --release \
    --target aarch64-unknown-linux-gnu
```

The QUIC tree is deliberately **ring-only** (no aws-lc-rs / no cmake), so cross builds do not need a
C crypto toolchain — this is what keeps the TV cross-compiles simple. `libopus` (for the in-core PCM
decode path) is the only C dependency and builds with the target `CC`.

### 2.4 Compiling and linking your app

```sh
# compile
cc -std=c11 -DSLIPSTREAM_FEATURE_QUIC -I path/to/include -c myclient.c

# link, dynamic
cc myclient.o -L target/aarch64-unknown-linux-gnu/release -lslipstream_core -o myclient

# link, static — you also need the native libs rustc pulls in
NATIVE=$(cargo rustc -p slipstream-core --features quic --release \
         --target aarch64-unknown-linux-gnu \
         --crate-type staticlib -- --print native-static-libs 2>&1 \
         | sed -n 's/.*native-static-libs: //p' | tail -1)
cc myclient.o target/aarch64-unknown-linux-gnu/release/libslipstream_core.a $NATIVE -o myclient
```

`--print native-static-libs` is the reliable way to discover the platform libs (`-lpthread`,
`-lm` and any other system libraries a static link needs. The C harness at
[`crates/slipstream-core/tests/c/run.sh`](../crates/slipstream-core/tests/c/run.sh) does exactly this
and is your smallest end-to-end reference.

### 2.5 Version check first, always

```c
#include "slipstream_core.h"

if (slipstream_abi_version() != ABI_VERSION) {
    // The .so you loaded is a different core than this header was generated from. Abort.
}
```

`ABI_VERSION` (the embeddable C surface) is distinct from `WIRE_VERSION` (what the handshake
carries). The ABI can grow — new `connect_ex` variants, new planes — without touching the wire, so a
newer client keeps talking to a deployed host. Only equal `WIRE_VERSION`s interoperate on the wire;
the core enforces that inside the handshake.

---

## 3. Core concepts

### 3.1 The connection handle

`SlipstreamConnection *` is an **opaque handle to one live client session** — QUIC control plane plus
UDP data plane, with all the I/O pumped on threads the core owns internally. You obtain it from a
`slipstream_connect*` call and release it with `slipstream_connection_close`.

### 3.2 The threading contract (read this before you design your loops)

The planes are **independent single-consumer queues**. The rule:

> Each plane (video, audio, rumble, HID-out, HDR, host-timing) may be pulled from **its own thread,
> at most one thread per plane**. Different planes may be pulled concurrently. Never pull the *same*
> plane from two threads.

A typical client runs **three threads**:

- **Video thread** — blocks on `slipstream_connection_next_au`, decodes, presents.
- **Audio thread** — blocks on `slipstream_connection_next_audio_pcm` (or `_next_audio`).
- **Feedback thread** — pulls rumble / HID-out / HDR-meta (all low-rate; poll or short-timeout).

Input is sent from whatever thread reads your input (send calls are non-blocking enqueues and are
safe to call from any thread).

### 3.3 Borrowed memory

`next_au`, `next_audio`, `next_audio_pcm` return a struct whose `data`/`samples` pointer **borrows
core-owned memory that is valid only until your next pull on that same handle/plane.** Decode or copy
before you call again. Cross-plane pulls do not invalidate each other (the audio pull won't free the
video buffer).

### 3.4 Status codes

Every fallible call returns `SlipstreamStatus`: `SLIPSTREAM_STATUS_OK == 0`, all errors negative
(`rc < 0`). The two you branch on constantly in the pull loops:

- `SLIPSTREAM_STATUS_NO_FRAME` (-5) — nothing ready within `timeout_ms`; loop again.
- `SLIPSTREAM_STATUS_CLOSED` (-10) — the session ended; tear down and leave the loop.

---

## 4. Identity, pairing, discovery, wake

### 4.1 Generate a persistent identity once

```c
char cert[4096], key[4096];
if (slipstream_generate_identity(cert, sizeof cert, key, sizeof key) != SLIPSTREAM_STATUS_OK) { /* … */ }
// Persist BOTH strings securely (platform secure storage / keychain).
// The certificate's SHA-256 is how a host recognizes this client after pairing.
```

Do this **once per device/install** and store the PEM strings. Pass them to every `pair` and
`connect` call. Anonymous (`NULL`/`NULL`) sessions are rejected by hosts running `--require-pairing`.

### 4.2 Pair (PIN ceremony)

```c
uint8_t host_fp[32];
SlipstreamStatus rc = slipstream_pair(host, port, cert, key,
                                    user_typed_pin, "Living Room TV",
                                    host_fp, /*timeout_ms=*/10000);
// rc == SLIPSTREAM_STATUS_CRYPTO  => wrong PIN.
// rc == SLIPSTREAM_STATUS_OK      => persist host_fp; it's the pin for future connects.
```

The host shows a PIN; the user types it into your UI. On success you get the host's verified
fingerprint — store it keyed by host, and pass it as `pin_sha256` on every later connect (that pins
the host and defeats MITM). For a first connect without prior pairing you may pass `NULL` for the pin
and capture `observed_sha256_out` (trust-on-first-use), then pin it thereafter.

### 4.3 Reachability probe and Wake-on-LAN

- `slipstream_probe(host, port, timeout_ms)` — a bounded, trust-agnostic handshake attempt. Returns
  `OK` if the host answered (drives "online" pips), `TIMEOUT` otherwise. Works over routed
  networks (Tailscale/VPN) where mDNS never reaches. Call off the UI thread.
- `slipstream_wake_on_lan(macs, mac_count, last_known_ip)` — sends WoL magic packets. The host's wake
  MAC(s) arrive out-of-band via the mDNS `mac` TXT record, so no connection is needed to wake a
  sleeping host.

> **Discovery is out of scope for the C ABI.** mDNS/DNS-SD browsing is the embedder's job (use the
> platform's Bonjour/Avahi/`nsd` API to find `_slipstream._udp` hosts, or let the user type an
> IP/hostname). The core only *connects*.

---

## 5. Connecting

There is a **ladder** of `connect` variants; each adds parameters and defaults the rest to the prior
variant's behavior. Use the highest one you need — they all return `SlipstreamConnection *` (NULL on
failure) and block up to `timeout_ms` for the handshake, so **call off your UI thread.**

| Variant          | Adds |
|------------------|------|
| `slipstream_connect`     | base: `width`, `height`, `refresh_hz`, pin, identity |
| `slipstream_connect_ex`  | `compositor` preference (Linux hosts) |
| `slipstream_connect_ex2` | `gamepad` backend (DualSense / Steam Deck / …) |
| `slipstream_connect_ex3` | `bitrate_kbps` |
| `slipstream_connect_ex4` | `launch_id` (auto-launch a library title) |
| `slipstream_connect_ex5` | `video_caps` (10-bit / HDR / 4:4:4 / host-timing) |
| `slipstream_connect_ex6` | `audio_channels` (2 / 6 / 8) |
| **`slipstream_connect_ex7`** | **`video_codecs` + `preferred_codec`** — the recommended full call |

```c
uint8_t caps   = 0;                       // add SLIPSTREAM_VIDEO_CAP_10BIT | _HDR | _444 as supported
uint8_t codecs = SLIPSTREAM_CODEC_HEVC;    // OR-in _H264 (needed for software hosts) / _AV1
uint8_t host_fp[32]; uint8_t observed[32];

SlipstreamConnection *c = slipstream_connect_ex7(
    "192.168.1.50", 9777,
    1920, 1080, 60,                        // mode
    SLIPSTREAM_COMPOSITOR_AUTO,
    SLIPSTREAM_GAMEPAD_DUALSENSE,
    /*bitrate_kbps=*/20000,
    caps,
    /*audio_channels=*/2,
    codecs, /*preferred_codec=*/SLIPSTREAM_CODEC_HEVC,
    /*launch_id=*/NULL,
    host_fp,                               // pin (or NULL for TOFU)
    observed,                              // filled with the observed fp
    cert, key,                             // identity
    /*timeout_ms=*/8000);
if (!c) { /* handshake failed */ }
```

> **Advertise only what you can actually decode and present.** The host upgrades to 10-bit / HDR /
> 4:4:4 / AV1 **only** when you set the matching bit *and* it opted in. Setting a cap you can't honor
> gives you a stream you can't render.

### 5.1 Read the resolved session (right after connect)

The host echoes what it actually chose. Build your decoder and present path from **these**, never
from your request:

```c
uint8_t codec, prim, trc, mtx, full, depth, chroma, ch;
slipstream_connection_codec(c, &codec);                       // SLIPSTREAM_CODEC_{H264,HEVC,AV1}
slipstream_connection_color_info(c, &prim, &trc, &mtx, &full, &depth); // CICP + bit depth
slipstream_connection_chroma_format(c, &chroma);              // 1 = 4:2:0, 3 = 4:4:4
slipstream_connection_audio_channels(c, &ch);                 // 2 / 6 / 8

int64_t clk = 0; slipstream_connection_clock_offset_ns(c, &clk); // host−client ns, for latency math

uint32_t w, h, hz; slipstream_connection_mode(c, &w, &h, &hz);
```

`trc == 16` (PQ) or `18` (HLG) means an **HDR** session — set up an HDR present path and drain
`slipstream_connection_next_hdr_meta`. On `SLIPSTREAM_COMPOSITOR_GAMESCOPE`
(`slipstream_connection_compositor`) draw a client-side cursor by default (that capture carries none).

---

## 6. The video loop

The core delivers **complete, in-order access units** with in-band parameter sets. The first AU is
an IDR — build your decoder from it (and from the resolved codec/color above). The stream is then an
**infinite-GOP** of P-frames: no periodic IDRs, so loss recovery is explicit and is the part you must
get right.

```c
SlipstreamFrame f;
for (;;) {
    SlipstreamStatus rc = slipstream_connection_next_au(c, &f, /*timeout_ms=*/20);
    if (rc == SLIPSTREAM_STATUS_NO_FRAME) continue;
    if (rc == SLIPSTREAM_STATUS_CLOSED)   break;
    if (rc != SLIPSTREAM_STATUS_OK)       continue;

    // (1) Loss recovery — let the core do the hard part. Call this EVERY frame:
    bool gap = false;
    slipstream_connection_note_frame_index(c, f.frame_index, &gap);
    //   On a forward gap it fires a throttled RFI request for you (clean P-frame recovery on
    //   AMD-LTR/NVENC hosts, no IDR spike). `gap == true` is your cue to freeze on the last good
    //   picture until re-anchor (see (3)).

    // (2) Clean re-anchor points (infinite-GOP has no periodic keyframes):
    bool recovery_point  = f.flags & USER_FLAG_RECOVERY_POINT;  // intra-refresh wave boundary
    bool recovery_anchor = f.flags & USER_FLAG_RECOVERY_ANCHOR; // definitive LTR/RFI re-anchor

    // (3) Decode + present. f.data/f.len are valid until the next next_au call.
    decode_and_present(f.data, f.len, f.pts_ns);
    //   If you implement freeze-until-reanchor: while frozen, keep redrawing the last good frame
    //   and lift the freeze on a real keyframe, on the FIRST recovery_anchor, or the SECOND
    //   recovery_point after the gap.

    // (4) Backstop trigger: poll the reassembler's unrecoverable-drop count. Under infinite GOP,
    //     unrecoverable loss yields reference-missing deltas the decoder *silently conceals*
    //     (frozen/garbage, no decode error) — so this counter, not a decode error, is the signal.
    uint64_t dropped = 0; slipstream_connection_frames_dropped(c, &dropped);
    if (dropped > last_dropped) {           // THROTTLE this (≤ ~1/100ms) — see below
        slipstream_connection_request_keyframe(c);
        last_dropped = dropped;
    }
}
```

**Recovery, in priority order:**

1. `note_frame_index` every frame — the cheapest, earliest signal. It issues a **throttled RFI**
   request (`request_rfi`) for the exact lost range so RFI-capable hosts recover with a clean
   P-frame (no 20–40× IDR bandwidth spike).
2. `frames_dropped` climbing → `request_keyframe` as the backstop when RFI can't help or the recovery
   frame itself was lost.
3. A wedged decoder (received AUs but no decoded output for several frames) → `request_keyframe`.

**Throttle** both `request_keyframe` and `request_rfi` — decode stays wedged for several frames until
recovery lands, so one request per ~100 ms is right; requesting per frame floods the control stream.
If you prefer not to hand-roll this, `note_frame_index` + a throttled `frames_dropped`→keyframe check
is a complete, correct recovery policy on its own.

### 6.1 Mid-session resolution / refresh changes

```c
slipstream_connection_request_mode(c, new_w, new_h, new_hz);
```

Non-blocking. On acceptance the next AU is a fresh IDR with in-band parameter sets — **rebuild your
decoder from it** — and `slipstream_connection_mode` reflects the switch. Use this when your window
resizes or the display refresh changes.

---

## 7. Audio

Two mutually-exclusive ways to consume audio — pick one, on one dedicated thread:

- **`slipstream_connection_next_audio_pcm`** — the core decodes to interleaved **f32 PCM** at 48 kHz
  in channel order `FL FR FC LFE RL RR SL SR`. Use this if you lack a *multistream*-capable Opus
  decoder (e.g. Apple's AudioToolbox is stereo-only; many TV audio stacks are too). Simplest path.
- **`slipstream_connection_next_audio`** — raw Opus packets (5 ms frames). Use only if you have a
  multistream Opus decoder and build it from `slipstream_connection_audio_channels` (see
  `audio::layout_for`). Do **not** mix the two on one connection.

```c
SlipstreamAudioPcm a;
for (;;) {
    SlipstreamStatus rc = slipstream_connection_next_audio_pcm(c, &a, /*timeout_ms=*/100);
    if (rc == SLIPSTREAM_STATUS_CLOSED) break;
    if (rc != SLIPSTREAM_STATUS_OK)     continue;
    // a.samples = a.frame_count * a.channels f32s, valid until the next PCM call.
    audio_sink_write(a.samples, a.frame_count, a.channels);
}
```

---

## 8. Input (uplink)

Fill a `SlipstreamInputEvent` and send it — non-blocking, from any thread:

```c
SlipstreamInputEvent ev; memset(&ev, 0, sizeof ev);
```

Field meaning depends on `kind` (`SlipstreamInputKind`). The contracts that trip people up:

- **Absolute pointer / touch** (`MOUSE_MOVE_ABS`, `TOUCH_DOWN/MOVE`): `x`/`y` are pixel coordinates
  and **`flags` must pack your coordinate space as `(width << 16) | height`** — the host normalizes
  against it. A **zero `flags` is dropped**, so always set it.
- **Gamepad button** (`GAMEPAD_BUTTON`): `code` = a `SLIPSTREAM_BTN_*` bit, `x != 0` = pressed,
  `flags` = pad index (0..15).
- **Gamepad axis** (`GAMEPAD_AXIS`): `code` = `SLIPSTREAM_AXIS_*`, `flags` = pad index. Sticks are
  **i16 (−32768..32767), +y = up** (opposite of screen coordinates); triggers are 0..255.
- **Relative mouse** (`MOUSE_MOVE`): `x`/`y` carry `dx`/`dy`. **Scroll** (`MOUSE_SCROLL`): `x` is the
  signed delta.

```c
// Example: press A on pad 0
ev.kind = SLIPSTREAM_INPUT_KIND_GAMEPAD_BUTTON;
ev.code = SLIPSTREAM_BTN_A; ev.x = 1; ev.flags = 0;
slipstream_connection_send_input(c, &ev);

// Example: absolute touch at (640,360) on a 1280x720 surface
ev.kind = SLIPSTREAM_INPUT_KIND_TOUCH_DOWN;
ev.code = 0 /*finger id*/; ev.x = 640; ev.y = 360;
ev.flags = (1280u << 16) | 720u;
slipstream_connection_send_input(c, &ev);
```

> Gamepad state on the C ABI uses **per-transition** button/axis events (above). The wire also has an
> idempotent full-pad *snapshot* mode (`HOST_CAP_GAMEPAD_STATE`), but it is not exposed as a
> dedicated C entry point — per-transition events are accepted by every host and are the C path.

**Other uplinks (all non-blocking):**

- `slipstream_connection_send_mic(c, opus, len, seq, pts_ns)` — Opus mic frames (you encode).
- `slipstream_connection_send_rich_input(c, &rich)` — DualSense touchpad contact / motion sample.
- `slipstream_connection_send_rich_input2(c, &richEx)` — the forward-compatible superset (Steam
  trackpads, signed coords, pressure); set `struct_size = sizeof(SlipstreamRichInputEx)`.

### Stylus / pen

Full-fidelity pen (pressure, tilt, azimuth, barrel roll, hover, eraser, barrel buttons) has its
own plane. **Gate on the capability first** — only send when
`slipstream_connection_host_caps(c, &caps)` shows `SLIPSTREAM_HOST_CAP_PEN`; without it the call
returns `UNSUPPORTED` and you keep your pen-as-touch fallback (what every client does today).

Samples are **state-full**: every sample carries the complete pen state (in-range / touching /
buttons in `state`, plus all axes — unknown axes take their `SLIPSTREAM_PEN_*_UNKNOWN` sentinel,
never 0). There are no down/up events to send; the host diffs consecutive samples and
synthesizes the transitions, so a lost datagram self-heals on the next one. Batch a capture
callback's coalesced samples (oldest first, `dt_us` = spacing) into one call, at most
`SLIPSTREAM_PEN_BATCH_MAX` per call — split longer runs into consecutive calls.

```c
// Example: an in-contact Apple Pencil sample, mapped into video-frame space
SlipstreamPenSample s; memset(&s, 0, sizeof s);
s.state = SLIPSTREAM_PEN_IN_RANGE | SLIPSTREAM_PEN_TOUCHING;
s.tool = SLIPSTREAM_PEN_TOOL_PEN;
s.x = 0.5f; s.y = 0.5f;                       // normalized 0..1 across the video frame
s.pressure = (uint16_t)(force_norm * 65535);  // 0 while hovering
s.tilt_deg = 90 - altitude_deg;               // polar tilt-from-normal
s.azimuth_deg = azimuth_deg;                  // 0..359, clockwise from north
s.roll_deg = SLIPSTREAM_PEN_ANGLE_UNKNOWN;     // Pencil Pro: rollAngle in degrees
s.distance = SLIPSTREAM_PEN_DISTANCE_UNKNOWN;
slipstream_connection_send_pen(c, &s, 1);
```

While hovering, send `SLIPSTREAM_PEN_IN_RANGE` samples (with `distance` if you have it); when the
pen leaves range, send one final sample with `state = 0` so the host releases proximity.

**Heartbeat**: while the pen is in range or touching, repeat the last sample at least every
~100 ms even when nothing changed — pen capture APIs are silent for a stationary pen, and the
host force-releases the stroke after 200 ms without samples (its dead-client failsafe). Run a
timer alongside your touch callbacks.

---

## 9. Feedback planes (rumble, HID, HDR)

Pull these on your feedback thread (or poll with `timeout_ms = 0`). Same
`NO_FRAME`/`CLOSED` semantics as everywhere.

- **Rumble** — `slipstream_connection_next_rumble2(c, &pad, &low, &high, &ttl_ms, timeout)`.
  Amplitudes 0..0xFFFF; `(0,0)` = stop. `ttl_ms` is a host-supplied self-terminating lease — render
  the level for that long unless renewed; `SLIPSTREAM_RUMBLE_NO_TTL` means fall back to your own
  staleness timeout. (The v1 `_next_rumble` drops the TTL — prefer v2.)
- **DualSense HID output** — `slipstream_connection_next_hidout(c, &out, timeout)`. `out.kind` selects
  lightbar RGB / player LEDs / adaptive-trigger effect / trackpad haptic. Replay on a real DualSense
  via the platform's controller API. Only a DualSense-backend session emits these.
- **HDR metadata** — `slipstream_connection_next_hdr_meta(c, &meta, timeout)`. ST.2086 mastering
  display + content light level, in HDR10 SEI fixed-point units. Pass it to the
  display API used by the client, such as Apple `CAEDRMetadata` or Android
  `KEY_HDR_STATIC_INFO`. Only an HDR
  session emits these; apply the latest to your display.
- **Host timing** (optional, if you advertised `VIDEO_CAP_HOST_TIMING`) —
  `slipstream_connection_next_host_timing` gives the host's capture→sent µs per AU, so your stats HUD
  can split `host` vs `network` latency.

---

## 10. Speed test and stats

```c
slipstream_connection_speed_test(c, /*target_kbps=*/50000, /*duration_ms=*/2000); // pauses video briefly
SlipstreamProbeResult r;
do { slipstream_connection_probe_result(c, &r); } while (!r.done);  // poll
// r.throughput_kbps drives a bitrate choice; r.loss_pct vs r.host_drop_pct
// distinguishes a lossy link from a host that can't keep up.
```

---

## 11. Teardown

```c
slipstream_connection_disconnect_quit(c);  // user pressed "stop": host tears down now, no linger
slipstream_connection_close(c);            // joins internal threads, frees the handle
```

Call `disconnect_quit` **only** on a deliberate user quit — it makes the host drop the virtual
display immediately. On a network drop / backgrounding, skip it (a plain `close`) so the host lingers
and a reconnect can resume. After `close`, the handle is dead; stop all your plane threads first.

---

# Platform integration blueprints

Everything above is exact — it's the slipstream side, which is identical on every platform. The
sections below are **blueprints**: they name the real decode/present/input subsystems on each target
and show where the slipstream glue plugs in. Treat the platform-SDK calls as illustrative — confirm
signatures against that platform's current SDK. The pattern is always the same:

> **connect → build HW decoder from the resolved codec/color → three threads (video decode+present,
> audio, feedback) → translate native input into `SlipstreamInputEvent`s.**

---

## 12. webOS (LG Smart TV)

**App model.** webOS ships both web apps and **native** apps via the **webOS NDK** (Linux, ARM —
usually `aarch64`, some older panels `armv7`). A low-latency streaming client wants the native path:
you get real threads, a hardware video pipeline, and SDL for window/input. (This mirrors how the
community Moonlight webOS client is built.)

**Cross-compile the core.** Build `libslipstream_core.so` with the webOS NDK sysroot:

```sh
rustup target add aarch64-unknown-linux-gnu
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=$WEBOS_NDK/.../aarch64-webos-linux-gnu-gcc
export CC_aarch64_unknown_linux_gnu=$WEBOS_NDK/.../aarch64-webos-linux-gnu-gcc
cargo build -p slipstream-core --features quic --release --target aarch64-unknown-linux-gnu
```

Bundle the `.so` in your `.ipk` alongside the header-compiled client.

**Video decode + present.** Use the webOS media pipeline for the SoC's hardware decoder — historically
**NDL-DirectMedia** and on newer panels the **Starfish** media pipeline
(`com.webos.service.mediapipeline`). You push the slipstream AUs (Annex-B H.264/HEVC/AV1) into the
pipeline's `feed`/`push` entry point; it decodes and composits to the TV's video plane, and you draw
your overlay/UI on the graphics plane over it. Present cadence is driven by the pipeline, not you.

**Audio.** Simplest is `slipstream_connection_next_audio_pcm` → **PulseAudio** (webOS's audio server)
via a simple playback stream. Or route the raw Opus into the media pipeline if it accepts a
secondary audio ES.

**Input.** The Magic Remote / standard remote and Bluetooth gamepads surface as **SDL2** events (LG
ships SDL for NDK apps). Map remote keys to `KEY_DOWN/UP`, the pointer to `MOUSE_MOVE_ABS` (set
`flags = (w<<16)|h`), and `SDL_GameController` axes/buttons to `GAMEPAD_AXIS`/`GAMEPAD_BUTTON`.

**Skeleton:**

```c
// 1. connect (HEVC, stereo, SDR to start)
SlipstreamConnection *c = slipstream_connect_ex7(host, 9777, 1920,1080,60,
    SLIPSTREAM_COMPOSITOR_AUTO, SLIPSTREAM_GAMEPAD_DUALSENSE, 20000,
    /*caps=*/0, /*ch=*/2, SLIPSTREAM_CODEC_HEVC, SLIPSTREAM_CODEC_HEVC,
    NULL, host_fp, observed, cert, key, 8000);

uint8_t codec; slipstream_connection_codec(c, &codec);
StarfishPipeline *p = starfish_open(codec /*→ "video/hevc" etc*/, 1920,1080);

// 2. video thread
SlipstreamFrame f;
while (running) {
    if (slipstream_connection_next_au(c, &f, 20) != SLIPSTREAM_STATUS_OK) continue;
    bool gap; slipstream_connection_note_frame_index(c, f.frame_index, &gap);
    starfish_feed(p, f.data, f.len, f.pts_ns);          // HW decode + present
    uint64_t d; slipstream_connection_frames_dropped(c, &d);
    if (d > last_d) { slipstream_connection_request_keyframe(c); last_d = d; }  // throttle
}
// 3. audio thread → next_audio_pcm → pulse_write(...)
// 4. SDL input thread → fill SlipstreamInputEvent → slipstream_connection_send_input(c, &ev)
```

**Gotchas.** webOS app networking is permitted for UDP/QUIC, but background/suspend policy is
aggressive — pull-loop `CLOSED` on backgrounding and reconnect on resume. Video-plane / graphics-plane
z-order and scaling are set through the pipeline's display-window API, not by drawing pixels yourself.

---

## 14. Tizen (Samsung Smart TV)

**App model.** Use **Tizen Native** (C, EFL) — .NET/web apps can't get low-latency HW-decode feed
access cleanly. TVs are ARM (`aarch64` on modern panels, `armv7` on older). Build with the Tizen
Studio / GBS toolchain and its sysroot.

**Cross-compile the core.**

```sh
rustup target add aarch64-unknown-linux-gnu       # or armv7-unknown-linux-gnueabihf
export CC_aarch64_unknown_linux_gnu=$TIZEN_TOOLCHAIN/bin/aarch64-tizen-linux-gnu-gcc
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=$CC_aarch64_unknown_linux_gnu
cargo build -p slipstream-core --features quic --release --target aarch64-unknown-linux-gnu
```

Add the `.so` to your project's `lib/` and link with `-lslipstream_core`, compiling with
`-DSLIPSTREAM_FEATURE_QUIC`.

**Video decode + present.** Use **`capi-media-codec`** (`mediacodec_*`) — Tizen's hardware codec
binding over OpenMAX. Configure it from the resolved codec (`MEDIACODEC_H264`/`_HEVC`/AV1), wrap each
AU in a **`media_packet`** (`capi-media-tool`) and `mediacodec_process_input`; on the output callback
render the decoded packet to an **Evas GL** surface (or hand tunpacked frames to the TV video plane).
For HDR, drive the panel's HDR mode from `slipstream_connection_color_info` + `next_hdr_meta`.

**Audio.** `slipstream_connection_next_audio_pcm` (f32, 48 kHz) → **`capi-media-audio-io`**
(`audio_out_*`). Convert f32→s16 if you open the sink as PCM S16.

**Input.** TV remote and Bluetooth gamepads arrive as **Ecore** key events (`Ecore_Event_Key`).
Register key grabs (`elm_win_keygrab_set`) for the remote, translate D-pad/OK/back to
`KEY_DOWN/UP` or gamepad buttons, and map any HID gamepad to `GAMEPAD_AXIS`/`GAMEPAD_BUTTON`.

**Skeleton:**

```c
SlipstreamConnection *c = slipstream_connect_ex7(host, 9777, 1920,1080,60,
    SLIPSTREAM_COMPOSITOR_AUTO, SLIPSTREAM_GAMEPAD_DUALSENSE, 20000,
    /*caps=*/0, /*ch=*/2, SLIPSTREAM_CODEC_HEVC | SLIPSTREAM_CODEC_H264,
    SLIPSTREAM_CODEC_HEVC, NULL, host_fp, observed, cert, key, 8000);

uint8_t codec; slipstream_connection_codec(c, &codec);
mediacodec_h mc; mediacodec_create(&mc);
mediacodec_set_codec(mc, codec==SLIPSTREAM_CODEC_HEVC?MEDIACODEC_HEVC:MEDIACODEC_H264,
                     MEDIACODEC_DECODER | MEDIACODEC_SUPPORT_TYPE_HW);
mediacodec_set_output_buffer_available_cb(mc, on_decoded /*→ Evas GL present*/, NULL);
mediacodec_prepare(mc);

// video thread
SlipstreamFrame f;
while (running) {
    if (slipstream_connection_next_au(c, &f, 20) != SLIPSTREAM_STATUS_OK) continue;
    bool gap; slipstream_connection_note_frame_index(c, f.frame_index, &gap);
    media_packet_h pkt = wrap_au(f.data, f.len, f.pts_ns);   // capi-media-tool
    mediacodec_process_input(mc, pkt, 0);
    uint64_t d; slipstream_connection_frames_dropped(c, &d);
    if (d > last_d) { slipstream_connection_request_keyframe(c); last_d = d; }  // throttle
}
// audio thread: next_audio_pcm → audio_out_write(...)
// Ecore key handler: fill SlipstreamInputEvent → slipstream_connection_send_input(c, &ev)
```

**Gotchas.** Declare the `internet` and `network.get` privileges in `tizen-manifest.xml` or the QUIC
socket won't open. `mediacodec` prefers Annex-B start codes and periodic parameter sets — the
slipstream IDR carries in-band parameter sets, but if your SoC decoder wants an explicit
codec-config/CSD, extract VPS/SPS/PPS from the first IDR and feed it before the stream. On a
mid-session mode change (`request_mode`), tear down and re-prepare the `mediacodec` from the new IDR.

---

## 15. Checklist for a new port

- [ ] Build the core with `--features quic`; compile your app with `-DSLIPSTREAM_FEATURE_QUIC`.
- [ ] `slipstream_abi_version() == ABI_VERSION` at startup.
- [ ] Persist a generated identity; implement PIN pairing; pin the host fingerprint.
- [ ] Connect off the UI thread; **build the decoder from the *resolved* codec/color/chroma**, not
      your request.
- [ ] One thread per plane; never two on the same plane; decode/copy borrowed buffers before the next
      pull.
- [ ] `note_frame_index` every video frame; throttled `frames_dropped`→`request_keyframe` backstop.
- [ ] Rebuild the decoder on the IDR after any accepted `request_mode`.
- [ ] Set `flags = (w<<16)|h` on absolute pointer/touch events (nonzero!).
- [ ] `disconnect_quit` only on deliberate user quit; always `close` and stop plane threads on
      teardown.

## 16. Reference

- **Header (the contract):** [`include/slipstream_core.h`](../include/slipstream_core.h)
- **Minimal C link + round-trip proof:** [`crates/slipstream-core/tests/c/`](../crates/slipstream-core/tests/c/)
- **Core crate README:** [`crates/slipstream-core/README.md`](../crates/slipstream-core/README.md)
- **A full reference client (Rust, same ABI surface):** `crates/ss-client-core/src/session.rs`
