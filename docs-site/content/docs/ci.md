---
title: "CI & Docker"
description: "GitHub Actions setup — workflows, the dockerized pieces, and the runners."
---

CI runs on **GitHub Actions** (`github.com/vindeckyy/slipstream`, org `unom`). Three workflows in
`.github/workflows/`, two runners, three images in the GitHub container registry.

## Workflows

| Workflow | Trigger | Runner | What it does |
|---|---|---|---|
| `ci.yml` | push to `main`, PRs | `ubuntu-24.04` | Rust workspace (fmt · clippy `-D warnings` · build · test · C-ABI harness · generated-header drift) inside the `slipstream-rust-ci` image; `web/` and `docs-site/` build + typecheck in `oven/bun:1` |
| `docker.yml` | push to `main`, `v*` tags, manual | `ubuntu-24.04` | Builds + pushes the three images below (`latest` + `sha-<short>` tags) |
| `apple.yml` | push to `main`, PRs, manual | `macos-arm64` | Rust core → `SlipstreamCore.xcframework` → `swift build` + `swift test` in `clients/apple` |
| `release.yml` | `v*` tags, manual | `macos-arm64` | Production Apple builds: sandboxed macOS `.dmg` (Developer ID, notarized, stapled) attached to the GitHub release + macOS/iOS/tvOS archives uploaded to TestFlight |

## Dockerized pieces

The host and the native clients are intentionally **not** containerized (the host needs
the GPU/compositor stack of the box it runs on). What is:

| Image | Source | Notes |
|---|---|---|
| `github.com/vindeckyy/slipstream/unom/slipstream-web` | `web/Dockerfile` (repo-root context — orval needs `docs/api/openapi.json`) | Nitro `bun` bundle; `PORT` (3000) and `SLIPSTREAM_MGMT_URL` env at runtime |
| `github.com/vindeckyy/slipstream/unom/slipstream-docs` | `docs-site/Dockerfile` | This site; `PORT` (3000) |
| `github.com/vindeckyy/slipstream/unom/slipstream-rust-ci` | `ci/rust-ci.Dockerfile` | Ubuntu 26.04 + FFmpeg 8/PipeWire/GL/GBM dev libs + a libcuda **link stub** (driver userspace, no kernel module) + pinned rustup — the container `ci.yml`'s Rust job runs in |

Registry pushes authenticate with the repo Actions secret **`REGISTRY_TOKEN`** (a PAT
with `write:package`; the login username in `docker.yml` is the token owner, not the
push actor).

## Runners

- **`ubuntu-24.04`** — the pre-existing Linux runner; runs the Rust/web/docs jobs (as
  docker containers) and the image build+push jobs.
