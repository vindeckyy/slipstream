#!/usr/bin/env bash
#
# Type-check and lint slipstream's PLATFORM-GATED Rust from a dev box that is neither platform.
#
#   scripts/xcheck.sh                    # windows, clippy -D warnings — the default
#   scripts/xcheck.sh linux              # the Linux backends instead
#   scripts/xcheck.sh windows check      # cargo check instead of clippy
#   scripts/xcheck.sh linux clippy --fix # extra args are passed through to cargo
#
# (`scripts/wincheck.sh` is a compat shim for the windows half — this script's former name.)
#
# CI compiles each platform's `#[cfg(target_os = ...)]` code only on that platform, so a Windows- or
# Linux-side edit made on a Mac is otherwise unverifiable until that CI job runs (minutes) or someone
# tests on a real box. This gets the same answer in ~1 s warm, against the WORKING TREE — the
# generated workspace symlinks each crate's real directory contents, so there is nothing to keep in
# sync and no copies to go stale.
#
# Coverage is deliberately asymmetric. Windows lints ss-frame, ss-win-display, ss-capture and
# ss-vdisplay; Linux lints ss-vdisplay only, because ss-capture's Linux half needs `pipewire`, whose
# build script wants libpipewire via pkg-config. ss-frame and ss-win-display stay generated MEMBERS
# for both targets even when nothing lints them: demote either to a real path dep and its
# slipstream-core dependency resolves to the REAL crate, dragging ring and opus back into the graph —
# the exact thing the stub exists to prevent.
#
# WHY A SEPARATE WORKSPACE — the obvious in-tree command
#
#     cargo check --target x86_64-pc-windows-msvc -p ss-capture
#
# fails, and NOT because of the Windows code: it dies in build scripts that compile C for the
# target. `audiopus_sys` first (cmake wants a Visual Studio generator), then `ring` (needs an MSVC C
# compiler plus the Windows SDK headers; clang-cl and lld-link exist locally but there is no SDK).
# Both enter the graph through `slipstream-core`'s `quic` feature. Stubbing opus with a fake
# `opus.pc` gets past audiopus but not ring.
#
# The whole Windows capture stack uses exactly FOUR items from slipstream-core — `quic::HdrMeta`,
# `Mode`, `CompositorPref` and its `as_str`, all plain data (re-verify with:
# `grep -rho 'slipstream_core::[A-Za-z_:]*' crates/ss-{capture,frame,win-display,vdisplay}/src | sort -u`).
# So this script generates a ~50-line stub core carrying just those, and rustls/ring/opus never
# enter the graph at all. Every path dep that is NOT a generated member points at the REAL crate by
# absolute path, so their own relative deps and `version.workspace` inheritance still resolve.
#
# ss-encode is stubbed for the same reason and needs the same care: ss-vdisplay's admission gate
# calls exactly ONE item from it (`can_open_another_session`, admission.rs), but the real crate
# drags ffmpeg-sys — whose build script wants a Windows FFmpeg tree this box does not have. The
# stub is a `cfg(windows) -> bool`; if admission ever consults a second encoder fact, mirror it
# here or the cross-check stops covering that call.
#
# Note: `cargo fmt` needs none of this — rustfmt follows `mod`/`#[path]` without evaluating cfg, so
# it already reaches Windows-only files. Mechanical edits can be normalised with plain `cargo fmt`.
set -euo pipefail

REPO="$(cd "$(dirname "$(readlink -f "$0")")/.." && pwd)"

plat="${1:-windows}"
case "$plat" in
  windows | linux) shift || true ;;
  # No platform given: the first arg (if any) is the cargo subcommand. Default to windows, which is
  # what this script did under its former name.
  *) plat=windows ;;
esac

case "$plat" in
  windows)
    TARGET=x86_64-pc-windows-msvc
    LINT=(ss-frame ss-win-display ss-capture ss-vdisplay)
    ;;
  linux)
    TARGET=x86_64-unknown-linux-gnu
    # ss-capture is absent on purpose — see the header (libpipewire).
    LINT=(ss-vdisplay)
    ;;
esac
OUT="${XCHECK_DIR:-${WINCHECK_DIR:-$REPO/target/xcheck-$plat}}"

# The generated members: every crate we lint on EITHER target, plus the two stubs they share. A path
# dep naming one of these must stay RELATIVE (resolving to the generated member); anything else is
# rewritten to the real crate. Getting that backwards silently drags the real slipstream-core — and
# with it ring and opus — back in through ss-frame, which is the entire thing this avoids.
MEMBERS_RE='slipstream-core|ss-encode|ss-frame|ss-win-display|ss-capture|ss-vdisplay'
REAL_MEMBERS=(ss-frame ss-win-display ss-capture ss-vdisplay)

cmd="${1:-clippy}"
[ $# -gt 0 ] && shift
case "$cmd" in
  check | clippy) ;;
  *)
    echo "usage: $(basename "$0") [windows|linux] [check|clippy] [extra cargo args...]" >&2
    exit 2
    ;;
esac

if ! rustc --print target-list | grep -qx "$TARGET"; then
  echo "xcheck: rustc does not know $TARGET" >&2
  exit 1
fi
if ! rustup target list --installed 2>/dev/null | grep -qx "$TARGET"; then
  echo "xcheck: target $TARGET not installed for the active toolchain — run:" >&2
  echo "    rustup target add $TARGET" >&2
  exit 1
fi

rm -rf "$OUT"
mkdir -p "$OUT"

