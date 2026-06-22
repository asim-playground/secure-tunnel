// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

import java.io.FileInputStream
import java.util.Base64
import java.util.Properties
import uniffi.secure_tunnel_sdk_ffi.AccountAuthMode
import uniffi.secure_tunnel_sdk_ffi.AccountAuthRequest
import uniffi.secure_tunnel_sdk_ffi.Carrier
import uniffi.secure_tunnel_sdk_ffi.ClientConfig
import uniffi.secure_tunnel_sdk_ffi.ConnectOptions
import uniffi.secure_tunnel_sdk_ffi.SecureTunnelClient
import uniffi.secure_tunnel_sdk_ffi.defaultClientConfig
import uniffi.secure_tunnel_sdk_ffi.protocolIdV1

private fun Properties.required(name: String): String =
    getProperty(name) ?: error("missing fixture property: $name")

private fun decodeBase64(value: String): ByteArray = Base64.getDecoder().decode(value)

private fun decodeMany(value: String): List<ByteArray> =
    value.split(",").filter(String::isNotEmpty).map(::decodeBase64)

fun main() {
    val propertiesPath = System.getProperty("secureTunnelFixtureProperties")
        ?: error("missing secureTunnelFixtureProperties system property")
    val fixture = Properties()
    FileInputStream(propertiesPath).use(fixture::load)

    val defaults = defaultClientConfig()
    val config = ClientConfig(
        quicReprobeDelaySeconds = 300u,
        connectTimeoutMs = defaults.connectTimeoutMs,
        quicConnectTimeoutMs = defaults.quicConnectTimeoutMs,
        wssConnectTimeoutMs = defaults.wssConnectTimeoutMs,
        secureReadyTimeoutMs = defaults.secureReadyTimeoutMs,
        recordReadTimeoutMs = defaults.recordReadTimeoutMs,
        recordWriteTimeoutMs = defaults.recordWriteTimeoutMs,
        outerRootCertificatesDer = decodeMany(fixture.required("outer_root_certificates_der_b64")),
        wssHttpProxy = null,
        descriptorTrustAnchors = defaults.descriptorTrustAnchors,
        pinnedServiceStaticPublicKeys = decodeMany(
            fixture.required("pinned_service_static_public_keys_b64"),
        ),
    )

    SecureTunnelClient(config).use { client ->
        client.connect(
            ConnectOptions(
                descriptorJson = String(
                    decodeBase64(fixture.required("descriptor_json_b64")),
                    Charsets.UTF_8,
                ),
                nowUnixSeconds = fixture.required("now_unix_seconds").toULong(),
                transportCache = null,
            ),
        ).use { connection ->
            val report = connection.report()
            check(report.selectedCarrier == Carrier.QUIC) {
                "expected QUIC, got ${report.selectedCarrier}"
            }
            val artifacts = connection.securityArtifacts()
            check(
                artifacts.serviceStaticPublicKey != null &&
                    config.pinnedServiceStaticPublicKeys.any {
                        it.contentEquals(artifacts.serviceStaticPublicKey)
                    },
            ) {
                "unexpected service static public key"
            }
            val auth = connection.authenticateAccount(
                AccountAuthRequest(
                    accountId = "kotlin-smoke",
                    credentialPayload = "credential".toByteArray(),
                    mode = AccountAuthMode.FRESH,
                ),
            )
            check(auth.accountId == "kotlin-smoke") {
                "unexpected account id: ${auth.accountId}"
            }

            val response = connection.request(decodeBase64(fixture.required("smoke_ping_b64")))
            check(response.contentEquals(decodeBase64(fixture.required("smoke_pong_b64")))) {
                "unexpected smoke response"
            }
            val close = connection.close(code = 1000u, drain = true)
            println(
                """{"language":"kotlin","protocol":"${protocolIdV1()}","carrier":"quic","close":"${close.classification}"}""",
            )
        }
    }
}