- **`macos-arm64`** — `home-mac-mini-1` (M-series, macOS 26), a **host-mode**
  `act_runner` (upstream now ships it as `github-runner`) provisioned by
  [`scripts/ci/setup-macos-runner.sh`](https://github.com/vindeckyy/slipstream.git/src/branch/main/scripts/ci/setup-macos-runner.sh):
  rustup (+ both darwin targets for the universal xcframework), Node.js (host-mode runners
  execute JS actions via `node` from PATH — nothing auto-provisions it), the runner binary
  in `~/.local/bin`, state in `~/ci/act-runner/` (config, `.runner` registration,
  `runner.log`), kept alive by the `io.github.act_runner` **root LaunchDaemon** — it cannot
  be a user LaunchAgent: macOS Local Network privacy silently blocks LAN dials
  ("no route to host") from unbundled CLI binaries in gui/user launchd domains, while
  system daemons are exempt. Needs full **Xcode** for `xcodebuild -create-xcframework`
  (CLT alone only covers `swift build/test`); if `xcode-select` still points at CLT, the
  script auto-detects `/Applications/Xcode*.app` and bakes a `DEVELOPER_DIR` override into
  the daemon environment — no `xcode-select -s` required.

Re-provisioning (idempotent) or first-time registration from a dev box:

```sh
# token: org unom → Settings → Actions → Runners → Create new runner
ssh enricobuehler@192.168.1.135 GITHUB_ACTIONS_TOKEN=<token> bash -s \
    < scripts/ci/setup-macos-runner.sh
```

## Apple releases

`release.yml` produces the production client builds on the Mac runner. All three app
targets share the bundle ID **`io.unom.slipstream`** (one App Store listing, universal
purchase — effectively unchangeable after first submission). Signing is **not** secret-based:
the runner uses its **login keychain** directly, so install the **Developer ID Application**,
**Apple Distribution**, and (for the Mac App Store `.pkg`) **3rd Party Mac Developer
Installer** identities once via Xcode, with the WWDR intermediate present so they show as
valid. The only secrets are `ASC_API_KEY_P8`/`ASC_API_KEY_ID`/`ASC_API_ISSUER_ID` (App Store
Connect API key — notarization + TestFlight upload). Per-platform state:

- **macOS (Developer ID)** — sandboxed app (`Config/Slipstream-macOS.entitlements`) → export
  → `notarytool` → stapled `.dmg` on the GitHub release.
- **macOS (App Store)** — manual-signed archive (Apple Distribution + the *Slipstream macOS
  App Store Distribution* profile) → upload to TestFlight. App Sandbox is **mandatory** here
  and is now declared (app-sandbox + network client/server + audio-input + bluetooth/usb).
  Prereqs (one-time, Apple portal): add the **macOS platform** to the App Store Connect app
  record (universal purchase), install the Mac App Store distribution profile + the installer
  cert above. `continue-on-error` until those exist.
- **iOS** — archive + upload to TestFlight (`method: app-store-connect`,
  `destination: upload`). Crypto is declared exempt (`ITSAppUsesNonExemptEncryption`,
  `Config/Info.plist`) so builds don't stall on the compliance question.
- **tvOS** — archive + upload to TestFlight (Rust core built from tier-3 targets, nightly
  `-Zbuild-std` via `build-xcframework.sh`).

Each macOS target uses its own entitlements: `Config/Slipstream-macOS.entitlements` (App
Sandbox is macOS-only) for the macOS app, and the shared `Config/Slipstream.entitlements`
(keychain-access-groups only) for iOS/tvOS — `com.apple.security.app-sandbox` is invalid on
iOS/tvOS and would fail upload validation.

The runner needs a **release (non-beta) Xcode** — App Store processing rejects beta-SDK
builds, and a beta is unusable for the Rust side too: a newer-than-OS ld emits dylibs the
running dyld rejects ("mis-aligned LINKEDIT string pool"), killing every proc-macro build
with a misleading `E0463 can't find crate`. `build-xcframework.sh` therefore resolves
toolchains itself: non-beta Xcode for everything; with only CLT + a beta present it
builds macOS slices against CLT (packaging via any Xcode — `-create-xcframework` does no
linking) and **refuses iOS/tvOS slices** (CLT has no iOS SDK).

## Deployment

`docker.yml`'s `deploy-docs` job ships this docs site after every image push: it syncs
`compose.production.yml` to `~/slipstream-docs` on **github-actions** (the DMZ services VM
website and cms deploy to) and runs `docker compose pull && up -d` there over SSH (same
pattern and secret set as `unom/website`: `DEPLOY_HOST` / `DEPLOY_USER` / `DEPLOY_PORT` /
`DEPLOY_SSH_KEY`, the `unom-ci-deploy` key). The container binds host port **3220**;
Caddy on `github-pages-1` serves it as <https://docs.slipstream.unom.io> (vhost in
`unom/reverse-proxy`, UniFi firewall allowlist Caddy→github-actions:3220 in `vindeckyy/slipstream`
`proxmox/github-actions`). The host and the web console are NOT deployed — the console
fronts a slipstream host's management API on whatever box runs the host.

## Troubleshooting

- **Mac runner offline** — `ssh <mac> tail -50 '~/ci/act-runner/runner.log'`; restart with
  `sudo launchctl kickstart -k system/io.github.act_runner`. "no route to host" in the log
  means the daemon is running in a gui/user domain again — see the Local Network note
  above.
- **`apple.yml` fails at the xcframework step** — Xcode missing or unselected:
  `sudo xcode-select -s /Applications/Xcode.app/Contents/Developer` and accept the license
  (`sudo xcodebuild -license accept`), then re-run.
- **Rust job can't pull `slipstream-rust-ci`** — the runner host's docker daemon needs a
  `docker login github.com/vindeckyy/slipstream` if the org/registry isn't anonymously readable.
- **Stale builder image after toolchain/dep changes** — `docker.yml` re-pushes it on every
  `main` push; a manual `workflow_dispatch` of `docker.yml` forces a rebuild.
