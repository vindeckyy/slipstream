<p align="center">
  <img src="assets/slipstream-logo.png" alt="Slipstream" width="420" />
</p>

<p align="center"><b>Low-latency desktop and game streaming for Linux and Windows hosts.</b></p>

Slipstream is a private streaming stack: a virtual-display host, native clients, and a browser
console. It speaks **GameStream** so [Moonlight](https://moonlight-stream.org/) works day one, and
ships its own faster **`slipstream/1`** protocol with QUIC control and packet-loss recovery. The host
console UX takes cues from [Sunshine](https://github.com/LizardByte/Sunshine)'s workflows (not
copied assets). Linux capture and compositor work also draws on ideas from local SolarFlare
experiments in this workspace.

Run the host on a Linux box or Windows PC. Connect from Mac, PC, phone, tablet, or TV. Each client
gets its **own native resolution and refresh** on the LAN.

**Docs:** [`docs-site/`](docs-site/) in this repo (no public docs host yet).

**Security:** report privately per [SECURITY.md](SECURITY.md). Do not open a public issue for security
reports.

**Source:** private GitHub at `https://github.com/vindeckyy/slipstream` (default branch `main`).

## Highlights

- **Per-client virtual display.** Exact WxH@Hz for each device. No letterboxing, no rearranging your
  real monitors.
- **Display policy you control.** Keep a game alive across disconnects, dedicate a couch box, or
  extend the desktop. Presets live in the console.
- **Windows IDD-push path.** Finished frames go into Slipstream's own indirect display driver, not a
  scrape of a physical screen.
- **GPU-first encode.** Zero-copy where the platform allows (dmabuf / CUDA / Vulkan / NVENC, plus
  AMF/QSV and software fallbacks).
- **Self-filling library.** Steam and plugins (ROM Manager, Playnite, VirtualHere, …) from the
  console Plugin store or `slipstream-host plugins add`.
- **PIN pairing, no accounts.** SPAKE2 once, then pinned identities. mDNS discovery on the LAN.

## Status (short)

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

## Install (from this tree)

Local / private setup is the default for now. Build from source, or use the scripts under
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

Desktop-specific host tips: [`docs-site/content/docs/`](docs-site/content/docs/) (KDE, GNOME,
gamescope, Sway, Steam Deck, Windows).

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
  ss-*               capture, encode, inject, vdisplay, client-core, presenter, …
clients/             apple · linux · session · windows · android · cli · probe · decky
web/                 TanStack management console
api/openapi.json     mgmt OpenAPI (from `slipstream-host openapi`)
docs-site/           Fumadocs documentation
packaging/           distro + Windows installer + winget + Flatpak + …
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
