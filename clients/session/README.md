# slipstream-session

The Vulkan session binary: one stream per invocation in an SDL3 window - no UI toolkit,
no widgets, terminal stats. The power-user / gamescope stream client, and the stage-2
presenter of the Linux client re-architecture (slipstream-planning:
`linux-client-rearchitecture.md`).

This binary is deliberately dumb: a renderer the front-ends call INTO - the GTK shell
(`slipstream-client`) and the `slipstream` CLI spawn it through the
same brain (`ss_client_core::orchestrate`), which resolves policy (profiles, settings,
wake) and hands the result down, normally as a `--resolved-spec` file. It reads the
shared stores only as the compat fallback for a bare hand-launched invocation.

```
slipstream-session --connect host[:port] [--fp HEX] [--launch id] [--fullscreen] [--stats]
slipstream-session --browse host[:port] [--mgmt PORT] [--fullscreen]
```

`--browse` opens the console game library (the Skia coverflow over the animated aurora)
instead of connecting: A launches the focused title as a stream in the same window,
session end returns to the library, B quits (Gaming Mode returns). Paired hosts only -
pairing is the desktop client / Decky plugin's job. `SLIPSTREAM_FAKE_LIBRARY=<file.json>`
feeds canned entries with no host (portrait paths starting with `/` load from disk).

Reads the same identity / known-hosts / settings stores as the desktop client
(`slipstream-client`), so enrolling on either side makes the other work; this binary never
connects to a host it has no pinned fingerprint for (`--fp HEX` overrides the store).

Pairing is `slipstream pair <host>` - the CLI, which ships alongside this binary in every
package and needs no window and no toolkit either. `slipstream-session --pair` still works
for one release (someone's provisioning script calls it today) but prints a deprecation
notice: pairing is a trust ceremony and belongs to the brain, not a renderer.

Stdout is the machine interface: `{"ready":true}` after the first presented frame,
`stats: ...` once per second while the overlay tier isn't Off (always the full detailed
text, whatever the OSD shows; `--stats` forces the overlay on), one
`{"error"|"ended": ...}` JSON line on the way out. Logs go to stderr. Exit codes: `0`
clean end, `2` connect failed, `3` trust rejected / pairing required, `4` presenter
init failed.

In-stream keys match the desktop client: click captures input (Ctrl+Alt+Shift+Q
releases), Ctrl+Alt+Shift+D disconnects, F11 toggles fullscreen; the controller escape
chord (L1+R1+Start+Select, hold to disconnect) works the same.

The default build carries the Skia console UI (`ui` feature): the stats OSD and capture
hint render in-window. Ctrl+Alt+Shift+S cycles the OSD tier live - Off → Compact (one
line: fps · latency · Mb/s) → Normal (mode + end-to-end percentiles) → Detailed (decoder
path + per-stage latency equation); any tier but Off also emits the stdout mirror.
`--no-default-features` is the ~5 MB power-user build - same streaming, stats on stdout
only, no Skia anywhere in the dependency tree.

Decode follows the Settings preference (auto: Vulkan Video -> VAAPI -> software on Linux).
FFmpeg's Vulkan Video decoder runs on the presenter's own device where the stack supports it
(every vendor, zero copy); VAAPI dmabufs import per-plane elsewhere; software is the
universal fallback. 10-bit Main10 and HDR10 are advertised
(`VIDEO_CAP_10BIT|HDR`): P010 decodes through each path, and PQ streams present
on an HDR10/ST.2084 swapchain when the desktop offers one (KDE HDR, gamescope) or
tone-map in-shader to SDR when it doesn't (`SLIPSTREAM_TONEMAP_PEAK` tunes the rolloff,
default ≈1000 nits). The host still gates the upgrade behind its `SLIPSTREAM_10BIT`
policy.

Debug/bisect knobs: `SLIPSTREAM_DECODER=vulkan|vaapi|software`, `SLIPSTREAM_PRESENT_MODE=
mailbox|immediate` (default FIFO), `SLIPSTREAM_VK_DEVICE=<index>` (multi-GPU), and
`SLIPSTREAM_HW_FAULT=import` (fault every VAAPI dmabuf import - proves the three-strike
demotion to software on healthy hardware).
