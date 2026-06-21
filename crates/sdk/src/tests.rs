// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::executor::block_on;
use futures::pin_mut;
use futures::task::noop_waker_ref;

mod mock;

use crate::client::SecureTunnelClient;
use crate::error::{ConnectError, SdkErrorKind};
use crate::planning::connect_plan_report;
use crate::{
    BootstrapDescriptor, CacheDisposition, CancellationHandle, Carrier, ClientConfig,
    ConnectOptions, FallbackReason, SessionState, TransportAttemptOutcome,
};
use mock::MockPorts;

#[test]
fn descriptor_validation_normalizes_json() {
    let descriptor_json = result_value(BootstrapDescriptor::example_json());
    let descriptor = result_value(BootstrapDescriptor::from_json(&descriptor_json));

    assert_eq!(descriptor.environment_id(), "prod");
    assert_eq!(descriptor.service_id(), "secure-tunnel-api");
    assert!(
        descriptor
            .normalized_json()
            .contains("\"descriptor_version\":1")
    );
}

#[test]
fn invalid_descriptor_maps_to_invalid_descriptor() {
    let descriptor_json = result_value(BootstrapDescriptor::example_json()).replace(
        "\"protocol_id\":\"secure-tunnel-v1\"",
        "\"protocol_id\":\"wrong\"",
    );

    let error = error_value(BootstrapDescriptor::from_json(&descriptor_json));

    assert_eq!(error.kind(), SdkErrorKind::InvalidDescriptor);
}

#[test]
fn functional_core_plan_is_deterministic_without_io() {
    let descriptor = example_descriptor();

    let plan = result_value(connect_plan_report(&descriptor, None, 1_742_000_000));

    assert_eq!(plan.len(), 2);
    assert_eq!(plan[0].carrier, Carrier::Quic);
    assert_eq!(plan[1].carrier, Carrier::Wss);
}

#[test]
fn connect_success_returns_session_and_report() {
    let ports = Arc::new(MockPorts::quic_success());
    let client = SecureTunnelClient::with_ports(ClientConfig::default(), ports);
    let descriptor = example_descriptor();

    let outcome = result_value(block_on(
        client.connect(ConnectOptions::new(descriptor, 1_742_000_000)),
    ));

    assert_eq!(outcome.session.state(), SessionState::SecureReady);
    assert_eq!(outcome.report.selected_carrier, Carrier::Quic);
    assert_eq!(outcome.report.cache_state, CacheDisposition::LiveProbe);
    assert_eq!(outcome.report.fallback_reason, None);
    assert_eq!(outcome.report.attempts.len(), 1);
    assert_eq!(
        outcome.report.attempts[0].outcome,
        TransportAttemptOutcome::SecureReady
    );
    assert_eq!(outcome.artifacts.handshake_hash, Some(vec![0xAA, 0xBB]));
    assert_eq!(outcome.artifacts.channel_binding, Some(vec![0xCC]));
}

#[test]
fn fallback_attempt_report_preserves_outer_reason() {
    let ports = Arc::new(MockPorts::quic_fallback_then_wss_success());
    let client = SecureTunnelClient::with_ports(ClientConfig::default(), ports);
    let descriptor = example_descriptor();

    let outcome = result_value(block_on(
        client.connect(ConnectOptions::new(descriptor, 1_742_000_000)),
    ));

    assert_eq!(outcome.report.selected_carrier, Carrier::Wss);
    assert_eq!(
        outcome.report.fallback_reason,
        Some(FallbackReason::OuterPathFailure)
    );
    assert_eq!(
        outcome.report.attempts[0].outcome,
        TransportAttemptOutcome::Fallback {
            reason: FallbackReason::OuterPathFailure
        }
    );
}

