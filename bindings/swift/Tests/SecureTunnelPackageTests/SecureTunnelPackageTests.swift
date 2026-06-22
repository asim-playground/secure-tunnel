// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

import Foundation
import SecureTunnel
import XCTest

private struct BindingFixture: Decodable {
    let descriptorJson: String
    let outerRootCertificatesDerB64: [String]
    let pinnedServiceStaticPublicKeysB64: [String]
    let nowUnixSeconds: UInt64
    let smokePingB64: String
    let smokePongB64: String
}

final class SecureTunnelPackageTests: XCTestCase {
    func testPackageMetadata() {
        XCTAssertEqual(SecureTunnelPackage.name, "SecureTunnel")
        XCTAssertEqual(protocolIdV1(), "secure-tunnel-v1")
    }

    func testPackageSessionSmoke() throws {
        let fixture = try loadFixture()
        let defaults = defaultClientConfig()
        let config = ClientConfig(
            quicReprobeDelaySeconds: 300,
            connectTimeoutMs: defaults.connectTimeoutMs,
            quicConnectTimeoutMs: defaults.quicConnectTimeoutMs,
            wssConnectTimeoutMs: defaults.wssConnectTimeoutMs,
            secureReadyTimeoutMs: defaults.secureReadyTimeoutMs,
            recordReadTimeoutMs: defaults.recordReadTimeoutMs,
            recordWriteTimeoutMs: defaults.recordWriteTimeoutMs,
            outerRootCertificatesDer: try fixture.outerRootCertificatesDerB64.map(decodeBase64),
            wssHttpProxy: nil,
            descriptorTrustAnchors: defaults.descriptorTrustAnchors,
            pinnedServiceStaticPublicKeys: try fixture.pinnedServiceStaticPublicKeysB64
                .map(decodeBase64)
        )
        let client = try SecureTunnelClient(config: config)
        let connection = try client.connect(
            options: ConnectOptions(
                descriptorJson: fixture.descriptorJson,
                nowUnixSeconds: fixture.nowUnixSeconds,
                transportCache: nil
            )
        )

        XCTAssertEqual(connection.report().selectedCarrier, .quic)
        let artifacts = connection.securityArtifacts()
        XCTAssertNotNil(artifacts.handshakeHash)
        XCTAssertTrue(
            artifacts.serviceStaticPublicKey
                .map(config.pinnedServiceStaticPublicKeys.contains) ?? false
        )

        let auth = try connection.authenticateAccount(
            request: AccountAuthRequest(
                accountId: "swift-ios-simulator-smoke",
                credentialPayload: Data("credential".utf8),
                mode: .fresh
            )
        )
        XCTAssertEqual(auth.accountId, "swift-ios-simulator-smoke")

        let response = try connection.request(payload: try decodeBase64(fixture.smokePingB64))
        XCTAssertEqual(response, try decodeBase64(fixture.smokePongB64))
        XCTAssertEqual(try connection.close(code: 1000, drain: true).classification, .graceful)
    }

    private func loadFixture() throws -> BindingFixture {
        let url = try XCTUnwrap(
            Bundle.module.url(forResource: "binding-fixture", withExtension: "json")
        )
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try decoder.decode(BindingFixture.self, from: Data(contentsOf: url))
    }

    private func decodeBase64(_ value: String) throws -> Data {
        try XCTUnwrap(Data(base64Encoded: value), "invalid base64 fixture value")
    }
}
