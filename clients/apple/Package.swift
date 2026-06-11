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
        .executable(name: "SlipstreamClient", targets: ["SlipstreamClient"]),
    ],
    targets: [
        .binaryTarget(name: "SlipstreamCore", path: "SlipstreamCore.xcframework"),
        .target(
            name: "SlipstreamKit",
            dependencies: ["SlipstreamCore"],
            linkerSettings: [
                // Rust staticlib system deps.
                .linkedFramework("Security"),
                .linkedFramework("SystemConfiguration"),
                .linkedLibrary("resolv"),
            ]
        ),
        // Development app shell (swift run SlipstreamClient): connect form → stream + input.
        .executableTarget(name: "SlipstreamClient", dependencies: ["SlipstreamKit"]),
        .testTarget(name: "SlipstreamKitTests", dependencies: ["SlipstreamKit"]),
    ]
)
