# Release process

Slipstream releases are built from GitHub Actions and published from tags.

## Before tagging

Update the version in the workspace metadata and add a matching entry under docs/releases/. Run the checks from the repository root:

~~~bash
cargo fmt --all --check
cargo test --workspace
cd web && bun install --frozen-lockfile && bun run lint && bun test && bun run build
cd ../docs-site && bun install --frozen-lockfile && bun run check:links && bun run lint && bun run build:pages
~~~

Review the generated package metadata and confirm that the release notes describe supported platforms and known limits.

## Publish

Create and push an annotated version tag:

~~~bash
git tag -a vX.Y.Z -m "Slipstream vX.Y.Z"
git push origin vX.Y.Z
~~~

The release workflow builds the host artifacts, writes an aggregate SHA-256 manifest, and creates the GitHub Release. The Pages workflow publishes the documentation when changes land on main.

## Fixing a release

Do not move an existing tag after publishing. Cut a new patch version and explain the correction in its release notes. Users can verify a downloaded artifact with:

~~~bash
sha256sum --check slipstream-*-SHA256SUMS
~~~

The checksum file and archive must report the same digest.
