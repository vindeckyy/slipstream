---
title: Release Channels
description: How Slipstream ships — the canary (main pushes) and stable (vX.Y.Z) tracks, how to subscribe to each, and how to cut a release.
---

Slipstream ships on **two tracks**. A push to `main` that touches a platform's sources publishes a
new **canary** build for that platform (fast iteration, possibly broken) — each workflow only
rebuilds from the paths its artifact is built from, so a docs-only push publishes nothing, and two
channels can sit on different commits. A `vX.Y.Z` git tag cuts a **stable** release:
every platform is built at that one version, published to the stable channels, and all the
artifacts (`.deb`, `.rpm`, `.msix`, host installer, `.apk`/`.aab`, `.dmg`, `.ipa`, flatpak, Decky
zip, winget manifests) are attached to a single
[GitHub Release](https://github.com/vindeckyy/slipstream.git/releases).

The two tracks are **separate repos / tracks per platform**, never a shared version line — so a
stable box never gets pulled onto a canary build, and a canary box always moves forward. Pick the
track per machine; switching is a one-line change.

## Which track should I be on?

- **Canary** — dev boxes, your own test fleet, "I want the latest main build." Updates land minutes
  after a merge that touched the parts you run; some merges won't move your channel at all.
- **Stable** — anything you don't want to babysit. Only moves when a `vX.Y.Z` tag is cut.

## Subscribe — per platform

| Platform | Canary | Stable |
|---|---|---|
| **apt** (host/client) | `deb [signed-by=…] https://github.com/vindeckyy/slipstream/api/packages/unom/debian canary main` | `… debian stable main` |
| **rpm** (host) | baseurl `…/rpm/bazzite-canary` (or `fedora-44-canary`) | `…/rpm/bazzite` (or `fedora-44`) |
| **sysext** (Bazzite host) | `sudo slipstream-sysext install --channel canary` | `… install` / default (feeds `…/slipstream-sysext/f43[-canary]`) |
| **pacman** (Arch host/client) | `[slipstream-canary]` repo section | `[slipstream]` (`Server = …/api/packages/unom/arch/$repo/$arch`) |
| **Flatpak** (client) | `flatpak install --user https://flatpak.unom.io/io.unom.Slipstream.Canary.flatpakref` | `…/io.unom.Slipstream.flatpakref` |
| **Decky** (Steam Deck) | install-from-URL `…/generic/slipstream-decky/canary/slipstream.zip` | `…/slipstream-decky/latest/slipstream.zip` |
| **Windows client** (MSIX) | `…/generic/slipstream-client-windows/canary/slipstream-client-windows_x64.msix` | `…/latest/…` + the release page |
| **Windows host** (installer) | `…/generic/slipstream-host-windows/canary/slipstream-host-setup.exe` | `…/latest/…` + the release page |
| **Windows host** (winget) | — *(stable only)* | `winget install unom.SlipstreamHost` / `winget upgrade unom.SlipstreamHost`, after `winget source add -n slipstream https://winget.slipstream.unom.io -t Microsoft.Rest` |
| **Android** | Play **Internal testing** + sideload `…/generic/slipstream-android/canary/slipstream-android.apk` | Play **closed (alpha)** track + the release page |
| **Apple** (mac/iOS/tvOS) | **TestFlight** | TestFlight + a notarized `.dmg` on the release page |

The apt distribution and the rpm group are just path segments in the URL — switching tracks is a
one-line edit of `/etc/apt/sources.list.d/slipstream.list` (`stable` ↔ `canary`) or
`/etc/yum.repos.d/slipstream.repo` (`…/rpm/bazzite` ↔ `…/rpm/bazzite-canary`), then
`apt update` / `rpm-ostree upgrade`.

> The OS-package channels (apt/rpm) are how Linux hosts get canary builds — they are **not**
> attached to a canary release page. The GitHub Releases page is stable-only.

**winget is stable-only by design.** A winget manifest pins one immutable artifact per version, so
there is nothing a rolling canary alias could point at. A Windows host on the canary channel updates
by running the canary installer again.

## How a box learns about a new build

You don't have to watch the release page. The host works out how it was installed and which channel
that install follows — from the marker its package wrote (`/usr/share/slipstream/install-kind`), the
sysext's own `/etc/slipstream-sysext.conf`, or, on Windows, from the installer having put it under
`Program Files\slipstream` (with the version number itself saying which channel) — then checks a small
signed manifest for **that** channel and shows the answer in the web console's **Host → Updates**
card: the version you run, the channel you follow, and the exact command that updates this install
(or a one-click **Update now** button on Windows). See [Updating the Host](/docs/updating).

## Pin a version, or roll back

A build broke something and you want the previous one? Every channel can serve an exact version.

| How you installed | Pin / roll back |
|---|---|
| **apt** | `apt-cache madison slipstream-host` to list versions, then `sudo apt install slipstream-host=<version>`. Add `sudo apt-mark hold slipstream-host` to stay there (`apt-mark unhold` to resume). |
| **dnf** | `sudo dnf --showduplicates list slipstream` to list versions, then `sudo dnf install slipstream-<version>` (or `sudo dnf downgrade slipstream`). |
| **pacman** | Reinstall from the package cache: `sudo pacman -U /var/cache/pacman/pkg/slipstream-host-<version>-x86_64.pkg.tar.zst`, then add `IgnorePkg = slipstream-host` to `/etc/pacman.conf` so the next `-Syu` leaves it alone. |
| **Bazzite sysext** | `slipstream-sysext status` prints your feed URL; download the `slipstream-<version>-x86-64.raw` you want from it, then `sudo slipstream-sysext install --from-file slipstream-<version>-x86-64.raw`. |
| **Windows installer** | Run the older `slipstream-host-setup-<version>.exe` over the current install. |
| **Windows / winget** | `winget install unom.SlipstreamHost --version <x.y.z>` — the source serves per-version manifests. |
| **Decky plugin** | Install-from-URL with an exact version: `https://github.com/vindeckyy/slipstream/api/packages/unom/generic/slipstream-decky/<version>/slipstream.zip`. |
| **SteamOS (on-device build)** | `git -C ~/slipstream checkout v<x.y.z>` then `bash ~/slipstream/scripts/steamdeck/update.sh` (no `--pull` — that would fetch `main` again). |
| **NixOS** | `sudo nixos-rebuild switch --rollback` for the previous generation, or pin the flake input to a `v<x.y.z>` tag and rebuild. |

Downgrading the host does **not** downgrade `~/.config/slipstream` (Linux/SteamOS) or
`%ProgramData%\slipstream` (Windows), so your config, console password and paired devices carry
across in both directions.

## Cut a stable release (maintainer)

1. Make sure `main` is green.
2. (Optional) bump any user-facing version that isn't derived from the tag — the Android
   `versionName` fallback (`clients/android/app/build.gradle.kts`) is a cosmetic self-reported
   string; everything else (binaries via `SLIPSTREAM_BUILD_VERSION`, MSIX, apt/rpm, the `.dmg`, and
   the **Decky** plugin version — CI stamps it into `package.json`, where it drives the plugin's own
   [self-update check](/docs/steam-deck#updating)) derives from the tag automatically.
3. Tag and push — **one** tag releases every platform:
   ```sh
   git tag v0.2.0
   git push origin v0.2.0
   ```
4. Every platform workflow fans out, builds at `0.2.0`, publishes to its **stable** channel, and
   attaches its artifact to the `v0.2.0` GitHub Release. Concurrent attaches are safe — the shared
   `scripts/ci/github-release.{sh,ps1}` helper creates the release once and the rest reuse it.
5. **Promote the app stores manually** (CI only uploads to testing tracks — see below).

That's the whole ritual: **push a tag, done.** There is nothing else to hand-edit.

### Versioning is derived — never hand-edited

Every workflow gets its version number from one place, `scripts/ci/pf-version.{sh,ps1}`
(the pwsh twin is for the Windows runners), so the number can never drift out of sync:

- **stable** (a `vX.Y.Z` tag) → the tag version (`-rc`/`+meta` dropped where a strictly-numeric
  version is required — MSIX, the App Store marketing version).
- **canary** (a `main` push) → **exactly one minor ahead of the latest stable tag** (latest
  `v0.6.0` → canary base `0.7.0`), with each channel's own build suffix (`-ciN`, `~ciN`,
  `<major>.<minor>.<run>`, …). Cutting `v0.7.0` automatically advances canary to `0.8.0` on the
  next `main` push.

This means canary is **always ahead of stable** with zero maintenance — the old footgun where a
canary showed up on TestFlight as `0.5.0` while `0.6.0` was already published is structurally
impossible now. If you ever need the next release to be something other than the next minor (a
major bump, or a patch), just tag it — the canary base re-derives from whatever the latest tag is.

Pre-release tags work too: `v0.2.0-rc1` builds a real release (the `-rc1` suffix is dropped where a
strictly-numeric version is required — MSIX, the App Store marketing version).

### App-store promotion (manual, after the tag)

CI uploads stable to **testing** tracks only — it never auto-publishes to the public stores:

- **Apple** — the build lands in **TestFlight**. Promote to the App Store from App Store Connect
  (submit for review). The notarized `.dmg` on the release page is the direct-download path.
- **Android** — the build lands in Play's **closed (alpha)** track. Promote alpha → production in
  the Play Console when ready.

## Why two tracks (the version-shadow trap)

apt/rpm/registries serve the **highest** version to every subscriber. If a stable release landed in
the same channel as rolling main builds, every box would jump to it and get **stuck** — the rolling
`0.3.0~ciN` build never climbs above a `0.3.0` release. Separate canary/stable channels remove the
trap by construction, which is why a single `vX.Y.Z` tag can safely release the whole project at
once (the old `host-v*` / `win-v*` / `host-win-v*` tag namespaces are retired — `v*` is the only
release tag now).

## Migrating an existing box to canary

Boxes added before this split point at the current stable channels, which now only move on releases.
Point your dev fleet at **canary**:

```sh
# apt
sudo sed -i 's/ stable main/ canary main/' /etc/apt/sources.list.d/slipstream.list
sudo apt update && sudo apt upgrade

# rpm-ostree (Bazzite / Fedora)
sudo sed -i 's#/rpm/bazzite#/rpm/bazzite-canary#' /etc/yum.repos.d/slipstream.repo   # or fedora-44 → fedora-44-canary
rpm-ostree upgrade

# Flatpak (Steam Deck client)
flatpak install --user https://flatpak.unom.io/io.unom.Slipstream.Canary.flatpakref
```
