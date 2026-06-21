// swift-tools-version: 5.10
//
// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

import PackageDescription

let package = Package(
    name: "SecureTunnel",
    platforms: [
        .iOS(.v16),
        .macOS(.v11),
    ],
    products: [
        .library(
            name: "SecureTunnel",
            targets: ["SecureTunnel"]
        ),
    ],
    targets: [
        .binaryTarget(
            name: "secure_tunnel_sdk_ffiFFI",
            path: "Artifacts/secure_tunnel_sdk_ffiFFI.xcframework"
        ),
        .target(
            name: "SecureTunnel",
            dependencies: ["secure_tunnel_sdk_ffiFFI"],
            path: "Sources/SecureTunnel"
        ),
        .testTarget(
            name: "SecureTunnelPackageTests",
            dependencies: ["SecureTunnel"],
            path: "Tests/SecureTunnelPackageTests",
            resources: [
                .process("Resources"),
            ]
        ),
    ]
)
