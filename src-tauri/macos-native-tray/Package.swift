// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "CodexSwitchNativeTray",
    platforms: [
        .macOS(.v12)
    ],
    products: [
        .library(
            name: "CodexSwitchNativeTray",
            type: .static,
            targets: ["CodexSwitchNativeTray"]
        )
    ],
    targets: [
        .target(name: "CodexSwitchNativeTray")
    ]
)
