// swift-tools-version: 5.9
// SlipstreamKit — Swift wrapper around the slipstream-core C ABI (slipstream/1 client connector) plus the
// SwiftUI/VideoToolbox presentation layer. Build SlipstreamCore.xcframework first:
//   bash ../../scripts/build-xcframework.sh   (on a Mac; see README.md)
import PackageDescription

let package = Package(
    name: "SlipstreamKit",
    platforms: [.macOS(.v14), .iOS(.v17), .tvOS(.v17)],
    products: [
        .library(name: "SlipstreamKit", targets: ["SlipstreamKit"]),
        // Dependency-free foundation (stored-host model + JSON codec, settings keys, App-Group
        // constant, deep-link grammar, Live Activity attributes). A separate PRODUCT so the widget
        // extension — which must never link SlipstreamKit (Rust staticlib + presentation layer) —
        // can link this and nothing else. SlipstreamKit re-exports it (see SharedReexport.swift).
        .library(name: "SlipstreamShared", targets: ["SlipstreamShared"]),
        .executable(name: "SlipstreamClient", targets: ["SlipstreamClient"]),
    ],
    targets: [
        .binaryTarget(name: "SlipstreamCore", path: "SlipstreamCore.xcframework"),
        // No dependencies by design — an extension process links this alone.
        .target(name: "SlipstreamShared"),
        .target(
            name: "SlipstreamKit",
            dependencies: ["SlipstreamCore", "SlipstreamShared"],
            // OSS attribution shown by the app's Acknowledgements screen. Bundled here (not in the
            // app target) so it rides along via Bundle.module in both `swift build` and the Xcode
            // app, which links the SlipstreamKit product. Refresh with
            // scripts/gen-third-party-notices.sh (it copies the generated file into Resources/).
            resources: [
                .copy("Resources/THIRD-PARTY-NOTICES.txt"),
                .copy("Resources/LICENSE-MIT.txt"),
                .copy("Resources/LICENSE-APACHE.txt"),
                // Geist (SIL OFL 1.1) — the brand typeface, shared with slipstream-website.
                // Registered with Core Text at first use; see BrandFont.swift.
                .copy("Resources/Fonts"),
            ],
            linkerSettings: [
                // Rust staticlib system deps.
                .linkedFramework("Security"),
                .linkedFramework("SystemConfiguration"),
                .linkedLibrary("resolv"),
            ]
        ),
        // Development app shell (swift run SlipstreamClient): connect form → stream + input.
        // (The tvOS slide-transition package is referenced by the Xcode PROJECT only —
        // its manifest breaks SwiftPM whole-graph validation on macOS, and only the
        // Slipstream-tvOS target links it; the #if os(tvOS) import never compiles here.)
        .executableTarget(name: "SlipstreamClient", dependencies: ["SlipstreamKit"]),
        // SlipstreamCore is a direct dep too so the wire tests can name the C ABI's
        // `SlipstreamInputEvent` / `SLIPSTREAM_INPUT_KIND_*` when asserting the gamepad byte layout.
        .testTarget(
            name: "SlipstreamKitTests",
            dependencies: ["SlipstreamKit", "SlipstreamShared", "SlipstreamCore"],
            resources: [
                // PyroWave golden fixtures: host-encoded AUs + upstream-decoded reference
                // planes (regenerate with slipstream-host's `pyrowave_dump_golden` on a
                // Vulkan box — see PyroWaveDecoderTests.swift).
                .copy("PyroWaveFixtures")
            ]),
    ]
)
