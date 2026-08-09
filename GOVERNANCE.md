# Governance

Slipstream is maintained in the open through this repository. The default branch is `main`.

## Changes

Changes should arrive through a reviewed pull request when branch protection is enabled. Small
maintenance changes may be committed directly by a maintainer while the project is being prepared
for wider distribution. Every change should leave CI, documentation, and generated artifacts in a
consistent state.

## Decisions

Maintainers decide whether a change fits the supported Linux host, Android client, Steam Deck client,
and optional Moonlight compatibility scope. Design decisions belong in the issue or pull request
where the tradeoff was discussed. Security decisions and exceptions must be recorded in the
security policy or the affected code.

## Releases

Stable releases are created from annotated `vX.Y.Z` tags. Release artifacts must be built by the
repository workflow, checksummed, accompanied by attribution data, and published only after the
required checks pass. Preview builds must be labeled as previews and must not be described as stable.
