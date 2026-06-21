// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! End-to-end smoke harness tests.

use secure_tunnel_harness::{SmokeScenario, run_smoke_scenario, run_smoke_scenarios};

#[tokio::test]
async fn quic_success_smoke_completes() {
    let report = run_smoke_scenario(SmokeScenario::QuicSuccess)
        .await
        .expect("QUIC smoke should pass");

    assert!(report.ok);
    assert_eq!(report.selected_carrier, secure_tunnel_sdk::Carrier::Quic);
    assert_eq!(report.fallback_reason, None);
    assert!(report.secure_ready);
    assert!(report.application_exchange);
    assert_eq!(
        report.session_state_before_close,
        secure_tunnel_sdk::SessionState::KnownDeviceAuthenticated
    );
    assert_eq!(
        report.close_final_state,
        secure_tunnel_sdk::SessionState::Closed
    );
}

#[tokio::test]
async fn wss_fallback_smoke_completes() {
    let report = run_smoke_scenario(SmokeScenario::WssFallback)
        .await
        .expect("WSS fallback smoke should pass");

    assert!(report.ok);
    assert_eq!(report.selected_carrier, secure_tunnel_sdk::Carrier::Wss);
    assert_eq!(
        report.fallback_reason,
        Some(secure_tunnel_sdk::FallbackReason::OuterQuicRejected)
    );
    assert_eq!(report.attempts.len(), 2);
}

#[tokio::test]
async fn smoke_suite_runs_all_scenarios() {
    let report = run_smoke_scenarios(&[SmokeScenario::QuicSuccess, SmokeScenario::WssFallback])
        .await
        .expect("smoke suite should pass");

    assert!(report.ok);
    assert_eq!(report.scenarios.len(), 2);
    let json = serde_json::to_string(&report).expect("report should serialize");
    assert_sanitized_smoke_json(&json);
}

fn assert_sanitized_smoke_json(json: &str) {
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
    ] {
        assert!(
            !json.contains(forbidden),
            "smoke JSON leaked forbidden fixture value or field: {forbidden}\n{json}"
        );
    }
}
