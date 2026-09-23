---
title: Windows (Host)
description: Run a Slipstream host on Windows. Capture and encode a monitor, or plug an indirect display, and stream it to the usual clients.
---

This is for using a **Windows PC as the host**, streaming *from* it. The Linux client, Android
app, and Steam Deck plugin are unchanged: they connect to this host the same way they connect to
a Linux one.

> New here? Read [Security & Safe Use](/docs/security) first. A streaming host is remote control of
> the machine, so keep it on a trusted LAN or VPN and require pairing.

There is no installer in this repository. Build `slipstream-host` and `slipstream-tray` on Windows
(MSVC or the GNU toolchain), then register the host with the Service Control Manager if you want
it running when you are not logged into a console.

## What this host does

| Path | What you get |
|---|---|
| Capture | Windows Graphics Capture, with DXGI as the fallback. The pointer is **embedded in the frame**. |
| Encode | NVIDIA NVENC through `nvEncodeAPI64.dll` (D3D11 BGRA textures) when the selected GPU is NVIDIA and the driver answers. Otherwise openh264 H.264. |
| Display | Mirror an existing monitor. An indirect display is optional and needs a driver you install yourself. |
| Audio | WASAPI loopback, plus a virtual microphone for client audio. |
| Input | `SendInput` for keyboard, mouse, and pen. A virtual gamepad is not wired up yet. |
| Console | The same web console as Linux, on port 47992. |

AMD AMF and Intel Quick Sync are not encode backends on Windows. `SLIPSTREAM_ENCODER=vaapi`,
`vulkan`, `pyrowave`, `amf`, or `qsv` fails at open instead of silently picking another encoder.

## Build

From a Windows checkout, with the pinned toolchain in `rust-toolchain.toml`:

```sh
cargo build --release -p slipstream-host -p slipstream-tray
```

The Windows host always compiles the NVENC backend. `nvEncodeAPI64.dll` is loaded at runtime, so
the same binary runs on AMD and Intel: those GPUs use the software H.264 encoder.

A Linux CI job cross-checks this crate graph with `x86_64-pc-windows-gnu`. That job does not open
a GPU or a display.

## Config and the service

Config, pairing state, and the console password live in `%APPDATA%\slipstream`
(`SLIPSTREAM_CONFIG_DIR` overrides that).

The service name is `slipstream-host`. The tray looks that name up in the Service Control Manager
and can start or stop it. Register it once, from an elevated prompt, with the path to the binary
you built:

```bat
sc create slipstream-host binPath= "C:\path\to\slipstream-host.exe" start= auto
sc start slipstream-host
```

`slipstream-host.exe` with no arguments tries the service dispatcher first. Started by the SCM, it
runs the default `serve` plane (management API and slipstream/1, GameStream off). Started from a
console, the dispatcher declines and the normal CLI runs, so `slipstream-host serve` and
`slipstream-host serve --gamestream` still work. The tray is `slipstream-tray.exe`.

## Encode

`SLIPSTREAM_ENCODER` selects the backend:

| Value | Result |
|---|---|
| unset, or `auto` | NVENC when the selected adapter is NVIDIA (`vendor 10DE`) **and** the driver probe succeeds. Otherwise openh264. |
| `nvenc`, `nvidia`, `cuda` | NVENC. A missing DLL or a non-NVIDIA adapter is an error, not a silent fallback. |
| `software`, `cpu`, `openh264`, `sw` | openh264. |

The probe asks the driver which of H.264, HEVC, and AV1 that chip can open, plus the HEVC 4:4:4
and 10-bit capability bits. The host advertises that set on both the native handshake and the
Moonlight `ServerCodecModeSupport` mask. If the probe does not answer, both planes advertise
**H.264 only**.

`auto` plus H.264 falls back to openh264 when NVENC refuses to open. HEVC and AV1 do not: those
sessions fail with the driver error.

openh264 accepts up to 3840×2160, or 2160×3840 in portrait. NVENC is not held to that ceiling.
Frame rate follows the session request (1–1000). 4K/120 is a legal request on NVENC; it has not
been measured on a Windows GPU from this repository.

## Picture

Windows capture is 8-bit BGRA, so the frame already has full chroma.

- **4:4:4** is offered on the native plane only when the session is HEVC, the client asks for it,
  and the NVENC probe reported `NV_ENC_CAPS_SUPPORT_YUV444_ENCODE`. GameStream stays 4:2:0.
- **HDR / 10-bit** is not offered. The encoder refuses a 10-bit session, and capture cannot
  produce a PQ frame, so the handshake stays at 8-bit even if the GPU's 10-bit bit is set.
- **Cursor.** Capture embeds the pointer. The host does not advertise a separate cursor blend,
  because it cannot also deliver a pointer-free frame plus a cursor bitmap.

## Display

The default is a **mirror**: the session changes the mode of an existing monitor with
`ChangeDisplaySettingsExW` and restores it when the session ends.

A headless virtual monitor needs `SLIPSTREAM_VIRTUAL_DISPLAY=idd` (also `1`, `on`, `true`, `yes`,
or `virtual`) **and** the [RustDesk indirect display driver](https://github.com/rustdesk/RustDeskIddDriver)
installed and loaded. The host speaks that driver's userspace plug/unplug protocol
(device interface `{781EF630-72B2-11d2-B852-00C04EAF5272}`). The driver is not shipped here:
Windows will not load an unsigned indirect-display driver. If you ask for `idd` and the driver is
absent, the session fails and names the install. It does not fall back onto someone else's
monitor. `mirror`, `0`, `off`, `false`, `no`, or an unset variable keeps the mirror.

## Network

`SLIPSTREAM_PERFORMANCE_PROFILE=low_latency` registers the capture, encode, send, and input
threads with MMCSS (`Games` on the critical path, `Playback` on the rest) and raises the thread
priority when MMCSS refuses. Generic segmentation offload and TXTIME pacing stay Linux-only.
Video and audio DSCP (CS5 / CS6) are applied through qWAVE when `qwave.dll` loads; the IP TOS
byte is still set when it does not. NVENC bitrate changes are applied in place. The software
encoder cannot retarget without rebuilding the session.

## Limits

These are current gaps, not settings you can flip:

- No on-glass Windows GPU run is part of this repository's CI. The cross compile checks that the
  host builds.
- HDR capture is not implemented.
- The pointer stays inside the captured frame.
- No AMF, Quick Sync, or PyroWave backend.
- No virtual gamepad.
- The software encoder is H.264 only and resolution-capped as above.
