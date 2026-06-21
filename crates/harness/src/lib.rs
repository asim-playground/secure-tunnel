// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Local end-to-end smoke harness for Secure Tunnel.
//!
//! The harness starts loopback `QUIC` and `WSS` services, then drives the
//! production SDK client through descriptor loading, transport selection,
//! `Secure Ready`, account auth, known-device auth, app exchange, and close.

mod fixture;
mod responder;
mod server;

use std::str::FromStr;

use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use fixture::{LocalServiceFixture, SMOKE_PING, SMOKE_PONG};
use server::{QuicServer, WssServer};

const NOW_UNIX_SECONDS: u64 = 1_742_000_000;
const NOW_UNIX_MS: u64 = 1_742_000_000_000;
const DEVICE_KEY_ID: &str = "device-ed25519-smoke";
const DEVICE_KEY_SEED: [u8; 32] = [11_u8; 32];

/// Result alias for local smoke harness operations.
pub type HarnessResult<T> = Result<T, HarnessError>;

/// Error returned by the local smoke harness.
#[derive(Debug, Error)]
pub enum HarnessError {
    /// Core protocol failure.
    #[error("{0}")]
    Core(#[from] secure_tunnel_core::ApiError),
    /// SDK facade failure after connect.
    #[error("{0}")]
    Sdk(#[from] secure_tunnel_sdk::SdkError),
    /// SDK connect failure.
    #[error("{0}")]
    Connect(#[from] secure_tunnel_sdk::ConnectError),
    /// JSON serialization failure.
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    /// External harness dependency failure.
    #[error("{0}")]
    External(Box<dyn std::error::Error + Send + Sync>),
    /// Harness invariant failure.
    #[error("{0}")]
    Invariant(&'static str),
}

impl HarnessError {
    fn external(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::External(Box::new(error))
    }
}

/// Smoke scenario to execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmokeScenario {
    /// Valid `QUIC` reaches `Secure Ready` directly.
    QuicSuccess,
    /// Broken `QUIC` falls back to valid `WSS`.
    WssFallback,
}

impl SmokeScenario {
    /// Returns the stable CLI spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::QuicSuccess => "quic-success",
            Self::WssFallback => "wss-fallback",
        }
    }
}

impl FromStr for SmokeScenario {
    type Err = HarnessError;

    fn from_str(value: &str) -> HarnessResult<Self> {
        match value {
            "quic-success" | "quic_success" => Ok(Self::QuicSuccess),
            "wss-fallback" | "wss_fallback" => Ok(Self::WssFallback),
            _ => Err(HarnessError::Invariant("unknown smoke scenario")),
        }
    }
}

/// JSON-friendly report for one smoke scenario.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmokeReport {
    /// Scenario that produced this report.
    pub scenario: SmokeScenario,
    /// True when every smoke step completed.
    pub ok: bool,
    /// Selected outer carrier.
    pub selected_carrier: secure_tunnel_sdk::Carrier,
    /// Fallback reason, when fallback occurred.
    pub fallback_reason: Option<secure_tunnel_sdk::FallbackReason>,
    /// Whether secure-ready artifacts were present.
    pub secure_ready: bool,
    /// Whether account authentication completed.
    pub account_authenticated: bool,
    /// Account freshness established by the service.
    pub account_freshness: secure_tunnel_sdk::AccountFreshness,
    /// Whether known-device authentication completed.
    pub device_authenticated: bool,
    /// Device state established by the service.
    pub device_state: secure_tunnel_sdk::DeviceState,
    /// Whether the application request/response payload matched.
    pub application_exchange: bool,
    /// Session state after known-device auth and before close.
    pub session_state_before_close: secure_tunnel_sdk::SessionState,
    /// Final close state.
    pub close_final_state: secure_tunnel_sdk::SessionState,
    /// Sanitized transport attempts.
    pub attempts: Vec<secure_tunnel_sdk::TransportAttemptReport>,
}

/// JSON-friendly report for a smoke suite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmokeSuiteReport {
    /// True when every scenario passed.
    pub ok: bool,
    /// Per-scenario reports.
    pub scenarios: Vec<SmokeReport>,
}

