# slipstream client — Flatpak (Steam Deck / SteamOS, and any flatpak distro)

The native Linux **client** — the shell (crate `slipstream-client-linux`, binary
`slipstream-client`) plus the Vulkan session binary it execs for streaming (crate
`slipstream-client-session`, binary `slipstream-session`) — is
built by CI (`.github/workflows/flatpak.yml`) when enabled. There is no public Flatpak remote.
Produce a **single-file `.flatpak` bundle** locally (see below) and install it with
`flatpak install --user --bundle`, or attach the bundle to a
[GitHub Release](https://github.com/vindeckyy/slipstream/releases). If you host your own OSTree
repo later, users can `flatpak update` from that remote instead.

> The **host** is NOT a flatpak (it needs unsandboxed `/dev/uinput` + zero-copy NVENC — see
> [`../README.md`](../README.md) "Why not Flatpak"). Only the **client** is sandbox-friendly.

## Why flatpak for the Steam Deck

SteamOS `/usr` is read-only and image-based, and the system is **missing `libadwaita` and
`libSDL3`** — so a bare `slipstream-client` binary dropped into `~/.local/bin` won't run. Flatpak
is the Deck's native, update-survivable app path (the user already runs Moonlight and chiaki-ng
as flatpaks), and the bundle carries libadwaita (from `org.gnome.Platform//50`) + a bundled SDL3,
with HEVC-capable FFmpeg supplied automatically by the runtime's `codecs-extra` extension.

App id: **`io.slipstream`** (matches the Apple bundle id family and the Decky plugin's
flatpak fallback).

## Install (local bundle)

Build the bundle, then install it per-user (no root; survives SteamOS updates on the Deck):

```sh
bash packaging/flatpak/build-flatpak.sh
# Flathub must be enabled so the GNOME runtime + codecs-extra extension pull in:
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user --bundle dist/slipstream-client-*.flatpak
flatpak run io.slipstream
```

Or download a `.flatpak` from [GitHub Releases](https://github.com/vindeckyy/slipstream/releases)
when attached and `flatpak install --user --bundle` that file.

Run it:

```sh
flatpak run io.slipstream                 # GUI host list (mDNS)
flatpak run io.slipstream --connect HOST:PORT
```

The **Decky plugin** launches exactly this (`flatpak run io.slipstream --connect …`) once
installed — see [`../../clients/decky/README.md`](../../clients/decky/README.md).

## Updating the bundle install

If you installed from the **bundle** (not the hosted repo), it has no remote to track, so updates
are "download the newer bundle and reinstall":

```sh
flatpak install --user --bundle /tmp/slipstream-client.flatpak   # same command, newer file
```

If you later point a Flatpak remote at your own OSTree host, updates are just `flatpak update`.

## Build locally / the CI fallback

CI builds this in a **`--privileged`** Fedora container, because `flatpak-builder` runs
`bubblewrap`, which needs user namespaces the default Docker executor denies. **If the CI
runner can't grant `--privileged`** (the job fails at `flatpak-builder` with
*"Creating new namespace failed: Operation not permitted"*), build it out-of-band and publish
by hand. The easiest place is **on the Deck itself** (it can run `org.flatpak.Builder`
user-scope, no root):

```sh
# On the Deck (or any flatpak box), one-time:
flatpak install --user -y flathub org.flatpak.Builder

# build-flatpak.sh auto-detects org.flatpak.Builder, generates cargo-sources.json (or reuses an
# existing one — see below), builds, and exports dist/slipstream-client-<version>.flatpak:
bash packaging/flatpak/build-flatpak.sh

# Optional: attach dist/slipstream-client-*.flatpak to a GitHub Release, or copy it to your own feed.
```

> `cargo-sources.json` generation needs `python3` + `aiohttp` + `tomlkit`, which the Deck lacks.
> Generate it on a dev box (`build-flatpak.sh` does it, or run the upstream
> `flatpak-cargo-generator.py Cargo.lock -o packaging/flatpak/cargo-sources.json`), rsync it next
> to the manifest, and `build-flatpak.sh` reuses it (it only regenerates when the file is absent
> or `FORCE_GEN=1`).

> The Mac build host **cannot** build a Linux flatpak (no flatpak-builder for macOS), and
> home-worker-2 has no flatpak and no passwordless sudo to install it — so the Deck or the
> privileged CI container are the only two viable build sites.

### aarch64

The manifest builds for aarch64 as well as x86_64. Two things are architecture-specific, and both
are now expressed properly rather than hardcoded:

* **`PKG_CONFIG_PATH`** contains the runtime's multiarch directory. flatpak-builder does *not*
  shell-expand `env` values, so `${FLATPAK_ARCH}` would be taken literally — a `build-options.arch`
  override supplies the aarch64 string instead, inheriting everything else.
