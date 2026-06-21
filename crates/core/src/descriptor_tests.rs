// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use base64::Engine;

use crate::{
    ApiError, CandidateSource, CarrierKind, FallbackReason, NOISE_SUITE_V1, PRODUCT_LABEL_V1,
    PROLOGUE_DOMAIN_V1, TransportCacheSnapshot, example_descriptor_trust_anchors,
    example_service_descriptor,
};

const VALID_NOW: u64 = 1_742_000_000;

#[test]
fn connect_plan_prefers_quic_on_unknown_network() {
    let descriptor = example_service_descriptor();

    let plan = descriptor.connect_plan(None, 1_742_000_000).unwrap();

    assert_eq!(plan.len(), 2);
    assert_eq!(plan[0].target.carrier(), CarrierKind::Quic);
    assert_eq!(plan[0].source, CandidateSource::PreferredCarrier);
    assert_eq!(plan[1].target.carrier(), CarrierKind::Wss);
    assert_eq!(plan[1].source, CandidateSource::FallbackCarrier);
}

#[test]
fn connect_plan_uses_only_wss_when_quic_bad_cache_is_active() {
    let descriptor = example_service_descriptor();
    let cache = TransportCacheSnapshot {
        last_successful_carrier: Some(CarrierKind::Wss),
        last_quic_failure: Some(FallbackReason::OuterPathFailure),
        next_quic_probe_after_unix_seconds: Some(VALID_NOW + 1),
        highest_descriptor_serial: Some(1),
    };

    let plan = descriptor.connect_plan(Some(&cache), VALID_NOW).unwrap();

    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].target.carrier(), CarrierKind::Wss);
    assert_eq!(plan[0].source, CandidateSource::CachedQuicBadNetwork);
}

#[test]
fn connect_plan_fails_fast_when_cache_blocks_quic_and_fallback_is_disabled() {
    let mut descriptor = example_service_descriptor();
    descriptor.selection_policy.allow_wss_fallback = false;
    descriptor.carriers.wss = None;
    let cache = TransportCacheSnapshot {
        last_successful_carrier: None,
        last_quic_failure: Some(FallbackReason::OuterPathFailure),
        next_quic_probe_after_unix_seconds: Some(VALID_NOW + 1),
        highest_descriptor_serial: Some(1),
    };

    let error = descriptor
        .connect_plan(Some(&cache), VALID_NOW)
        .unwrap_err();

    assert_eq!(
        error,
        ApiError::TransportPlanBlocked(
            "cached QUIC-bad posture requires WSS fallback or cache expiry"
        )
    );
}

#[test]
fn connect_plan_reprobes_quic_after_cache_deadline() {
    let descriptor = example_service_descriptor();
    let cache = TransportCacheSnapshot {
        last_successful_carrier: Some(CarrierKind::Wss),
        last_quic_failure: Some(FallbackReason::OuterPathFailure),
        next_quic_probe_after_unix_seconds: Some(VALID_NOW),
        highest_descriptor_serial: Some(1),
    };

    let plan = descriptor.connect_plan(Some(&cache), VALID_NOW).unwrap();

    assert_eq!(plan[0].target.carrier(), CarrierKind::Quic);
    assert_eq!(
        plan[0].source,
        CandidateSource::QuicReprobeAfterCachedFallback
    );
}

#[test]
fn connect_plan_rejects_expired_descriptor() {
    let mut descriptor = example_service_descriptor();
    descriptor.not_after = "2024-01-02T00:00:00Z".to_owned();

    let error = descriptor.connect_plan(None, VALID_NOW).unwrap_err();

    assert_eq!(
        error,
        ApiError::InvalidServiceDescriptor("descriptor is outside its validity window")
    );
}

#[test]
fn connect_plan_rejects_serial_rollback() {
    let descriptor = example_service_descriptor();
    let cache = TransportCacheSnapshot {
        last_successful_carrier: Some(CarrierKind::Quic),
        last_quic_failure: None,
        next_quic_probe_after_unix_seconds: None,
        highest_descriptor_serial: Some(2),
    };

    let error = descriptor
        .connect_plan(Some(&cache), VALID_NOW)
        .unwrap_err();

    assert_eq!(
        error,
        ApiError::InvalidServiceDescriptor(
            "descriptor_serial is older than the cached accepted descriptor"
        )
    );
}

#[test]
fn validate_requires_wss_target_when_fallback_enabled() {
    let mut descriptor = example_service_descriptor();
    descriptor.carriers.wss = None;

    let error = descriptor.validate().unwrap_err();

    assert_eq!(
        error,
        ApiError::InvalidServiceDescriptor("allow_wss_fallback requires a WSS carrier target")
    );
}

#[test]
fn validate_requires_quic_as_preferred_carrier() {
    let mut descriptor = example_service_descriptor();
    descriptor.selection_policy.preferred_carrier = CarrierKind::Wss;

    let error = descriptor.validate().unwrap_err();

    assert_eq!(
        error,
        ApiError::InvalidServiceDescriptor("v1 requires QUIC as the preferred carrier")
    );
}

