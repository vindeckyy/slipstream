<div align="center">
<pre style="display: inline-block; text-align: left;">
▞▀▖▌  ▜▘▛▀▖▞▀▖▀▛▘▛▀▖▛▀▘▞▀▖▙▗▌
▚▄ ▌  ▐ ▙▄▘▚▄  ▌ ▙▄▘▙▄ ▙▄▌▌▘▌
▖ ▌▌  ▐ ▌  ▖ ▌ ▌ ▌▚ ▌  ▌ ▌▌ ▌
▝▀ ▀▀▘▀▘▘  ▝▀  ▘ ▘ ▘▀▀▘▘ ▘▘ ▘
</pre>
</div>

<p align="center"><b>Low-latency desktop and game streaming for Linux and Windows hosts. Play from the couch, or use your real desktop while you&apos;re at work.</b></p>

<p align="center">
  <a href="#what-it-is">What it is</a> ·
  <a href="#web-console">Web console</a> ·
  <a href="#how-it-works">How it works</a> ·
  <a href="#status">Status</a> ·
  <a href="#install">Install</a> ·
  <a href="#connect-a-client">Connect</a> ·
  <a href="#build-and-test">Build</a> ·
  <a href="#design-invariants">Design</a>
</p>

<p align="center">
  <img src="assets/screenshots/dashboard.png" alt="Slipstream live status dashboard while streaming 1080p120 HEVC" width="900" />
</p>

---

## What it is

Slipstream turns a Linux or Windows machine into a private **desktop and game streaming** host.
You install the host, pair a client once, and stream to any screen on your LAN or VPN at that
screen's own resolution and refresh rate, for games on the couch, or for focused work on your
real desktop from an office laptop. No accounts, no cloud relay, no subscription.

The stack has three pieces that work together:

- A **host** that creates a virtual display per client, captures it, encodes it, and ships it
  over the network.
- **Clients** for Mac, iPhone, iPad, Apple TV, Linux, Windows, Android, and the Steam Deck.
- A **browser console** that manages pairing, displays, the game library, plugins, and host
  settings through a REST API.

