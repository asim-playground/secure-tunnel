// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::future::Future;

use super::scripted_responder::{AuthorizationMode, scripted_session_fixture};
use super::{PrototypeQuicConnector, PrototypeWssConnector};
use crate::constants::{QUIC_ALPN_V1, WSS_SUBPROTOCOL_V1};
use crate::selector::{TransportConnectors, TransportSelector};
use crate::session::CacheDisposition;
use crate::transport::{CarrierKind, FallbackReason};
use crate::{ApiError, SnowNk1ClientEvaluator, example_descriptor_trust_anchors};

#[test]
fn quic_binding_reaches_secure_ready_and_transports_application_data() {
    let fixture = scripted_session_fixture(
        CarrierKind::Quic,
        AuthorizationMode::Valid,
        vec![b"pong".to_vec()],
    );
    let quic = PrototypeQuicConnector::success(Box::new(fixture.transport));
    let wss = PrototypeWssConnector::failure(ApiError::TransportClosed);
    let evaluator = evaluator_for_descriptor(&fixture.descriptor);

    let mut selected = block_on(TransportSelector::new(300).select(
        &fixture.descriptor,
        None,
        1_742_000_000,
        TransportConnectors::new(Some(&quic), Some(&wss)),
        &evaluator,
    ))
    .expect("quic should succeed");

    assert_eq!(selected.report.carrier, CarrierKind::Quic);
    assert_eq!(selected.report.cache_state, CacheDisposition::LiveProbe);
    assert_eq!(selected.report.fallback_reason, None);
    assert_eq!(quic.call_count(), 1);
    assert_eq!(wss.call_count(), 0);

    let observations = quic.observations();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].carrier, CarrierKind::Quic);
    assert_eq!(observations[0].target_summary, "quic://api.example.com:443");
    assert_eq!(observations[0].selector_field, "alpn");
    assert_eq!(observations[0].selector_value, QUIC_ALPN_V1);
    assert_eq!(observations[0].logical_record_channels, 1);
    assert_eq!(
        observations[0].outcome,
        super::ConnectorOutcome::Established
    );

    block_on(selected.transport.send_record(b"ping")).expect("secure transport send");
    let reply = block_on(selected.transport.receive_record())
        .expect("secure transport receive")
        .expect("reply");
    assert_eq!(reply, b"pong");

    let guard = fixture.state.lock().expect("responder state lock");
    assert_eq!(guard.received_plaintexts, vec![b"ping".to_vec()]);
    assert!(guard.handshake_completed);
    drop(guard);
}

#[test]
fn quic_binding_falls_back_to_wss_when_outer_path_fails() {
    let quic = PrototypeQuicConnector::fallback(FallbackReason::OuterPathFailure);
    let fixture = scripted_session_fixture(
        CarrierKind::Wss,
        AuthorizationMode::Valid,
        vec![b"pong".to_vec()],
    );
    let wss = PrototypeWssConnector::success(Box::new(fixture.transport));
    let evaluator = evaluator_for_descriptor(&fixture.descriptor);

    let mut selected = block_on(TransportSelector::new(300).select(
        &fixture.descriptor,
        None,
        1_742_000_000,
        TransportConnectors::new(Some(&quic), Some(&wss)),
        &evaluator,
    ))
    .expect("wss fallback should succeed");

    assert_eq!(selected.report.carrier, CarrierKind::Wss);
    assert_eq!(selected.report.cache_state, CacheDisposition::LiveProbe);
    assert_eq!(
        selected.report.fallback_reason,
        Some(FallbackReason::OuterPathFailure)
    );
    assert_eq!(quic.call_count(), 1);
    assert_eq!(wss.call_count(), 1);

    let quic_observations = quic.observations();
    assert_eq!(quic_observations.len(), 1);
    assert_eq!(quic_observations[0].carrier, CarrierKind::Quic);
    assert_eq!(
        quic_observations[0].target_summary,
        "quic://api.example.com:443"
    );
    assert_eq!(quic_observations[0].selector_field, "alpn");
    assert_eq!(quic_observations[0].selector_value, QUIC_ALPN_V1);
    assert_eq!(
        quic_observations[0].outcome,
        super::ConnectorOutcome::Fallback(FallbackReason::OuterPathFailure)
    );

    let wss_observations = wss.observations();
    assert_eq!(wss_observations.len(), 1);
    assert_eq!(wss_observations[0].carrier, CarrierKind::Wss);
    assert_eq!(
        wss_observations[0].target_summary,
        "wss://api.example.com/tunnel/v1"
    );
    assert_eq!(wss_observations[0].selector_field, "subprotocol");
    assert_eq!(wss_observations[0].selector_value, WSS_SUBPROTOCOL_V1);
    assert_eq!(wss_observations[0].logical_record_channels, 1);
    assert_eq!(
        wss_observations[0].outcome,
        super::ConnectorOutcome::Established
    );

    block_on(selected.transport.send_record(b"ping")).expect("secure transport send");
    let reply = block_on(selected.transport.receive_record())
        .expect("secure transport receive")
        .expect("reply");
    assert_eq!(reply, b"pong");

    let guard = fixture.state.lock().expect("responder state lock");
    assert_eq!(guard.received_plaintexts, vec![b"ping".to_vec()]);
    assert!(guard.handshake_completed);
    drop(guard);
}