* **The prebuilt Skia archive** is per-target and pinned by sha256. There are now two `type: file`
  sources discriminated by `only-arches`, both landing on the same `dest-filename`, so
  `SKIA_BINARIES_URL` stays one literal path. Upstream publishes the aarch64 archive under the
  same skia commit hash and the same resolved-feature key (`pdf-textlayout-vulkan`), so on a
  skia-safe bump update both URLs and both hashes together.

```sh
ARCH=aarch64 bash packaging/flatpak/build-flatpak.sh
# -> dist/slipstream-client-<version>-aarch64.flatpak
```

`ARCH` defaults to this machine's, and the bundle name now carries the architecture so an x86_64
and an aarch64 build can coexist in `dist/`. This is **not** a cross-compile: flatpak-builder runs
the build in a sandbox for the target arch, so building aarch64 anywhere but an arm64 machine
needs qemu binfmt and is very slow. Not yet verified end to end — the manifest is correct by
construction and the Skia hash was checked against the published archive, but no aarch64 flatpak
has been built.

## Manifest

[`io.slipstream.yml`](io.slipstream.yml). Runtime `org.gnome.Platform//50`
(GTK 4.20 + libadwaita 1.8 ≥ the crate floors of v4_16 / v1_5), built on freedesktop-sdk 25.08,
with two build-time SDK extensions: `org.freedesktop.Sdk.Extension.rust-stable` (→ //25.08,
**rustc 1.96** — the GTK4 dep chain, e.g. pango-sys 0.22, needs ≥ 1.92, which the EOL GNOME-48 /
24.08 rust-stable at 1.89 could not provide) and `org.freedesktop.Sdk.Extension.llvm20` (libclang,
needed by bindgen in ffmpeg-sys-next / sdl3-sys). HEVC-capable libavcodec (soname 61, accepted by
ffmpeg-next 8.x) is supplied **automatically at runtime** by the freedesktop `codecs-extra`
extension point (auto-downloaded with the runtime; no app-side codec declaration). A bundled
**SDL3 3.4.10** module (pinned to match `sdl3-sys 0.6.6+SDL-3.4.10`), and finish-args for Wayland +
`--device=all` (GPU/VAAPI render node + evdev + the hidraw char-devices SDL3 needs for DualSense)
+ `--socket=pulseaudio` (PipeWire-pulse: playback + mic) + `--share=network`. Alongside it:
`io.slipstream.desktop`, `io.slipstream.metainfo.xml`, `io.slipstream.svg` (all
installed by the manifest). A `vulkan-headers` module supplies what the session binary's ash/Vulkan
build needs. `cargo-sources.json` (the offline crate cache) is a pure function of
`Cargo.lock`; CI regenerates it each build and it is **gitignored** — generate it on any box with
network + `python3`/`aiohttp`/`tomlkit` (`build-flatpak.sh` does this automatically) and, for a
build host that lacks those (the Deck), rsync the generated file in alongside the manifest.

**Offline Skia:** the session binary's Skia console UI (`ss-console-ui` → `skia-safe`) normally
downloads prebuilt `libskia` binaries at build time, which is dead in the offline sandbox — so the
manifest pins a `skia-binaries-….tar.gz` source and points the build at it with
`SKIA_BINARIES_URL: file://…`. When bumping the `skia-safe`/`skia-bindings` crate version, update
that pinned tarball (URL + sha256) to the matching `skia-binaries` release or the build breaks
offline.

## Hosting your own OSTree remote (optional)

If you want `flatpak update` instead of reinstalling bundles, host the OSTree repo flatpak-builder
produces (GPG-signed) behind any static HTTP server, and publish a `.flatpakrepo` / `.flatpakref`
that points at it. The scripts under `server/` in this directory are a starting point for a local
Caddy container. There is no public Slipstream Flatpak remote.

**Signing key:** generate a dedicated Flatpak repo signing key; put the public half in
[`unom-flatpak.gpg`](unom-flatpak.gpg) (or your own filename) and the private half in a CI secret
such as `FLATPAK_GPG_PRIVATE_KEY`.

## Alternatives considered

- **Self-hosted OSTree repo:** the option that gives `flatpak update` once you front the static
  tree with HTTPS.
- **Single-file `.flatpak` bundle (default here):** one build, one `flatpak install --bundle`; no
  auto-update — fine for Decky and offline installs.
- **Release attachment:** attach the bundle to a GitHub Release for a human-facing download page.
- **Flathub (deferred):** best discoverability + zero hosting, but a separate submission/review
  process; revisit once the client is past scaffold quality.
