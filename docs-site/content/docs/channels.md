---
title: Release channels
description: How Slipstream preview builds and tagged Linux host releases are published.
---

Slipstream currently has two public distribution states:

- **Preview:** the Android APK attached to the `android-preview` GitHub release. It is intended for
  testers and is not a stable store release.
- **Tagged release:** a `vX.Y.Z` tag builds and publishes the Linux host archive, checksum, SBOM,
  attribution file, and provenance attestation through GitHub Actions.

Android debug builds run in CI. Signed APK publication requires the protected signing environment;
store publication is a separate operator step. Steam Deck Flatpak and Decky packages can be built
from this repository, but they do not currently have a public package feed.

## Install sources

| Surface | Current source |
|---|---|
| Linux host | Source or the Linux archive attached to a tagged GitHub release |
| Android | Preview APK from [GitHub Releases](https://github.com/vindeckyy/slipstream/releases) |
| Steam Deck | Local Flatpak or Decky build from the repository |
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
   generates checksums, third-party notices, an SBOM, and a provenance attestation, then publishes
   the files to the matching GitHub Release.
4. Android signing and store promotion happen only after the protected signing credentials and store
   access are configured.

## Preview builds

Preview artifacts can change without compatibility guarantees. Keep the preview APK separate from
tagged host releases, record the exact release tag when reporting a problem, and verify the checksum
before installation.

## Versioning

The tagged release version is derived from the Git tag. Android local builds use the workspace version
unless `VERSION_NAME` is supplied. Package feeds and automatic update channels are not currently
provided by the public repository.
