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
use crate::{DEVICE_KEY_SEED, HarnessResult, NOW_UNIX_SECONDS};

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