#[test]
fn inner_trust_failure_maps_to_stable_error() {
    let ports = Arc::new(MockPorts::inner_trust_failure());
    let client = SecureTunnelClient::with_ports(ClientConfig::default(), ports);
    let descriptor = example_descriptor();

    let error = connect_error_value(block_on(
        client.connect(ConnectOptions::new(descriptor, 1_742_000_000)),
    ));

    assert_eq!(error.kind(), SdkErrorKind::InnerTrustFailure);
    assert_eq!(error.attempts.len(), 1);
    assert_eq!(
        error.attempts[0].outcome,
        TransportAttemptOutcome::Failed {
            kind: SdkErrorKind::InnerTrustFailure,
            message: "inner trust check failed".to_owned()
        }
    );
}

#[test]
fn cancellation_during_mock_connect_returns_cancelled() {
    let cancellation = CancellationHandle::new();
    let ports = Arc::new(MockPorts::cancel_during_quic(cancellation.clone()));
    let client = SecureTunnelClient::with_ports(ClientConfig::default(), ports);
    let descriptor = example_descriptor();
    let options = ConnectOptions::new(descriptor, 1_742_000_000).with_cancellation(cancellation);

    let error = connect_error_value(block_on(client.connect(options)));

    assert_eq!(error.kind(), SdkErrorKind::Cancelled);
    assert_eq!(error.attempts.len(), 1);
}

#[test]
fn session_send_receive_request_and_close_use_mock_transport() {
    let ports = Arc::new(MockPorts::quic_success_with_receives([
        Some(vec![0x02]),
        Some(vec![0x04]),
    ]));
    let client = SecureTunnelClient::with_ports(ClientConfig::default(), ports.clone());
    let descriptor = example_descriptor();
    let outcome = result_value(block_on(
        client.connect(ConnectOptions::new(descriptor, 1_742_000_000)),
    ));

    result_value(block_on(outcome.session.send(vec![0x01])));
    assert_eq!(
        result_value(block_on(outcome.session.receive())),
        Some(vec![0x02])
    );
    assert_eq!(
        result_value(block_on(outcome.session.request(vec![0x03]))),
        Some(vec![0x04])
    );
    let close_report = result_value(block_on(outcome.session.close(1000, true)));

    assert_eq!(ports.sent_records(), vec![vec![0x01], vec![0x03]]);
    assert_eq!(ports.close_count(), 1);
    assert_eq!(close_report.final_state, SessionState::Closed);
    assert_eq!(outcome.session.state(), SessionState::Closed);
    let error = error_value(block_on(outcome.session.send(vec![0x05])));
    assert_eq!(error.kind(), SdkErrorKind::Closed);
}

#[test]
fn dropped_pending_session_send_restores_transport() {
    let ports = Arc::new(MockPorts::quic_success_with_pending_send());
    let client = SecureTunnelClient::with_ports(ClientConfig::default(), ports.clone());
    let descriptor = example_descriptor();
    let outcome = result_value(block_on(
        client.connect(ConnectOptions::new(descriptor, 1_742_000_000)),
    ));
    {
        let send_future = outcome.session.send(vec![0x09]);
        pin_mut!(send_future);
        let mut context = Context::from_waker(noop_waker_ref());

        assert!(matches!(
            send_future.as_mut().poll(&mut context),
            Poll::Pending
        ));
    }

    result_value(block_on(outcome.session.send(vec![0x0A])));
    assert_eq!(ports.sent_records(), vec![vec![0x09], vec![0x0A]]);
    assert_eq!(outcome.session.state(), SessionState::SecureReady);
}

fn example_descriptor() -> BootstrapDescriptor {
    let descriptor_json = result_value(BootstrapDescriptor::example_json());
    result_value(BootstrapDescriptor::from_json(&descriptor_json))
}

fn result_value<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("expected Ok, got {error:?}"),
    }
}

fn error_value<T>(result: crate::SdkResult<T>) -> crate::SdkError {
    match result {
        Ok(_) => panic!("expected Err"),
        Err(error) => error,
    }
}

fn connect_error_value<T>(result: crate::ConnectResult<T>) -> ConnectError {
    match result {
        Ok(_) => panic!("expected Err"),
        Err(error) => error,
    }
}