Two protocols come out of the host. **GameStream** is spoken natively, so stock
[Moonlight](https://moonlight-stream.org/) connects the way it always has. **`slipstream/1`** is
the project's own protocol: QUIC control, UDP data, forward error correction, and AES-128-GCM
sealing, built for lower latency and larger frame protection than GameStream allows.

The console's workflows take cues from [Sunshine](https://github.com/LizardByte/Sunshine), without
copying its assets. Linux capture and compositor work draws on ideas from SolarFlare.

## How it works

Every stream follows the same path:

```
compositor → capture → encode → FEC/seal → network → reassemble → decode → present
```

Each client gets its **own virtual output** at an exact WxH@Hz. A 4K TV and a 1080p phone can
watch the same host simultaneously, each rendered natively, with no letterboxing and no
rearranging of your real monitors.

On the wire, frames are split into MTU-sized shards, protected with forward error correction,
sealed with AES-128-GCM, and paced out so a real NIC does not drop a whole frame as a line-rate
burst. The client reassembles, recovers lost shards from FEC, decodes, and presents. The control
plane (QUIC) handles pairing, mode negotiation, and feedback without touching the frame path.

The latency budget is measured end to end: capture-to-received on the client, with a host/network
split, plus decode and display stages where the client can see them. The docs explain
[which number means what](docs-site/content/docs/stats.md).

## Highlights

- **Per-client virtual display.** Exact WxH@Hz for each device. No letterboxing, no rearranging
  your real monitors.
- **Play and work on the same host.** Stream games to a TV or Deck, or use Workstation / Hot-desk
  presets, absolute mouse, and clipboard when you need the full desktop from the office.
- **Display policy you control.** Keep a game alive across disconnects, dedicate a couch box, or
  extend the desktop. Presets live in the console.
- **Windows IDD-push path.** Finished frames go into Slipstream's own indirect display driver,
  not a scrape of a physical screen.
- **GPU-first encode.** Zero-copy where the platform allows (dmabuf / CUDA / Vulkan / NVENC, plus
  AMF/QSV and software fallbacks).
- **Self-filling library.** Steam and plugins (ROM Manager, Playnite, VirtualHere, ...) from the
  console Plugin store or `slipstream-host plugins add`.
- **PIN pairing, no accounts.** SPAKE2 once, then pinned identities. mDNS discovery on the LAN.

## Web console

Manage the host from a browser: pairing, virtual-display presets, live sessions, performance,
configuration, and the plugin store. Same Sunshine-style workflows, Slipstream branding.

<p align="center">
  <img src="assets/screenshots/virtual-displays.png" alt="Virtual display presets (shared desktop, hot-desk, workstation, headless)" width="900" />
</p>

<p align="center">
  <img src="assets/screenshots/pairing.png" alt="PIN pairing with slipstream/1 and Moonlight clients" width="900" />
</p>

<p align="center">
  <img src="assets/screenshots/performance.png" alt="Per-session latency by stage and throughput charts" width="900" />
</p>

<p align="center">
  <img src="assets/screenshots/host.png" alt="Host identity and preflight checks" width="900" />
</p>

<p align="center">
  <img src="assets/screenshots/configuration.png" alt="Recommended host configuration with clickable toggles" width="900" />
</p>

## Status

| Piece | State |
|-------|-------|
| `slipstream-core` + C ABI | Complete |
| GameStream → Moonlight | Live (opt-in `--gamestream` on trusted LAN) |
| `slipstream/1` native path | Live |
| Windows host | Beta (signed installer) |
| Apple / Linux / Android / Windows clients | Streaming |
| Web console (`web/`) | Live over the OpenAPI mgmt API |

Bare `slipstream-host serve` is **native-only** (`slipstream/1` + mgmt/console). Add `--gamestream`
only on a LAN you trust.

## Install

Local and private setup is the default for now. Build from source, or use the scripts under
[`packaging/`](packaging/) for the platform you care about.

```sh
# Host + workspace (Linux example)
cargo build -p slipstream-host
./target/debug/slipstream-host serve --mgmt-bind 127.0.0.1:47990

# Console (dev)
cd web && bun install && bun run dev   # http://127.0.0.1:47992
```

Packaged installs (apt / rpm / Arch / Bazzite / winget / Flatpak) live under `packaging/` with their
own READMEs. Point package URLs and update feeds at **your** registry when you publish; this tree
does not assume a public package host.

Desktop-specific host tips live in [`docs-site/content/docs/`](docs-site/content/docs/) (KDE,
GNOME, gamescope, Sway, Steam Deck, Windows).

## Connect a client

| Device | Client |
|--------|--------|
| Mac / iPhone / iPad / Apple TV | `clients/apple` |
| Linux | `slipstream-client` (`clients/linux` + `clients/session`) |
| Steam Deck | Decky plugin (`clients/decky`) or Flatpak |
| Android | `clients/android` |
| Windows | `clients/windows` or Moonlight |
| Scripts | `slipstream` CLI (`clients/cli`) |
| Anything else | Moonlight over GameStream |

Pairing is a one-time PIN. See `docs-site/content/docs/pairing.md`.

## Build and test

```sh
cargo build --workspace
cargo test  --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check

cargo run -p loss-harness
bash crates/slipstream-core/tests/c/run.sh
```

The C header regenerates into `include/slipstream_core.h` on build. Apple, Android, and Windows
clients have their own toolchains (see each client's README).

## Layout

```
crates/
  slipstream-core/   protocol · FEC · crypto · QUIC · C ABI
  slipstream-host/   host: displays · capture · encode · GameStream · slipstream/1 · mgmt
  ss-*               capture, encode, inject, vdisplay, client-core, presenter, ...
clients/             apple · linux · session · windows · android · cli · probe · decky
web/                 TanStack management console
api/openapi.json     mgmt OpenAPI (from `slipstream-host openapi`)
docs-site/           Fumadocs documentation
packaging/           distro + Windows installer + winget + Flatpak + ...
include/             slipstream_core.h
```

## Design invariants

- **One core.** Protocol, FEC, and crypto live in `slipstream-core` once; native clients share it
  (Rust crate or C ABI).
- **No async on the frame path.** Native threads only; `tokio`/`quinn` stay on the control plane.
- **Native client resolution.** Each session gets a virtual output at exact WxH@Hz.
- **Packet-loss recovery scales.** GameStream stays Moonlight-compatible; `slipstream/1` can
  protect larger frames without retransmitting them.

## License

MIT OR Apache-2.0. See [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE), and
[CONTRIBUTING.md](CONTRIBUTING.md).

Third-party notices for shipped binaries: [`THIRD-PARTY-NOTICES.txt`](THIRD-PARTY-NOTICES.txt).
Historical copyright lines for earlier lineage (where required) live in [NOTICE](NOTICE).

### Trademarks

Slipstream is independent and is not affiliated with NVIDIA, Microsoft, Sony, Valve, or Moonlight.
"GameStream", "Moonlight", "Xbox", "DualSense", "DualShock", and "PlayStation" are trademarks of
their owners and are used only to describe interoperability.
