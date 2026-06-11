#!/usr/bin/env bash
# Build SlipstreamCore.xcframework for the Apple clients — run ON A MAC with Xcode + rustup.
#
#   rustup target add aarch64-apple-darwin x86_64-apple-darwin   # + aarch64-apple-ios for iOS
#   bash scripts/build-xcframework.sh
#
# Output: clients/apple/SlipstreamCore.xcframework (consumed by clients/apple/Package.swift).
# The library is built WITH the `quic` feature (the slipstream/1 connection API), so the bundled
# header gets SLIPSTREAM_FEATURE_QUIC pre-defined — Swift sees slipstream_connect & co. unconditionally.
set -euo pipefail
cd "$(dirname "$0")/.."

TARGETS_MAC=(aarch64-apple-darwin x86_64-apple-darwin)
BUILD_IOS="${BUILD_IOS:-0}" # BUILD_IOS=1 adds iOS device + simulator slices (rustup targets aarch64-apple-ios{,-sim})

# Deployment targets must match Package.swift's platforms, or every consumer link emits
# "object file was built for newer macOS version" warnings.
for t in "${TARGETS_MAC[@]}"; do
    MACOSX_DEPLOYMENT_TARGET=14.0 cargo build --release -p slipstream-core --features quic --target "$t"
done
if [[ "$BUILD_IOS" == "1" ]]; then
    IPHONEOS_DEPLOYMENT_TARGET=17.0 cargo build --release -p slipstream-core --features quic --target aarch64-apple-ios
    IPHONEOS_DEPLOYMENT_TARGET=17.0 cargo build --release -p slipstream-core --features quic --target aarch64-apple-ios-sim
    IPHONEOS_DEPLOYMENT_TARGET=17.0 cargo build --release -p slipstream-core --features quic --target x86_64-apple-ios
fi

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

# Universal macOS static lib.
mkdir -p "$STAGE/macos"
lipo -create \
    target/aarch64-apple-darwin/release/libslipstream_core.a \
    target/x86_64-apple-darwin/release/libslipstream_core.a \
    -output "$STAGE/macos/libslipstream_core.a"

# Headers dir: the generated C header (with the quic API force-enabled) + a modulemap so
# Swift can `import SlipstreamCore`.
mkdir -p "$STAGE/include"
{
    echo "#define SLIPSTREAM_FEATURE_QUIC 1"
    cat include/slipstream_core.h
} > "$STAGE/include/slipstream_core.h"
cat > "$STAGE/include/module.modulemap" <<'EOF'
module SlipstreamCore {
    header "slipstream_core.h"
    export *
}
EOF

ARGS=(-library "$STAGE/macos/libslipstream_core.a" -headers "$STAGE/include")
if [[ "$BUILD_IOS" == "1" ]]; then
    # Universal simulator lib (arm64 Macs run arm64 sims, but generic builds link x86_64 too).
    mkdir -p "$STAGE/iossim"
    lipo -create \
        target/aarch64-apple-ios-sim/release/libslipstream_core.a \
        target/x86_64-apple-ios/release/libslipstream_core.a \
        -output "$STAGE/iossim/libslipstream_core.a"
    ARGS+=(-library target/aarch64-apple-ios/release/libslipstream_core.a -headers "$STAGE/include")
    ARGS+=(-library "$STAGE/iossim/libslipstream_core.a" -headers "$STAGE/include")
fi

# Cargo does NOT fingerprint MACOSX_DEPLOYMENT_TARGET — units cached from a build without
# it keep their old minos forever. Refuse to ship anything newer than the package floor
# (objects BELOW it, e.g. rustup's precompiled std at 11.0, are fine and unavoidable).
for obj in "$STAGE"/macos/libslipstream_core.a; do
    bad=$(otool -l "$obj" 2>/dev/null | awk '/minos/ {print $2}' | sort -uV | awk -F. '$1 > 14' | head -1)
    if [[ -n "$bad" ]]; then
        echo "ERROR: $obj contains objects built for macOS $bad (> 14.0)." >&2
        echo "Stale cache — rm -rf target/{aarch64,x86_64}-apple-darwin and rebuild." >&2
        exit 1
    fi
done

rm -rf clients/apple/SlipstreamCore.xcframework
xcodebuild -create-xcframework "${ARGS[@]}" -output clients/apple/SlipstreamCore.xcframework
echo "OK: clients/apple/SlipstreamCore.xcframework"
