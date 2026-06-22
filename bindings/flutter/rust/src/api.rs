// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use flutter_rust_bridge::frb;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlutterCarrier {
    Quic,
    Wss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlutterAccountAuthMode {
    Fresh,
    Resume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlutterCloseClassification {
    Graceful,
    Abrupt,
    Truncated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlutterClientConfig {
    pub quic_reprobe_delay_seconds: u64,
    pub connect_timeout_ms: u64,
    pub quic_connect_timeout_ms: u64,
    pub wss_connect_timeout_ms: u64,
    pub secure_ready_timeout_ms: u64,
    pub record_read_timeout_ms: u64,
    pub record_write_timeout_ms: u64,
    pub outer_root_certificates_der: Vec<Vec<u8>>,
    pub descriptor_trust_anchors: Vec<FlutterDescriptorTrustAnchor>,
    pub pinned_service_static_public_keys: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlutterDescriptorTrustAnchor {
    pub key_id: String,
    pub algorithm: String,
    pub public_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlutterConnectOptions {
    pub descriptor_json: String,
    pub now_unix_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlutterConnectReport {
    pub selected_carrier: FlutterCarrier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlutterSecureChannelArtifacts {
    pub service_static_public_key: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlutterAccountAuthRequest {
    pub account_id: String,
    pub credential_payload: Vec<u8>,
    pub mode: FlutterAccountAuthMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlutterAccountAuthReport {
    pub account_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlutterCloseReport {
    pub classification: FlutterCloseClassification,
}

#[frb(opaque)]
pub struct SecureTunnelFlutterClient {
    client: secure_tunnel_sdk::SecureTunnelClient,
    runtime: Arc<tokio::runtime::Runtime>,
}

#[frb(opaque)]
pub struct SecureTunnelFlutterConnection {
    session: secure_tunnel_sdk::SecureTunnelSession,
    report: FlutterConnectReport,
    artifacts: FlutterSecureChannelArtifacts,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl SecureTunnelFlutterClient {
    #[frb(sync)]
    pub fn new_instance(config: FlutterClientConfig) -> Result<Self, String> {
        let client = secure_tunnel_sdk::SecureTunnelClient::new(sdk_config(config)?);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            client,
            runtime: Arc::new(runtime),
        })
    }

    pub fn connect(
        &self,
        options: FlutterConnectOptions,
    ) -> Result<SecureTunnelFlutterConnection, String> {
        let descriptor =
            secure_tunnel_sdk::BootstrapDescriptor::from_json(&options.descriptor_json)
                .map_err(|error| error.to_string())?;
        let sdk_options =
            secure_tunnel_sdk::ConnectOptions::new(descriptor, options.now_unix_seconds);
        let outcome = self
            .runtime
            .block_on(self.client.connect(sdk_options))
            .map_err(|error| error.to_string())?;
        Ok(SecureTunnelFlutterConnection {
            report: FlutterConnectReport {
                selected_carrier: carrier(outcome.report.selected_carrier),
            },
            artifacts: FlutterSecureChannelArtifacts {
                service_static_public_key: outcome.artifacts.service_static_public_key,
            },
            session: outcome.session,
            runtime: Arc::clone(&self.runtime),
        })
    }
}

impl SecureTunnelFlutterConnection {
    #[frb(sync)]
    pub fn report(&self) -> FlutterConnectReport {
        self.report.clone()
    }

    #[frb(sync)]
    pub fn security_artifacts(&self) -> FlutterSecureChannelArtifacts {
        self.artifacts.clone()
    }

    pub fn authenticate_account(
        &self,
        request: FlutterAccountAuthRequest,
    ) -> Result<FlutterAccountAuthReport, String> {
        let sdk_request = secure_tunnel_sdk::AccountAuthRequest {
            account_id: request.account_id,
            credential_payload: request.credential_payload,
            mode: account_auth_mode(request.mode),
        };
        let report = self
            .runtime
            .block_on(self.session.authenticate_account(sdk_request))
            .map_err(|error| error.to_string())?;
        Ok(FlutterAccountAuthReport {
            account_id: report.account_id,
        })
    }

    pub fn request(&self, payload: Vec<u8>) -> Result<Vec<u8>, String> {
        self.runtime
            .block_on(self.session.request(payload))
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "missing application response".to_owned())
    }

    pub fn close(&self, code: u16, drain: bool) -> Result<FlutterCloseReport, String> {
        let report = self
            .runtime
            .block_on(self.session.close(code, drain))
            .map_err(|error| error.to_string())?;
        Ok(FlutterCloseReport {
            classification: close_classification(report.classification),
        })
    }
}

#[frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

fn sdk_config(config: FlutterClientConfig) -> Result<secure_tunnel_sdk::ClientConfig, String> {
    let defaults = secure_tunnel_sdk::ClientConfig::default();
    let mut pinned = Vec::with_capacity(config.pinned_service_static_public_keys.len());
    for key in config.pinned_service_static_public_keys {
        pinned.push(
            key.try_into()
                .map_err(|_| "pinned service static public keys must be 32 bytes".to_owned())?,
        );
    }
    Ok(secure_tunnel_sdk::ClientConfig {
        transport_policy: secure_tunnel_sdk::TransportPolicyConfig {
            quic_reprobe_delay_seconds: config.quic_reprobe_delay_seconds,
            connect_timeout_ms: config.connect_timeout_ms,
            quic_connect_timeout_ms: config.quic_connect_timeout_ms,
            wss_connect_timeout_ms: config.wss_connect_timeout_ms,
            secure_ready_timeout_ms: config.secure_ready_timeout_ms,
            record_read_timeout_ms: config.record_read_timeout_ms,
            record_write_timeout_ms: config.record_write_timeout_ms,
        },
        outer_root_certificates_der: if config.outer_root_certificates_der.is_empty() {
            None
        } else {
            Some(config.outer_root_certificates_der)
        },
        wss_http_proxy: None,
        descriptor_trust_anchors: if config.descriptor_trust_anchors.is_empty() {
            defaults.descriptor_trust_anchors
        } else {
            config
                .descriptor_trust_anchors
                .into_iter()
                .map(|anchor| secure_tunnel_core::TrustAnchor {
                    key_id: anchor.key_id,
                    algorithm: anchor.algorithm,
                    public_key: anchor.public_key,
                })
                .collect()
        },
        pinned_service_static_public_keys: if pinned.is_empty() {
            defaults.pinned_service_static_public_keys
        } else {
            pinned
        },
    })
}

const fn carrier(value: secure_tunnel_sdk::Carrier) -> FlutterCarrier {
    match value {
        secure_tunnel_sdk::Carrier::Quic => FlutterCarrier::Quic,
        secure_tunnel_sdk::Carrier::Wss => FlutterCarrier::Wss,
    }
}

const fn account_auth_mode(value: FlutterAccountAuthMode) -> secure_tunnel_sdk::AccountAuthMode {
    match value {
        FlutterAccountAuthMode::Fresh => secure_tunnel_sdk::AccountAuthMode::Fresh,
        FlutterAccountAuthMode::Resume => secure_tunnel_sdk::AccountAuthMode::Resume,
    }
}

const fn close_classification(
    value: secure_tunnel_sdk::CloseClassification,
) -> FlutterCloseClassification {
    match value {
        secure_tunnel_sdk::CloseClassification::Graceful => FlutterCloseClassification::Graceful,
        secure_tunnel_sdk::CloseClassification::Abrupt => FlutterCloseClassification::Abrupt,
        secure_tunnel_sdk::CloseClassification::Truncated => FlutterCloseClassification::Truncated,
    }
}
