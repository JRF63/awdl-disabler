// swift-tools-version: 6.2

import PackageDescription

let package = Package(
    name: "AWDLDisabler",
    platforms: [
        .macOS(.v13)
    ],
    targets: [
        .executableTarget(
            name: "AWDLDisablerApp"
        )
    ]
)