# Mirror the real [workspace.package] so the members' `version.workspace = true` resolves.
{
  echo '# GENERATED by scripts/xcheck.sh — do not edit; re-run the script.'
  echo '[workspace]'
  echo 'resolver = "2"'
  echo 'members = ["slipstream-core", "ss-encode", "ss-frame", "ss-win-display", "ss-capture", "ss-vdisplay"]'
  echo
  echo '[workspace.package]'
  sed -n '/^\[workspace\.package\]/,/^$/p' "$REPO/Cargo.toml" | sed '1d;/^$/d'
  # …and the real [workspace.lints.*] tables. Every crate manifest now carries
  # `[lints] workspace = true` (the workspace-wide unsafe discipline), and a member inheriting a
  # lint table the generated ROOT does not define does not merely lose the lint — cargo refuses to
  # parse the manifest at all ("error inheriting `lints` from workspace root manifest's
  # `workspace.lints`"), which broke this script outright the day that landed. Mirrored generically
  # so a new table (clippy, rustdoc, …) is picked up without touching this script again.
  echo
  awk '/^\[/ { in_lints = ($0 ~ /^\[workspace\.lints/) } in_lints' "$REPO/Cargo.toml"
} > "$OUT/Cargo.toml"

mkdir -p "$OUT/slipstream-core/src"
cat > "$OUT/slipstream-core/Cargo.toml" <<'EOF'
# GENERATED by scripts/xcheck.sh — a STUB, not the real crate.
[package]
name = "slipstream-core"
version.workspace = true
edition = "2021"
rust-version.workspace = true

[features]
default = []
# Present so dependents' `features = ["quic"]` resolves; carries no quinn/rustls/opus.
quic = []
EOF

# Field layout and derives must match the real definitions, or the cross-check goes stale exactly
# where it matters (the capture code builds HdrMeta literally and compares Modes).
cat > "$OUT/slipstream-core/src/lib.rs" <<'EOF'
//! GENERATED by scripts/xcheck.sh — a STUB of slipstream-core carrying only the two items the
//! Windows capture stack uses. Mirrors crates/slipstream-core: `Mode` from config.rs, `HdrMeta`
//! from quic/datagram.rs. If either grows a field, mirror it here.

/// Mirrors `slipstream_core::Mode`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mode {
    pub width: u32,
    pub height: u32,
    pub refresh_hz: u32,
}

/// Mirrors `slipstream_core::CompositorPref` (config.rs). ss-vdisplay matches on every variant and
/// calls `as_str`, so both the variant set and the strings must stay in step with the real enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CompositorPref {
    #[default]
    Auto,
    Kwin,
    Wlroots,
    Mutter,
    Gamescope,
}

impl CompositorPref {
    pub fn as_str(self) -> &'static str {
        match self {
            CompositorPref::Auto => "auto",
            CompositorPref::Kwin => "kwin",
            CompositorPref::Wlroots => "wlroots",
            CompositorPref::Mutter => "mutter",
            CompositorPref::Gamescope => "gamescope",
        }
    }
}

pub mod quic {
    /// Mirrors `slipstream_core::quic::HdrMeta`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
    pub struct HdrMeta {
        pub display_primaries: [[u16; 2]; 3],
        pub white_point: [u16; 2],
        pub max_display_mastering_luminance: u32,
        pub min_display_mastering_luminance: u32,
        pub max_cll: u16,
        pub max_fall: u16,
    }
}
EOF

mkdir -p "$OUT/ss-encode/src"
cat > "$OUT/ss-encode/Cargo.toml" <<'EOF'
# GENERATED by scripts/xcheck.sh — a STUB, not the real crate.
[package]
name = "ss-encode"
version.workspace = true
edition = "2021"
rust-version.workspace = true
EOF

cat > "$OUT/ss-encode/src/lib.rs" <<'EOF'
//! GENERATED by scripts/xcheck.sh — a STUB of ss-encode carrying only the one item ss-vdisplay's
//! admission gate uses. The real crate pulls ffmpeg-sys, whose build script needs a Windows FFmpeg
//! tree; nothing in the display path needs the encoder itself, only this predicate.

/// Mirrors `ss_encode::can_open_another_session` (lib.rs, `#[cfg(target_os = "windows")]`).
#[cfg(target_os = "windows")]
pub fn can_open_another_session() -> bool {
    true
}
EOF

for name in "${REAL_MEMBERS[@]}"; do
  mkdir -p "$OUT/$name"
  # Park the member paths behind a sentinel first: a plain self-substitution would be a no-op and
  # the next rule would rewrite them to absolute along with everything else.
  {
    echo "# GENERATED by scripts/xcheck.sh from crates/$name/Cargo.toml — contents are symlinks."
    sed -E "s#path = \"\.\./($MEMBERS_RE)\"#path = \"@@M@@\1\"#g; \
            s#path = \"\.\./([A-Za-z0-9_-]+)\"#path = \"$REPO/crates/\1\"#g; \
            s#path = \"@@M@@#path = \"../#g" "$REPO/crates/$name/Cargo.toml"
  } > "$OUT/$name/Cargo.toml"
  # Symlink EVERY entry, not just src/: ss-vdisplay's `wayland_scanner::generate_interfaces!` reads
  # `protocols/*.xml` relative to CARGO_MANIFEST_DIR, which is this generated member. Without the
  # protocols dir the proc macro panics and every generated Wayland type resolves to nothing — ~20
  # cascading errors that look like a code problem and are not.
  for entry in "$REPO/crates/$name"/*; do
    case "$(basename "$entry")" in
      Cargo.toml | target) continue ;;
    esac
    ln -sfn "$entry" "$OUT/$name/$(basename "$entry")"
  done
done

cd "$OUT"
for name in "${LINT[@]}"; do
  echo "=== $cmd $name ($TARGET) ==="
  if [ "$cmd" = clippy ]; then
    cargo clippy --target "$TARGET" -p "$name" --all-targets "$@" -- -D warnings
  else
    cargo check --target "$TARGET" -p "$name" --all-targets "$@"
  fi
done