#[test]
fn quic_binding_rejects_descriptor_with_alpn_mismatch() {
    let mut quic_fixture =
        scripted_session_fixture(CarrierKind::Quic, AuthorizationMode::Valid, Vec::new());
    quic_fixture
        .descriptor
        .carriers
        .quic
        .as_mut()
        .expect("quic target")
        .alpn = "bogus-alpn".to_owned();
    let quic = PrototypeQuicConnector::success(Box::new(quic_fixture.transport));
    let wss_fixture = scripted_session_fixture(
        CarrierKind::Wss,
        AuthorizationMode::Valid,
        vec![b"pong".to_vec()],
    );
    let wss = PrototypeWssConnector::success(Box::new(wss_fixture.transport));
    let evaluator = evaluator_for_descriptor(&quic_fixture.descriptor);

    let result = block_on(TransportSelector::new(300).select(
        &quic_fixture.descriptor,
        None,
        1_742_000_000,
        TransportConnectors::new(Some(&quic), Some(&wss)),
        &evaluator,
    ));
    let Err(error) = result else {
        panic!("selector should reject a descriptor with an invalid QUIC ALPN");
    };

    assert_eq!(
        error.cause,
        ApiError::InvalidServiceDescriptor("QUIC ALPN must match the v1 descriptor value")
    );
    assert_eq!(quic.call_count(), 0);
    assert_eq!(wss.call_count(), 0);
}

#[test]
fn wss_binding_records_malformed_target_without_falling_back() {
    let mut fixture =
        scripted_session_fixture(CarrierKind::Wss, AuthorizationMode::Valid, Vec::new());
    fixture
        .descriptor
        .carriers
        .wss
        .as_mut()
        .expect("wss target")
        .subprotocol = "bogus-subprotocol".to_owned();
    let quic = PrototypeQuicConnector::fallback(FallbackReason::OuterPathFailure);
    let wss = PrototypeWssConnector::success(Box::new(fixture.transport));
    let evaluator = evaluator_for_descriptor(&fixture.descriptor);

    let result = block_on(TransportSelector::new(300).select(
        &fixture.descriptor,
        None,
        1_742_000_000,
        TransportConnectors::new(Some(&quic), Some(&wss)),
        &evaluator,
    ));
    let Err(error) = result else {
        panic!("selector should stop on a malformed WSS target");
    };

    assert_eq!(
        error.cause,
        ApiError::InvalidServiceDescriptor("WSS subprotocol must match the v1 descriptor value")
    );
    assert_eq!(quic.call_count(), 0);
    assert_eq!(wss.call_count(), 0);
}

#[test]
fn quic_binding_stops_on_inner_trust_failure_without_trying_wss() {
    let fixture = scripted_session_fixture(
        CarrierKind::Quic,
        AuthorizationMode::HandshakePayload,
        Vec::new(),
    );
    let quic = PrototypeQuicConnector::success(Box::new(fixture.transport));
    let wss = PrototypeWssConnector::failure(ApiError::TransportClosed);
    let evaluator = evaluator_for_descriptor(&fixture.descriptor);

    let result = block_on(TransportSelector::new(300).select(
        &fixture.descriptor,
        None,
        1_742_000_000,
        TransportConnectors::new(Some(&quic), Some(&wss)),
        &evaluator,
    ));
    let Err(error) = result else {
        panic!("selector should stop on inner trust failure");
    };

    assert_eq!(error.cause, ApiError::InnerTrustFailure);
    assert_eq!(quic.call_count(), 1);
    assert_eq!(wss.call_count(), 0);

    let quic_observations = quic.observations();
    assert_eq!(quic_observations.len(), 1);
    assert_eq!(
        quic_observations[0].outcome,
        super::ConnectorOutcome::Established
    );
}

fn block_on<F>(future: F) -> F::Output
where
    F: Future,
{
    futures::executor::block_on(future)
}

fn evaluator_for_descriptor(descriptor: &crate::ServiceDescriptor) -> SnowNk1ClientEvaluator {
    SnowNk1ClientEvaluator::with_pinned_trust(
        example_descriptor_trust_anchors(),
        vec![descriptor.service_static_public_key_bytes().unwrap()],
    )
}
