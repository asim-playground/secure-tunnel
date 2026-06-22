// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

use crate::{SecureTunnelStatus, SecureTunnelStringResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct GoClientConfigJson {
    #[serde(default)]
    transport_policy: Option<secure_tunnel_sdk::TransportPolicyConfig>,
    #[serde(default)]
    outer_root_certificates_der_b64: Vec<String>,
    #[serde(default)]
    descriptor_trust_anchors: Vec<secure_tunnel_core::TrustAnchor>,
    #[serde(default)]
    pinned_service_static_public_keys_b64: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct GoSecureChannelArtifactsJson {
    #[serde(rename = "handshake_hash_b64")]
    handshake_hash: Option<String>,
    #[serde(rename = "channel_binding_b64")]
    channel_binding: Option<String>,
    #[serde(rename = "service_static_public_key_b64")]
    service_static_public_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct GoAccountAuthReportJson {
    account_id: String,
    session_context_id: String,
    account_context_hash_b64: String,
    freshness: secure_tunnel_sdk::AccountFreshness,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct GoConnectErrorJson {
    kind: String,
    message: String,
    attempts: Vec<secure_tunnel_sdk::TransportAttemptReport>,
}

impl GoConnectErrorJson {
    pub(super) fn from_sdk_error(value: &secure_tunnel_sdk::SdkError) -> Self {
        Self {
            kind: format!("{:?}", value.kind()),
            message: value.message(),
            attempts: Vec::new(),
        }
    }

    pub(super) fn from_connect_error(value: &secure_tunnel_sdk::ConnectError) -> Self {
        Self {
            kind: format!("{:?}", value.kind()),
            message: value.message(),
            attempts: value.attempts.clone(),
        }
    }
}

impl GoClientConfigJson {
    pub(super) fn from_sdk_default() -> Self {
        let config = secure_tunnel_sdk::ClientConfig::default();
        Self {
            transport_policy: Some(config.transport_policy),
            outer_root_certificates_der_b64: config
                .outer_root_certificates_der
                .unwrap_or_default()
                .iter()
                .map(|value| STANDARD.encode(value))
                .collect(),
            descriptor_trust_anchors: config.descriptor_trust_anchors,
            pinned_service_static_public_keys_b64: config
                .pinned_service_static_public_keys
                .iter()
                .map(|value| STANDARD.encode(value))
                .collect(),
        }
    }

    fn into_sdk(self) -> Result<secure_tunnel_sdk::ClientConfig, String> {
        let defaults = secure_tunnel_sdk::ClientConfig::default();
        let outer_roots = decode_b64_vecs(&self.outer_root_certificates_der_b64)?;
        let service_pins = decode_service_pins(&self.pinned_service_static_public_keys_b64)?;
        Ok(secure_tunnel_sdk::ClientConfig {
            transport_policy: self.transport_policy.unwrap_or(defaults.transport_policy),
            outer_root_certificates_der: if outer_roots.is_empty() {
                None
            } else {
                Some(outer_roots)
            },
            descriptor_trust_anchors: if self.descriptor_trust_anchors.is_empty() {
                defaults.descriptor_trust_anchors
            } else {
                self.descriptor_trust_anchors
            },
            pinned_service_static_public_keys: if service_pins.is_empty() {
                defaults.pinned_service_static_public_keys
            } else {
                service_pins
            },
        })
    }
}

impl From<&secure_tunnel_sdk::SecureChannelArtifacts> for GoSecureChannelArtifactsJson {
    fn from(value: &secure_tunnel_sdk::SecureChannelArtifacts) -> Self {
        Self {
            handshake_hash: value
                .handshake_hash
                .as_ref()
                .map(|value| STANDARD.encode(value)),
            channel_binding: value
                .channel_binding
                .as_ref()
                .map(|value| STANDARD.encode(value)),
            service_static_public_key: value
                .service_static_public_key
                .as_ref()
                .map(|value| STANDARD.encode(value)),
        }
    }
}

impl From<secure_tunnel_sdk::AccountAuthReport> for GoAccountAuthReportJson {
    fn from(value: secure_tunnel_sdk::AccountAuthReport) -> Self {
        Self {
            account_id: value.account_id,
            session_context_id: value.session_context_id,
            account_context_hash_b64: STANDARD.encode(value.account_context_hash),
            freshness: value.freshness,
        }
    }
}

pub(super) fn decode_config(
    config_json: Result<String, SecureTunnelStringResult>,
) -> Result<secure_tunnel_sdk::ClientConfig, SecureTunnelStringResult> {
    let config_json = config_json?;
    serde_json::from_str::<GoClientConfigJson>(&config_json)
        .map_err(|error| {
            crate::string_result(
                SecureTunnelStatus::SecureTunnelStatusInvalidJson,
                error.to_string(),
            )
        })?
        .into_sdk()
        .map_err(|error| {
            crate::string_result(SecureTunnelStatus::SecureTunnelStatusInvalidConfig, error)
        })
}

pub(super) fn decode_transport_cache_json(
    cache_json: Option<String>,
) -> Result<Option<secure_tunnel_sdk::TransportCacheSnapshot>, SecureTunnelStringResult> {
    let Some(cache_json) = cache_json else {
        return Ok(None);
    };
    serde_json::from_str::<secure_tunnel_sdk::TransportCacheSnapshot>(&cache_json)
        .map(Some)
        .map_err(|error| {
            crate::string_result(
                SecureTunnelStatus::SecureTunnelStatusInvalidJson,
                error.to_string(),
            )
        })
}

pub(super) fn encode_json_string(
    value: impl Serialize,
    error_status: SecureTunnelStatus,
) -> SecureTunnelStringResult {
    match serde_json::to_string(&value) {
        Ok(json) => crate::string_result(SecureTunnelStatus::SecureTunnelStatusSuccess, json),
        Err(error) => crate::string_result(error_status, error.to_string()),
    }
}

fn decode_b64_vecs(values: &[String]) -> Result<Vec<Vec<u8>>, String> {
    values
        .iter()
        .map(|value| STANDARD.decode(value).map_err(|error| error.to_string()))
        .collect()
}

fn decode_service_pins(
    values: &[String],
) -> Result<Vec<secure_tunnel_core::NoisePublicKey>, String> {
    values
        .iter()
        .map(|value| {
            let bytes = STANDARD.decode(value).map_err(|error| error.to_string())?;
            bytes
                .try_into()
                .map_err(|_| "service static public key must be 32 bytes".to_owned())
        })
        .collect()
}
