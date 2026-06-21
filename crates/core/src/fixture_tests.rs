// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::fmt::Write;

use crate::{DeviceProofInput, DeviceProofPurpose, InnerChannelContext};

const VECTORS: &str = include_str!("../tests/fixtures/secure_tunnel_v1_vectors.json");

#[test]
fn fixture_vectors_match_canonical_protocol_bytes() {
    let context = fixture_context();
    assert_fixture_contains("prologue_hex", &to_hex(&context.prologue_bytes().unwrap()));

    let input = DeviceProofInput {
        noise_handshake_hash: [1_u8; 32],
        server_challenge: [2_u8; 32],
        context,
        account_context_hash: [4_u8; 32],
        device_key_id: "device-ed25519-1".to_owned(),
        purpose: DeviceProofPurpose::KnownDeviceReauth,
        expires_at_unix_ms: 1_760_000_010_000,
    };
    assert_fixture_contains(
        "known_device_reauth_proof_hex",
        &to_hex(&input.canonical_bytes().unwrap()),
    );

    let enrollment_input = DeviceProofInput {
        purpose: DeviceProofPurpose::NewDeviceEnrollment,
        ..input
    };
    assert_fixture_contains(
        "new_device_enrollment_proof_hex",
        &to_hex(&enrollment_input.canonical_bytes().unwrap()),
    );
}

fn fixture_context() -> InnerChannelContext {
    InnerChannelContext::v1(
        "secure-tunnel-api".to_owned(),
        "prod".to_owned(),
        "api.example.com".to_owned(),
        [3_u8; 32],
    )
    .unwrap()
}

fn assert_fixture_contains(key: &str, value: &str) {
    let expected = format!("\"{key}\": \"{value}\"");
    assert!(VECTORS.contains(&expected), "missing {key} vector");
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut out, "{byte:02x}").unwrap();
    }
    out
}
