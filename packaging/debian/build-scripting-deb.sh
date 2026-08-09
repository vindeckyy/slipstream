#!/usr/bin/env bash
# Build the slipstream-scripting .deb — the plugin/script runner (the SDK's `slipstream-scripting`,
# built on Effect, run on bun).
#
# Runtime is BUN: the runner `import()`s the operator's `.ts` plugin/script files directly, which
# only bun can do. Like the web console, we VENDOR a bun binary into the package (bun isn't in apt),
# which makes the package per-arch (amd64/arm64), NOT `all`. Unlike the console it is NOT a Nitro
# bundle: we `bun build` the runner CLI into ONE self-contained JS (effect + the SDK inlined; the
# dynamic plugin import stays a runtime import), so there is no node_modules to ship. The host's
# slipstream-host .deb Recommends this so a default `apt install slipstream-host` pulls the runner too;
# its systemd --user unit is installed but NOT auto-enabled (the runner is inert until you add
# scripts/plugins — enable it with `systemctl --user enable --now slipstream-scripting`).
#
# Usage: VERSION=0.23.0~ci42.gdeadbee [DEB_ARCH=amd64] [BUN_BIN=/path/to/bun] bash packaging/debian/build-scripting-deb.sh
# Output: dist/slipstream-scripting_<version>_<arch>.deb
set -euo pipefail

VERSION="${VERSION:?set VERSION (e.g. 0.23.0 or 0.23.0~ci42.gdeadbee)}"
PKG="slipstream-scripting"
ROOTDIR="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOTDIR"

# Per-arch: vendor bun for the target Debian arch. Map deb arch → bun's release arch tag.
DEB_ARCH="${DEB_ARCH:-$(dpkg --print-architecture)}"
BUN_VERSION="${BUN_VERSION:-1.3.14}" # pinned bun build vendored into the package (matches build-web-deb.sh)
case "$DEB_ARCH" in
  amd64) BUN_ARCH=x64 ;;
  arm64) BUN_ARCH=aarch64 ;;
  *) echo "ERROR: unsupported DEB_ARCH=$DEB_ARCH (want amd64 or arm64)" >&2; exit 1 ;;
esac

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
SHAREDIR="$STAGE/usr/share/$PKG"
DOCDIR="$STAGE/usr/share/doc/$PKG"
LIBDIR="$STAGE/usr/lib/$PKG"

# --- build the runner bundle -------------------------------------------------
# One self-contained JS: `bun build --target=bun` inlines effect + the @slipstream/host SDK; the
# runner's dynamic `import()` of the operator's plugin files is left as a runtime import (bun keeps
# unresolvable dynamic specifiers external). `--ignore-scripts` on install: nothing needs a
# postinstall, and we skip the `prepare` codegen (it wants ../api/openapi.json — not needed here).
mkdir -p "$SHAREDIR"
(
  cd sdk
  bun install --frozen-lockfile --ignore-scripts
  bun build src/runner-cli.ts --target=bun --outfile="$SHAREDIR/runner-cli.js"
)
grep -q 'attempt=' "$SHAREDIR/runner-cli.js" \
  || { echo "ERROR: runner bundle missing the dynamic plugin import — wrong build" >&2; exit 1; }

# --- vendor the bun runtime --------------------------------------------------
# Honor a pre-fetched bun (CI may cache it) via BUN_BIN; else download the pinned release.
mkdir -p "$LIBDIR"
if [ -n "${BUN_BIN:-}" ]; then
  echo "==> vendoring bun from BUN_BIN=$BUN_BIN"
  install -m0755 "$BUN_BIN" "$LIBDIR/bun"
else
  url="https://github.com/oven-sh/bun/releases/download/bun-v${BUN_VERSION}/bun-linux-${BUN_ARCH}.zip"
  echo "==> downloading bun $BUN_VERSION ($BUN_ARCH) from $url"
  tmp="$(mktemp -d)"
  curl -fsSL "$url" -o "$tmp/bun.zip"
  unzip -q "$tmp/bun.zip" -d "$tmp"
  install -m0755 "$tmp/bun-linux-${BUN_ARCH}/bun" "$LIBDIR/bun"
  rm -rf "$tmp"
