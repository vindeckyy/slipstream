---
title: Release channels
description: How Slipstream preview builds and tagged releases are published.
---

Slipstream currently has two public distribution states:

- **Public:** the Android APK attached to the `v0.23.0-public` GitHub release. It is intended for
  testers and is not a stable store release.
- **Tagged release:** a `vX.Y.Z` tag builds and publishes the Linux host archive, Steam Deck
  Flatpak and Decky packages, checksums, SBOM, attribution file, and provenance attestation through
  GitHub Actions.

Android debug builds run in CI. The preview APK is built and signed with the Android debug key and
uploaded manually. There is no public app store, Flatpak remote, or Decky package feed.

## Install sources

| Surface | Current source |
|---|---|
| Linux host | Source or the Linux archive attached to a tagged GitHub release |
| Android | Public APK from [GitHub Releases](https://github.com/vindeckyy/slipstream/releases) |
| Steam Deck | Flatpak or Decky package attached to a tagged GitHub release |
| Moonlight | Any Moonlight client when GameStream is explicitly enabled |

Keep the host on a trusted LAN or private VPN. Do not expose management or streaming ports to the
public internet.

## Cut a tagged release

1. Make sure `main` is green and the version in `Cargo.toml` matches the tag.
2. Create and push an annotated tag:

   ```sh
   git tag -a v0.23.0 -m "v0.23.0"
   git push origin v0.23.0
   ```

3. The release workflow checks that the tag points at the checked-out commit, builds the Linux host,
   Flatpak, and Decky package, generates checksums, third-party notices, an SBOM, and a provenance
   attestation, then publishes the files to the matching GitHub Release.
4. Android preview publication remains a separate manual step.

## Preview builds

Preview artifacts can change without compatibility guarantees. Keep the preview APK separate from
tagged host releases, record the exact release tag when reporting a problem, and verify the checksum
before installation.

## Versioning

The tagged release version is derived from the Git tag. Android local builds use the workspace version
unless `VERSION_NAME` is supplied. Package feeds and automatic update channels are not currently
provided by the public repository.
