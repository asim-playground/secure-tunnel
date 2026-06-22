// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::client::SecureTunnelClient;
use crate::{
    CancellationHandle, Carrier, ClientConfig, ConnectOptions, FallbackReason, SdkErrorKind,
    TransportAttemptOutcome,
};

use super::mock::MockPorts;
use super::{connect_error_value, example_descriptor, result_value};

#[test]
fn cancellation_interrupts_pending_selector() {
    let cancellation = CancellationHandle::new();
    let ports = Arc::new(MockPorts::pending_quic());
    let mut config = ClientConfig::default();
    config.transport_policy.connect_timeout_ms = 60_000;
    let client = SecureTunnelClient::with_ports(config, ports);
    let descriptor = example_descriptor();
    let options =
        ConnectOptions::new(descriptor, 1_742_000_000).with_cancellation(cancellation.clone());
    let runtime = runtime();

    let started = Instant::now();
    let error = runtime.block_on(async move {
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            cancellation.cancel();
        });
        client.connect(options).await
    });

    let error = connect_error_value(error);
    assert_eq!(error.kind(), SdkErrorKind::Cancelled);
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "cancellation must interrupt pending transport selection promptly"
    );
}

#[test]
fn secure_ready_timeout_falls_back_to_wss() {
    let ports = Arc::new(MockPorts::pending_quic_secure_ready_then_wss_success());
    let mut config = ClientConfig::default();
    config.transport_policy.secure_ready_timeout_ms = 20;
    config.transport_policy.connect_timeout_ms = 1_000;
    let client = SecureTunnelClient::with_ports(config, ports);
    let descriptor = example_descriptor();

    let outcome = result_value(
        runtime().block_on(client.connect(ConnectOptions::new(descriptor, 1_742_000_000))),
    );

    assert_eq!(outcome.report.selected_carrier, Carrier::Wss);
    assert_eq!(
        outcome.report.fallback_reason,
        Some(FallbackReason::OuterQuicClosedEarly)
    );
    assert_eq!(
        outcome.report.attempts[0].outcome,
        TransportAttemptOutcome::Fallback {
            reason: FallbackReason::OuterQuicClosedEarly
        }
    );
}

#[test]
fn connect_timeout_bounds_selector_without_cancellation() {
    let ports = Arc::new(MockPorts::pending_quic());
    let mut config = ClientConfig::default();
    config.transport_policy.connect_timeout_ms = 20;
    let client = SecureTunnelClient::with_ports(config, ports);
    let descriptor = example_descriptor();
    let started = Instant::now();

    let error = connect_error_value(
        runtime().block_on(client.connect(ConnectOptions::new(descriptor, 1_742_000_000))),
    );

    assert_eq!(error.kind(), SdkErrorKind::OuterPathFailure);
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "connect timeout must bound transport selection"
    );
}

#[test]
fn cancellation_after_quic_fallback_preserves_attempts() {
    let cancellation = CancellationHandle::new();
    let ports = Arc::new(MockPorts::quic_fallback_then_pending_wss());
    let mut config = ClientConfig::default();
    config.transport_policy.connect_timeout_ms = 60_000;
    let client = SecureTunnelClient::with_ports(config, ports);
    let descriptor = example_descriptor();
    let options =
        ConnectOptions::new(descriptor, 1_742_000_000).with_cancellation(cancellation.clone());

    let error = runtime().block_on(async move {
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            cancellation.cancel();
        });
        client.connect(options).await
    });

    let error = connect_error_value(error);
    assert_eq!(error.kind(), SdkErrorKind::Cancelled);
    assert_eq!(error.attempts.len(), 2);
    assert_quic_outer_path_fallback(&error.attempts[0]);
    assert_eq!(error.attempts[1].carrier, Carrier::Wss);
    assert_eq!(
        error.attempts[1].outcome,
        TransportAttemptOutcome::Failed {
            kind: SdkErrorKind::Cancelled,
            message: "operation cancelled".to_owned(),
        }
    );
}

#[test]
fn connect_timeout_after_quic_fallback_preserves_wss_attempt() {
    let ports = Arc::new(MockPorts::quic_fallback_then_pending_wss());
    let mut config = ClientConfig::default();
    config.transport_policy.connect_timeout_ms = 20;
    let client = SecureTunnelClient::with_ports(config, ports);
    let descriptor = example_descriptor();

    let error = connect_error_value(
        runtime().block_on(client.connect(ConnectOptions::new(descriptor, 1_742_000_000))),
    );

    assert_eq!(error.kind(), SdkErrorKind::OuterPathFailure);
    assert_eq!(error.attempts.len(), 2);
    assert_quic_outer_path_fallback(&error.attempts[0]);
    assert_eq!(error.attempts[1].carrier, Carrier::Wss);
    assert_eq!(
        error.attempts[1].outcome,
        TransportAttemptOutcome::Failed {
            kind: SdkErrorKind::OuterPathFailure,
            message: "outer `wss` path failed".to_owned(),
        }
    );
}

fn assert_quic_outer_path_fallback(attempt: &crate::TransportAttemptReport) {
    assert_eq!(attempt.carrier, Carrier::Quic);
    assert_eq!(
        attempt.outcome,
        TransportAttemptOutcome::Fallback {
            reason: FallbackReason::OuterPathFailure
        }
    );
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap_or_else(|error| panic!("test runtime builds: {error}"))
}