fi
"$LIBDIR/bun" --version

# --- file layout -------------------------------------------------------------
# Stable PATH-independent launcher (the systemd unit's ExecStart) — runs the bundle on vendored bun.
install -d "$STAGE/usr/bin"
cat > "$STAGE/usr/bin/slipstream-scripting" <<'WRAP'
#!/bin/sh
# The runner runs on the vendored bun (it import()s the operator's .ts plugins); bun lives privately
# under /usr/lib/slipstream-scripting so it never collides with a system-wide bun on PATH.
exec /usr/lib/slipstream-scripting/bun /usr/share/slipstream-scripting/runner-cli.js "$@"
WRAP
chmod 0755 "$STAGE/usr/bin/slipstream-scripting"
install -Dm0644 scripts/slipstream-scripting.service "$STAGE/usr/lib/systemd/user/slipstream-scripting.service"
install -Dm0644 LICENSE-MIT    "$DOCDIR/LICENSE-MIT"
install -Dm0644 LICENSE-APACHE "$DOCDIR/LICENSE-APACHE"
install -Dm0644 sdk/README.md  "$DOCDIR/README.md"

cat > "$DOCDIR/copyright" <<EOF
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: slipstream
Source: https://github.com/vindeckyy/slipstream

Files: *
Copyright: slipstream contributors
License: MIT or Apache-2.0
 Dual-licensed. Full texts in /usr/share/doc/$PKG/LICENSE-MIT and
 /usr/share/doc/$PKG/LICENSE-APACHE.
EOF
printf '%s (%s) stable; urgency=medium\n\n  * Automated build %s.\n\n -- unom <packages@unom.io>  %s\n' \
  "$PKG" "$VERSION" "$VERSION" "$(date -uR 2>/dev/null || echo 'Thu, 01 Jan 1970 00:00:00 +0000')" \
  | gzip -9n > "$DOCDIR/changelog.Debian.gz"

INSTALLED_KB="$(du -k -s "$STAGE" | cut -f1)"

install -d "$STAGE/DEBIAN"
cat > "$STAGE/DEBIAN/control" <<EOF
Package: $PKG
Version: $VERSION
Architecture: $DEB_ARCH
Maintainer: unom <packages@unom.io>
Installed-Size: $INSTALLED_KB
Section: net
Priority: optional
Homepage: https://github.com/vindeckyy/slipstream
Description: slipstream plugin/script runner (Effect SDK on bun)
 Runs a slipstream host's automation: loose scripts in ~/.config/slipstream/scripts and installed
 slipstream-plugin-* packages under ~/.config/slipstream/plugins, each supervised (Effect fibers with
 capped-jittered restart; SIGTERM shuts the whole tree down structurally so plugin finalizers run).
 Bundles its own bun runtime (no system nodejs/bun dependency).
 .
 OPT-IN: the systemd --user unit is installed but not auto-enabled (the runner is inert until you add
 scripts or plugins). A plugin auto-wires to the host's mgmt token + identity cert on the same box —
 no env editing. Enable it with: systemctl --user enable --now slipstream-scripting
EOF

cat > "$STAGE/DEBIAN/postinst" <<'EOF'
#!/bin/sh
set -e
if [ "$1" = "configure" ]; then
    echo "slipstream-scripting installed. It runs your automation — add scripts to"
    echo "    ~/.config/slipstream/scripts/  (loose .ts/.js files)"
    echo "or install plugins into ~/.config/slipstream/plugins/ (bun add slipstream-plugin-<name>),"
    echo "then enable the runner for your user:"
    echo "    systemctl --user enable --now slipstream-scripting"
fi
exit 0
EOF
chmod 0755 "$STAGE/DEBIAN/postinst"

mkdir -p dist
OUT="dist/${PKG}_${VERSION}_${DEB_ARCH}.deb"
dpkg-deb --root-owner-group --build "$STAGE" "$OUT" >/dev/null
echo "built $OUT"
dpkg-deb -I "$OUT" | sed -n 's/^/  /p' | grep -E 'Version|Installed-Size|Depends' || true
