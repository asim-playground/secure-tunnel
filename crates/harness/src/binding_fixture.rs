// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::SigningKey;
use serde::{Deserialize, Serialize};

use crate::fixture::{LocalServiceFixture, SMOKE_PING, SMOKE_PONG};
use crate::server::{QuicServer, WssServer};
use crate::{DEVICE_KEY_SEED, HarnessError, HarnessResult, NOW_UNIX_SECONDS, map_external};

/// JSON fixture consumed by generated-language smoke clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingFixtureReport {
    /// Service descriptor JSON for the loopback server.
    pub descriptor_json: String,
    /// DER-encoded outer TLS roots, base64 encoded for portable clients.
    pub outer_root_certificates_der_b64: Vec<String>,
    /// Accepted service static public keys, base64 encoded for portable clients.
    pub pinned_service_static_public_keys_b64: Vec<String>,
    /// Timestamp clients should use for deterministic descriptor validation.
    pub now_unix_seconds: u64,
    /// Request payload the local server echoes through the secure application path.
    pub smoke_ping_b64: String,
    /// Expected response payload.
    pub smoke_pong_b64: String,
}

/// JSON-friendly report produced when a Rust client consumes a binding fixture.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingFixtureClientReport {
    /// True when every client-side fixture step completed.
    pub ok: bool,
    /// Selected outer carrier.
    pub selected_carrier: secure_tunnel_sdk::Carrier,
    /// Whether secure-ready artifacts were present.
    pub secure_ready: bool,
    /// Whether account authentication completed.
    pub account_authenticated: bool,
    /// Whether the application request/response payload matched.
    pub application_exchange: bool,
    /// Session state before graceful close.
    pub session_state_before_close: secure_tunnel_sdk::SessionState,
    /// Final close state.
    pub close_final_state: secure_tunnel_sdk::SessionState,
    /// Final close classification.
    pub close_classification: secure_tunnel_sdk::CloseClassification,
    /// Sanitized transport attempts.
    pub attempts: Vec<secure_tunnel_sdk::TransportAttemptReport>,
}

/// Running local Rust server fixture for generated-language smoke clients.
pub struct BindingFixtureServer {
    report: BindingFixtureReport,
    _quic: QuicServer,
    _wss: WssServer,
}

impl BindingFixtureServer {
    /// Returns the JSON-friendly fixture report.
    #[must_use]
    pub const fn report(&self) -> &BindingFixtureReport {
        &self.report
    }
}

/// Starts a local Rust server fixture for generated SDK clients.
///
/// # Errors
///
/// Returns an error when the local service descriptor, `QUIC`, or `WSS` server
/// cannot be started.
pub async fn start_binding_fixture_server() -> HarnessResult<BindingFixtureServer> {
    let device_key = SigningKey::from_bytes(&DEVICE_KEY_SEED);
    let fixture = LocalServiceFixture::new(device_key.verifying_key().to_bytes())?;
    let wss = WssServer::start(fixture.clone()).await?;
    let quic = QuicServer::start(
        fixture.clone(),
        vec![secure_tunnel_core::QUIC_ALPN_V1.as_bytes().to_vec()],
    )?;
    let descriptor = fixture.descriptor_for_ports(quic.port(), wss.port())?;
    let report = BindingFixtureReport {
        descriptor_json: serde_json::to_string(&descriptor)?,
        outer_root_certificates_der_b64: vec![
            STANDARD.encode(quic.root_certificate_der()),
            STANDARD.encode(wss.root_certificate_der()),
        ],
        pinned_service_static_public_keys_b64: vec![STANDARD.encode(fixture.server_public_key())],
        now_unix_seconds: NOW_UNIX_SECONDS,
        smoke_ping_b64: STANDARD.encode(SMOKE_PING),
        smoke_pong_b64: STANDARD.encode(SMOKE_PONG),
    };
    Ok(BindingFixtureServer {
        report,
        _quic: quic,
        _wss: wss,
    })
}

/// Runs the Rust SDK client against a generated-language binding fixture.
///
/// # Errors
///
/// Returns an error when the fixture report is invalid, the SDK cannot connect,
/// account auth fails, the application payload mismatches, or close fails.
pub async fn run_binding_fixture_client(
    report: &BindingFixtureReport,
) -> HarnessResult<BindingFixtureClientReport> {
    let descriptor = secure_tunnel_sdk::BootstrapDescriptor::from_json(&report.descriptor_json)
        .map_err(HarnessError::Sdk)?;
    let root_certificates = decode_vecs(&report.outer_root_certificates_der_b64)?;
    let service_pins = decode_service_pins(&report.pinned_service_static_public_keys_b64)?;
    let request_payload = map_external(STANDARD.decode(&report.smoke_ping_b64))?;
    let expected_response = map_external(STANDARD.decode(&report.smoke_pong_b64))?;
    let config = secure_tunnel_sdk::ClientConfig::default()
        .with_outer_root_certificates_der(root_certificates)
        .with_descriptor_trust_anchors(secure_tunnel_core::example_descriptor_trust_anchors())
        .with_pinned_service_static_public_keys(service_pins);
    let client = secure_tunnel_sdk::SecureTunnelClient::new(config);
    let outcome = client
        .connect(secure_tunnel_sdk::ConnectOptions::new(
            descriptor,
            report.now_unix_seconds,
        ))
        .await?;
    let secure_ready = outcome
        .artifacts
        .handshake_hash
        .as_ref()
        .is_some_and(|hash| !hash.is_empty());
    let selected_carrier = outcome.report.selected_carrier;
    let attempts = outcome.report.attempts.clone();
    let session = outcome.session;

    session
        .authenticate_account(secure_tunnel_sdk::AccountAuthRequest {
            account_id: "acct-rust-binding-fixture".to_owned(),
            credential_payload: b"rust-binding-fixture-credential".to_vec(),
            mode: secure_tunnel_sdk::AccountAuthMode::Fresh,
        })
        .await?;
    let response = session
        .request(request_payload)
        .await?
        .ok_or(HarnessError::Invariant("missing app response"))?;
    let application_exchange = response == expected_response;
    if !application_exchange {
        return Err(HarnessError::Invariant("unexpected app response"));
    }
    let session_state_before_close = session.state();
    let close = session.close(1000, true).await?;

    Ok(BindingFixtureClientReport {
        ok: true,
        selected_carrier,
        secure_ready,
        account_authenticated: true,
        application_exchange,
        session_state_before_close,
        close_final_state: close.final_state,
        close_classification: close.classification,
        attempts,
    })
}

fn decode_vecs(values: &[String]) -> HarnessResult<Vec<Vec<u8>>> {
    values
        .iter()
        .map(|value| map_external(STANDARD.decode(value)))
        .collect()
}

fn decode_service_pins(
    values: &[String],
) -> HarnessResult<Vec<secure_tunnel_core::NoisePublicKey>> {
    values
        .iter()
        .map(|value| {
            let bytes = map_external(STANDARD.decode(value))?;
            bytes
                .try_into()
                .map_err(|_| HarnessError::Invariant("service static public key must be 32 bytes"))
        })
        .collect()
}
