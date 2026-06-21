// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! CLI smoke command tests.

use std::process::Command;

#[test]
fn cli_smoke_quic_success_outputs_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_secure-tunnel-cli"))
        .args(["smoke", "--scenario", "quic-success", "--format", "json"])
        .output()
        .expect("CLI smoke command should start");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(value["ok"], true);
    assert_eq!(value["scenarios"][0]["scenario"], "quic_success");
    assert_eq!(value["scenarios"][0]["selected_carrier"], "quic");
    assert_eq!(value["scenarios"][0]["application_exchange"], true);
    assert_sanitized_stdout(&String::from_utf8_lossy(&output.stdout));
}

#[test]
fn cli_smoke_help_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_secure-tunnel-cli"))
        .args(["smoke", "--help"])
        .output()
        .expect("CLI smoke help command should start");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("secure-tunnel-cli smoke"),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn cli_conformance_outputs_sanitized_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_secure-tunnel-cli"))
        .args([
            "conformance",
            "--scenario",
            "stale-device-challenge",
            "--format",
            "json",
        ])
        .output()
        .expect("CLI conformance command should start");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(value["ok"], true);
    assert_eq!(value["scenarios"][0]["scenario"], "stale_device_challenge");
    assert_eq!(value["scenarios"][0]["terminal_error_kind"], "auth_failure");
    assert_sanitized_stdout(&String::from_utf8_lossy(&output.stdout));
}

fn assert_sanitized_stdout(stdout: &str) {
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
            !stdout.contains(forbidden),
            "smoke CLI leaked forbidden fixture value or field: {forbidden}\n{stdout}"
        );
    }
}
