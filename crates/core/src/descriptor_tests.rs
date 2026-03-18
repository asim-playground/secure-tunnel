// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use crate::{
    ApiError, CandidateSource, CarrierKind, FallbackReason, TransportCacheSnapshot,
    example_service_descriptor,
};

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
        next_quic_probe_after_unix_seconds: Some(2_000),
    };

    let plan = descriptor.connect_plan(Some(&cache), 1_999).unwrap();

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
        next_quic_probe_after_unix_seconds: Some(2_000),
    };

    let error = descriptor.connect_plan(Some(&cache), 1_999).unwrap_err();

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
        next_quic_probe_after_unix_seconds: Some(2_000),
    };

    let plan = descriptor.connect_plan(Some(&cache), 2_000).unwrap();

    assert_eq!(plan[0].target.carrier(), CarrierKind::Quic);
    assert_eq!(
        plan[0].source,
        CandidateSource::QuicReprobeAfterCachedFallback
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
fn noise_prologue_uses_canonical_field_order_and_length_prefixes() {
    let descriptor = example_service_descriptor();

    let prologue = descriptor.noise_prologue().unwrap();
    let expected = [
        0x00, 0x10, b's', b'e', b'c', b'u', b'r', b'e', b'-', b't', b'u', b'n', b'n', b'e', b'l',
        b'-', b'v', b'1', 0x00, 0x04, b'p', b'r', b'o', b'd', 0x00, 0x11, b's', b'e', b'c', b'u',
        b'r', b'e', b'-', b't', b'u', b'n', b'n', b'e', b'l', b'-', b'a', b'p', b'i', 0x00, 0x0f,
        b'a', b'p', b'i', b'.', b'e', b'x', b'a', b'm', b'p', b'l', b'e', b'.', b'c', b'o', b'm',
    ];

    assert_eq!(prologue, expected);
}
