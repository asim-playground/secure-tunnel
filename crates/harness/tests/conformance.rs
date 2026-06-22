// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! End-to-end conformance harness tests.

use secure_tunnel_harness::{ConformanceScenario, run_conformance_scenario, run_conformance_suite};

#[tokio::test]
async fn conformance_suite_runs_current_scenarios() {
    let report = run_conformance_suite()
        .await
        .expect("conformance suite should run");

    assert!(report.ok);
    let actual: Vec<ConformanceScenario> = report
        .scenarios
        .iter()
        .map(|scenario| scenario.scenario)
        .collect();
    assert_eq!(
        actual.as_slice(),
        [
            ConformanceScenario::QuicSuccess,
            ConformanceScenario::QuicRejectedWssFallback,
            ConformanceScenario::CachedQuicBadWssFirst,
            ConformanceScenario::FallbackDisabled,
            ConformanceScenario::WrongServiceStaticKeyPin,
            ConformanceScenario::WrongDescriptorTrustAnchor,
            ConformanceScenario::ExpiredDescriptor,
            ConformanceScenario::DescriptorRollback,
            ConformanceScenario::ServiceKeyRotationValid,
            ConformanceScenario::ServiceKeyRotationInvalid,
            ConformanceScenario::StaleDeviceChallenge,
            ConformanceScenario::ReplayedDeviceChallenge,
            ConformanceScenario::GracefulClose,
            ConformanceScenario::CustomCaQuicSuccess,
            ConformanceScenario::CustomCaWssSuccess,
            ConformanceScenario::CustomCaQuicRejectedWssFallback,
            ConformanceScenario::CustomCaInnerTrustFailure,
            ConformanceScenario::CustomCaWrongRootTlsFailure,
            ConformanceScenario::ProxiedWss,
        ],
    );
    let pending: Vec<&str> = report
        .pending
        .iter()
        .map(|row| row.scenario.as_str())
        .collect();
    assert_eq!(pending.as_slice(), ["abrupt-close", "truncated-close",],);
    let json = serde_json::to_string(&report).expect("report should serialize");
    assert_sanitized_json(&json);
}

#[tokio::test]
async fn stale_device_challenge_is_conformant_auth_failure() {
    let report = run_conformance_scenario(ConformanceScenario::StaleDeviceChallenge)
        .await
        .expect("stale challenge scenario should run");

    assert!(report.ok);
    assert_eq!(
        report.terminal_error_kind,
        Some(secure_tunnel_sdk::SdkErrorKind::AuthFailure)
    );
}

fn assert_sanitized_json(json: &str) {
    for forbidden in [
        "acct-smoke",
        "local-smoke-credential",
        "session-smoke",
        "device-ed25519-smoke",
        "smoke-ping",
        "smoke-pong",
        "account_id",
        "credential_payload",
        "canonical_bytes",
        "session_context_hash",
        "session_id",
        "signature",
        "service_static_public_key",
        "account_context_hash",
        "server_challenge",
        "device_public_key",
        "handshake_hash",
        "channel_binding",
    ] {
        assert!(
            !json.contains(forbidden),
            "conformance JSON leaked forbidden fixture value or field: {forbidden}\n{json}"
        );
    }
}
