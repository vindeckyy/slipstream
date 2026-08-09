# Slipstream

<p align="center">
  <img src="assets/slipstream-logo.png" alt="Slipstream" width="420">
</p>

<p align="center">Low-latency desktop and game streaming from a Linux host to the devices you already use.</p>

<p align="center">
  <a href="https://vindeckyy.github.io/slipstream/">Documentation</a> |
  <a href="https://github.com/vindeckyy/slipstream/releases">Releases</a> |
  <a href="SECURITY.md">Security</a>
</p>

Slipstream turns a Linux machine into a private streaming host. Pair a client once, then stream a
virtual display at the client's resolution and refresh rate over a trusted LAN or private VPN.

The host captures the compositor output, encodes it, sends it over the selected transport, and the
client decodes and presents it. Native clients are available for iPhone, Android, and Steam Deck.
Moonlight clients are supported through the optional GameStream compatibility path.

## What you get

| Surface | Purpose |
| --- | --- |
| Native streaming | The slipstream/1 path with QUIC control, UDP media, and forward error correction. |
| GameStream compatibility | Optional Moonlight support for clients that need the established protocol. |
| Linux host | Per-client virtual displays, compositor-specific capture, GPU-first encoding, and input injection. |
| Web console | Pairing, display presets, library management, plugins, configuration, and live status. |
| Trusted networking | LAN or private VPN use. Public port forwarding is unsupported. |

Slipstream is designed for two sessions on the same machine: Play for a TV or Steam Deck, and Work
for a full desktop from another location. The web console exposes the current host state so a stream
can be diagnosed from the same place it is configured.

## Install the host

The supported host is Linux. Start with the platform requirements and install guide in the
[documentation](https://vindeckyy.github.io/slipstream/docs/requirements/).

For a source build, install the system libraries listed in
[CONTRIBUTING.md](CONTRIBUTING.md), then run:

    cargo build -p slipstream-host
    ./target/debug/slipstream-host serve --mgmt-bind 127.0.0.1:47990

The web console runs locally during development:

    cd web
    bun install
    bun run dev

Open http://127.0.0.1:47992 after the host is running. The management token stays on the host;
the browser receives a session cookie, never the token.

## Connect a client

1. Install a client for iPhone, Android, or Steam Deck.
2. Put the client and host on the same LAN or on a private VPN.
3. Pair with the one-time PIN shown by the host.
4. Choose Play or Work settings for the session.

Moonlight discovery requires GameStream broadcast to be enabled, usually with the host's
--gamestream option. Slipstream cannot share GameStream ports, mDNS, or virtual-display drivers
with Sunshine or another Moonlight-compatible host while that host is active.

## Screenshots

<p align="center">
  <img src="assets/screenshots/dashboard.png" alt="Slipstream host dashboard with live stream status" width="900">
</p>

<p align="center">
  <img src="assets/screenshots/pairing.png" alt="Slipstream client pairing screen" width="900">
</p>

<p align="center">
  <img src="assets/screenshots/configuration.png" alt="Slipstream host configuration controls" width="900">
</p>

## Documentation

The [documentation site](https://vindeckyy.github.io/slipstream/) covers installation, compositor
support, client pairing, network setup, security, input, picture quality, troubleshooting, and the
[OpenAPI reference](https://vindeckyy.github.io/slipstream/api/).

Source documentation is under [docs-site/content/docs](docs-site/content/docs). Product behavior
described there is tied to the host and client paths in this repository.

## Development

Run the same checks used by the public CI:

    cargo fmt --all --check
    cargo clippy --workspace --all-targets --locked -- -D warnings
    cargo test --workspace --locked

The web console and documentation site use Bun. Each directory has its own lockfile and validation
commands. See [CONTRIBUTING.md](CONTRIBUTING.md) before changing generated API, client, or license
artifacts.

## Security

Slipstream controls a machine through the network. Keep it on a trusted LAN or private VPN, use the
pairing boundary, and do not expose it through public port forwarding. Report vulnerabilities
through the [GitHub security policy](https://github.com/vindeckyy/slipstream/security/policy).

## License

MIT OR Apache-2.0. See [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE), [NOTICE](NOTICE),
and [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt).

Slipstream is independent and is not affiliated with NVIDIA, Microsoft, Sony, Valve, or Moonlight.
Their names are used only to describe interoperability.
