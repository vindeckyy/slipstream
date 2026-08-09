---
title: Host CLI
description: The slipstream-host commands and the flags you'll actually use, plus slipstream, the command on the client machine.
---

The host is one binary, `slipstream-host`. Most of the time you'll run a single command; the rest reads
its settings from [`host.env`](/docs/configuration). On the machine you stream *to*, there's a second
command, [`slipstream`](#slipstream-on-the-client-machine), which ships with the client.

| Command | What it does |
|---|---|
| [`serve`](#serve) | Run the host. |
| [`slipstream1-host`](#slipstream1-host) | Standalone native-only test host. |
| [`plugins`](#plugins) | Install, remove and list plugins, and switch the runner on. |
| [`list-monitors`](#list-monitors) | List the physical monitors, by connector name. |
| `mirror-test` | Prove capture works from one of them, see [`list-monitors`](#list-monitors). |
| [`hdr-probe`](#hdr-probe-and-probe-compositor) | Report whether this box can deliver a 10-bit HDR stream, and what's missing. |
| [`probe-capture`](#probe-capture) | Show the compositor-aware capture order and runtime availability. |
| [`probe-compositor`](#hdr-probe-and-probe-compositor) | Exit 0 when the compositor is up and can create a virtual output. |
| [`detect-conflicts`](#detect-conflicts) | Report other Moonlight-compatible hosts on this machine. |
| [`doctor`](#doctor) | Run read-only host readiness checks before starting a stream. |
| `library` | Print [the resolved game library](/docs/game-library) as JSON, "does the host see my games?". |
| `openapi` | Print the management API's OpenAPI document. |
| `--version` | Print the host version. |

`slipstream-host --help` prints the most-used of these. `plugins` prints its own usage when you run
it with no arguments.

## `doctor`

`doctor` runs the same read-only preflight report exposed in the web console. It checks config
storage, saved settings, the detected encoder, running competing hosts, and the Linux compositor
and capture environment. A blocked check exits with status 1, which makes it safe to use from a
service install or recovery script.

```sh
slipstream-host doctor
slipstream-host doctor --json
```

The JSON form is schema-versioned and contains stable check ids, status, detail, and one suggested
operator action. It does not create a display, pair a client, or start a stream.

## `serve`

The normal way to run a host. By default `serve` starts the **secure native host**: the native
`slipstream/1` server (QUIC, SPAKE2 PIN pairing, per-direction AEAD) plus the management API/web
console, all in one process. The native plane is **always on**; there is no flag to turn it off.

```sh
slipstream-host serve
```

Add `--gamestream` (alias `--moonlight`) to **also** run the GameStream/Moonlight-compatible planes
(nvhttp pairing, RTSP, ENet control, `_nvstream` mDNS), required for stock [Moonlight](/docs/moonlight)
clients. This is **opt-in** because GameStream carries inherent on-path weaknesses (pairing over plain
HTTP; its legacy control encryption can reuse GCM nonces), so enable it **only on a trusted LAN**. The
native plane is immune to those issues.

```sh
slipstream-host serve --gamestream
```

| Flag | Meaning |
|---|---|
| `--gamestream` / `--moonlight` | Also run the GameStream/Moonlight-compat planes (for stock Moonlight clients). Opt-in, trusted-LAN only, see above. |
| `--native` | No-op. The native `slipstream/1` server always runs in `serve`; kept only for backward compatibility. |
| `--native-port <PORT>` | Native QUIC port (default `9777`). |
| `--open` | Don't require pairing, serve any device on the network. Off by default; only for trusted single-user setups. |
| `--mgmt-bind <IP:PORT>` | Management API address (default `0.0.0.0:47990`, all interfaces, so paired clients can browse the game library over mTLS; pass `127.0.0.1:47990` to keep it loopback-only). |
| `--mgmt-token <TOKEN>` | Override the bearer token for the management API. |
| `--no-mdns` | Skip the mDNS adverts (native + GameStream), for networks/containers where multicast doesn't work. Clients connect via a manually added host instead. Same as `SLIPSTREAM_MDNS=0`. |
| `--data-port <PORT>` | Pin the per-session video data plane to this fixed UDP port and stream direct (no hole-punch), open exactly that port in the host firewall. Same as `SLIPSTREAM_DATA_PORT`; default is a random port + hole-punch. |

These are the only flags `serve` accepts.

The management API is **always HTTPS**. It binds all interfaces by default so a **paired client** can
fetch the game library over its mTLS certificate, but off loopback that certificate reaches only the
read-only status + library endpoints. The **admin surface** (arming pairing, removing devices, session
control, library edits) authenticates with a **bearer token** and is honored **from loopback only**, so
it is never LAN-exposed even under the default wide bind. If you don't pass `--mgmt-token`, a token is
auto-generated and persisted to `~/.config/slipstream/mgmt-token` (the bundled web console reads the same
file); `--mgmt-token` only overrides it. Pass `--mgmt-bind 127.0.0.1:47990` to keep 47990 loopback-only.
Every endpoint is documented in the interactive [**API Reference**](/api).

By default the host **requires pairing**, see [Pairing & Trust](/docs/pairing). On `serve` you
**arm pairing from the web console** (or mgmt API); the host then displays a 4-digit PIN. Pass `--open` to
turn off the mandatory-pairing default and serve any device on the network (trusted single-user setups
only). `slipstream1-host` (below) requires pairing by default too; its `--allow-tofu` flag is the
test-host equivalent of `--open`.

## `slipstream1-host`

A standalone native-only host, mainly for testing the `slipstream/1` path without the GameStream server
or web console.

```sh
slipstream-host slipstream1-host --source virtual
```

| Flag | Meaning |
|---|---|
| `--port <N>` | QUIC listen port (default `9777`). |
| `--source synthetic` · `virtual` | `virtual` uses a real virtual display + NVENC; `synthetic` emits test frames. |
| `--seconds <N>` / `--frames <N>` | Bound each session by wall-clock seconds or frame count. |
| `--max-concurrent <N>` | Stream at most N sessions at once (default 4); overflow waits in the queue. |
| `--max-sessions <N>` | Exit after N sessions (0 = serve forever). |
| `--allow-tofu` | Also accept **unpaired** clients (trust-on-first-use) and advertise pairing as optional. Pairing is required by default; trusted LANs only. (`--allow-pairing`/`--require-pairing` are the old names for the default behaviour and are accepted as no-ops.) |
| `--pairing-pin <PIN>` | Use a fixed pairing PIN instead of a fresh random one per ceremony. For test harnesses/CI only, a guessable PIN defeats the ceremony's rate limit. |
| `--data-port <PORT>` | Pin the video data plane to this fixed UDP port and stream direct (no hole-punch). Same as `SLIPSTREAM_DATA_PORT`. |
| `--idle-timeout-ms <MS>` | Disconnect-detection latency, the QUIC control-connection idle timeout (default 8000). |
| `--no-mdns` | Skip the `_slipstream._udp` advert; clients use `--connect HOST:PORT`. Same as `SLIPSTREAM_MDNS=0`. |

`--max-concurrent` and `--allow-tofu` are **`slipstream1-host`-only**, `serve` does not accept them.
On `serve` you arm pairing from the web console instead (`--open` is its serve-any-device switch),
and concurrency is fixed at the built-in default (4 sessions) rather than settable from the command
line.

Both `serve` and `slipstream1-host` advertise the host on the network so clients can discover it. The
graphical client browses the LAN for you, so it needs no command; from a terminal on the client
machine, [`slipstream hosts list --probe`](/docs/host-cli#slipstream-on-the-client-machine) re-checks the hosts you
have already saved by asking each one directly, which is how you confirm a routed or VPN host that mDNS never reaches.
(`slipstream-probe --discover` also browses the LAN, but it is a developer tool built from the repo,
`cargo run -p slipstream-probe -- --discover`, and no package installs it.)
Where multicast doesn't work (some Docker/VLAN setups), pass `--no-mdns` (or set
`SLIPSTREAM_MDNS=0`) and add the host in the client by address instead.


## `plugins`

`slipstream-host plugins add|remove|list|enable|disable|status` installs plugins and switches the
plugin/scripting runner on, the same thing the web console's **Plugins** page does. Plugins run
with the host's privileges, so read [Plugins](/docs/plugins) before installing one.

## `list-monitors`

`slipstream-host list-monitors` prints the **physical** monitors this host's compositor has, by
connector name, which is how you name one for [Streamed
screen](/docs/virtual-displays#stream-a-real-monitor-instead) (in the console, or as
`SLIPSTREAM_CAPTURE_MONITOR`).

```sh
slipstream-host list-monitors
```

```
Kwin:
  HDMI-A-1        1920x1080@60 at +0,+0    scale 1  Dell U2412M  [primary]
  DP-2            2560x1440@144 at +1920,+0  scale 1  ACME 27  [PINNED]
```

Tags flag what's worth knowing before you pick: `primary`, `disabled` (nothing to stream),
`slipstream virtual display` (one of ours, not a real head), and `PINNED` for the one currently
selected. It reads the live compositor, so run it in (or with the environment of) the session you
want to stream.

`slipstream-host mirror-test --monitor <CONNECTOR> [--seconds N] [--cpu]` then proves the whole path,
mirror, capture, frames, with no client involved. It reports the first frame, the frame count and
the negotiated size. Screen recording is damage-driven, so move the mouse on the host while it runs;
an idle desktop legitimately yields almost nothing.

## `probe-capture`

`slipstream-host probe-capture` reports the compositor detected for the current session, the
effective physical-monitor pin, the candidate order selected for an existing desktop, and each
candidate's best-effort runtime availability. It does not open a portal or create a display. The
NvFBC check briefly creates and destroys a capture session so CUDA setup failures are visible
without changing the display topology.

```sh
slipstream-host probe-capture
```

Use it with the same environment as the host service. KMS availability tests primary-plane discovery
and dma-buf export, while NvFBC creates and tears down a lightweight session through CUDA setup;
portal, X11, and wlroots availability are prerequisite hints because their sessions are negotiated
with the compositor when a stream opens.

## `hdr-probe` and `probe-compositor`

Two Linux readiness checks that need no client and no session of their own.

```sh
slipstream-host hdr-probe
slipstream-host probe-compositor
```

`hdr-probe` answers "why isn't my stream HDR?", it reports, for both Linux HDR routes, whether the
box can deliver 10-bit PQ right now: is a monitor in HDR colour mode (the GNOME monitor-mirror
route), is the resolved gamescope the `slipstream-gamescope` build with the knob on, and does the
encoder probe Main10 for HEVC/AV1. Run it with the same environment the host service has, or the
answers describe your shell rather than the host, [HDR -> Check it](/docs/hdr#check-it) has that
one-liner and reads the output line by line. See
[HDR on gamescope](/docs/gamescope#hdr-on-gamescope) for the gamescope half.

`probe-compositor` exits **0** only when the compositor is up and can create a virtual output now,
what a session-bringup script should gate on instead of a blind `sleep`.

## `detect-conflicts`

`slipstream-host detect-conflicts` reports other Moonlight-compatible hosts (Sunshine, Apollo, and
forks) installed or running on this machine. Running one alongside Slipstream is **unsupported**,
they fight over the same ports and virtual-display driver. Prints what it found and exits **1** if
any conflict exists, **0** if clean (so installers and scripts can gate on it). The host also runs
this check at `serve` startup and reports it in the logs and in the management API's status
summary. An installed-but-idle Sunshine isn't a conflict until it runs.
See [Troubleshooting -> another streaming host is installed](/docs/troubleshooting#another-streaming-host-sunshine-apollo--is-installed).

## `slipstream` on the client machine

The client half has its own command, `slipstream`, the same core the graphical apps use, with no
window, so a script gets what a click gets, including waking a sleeping host and waiting for it.

Its verbs, where it ships, the `<host-ref>` grammar and the stable exit codes are on [Clients -> the
`slipstream` CLI](/docs/host-cli#slipstream-on-the-client-machine); `slipstream help <command>` prints one
verb's flags. `slipstream wake` has its own exit codes, on [Wake on LAN -> From the command
line](/docs/wake-on-lan#from-the-command-line).

## Environment

Most behaviour (compositor, video source, input backend, zero-copy) is set in
[`host.env`](/docs/configuration), not on the command line. When running as a
[service](/docs/running-as-a-service), the unit loads `host.env` for you.