/// Runs a set of smoke scenarios.
///
/// # Errors
///
/// Returns an error when any scenario fails.
pub async fn run_smoke_scenarios(scenarios: &[SmokeScenario]) -> HarnessResult<SmokeSuiteReport> {
    let mut reports = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        reports.push(run_smoke_scenario(*scenario).await?);
    }
    Ok(SmokeSuiteReport {
        ok: reports.iter().all(|report| report.ok),
        scenarios: reports,
    })
}

/// Runs one local end-to-end smoke scenario.
///
/// # Errors
///
/// Returns an error when the local service, SDK connect, session protocol, app
/// exchange, or close fails.
pub async fn run_smoke_scenario(scenario: SmokeScenario) -> HarnessResult<SmokeReport> {
    let device_signing_key = SigningKey::from_bytes(&DEVICE_KEY_SEED);
    let fixture = LocalServiceFixture::new(device_signing_key.verifying_key().to_bytes())?;
    let wss = WssServer::start(fixture.clone()).await?;
    let quic_alpn = match scenario {
        SmokeScenario::QuicSuccess => secure_tunnel_core::QUIC_ALPN_V1.as_bytes().to_vec(),
        SmokeScenario::WssFallback => b"wrong-alpn".to_vec(),
    };
    let quic = QuicServer::start(fixture.clone(), vec![quic_alpn])?;
    let descriptor = fixture.descriptor_for_ports(quic.port(), wss.port())?;
    let descriptor_json = serde_json::to_string(&descriptor)?;
    let descriptor = secure_tunnel_sdk::BootstrapDescriptor::from_json(&descriptor_json)
        .map_err(HarnessError::Sdk)?;
    let config = secure_tunnel_sdk::ClientConfig::default()
        .with_outer_root_certificates_der(vec![
            quic.root_certificate_der(),
            wss.root_certificate_der(),
        ])
        .with_descriptor_trust_anchors(secure_tunnel_core::example_descriptor_trust_anchors())
        .with_pinned_service_static_public_keys(vec![fixture.server_public_key()]);
    let client = secure_tunnel_sdk::SecureTunnelClient::new(config);

    let outcome = client
        .connect(secure_tunnel_sdk::ConnectOptions::new(
            descriptor,
            NOW_UNIX_SECONDS,
        ))
        .await?;
    let secure_ready = outcome
        .artifacts
        .handshake_hash
        .as_ref()
        .is_some_and(|h| !h.is_empty());
    let selected_carrier = outcome.report.selected_carrier;
    let fallback_reason = outcome.report.fallback_reason;
    let attempts = outcome.report.attempts.clone();
    let session = outcome.session;

    let account = session
        .authenticate_account(secure_tunnel_sdk::AccountAuthRequest {
            account_id: "acct-smoke".to_owned(),
            credential_payload: b"local-smoke-credential".to_vec(),
            mode: secure_tunnel_sdk::AccountAuthMode::Fresh,
        })
        .await?;
    let challenge = session
        .begin_known_device_auth(DEVICE_KEY_ID.to_owned())
        .await?;
    let signature = device_signing_key
        .sign(&challenge.canonical_bytes)
        .to_bytes()
        .to_vec();
    let device = session
        .finish_known_device_auth(challenge, signature, NOW_UNIX_MS)
        .await?;
    let response = session
        .request(SMOKE_PING.to_vec())
        .await?
        .ok_or(HarnessError::Invariant("missing app response"))?;
    let application_exchange = response == SMOKE_PONG;
    if !application_exchange {
        return Err(HarnessError::Invariant("unexpected app response"));
    }
    let session_state_before_close = session.state();
    let close = session.close(1000, true).await?;

    Ok(SmokeReport {
        scenario,
        ok: true,
        selected_carrier,
        fallback_reason,
        secure_ready,
        account_authenticated: true,
        account_freshness: account.freshness,
        device_authenticated: true,
        device_state: device.state,
        application_exchange,
        session_state_before_close,
        close_final_state: close.final_state,
        attempts,
    })
}

fn map_external<T>(
    result: Result<T, impl std::error::Error + Send + Sync + 'static>,
) -> HarnessResult<T> {
    result.map_err(HarnessError::external)
}