#[test]
fn validate_rejects_invalid_quic_alpn() {
    let mut descriptor = example_service_descriptor();
    descriptor.carriers.quic.as_mut().expect("quic target").alpn = "wrong".to_owned();

    let error = descriptor.validate().unwrap_err();

    assert_eq!(
        error,
        ApiError::InvalidServiceDescriptor("QUIC ALPN must match the v1 descriptor value")
    );
}

#[test]
fn validate_rejects_invalid_wss_subprotocol() {
    let mut descriptor = example_service_descriptor();
    descriptor
        .carriers
        .wss
        .as_mut()
        .expect("wss target")
        .subprotocol = "wrong".to_owned();

    let error = descriptor.validate().unwrap_err();

    assert_eq!(
        error,
        ApiError::InvalidServiceDescriptor("WSS subprotocol must match the v1 descriptor value")
    );
}

#[test]
fn validate_rejects_wss_url_without_authority() {
    let mut descriptor = example_service_descriptor();
    descriptor.carriers.wss.as_mut().expect("wss target").url = "wss:///tunnel/v1".to_owned();

    let error = descriptor.validate().unwrap_err();

    assert_eq!(
        error,
        ApiError::InvalidServiceDescriptor(
            "WSS target URL must use wss:// with a non-empty authority"
        )
    );
}

#[test]
fn validate_rejects_wss_url_with_query_but_no_authority() {
    let mut descriptor = example_service_descriptor();
    descriptor.carriers.wss.as_mut().expect("wss target").url = "wss://?q".to_owned();

    let error = descriptor.validate().unwrap_err();

    assert_eq!(
        error,
        ApiError::InvalidServiceDescriptor(
            "WSS target URL must use wss:// with a non-empty authority"
        )
    );
}

#[test]
fn validate_rejects_invalid_trust_anchor_public_key() {
    let mut descriptor = example_service_descriptor();
    descriptor.trust_anchors[0].public_key = "<base64>".to_owned();

    let error = descriptor.validate().unwrap_err();

    assert_eq!(
        error,
        ApiError::InvalidServiceDescriptor(
            "trust anchor public_key must be a valid Ed25519 verifying key"
        )
    );
}

#[test]
fn validate_rejects_invalid_service_static_public_key() {
    let mut descriptor = example_service_descriptor();
    descriptor.service_static_public_key = "<base64>".to_owned();

    let error = descriptor.validate().unwrap_err();

    assert_eq!(
        error,
        ApiError::InvalidServiceDescriptor(
            "service_static_public_key must be base64-encoded 32-byte public key"
        )
    );
}

#[test]
fn validate_rejects_invalid_signed_descriptor_hash() {
    let mut descriptor = example_service_descriptor();
    descriptor.signed_descriptor_hash = "<base64>".to_owned();

    let error = descriptor.validate().unwrap_err();

    assert_eq!(
        error,
        ApiError::InvalidServiceDescriptor(
            "signed_descriptor_hash must be base64-encoded 32-byte hash"
        )
    );
}

#[test]
fn authorize_at_accepts_signed_example_descriptor() {
    let descriptor = example_service_descriptor();

    descriptor
        .authorize_at(VALID_NOW, &example_descriptor_trust_anchors())
        .unwrap();
}

#[test]
fn authorize_at_rejects_unpinned_descriptor_root() {
    let descriptor = example_service_descriptor();

    let error = descriptor.authorize_at(VALID_NOW, &[]).unwrap_err();

    assert_eq!(
        error,
        ApiError::InvalidServiceDescriptor(
            "at least one pinned descriptor trust anchor is required"
        )
    );
}

#[test]
fn authorize_at_rejects_tampered_service_static_key() {
    let mut descriptor = example_service_descriptor();
    descriptor.service_static_public_key =
        base64::engine::general_purpose::STANDARD.encode([8_u8; 32]);

    let error = descriptor
        .authorize_at(VALID_NOW, &example_descriptor_trust_anchors())
        .unwrap_err();

    assert_eq!(
        error,
        ApiError::InvalidServiceDescriptor(
            "signed_descriptor_hash must match the canonical descriptor body"
        )
    );
}

#[test]
fn noise_prologue_uses_canonical_field_order_and_length_prefixes() {
    let descriptor = example_service_descriptor();

    let prologue = descriptor.noise_prologue().unwrap();
    let mut expected = Vec::new();
    expected.extend_from_slice(PROLOGUE_DOMAIN_V1);
    push_string(&mut expected, PRODUCT_LABEL_V1);
    expected.extend_from_slice(&1_u16.to_be_bytes());
    push_string(&mut expected, "secure-tunnel-api");
    push_string(&mut expected, "prod");
    push_string(&mut expected, "api.example.com");
    expected.extend_from_slice(&descriptor.signed_descriptor_hash_bytes().unwrap());
    push_string(&mut expected, NOISE_SUITE_V1);

    assert_eq!(prologue, expected);
}

fn push_string(out: &mut Vec<u8>, value: &str) {
    let len = match u16::try_from(value.len()) {
        Ok(len) => len,
        Err(error) => panic!("test string should fit in u16: {error}"),
    };
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}
