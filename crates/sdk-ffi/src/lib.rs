// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

#![allow(clippy::missing_const_for_fn)]

//! `UniFFI` facade for generated Secure Tunnel SDK bindings.
//!
//! This crate intentionally exposes a small product SDK contract for Swift,
//! Kotlin, and Python rather than the internal Rust implementation surface.
//!
//! The generated `UniFFI` scaffolding emits checksum functions that are not
//! declared `const fn`, so this crate allows `clippy::missing_const_for_fn`.

mod client;
mod convert;
mod error;
mod types;
mod types_more;

pub use client::{SecureTunnelClient, SecureTunnelConnection};
pub use error::SecureTunnelError;
pub use types::{
    AccountAuthMode, AccountAuthReport, AccountAuthRequest, AccountFreshness, CacheDisposition,
    CandidateSource, Carrier, ClientConfig, CloseClassification, ConnectOptions, ConnectReport,
    DescriptorTrustAnchor, DeviceState, FallbackReason, SessionState, TransportAttemptOutcome,
    TransportAttemptReport, TransportCacheSnapshot,
};
pub use types_more::{CloseReport, DeviceAuthChallenge, DeviceAuthReport, SecureChannelArtifacts};

use crate::error::{FfiResult, IntoFfiResult};

/// Returns the stable v1 protocol identifier.
#[must_use]
pub fn protocol_id_v1() -> String {
    secure_tunnel_core::protocol_id_v1().to_owned()
}

/// Returns a validated example descriptor JSON document.
///
/// # Errors
///
/// Returns an error if the built-in example descriptor fails SDK validation.
pub fn example_descriptor_json() -> FfiResult<String> {
    secure_tunnel_sdk::BootstrapDescriptor::example_json().into_ffi()
}

/// Parses, validates, and normalizes a descriptor JSON document.
///
/// # Errors
///
/// Returns an error if `descriptor_json` is malformed or fails SDK validation.
#[allow(clippy::needless_pass_by_value)]
pub fn normalize_descriptor_json(descriptor_json: String) -> FfiResult<String> {
    let descriptor =
        secure_tunnel_sdk::BootstrapDescriptor::from_json(&descriptor_json).into_ffi()?;
    Ok(descriptor.normalized_json())
}

/// Returns a default generated-binding client configuration.
#[must_use]
pub fn default_client_config() -> ClientConfig {
    ClientConfig {
        quic_reprobe_delay_seconds: secure_tunnel_sdk::TransportPolicyConfig::default()
            .quic_reprobe_delay_seconds,
        outer_root_certificates_der: Vec::new(),
        descriptor_trust_anchors: secure_tunnel_core::example_descriptor_trust_anchors()
            .into_iter()
            .map(|anchor| DescriptorTrustAnchor {
                key_id: anchor.key_id,
                algorithm: anchor.algorithm,
                public_key: anchor.public_key,
            })
            .collect(),
        pinned_service_static_public_keys: vec![
            secure_tunnel_core::obfuscated_service_static_public_key().to_vec(),
        ],
    }
}

uniffi::include_scaffolding!("secure_tunnel_sdk_ffi");

#[cfg(test)]
mod tests {
    use super::{default_client_config, example_descriptor_json, normalize_descriptor_json};
    use crate::error::SecureTunnelError;
    use crate::types::{CandidateSource, Carrier, TransportAttemptOutcome, TransportAttemptReport};

    #[test]
    fn facade_helpers_expose_stable_sdk_defaults() {
        let config = default_client_config();
        assert_eq!(config.quic_reprobe_delay_seconds, 300);
        assert_eq!(config.descriptor_trust_anchors.len(), 1);
        assert_eq!(config.descriptor_trust_anchors[0].algorithm, "ed25519");
        assert_eq!(config.pinned_service_static_public_keys.len(), 1);
        assert_eq!(config.pinned_service_static_public_keys[0].len(), 32);

        let Ok(descriptor) = example_descriptor_json() else {
            panic!("example descriptor is valid");
        };
        let Ok(normalized) = normalize_descriptor_json(descriptor) else {
            panic!("example descriptor normalizes");
        };
        assert!(normalized.contains("secure-tunnel-v1"));
        assert!(normalized.contains("secure-tunnel-api"));
    }

    #[test]
    fn error_attempts_and_security_artifacts_remain_accessible() {
        let error = SecureTunnelError::with_attempts(
            "OuterPathFailure",
            "udp path failed",
            vec![TransportAttemptReport {
                carrier: Carrier::Quic,
                source: CandidateSource::PreferredCarrier,
                outcome: TransportAttemptOutcome::Failed,
                fallback_reason: None,
                failure_kind: Some("OuterPathFailure".to_owned()),
                failure_message: Some("udp path failed".to_owned()),
            }],
        );
        assert_eq!(error.attempts().len(), 1);

        let artifacts =
            crate::convert::security_artifacts(&secure_tunnel_sdk::SecureChannelArtifacts {
                handshake_hash: Some(vec![1, 2, 3]),
                channel_binding: Some(vec![4, 5, 6]),
                service_static_public_key: Some(vec![7; 32]),
            });
        assert_eq!(artifacts.handshake_hash, Some(vec![1, 2, 3]));
        assert_eq!(artifacts.channel_binding, Some(vec![4, 5, 6]));
        assert_eq!(artifacts.service_static_public_key, Some(vec![7; 32]));
    }
}
