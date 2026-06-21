// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

import Foundation

struct BindingFixture: Decodable {
    let descriptorJson: String
    let outerRootCertificatesDerB64: [String]
    let pinnedServiceStaticPublicKeysB64: [String]
    let nowUnixSeconds: UInt64
    let smokePingB64: String
    let smokePongB64: String
}

enum SmokeError: Error {
    case usage
    case invalidBase64(String)
    case unexpectedCarrier(Carrier)
    case unexpectedAccount(String)
    case unexpectedResponse
}

func decodeBase64(_ value: String) throws -> Data {
    guard let data = Data(base64Encoded: value) else {
        throw SmokeError.invalidBase64(value)
    }
    return data
}

@main
struct SmokeMain {
    static func main() throws {
        guard CommandLine.arguments.count == 2 else {
            throw SmokeError.usage
        }

        let fixtureUrl = URL(fileURLWithPath: CommandLine.arguments[1])
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        let fixture = try decoder.decode(
            BindingFixture.self,
            from: Data(contentsOf: fixtureUrl)
        )

        let defaults = defaultClientConfig()
        let config = ClientConfig(
            quicReprobeDelaySeconds: 300,
            outerRootCertificatesDer: try fixture.outerRootCertificatesDerB64.map(decodeBase64),
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
        let report = connection.report()
        guard report.selectedCarrier == .quic else {
            throw SmokeError.unexpectedCarrier(report.selectedCarrier)
        }
        let artifacts = connection.securityArtifacts()
        guard let serviceKey = artifacts.serviceStaticPublicKey,
              config.pinnedServiceStaticPublicKeys.contains(serviceKey)
        else {
            throw SmokeError.invalidBase64("service_static_public_key")
        }

        let auth = try connection.authenticateAccount(
            request: AccountAuthRequest(
                accountId: "swift-smoke",
                credentialPayload: Data("credential".utf8),
                mode: .fresh
            )
        )
        guard auth.accountId == "swift-smoke" else {
            throw SmokeError.unexpectedAccount(auth.accountId)
        }

        let response = try connection.request(payload: try decodeBase64(fixture.smokePingB64))
        guard response == (try decodeBase64(fixture.smokePongB64)) else {
            throw SmokeError.unexpectedResponse
        }
        let close = try connection.close(code: 1000, drain: true)
        print(
            #"{"language":"swift","protocol":"\#(protocolIdV1())","carrier":"quic","close":"\#(close.classification)"}"#
        )
    }
}
