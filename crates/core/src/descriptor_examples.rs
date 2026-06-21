// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::constants::{NOISE_SUITE_V1, PROTOCOL_ID_V1, QUIC_ALPN_V1, WSS_SUBPROTOCOL_V1};
use crate::descriptor::{
    CarrierSet, DescriptorSignature, QuicTarget, SelectionPolicy, ServiceDescriptor, TrustAnchor,
    WssTarget,
};
use crate::descriptor_auth::{
    example_trust_anchors as descriptor_example_trust_anchors, sign_example_descriptor,
};
use crate::service_key::obfuscated_service_static_public_key;
use crate::transport::CarrierKind;

/// Returns the trusted roots that authorize the built-in example descriptor.
#[must_use]
pub fn example_descriptor_trust_anchors() -> Vec<TrustAnchor> {
    descriptor_example_trust_anchors()
}

/// Returns a sample descriptor with one `QUIC` target and one `WSS` fallback.
///
/// # Panics
///
/// Panics only if the checked-in example descriptor cannot be signed with the
/// built-in local fixture root.
#[must_use]
pub fn example_service_descriptor() -> ServiceDescriptor {
    let descriptor = ServiceDescriptor {
        descriptor_version: 1,
        descriptor_serial: 1,
        not_before: "2024-01-01T00:00:00Z".to_owned(),
        not_after: "2027-01-01T00:00:00Z".to_owned(),
        environment_id: "prod".to_owned(),
        service_id: "secure-tunnel-api".to_owned(),
        service_authority: "api.example.com".to_owned(),
        protocol_id: PROTOCOL_ID_V1.to_owned(),
        noise_suite: NOISE_SUITE_V1.to_owned(),
        service_static_public_key: STANDARD.encode(obfuscated_service_static_public_key()),
        signed_descriptor_hash: String::new(),
        descriptor_signature: DescriptorSignature {
            key_id: String::new(),
            algorithm: "ed25519".to_owned(),
            signature: String::new(),
        },
        trust_anchors: Vec::new(),
        selection_policy: SelectionPolicy {
            preferred_carrier: CarrierKind::Quic,
            allow_wss_fallback: true,
        },
        carriers: CarrierSet {
            quic: Some(QuicTarget {
                connect_host: "api.example.com".to_owned(),
                port: 443,
                alpn: QUIC_ALPN_V1.to_owned(),
                sni_override: None,
            }),
            wss: Some(WssTarget {
                url: "wss://api.example.com/tunnel/v1".to_owned(),
                subprotocol: WSS_SUBPROTOCOL_V1.to_owned(),
                authority_override: None,
            }),
        },
    };
    match sign_example_descriptor(descriptor) {
        Ok(descriptor) => descriptor,
        Err(error) => panic!("example descriptor must sign: {error}"),
    }
}
