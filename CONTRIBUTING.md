# Contributing to Slipstream

Thanks for your interest in contributing!

## Licensing of contributions (inbound = outbound)

Slipstream is dual-licensed under **MIT OR Apache-2.0**.

> Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
> the work by you, as defined in the Apache-2.0 license, shall be dual licensed as **MIT OR
> Apache-2.0**, without any additional terms or conditions.

By opening a pull request you agree to license your contribution under these terms. This is the
standard Rust-ecosystem "inbound = outbound" model; it keeps the project's licensing unambiguous
(including the Apache-2.0 §5 contributor patent grant) and any future relicensing clean. You retain
the copyright to your contributions.

### Do not paste copyleft (or otherwise incompatibly-licensed) code

The single thing that could poison the permissive license is **copied source from a copyleft
project**. Several adjacent projects (Sunshine, Apollo, Moonlight) are GPL-3.0. You may study them
and reimplement a *technique*, protocol, or wire format, those are not copyrightable, but **never
paste their code**, and do not translate a GPL implementation line-by-line. When a comment credits
prior art, make clear it is an independent reimplementation, not a copy. The same applies to any
third party's code under a license incompatible with MIT/Apache.

If you add a new third-party dependency, it must be permissive (MIT / Apache-2.0 / BSD / ISC / Zlib /
Unicode-3.0 / etc.). `about.toml` holds the accepted-license allow-list; regenerate the attribution
file with `scripts/gen-third-party-notices.sh` when the dependency tree changes.

Automated dependency pull requests are disabled. Maintainers update dependencies and lockfiles in
reviewed batches, then run the security and license checks before publishing.

## Prerequisites

The development and release toolchain is pinned exactly in `rust-toolchain.toml`, and the
portable-crate CI job uses the same version; the workspace `rust-version` tracks that pin (the
vendored FEC GFNI backend and edition-2024 dependencies rule out older toolchains).

The workspace links system libraries, so a bare `cargo build --workspace` fails on a stock machine.
GitHub Actions runs on Ubuntu 24.04 and installs the authoritative dependency set in
`.github/workflows/ci.yml`:

```sh
sudo apt install build-essential clang libclang-dev pkg-config cmake \
  libavcodec-dev libavformat-dev libavutil-dev libswscale-dev libavfilter-dev libavdevice-dev \
  libpipewire-0.3-dev libopus-dev libwayland-dev libxkbcommon-dev \
  libgl-dev libegl-dev libgbm-dev \
  libgtk-4-dev libadwaita-1-dev libsdl3-dev \
  libvulkan-dev
```

(The last two groups are the Linux client shell and `ss-ffvk`; skip them only if you never build
those crates. `scripts/bootstrap-ubuntu.sh` sets up an Ubuntu **capture-test host**, NVIDIA, Sway,
PipeWire, and is not a substitute for the list above.)

## Before you push

Enable the repo git hooks once per clone. They run the exact rustfmt gates CI runs on every
commit and push, so a push cannot fail CI on formatting alone:

```sh
git config core.hooksPath scripts/git-hooks
```

Use `--locked` as CI does:

```sh
cargo fmt --all --check
cargo clippy -p slipstream-core --features quic --all-targets --locked -- -D warnings
cargo clippy -p ss-host-config -p ss-update-check --all-targets --locked -- -D warnings
cargo test -p slipstream-core --features quic --locked
cargo test -p ss-host-config -p ss-update-check -p slipstream-host --locked
cargo check -p slipstream-host --locked
```

Two more gates that only apply to some changes:

- **Touched `web/` or `docs-site/`?** CI builds and typechecks both. Run, in that directory:
  ```sh
  bun install && bun run build && bun run lint
  ```
  Build first, it generates the API client / MDX typegen that the typecheck imports.
- **Touched Linux-gated code from another OS?** `scripts/xcheck.sh` (or `scripts/xcheck.sh check`)
  type-checks and lints the Linux `#[cfg(target_os = ...)]` code instead of waiting for CI.

Generated artifacts are checked in. CI verifies the C header, OpenAPI snapshots, and TypeScript SDK
client. Regenerate them whenever their source changes:

```sh
cargo run -p slipstream-host -- openapi > api/openapi.json
cp api/openapi.json docs-site/public/openapi.json
cd sdk && bun run gen
```

Match the surrounding code's comment density and naming. Keep commit messages short and specific.

## Repository layout

```
crates/
  slipstream-core/   protocol · FEC · crypto · QUIC · C ABI
  slipstream-host/   host: displays · capture · encode · GameStream · slipstream/1 · mgmt
  ss-*               capture, encode, inject, vdisplay, client-core, presenter, ...
clients/             apple · android · decky · cli · ...
web/                 TanStack management console
api/openapi.json     mgmt OpenAPI (from `slipstream-host openapi`)
docs-site/           Fumadocs documentation
packaging/           distro packages + Flatpak + ...
include/             slipstream_core.h
```

## Design invariants

- **One core.** Protocol, FEC, and crypto live in `slipstream-core` once; native clients share it
  (Rust crate or C ABI).
- **No async on the frame path.** Native threads only; `tokio`/`quinn` stay on the control plane.
- **Native client resolution.** Each session gets a virtual output at exact WxH@Hz.
- **Packet-loss recovery scales.** GameStream stays Moonlight-compatible; `slipstream/1` can
  protect larger frames without retransmitting them.

See the [README's Developing section](README.md#development) for the extra dev commands (the FEC
loss harness, the standalone C-ABI proof), the [Repository layout](#repository-layout) and
[Design invariants](#design-invariants) above for the rules a change is expected to hold to, and
the [local docs site](docs-site/) (`docs-site/content/docs/`) for architecture and per-platform
guides.
