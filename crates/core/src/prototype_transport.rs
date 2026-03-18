// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Test-only prototype QUIC and WSS transport bindings.
//!
//! This module keeps the task-00000012 prototype local to `crates/core` by
//! combining the real `SnowNxClientEvaluator` with in-memory carrier adapters
//! that validate carrier-specific selector values, record connection metrics,
//! and expose one bidirectional framed channel per successful carrier.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::Signer;
use snow::TransportState;

use crate::constants::{NOISE_SUITE_V1, QUIC_ALPN_V1, WSS_SUBPROTOCOL_V1};
use crate::descriptor::ServiceDescriptor;
use crate::descriptor::example_service_descriptor;
use crate::error::{ApiError, ApiResult};
use crate::session::CloseDirective;
use crate::transport::{
    BoxFuture, CarrierConnector, CarrierKind, FallbackReason, FramedDuplex, TransportTarget,
};
use crate::trust::ServerKeyAuthorizationV1;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthorizationMode {
    Valid,
    BadSignature,
    WrongServiceId,
}

struct PrototypeSessionFixture {
    descriptor: ServiceDescriptor,
    transport: ScriptedNxResponderTransport,
    state: Arc<Mutex<ResponderState>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConnectorObservation {
    carrier: CarrierKind,
    target_summary: String,
    selector_field: &'static str,
    selector_value: String,
    logical_record_channels: usize,
    outcome: ConnectorOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConnectorOutcome {
    Established,
    Fallback(FallbackReason),
    Failed(ApiError),
}

enum ConnectorPlan {
    Ready {
        transport: Option<Box<dyn FramedDuplex>>,
    },
    Fallback(FallbackReason),
    Failure(ApiError),
}

struct PrototypeCarrierConnector {
    carrier: CarrierKind,
    plan: Mutex<Option<ConnectorPlan>>,
    observations: Arc<Mutex<Vec<ConnectorObservation>>>,
}

impl PrototypeCarrierConnector {
    fn ready(carrier: CarrierKind, transport: Box<dyn FramedDuplex>) -> Self {
        Self {
            carrier,
            plan: Mutex::new(Some(ConnectorPlan::Ready {
                transport: Some(transport),
            })),
            observations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn fallback(carrier: CarrierKind, reason: FallbackReason) -> Self {
        Self {
            carrier,
            plan: Mutex::new(Some(ConnectorPlan::Fallback(reason))),
            observations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn failure(carrier: CarrierKind, error: ApiError) -> Self {
        Self {
            carrier,
            plan: Mutex::new(Some(ConnectorPlan::Failure(error))),
            observations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn call_count(&self) -> usize {
        self.observations
            .lock()
            .expect("observation lock poisoned")
            .len()
    }

    fn observations(&self) -> Vec<ConnectorObservation> {
        self.observations
            .lock()
            .expect("observation lock poisoned")
            .clone()
    }

    fn connect<'a>(
        &'a self,
        target: &'a TransportTarget,
    ) -> BoxFuture<'a, ApiResult<Box<dyn FramedDuplex>>> {
        let attempt = ConnectorAttempt::new(self.carrier, target);

        Box::pin(async move { self.connect_attempt(&attempt, target) })
    }

    fn connect_attempt(
        &self,
        attempt: &ConnectorAttempt,
        target: &TransportTarget,
    ) -> ApiResult<Box<dyn FramedDuplex>> {
        if target.carrier() != self.carrier {
            return Err(self.observe_error(
                attempt,
                ApiError::TransportSelectorInvariant(
                    "prototype connector received the wrong carrier target",
                ),
            ));
        }

        validate_target(target).map_err(|error| self.observe_error(attempt, error))?;

        match self.take_plan() {
            ConnectorPlan::Ready { mut transport } => {
                let transport = transport
                    .take()
                    .ok_or(ApiError::TransportClosed)
                    .map_err(|error| self.observe_error(attempt, error))?;
                self.observe_success(attempt, 1);
                Ok(transport)
            }
            ConnectorPlan::Fallback(reason) => {
                Err(self.observe_error(attempt, ApiError::TransportFallback(reason)))
            }
            ConnectorPlan::Failure(error) => Err(self.observe_error(attempt, error)),
        }
    }

    fn take_plan(&self) -> ConnectorPlan {
        self.plan
            .lock()
            .expect("plan lock poisoned")
            .take()
            .unwrap_or(ConnectorPlan::Failure(ApiError::TransportClosed))
    }

    fn observe_success(&self, attempt: &ConnectorAttempt, logical_record_channels: usize) {
        self.record_attempt(
            attempt.observation(logical_record_channels, ConnectorOutcome::Established),
        );
    }

    fn observe_error(&self, attempt: &ConnectorAttempt, error: ApiError) -> ApiError {
        self.record_attempt(attempt.observation(0, connector_outcome_for_error(&error)));
        error
    }

    fn record_attempt(&self, observation: ConnectorObservation) {
        record_observation(&self.observations, observation);
    }
}

fn record_observation(
    observations: &Arc<Mutex<Vec<ConnectorObservation>>>,
    observation: ConnectorObservation,
) {
    observations
        .lock()
        .expect("observation lock poisoned")
        .push(observation);
}

fn connector_observation(
    carrier: CarrierKind,
    target_summary: String,
    selector_field: &'static str,
    selector_value: String,
    logical_record_channels: usize,
    outcome: ConnectorOutcome,
) -> ConnectorObservation {
    ConnectorObservation {
        carrier,
        target_summary,
        selector_field,
        selector_value,
        logical_record_channels,
        outcome,
    }
}

fn connector_outcome_for_error(error: &ApiError) -> ConnectorOutcome {
    match error {
        ApiError::TransportFallback(reason) => ConnectorOutcome::Fallback(*reason),
        _ => ConnectorOutcome::Failed(error.clone()),
    }
}

struct ConnectorAttempt {
    carrier: CarrierKind,
    target_summary: String,
    selector_field: &'static str,
    selector_value: String,
}

impl ConnectorAttempt {
    fn new(carrier: CarrierKind, target: &TransportTarget) -> Self {
        let (selector_field, selector_value) = selector_metadata(target);

        Self {
            carrier,
            target_summary: target_summary(target),
            selector_field,
            selector_value,
        }
    }

    fn observation(
        &self,
        logical_record_channels: usize,
        outcome: ConnectorOutcome,
    ) -> ConnectorObservation {
        connector_observation(
            self.carrier,
            self.target_summary.clone(),
            self.selector_field,
            self.selector_value.clone(),
            logical_record_channels,
            outcome,
        )
    }
}

fn target_summary(target: &TransportTarget) -> String {
    match target {
        TransportTarget::Quic(quic) => format!("quic://{}:{}", quic.connect_host, quic.port),
        TransportTarget::Wss(wss) => wss.url.clone(),
    }
}

fn selector_metadata(target: &TransportTarget) -> (&'static str, String) {
    match target {
        TransportTarget::Quic(quic) => ("alpn", quic.alpn.clone()),
        TransportTarget::Wss(wss) => ("subprotocol", wss.subprotocol.clone()),
    }
}

fn validate_target(target: &TransportTarget) -> ApiResult<()> {
    match target {
        TransportTarget::Quic(quic_target) => validate_quic_target(quic_target),
        TransportTarget::Wss(wss_target) => validate_wss_target(wss_target),
    }
}

fn validate_quic_target(target: &crate::descriptor::QuicTarget) -> ApiResult<()> {
    if target.alpn != QUIC_ALPN_V1 {
        return Err(ApiError::TransportFallback(
            FallbackReason::OuterQuicRejected,
        ));
    }

    if target.connect_host.is_empty() || target.port == 0 {
        return Err(ApiError::TransportFallback(
            FallbackReason::OuterPathFailure,
        ));
    }

    Ok(())
}

fn validate_wss_target(target: &crate::descriptor::WssTarget) -> ApiResult<()> {
    if target.subprotocol != WSS_SUBPROTOCOL_V1 {
        return Err(ApiError::TransportSelectorInvariant(
            "WSS subprotocol must match the v1 descriptor value",
        ));
    }

    if !target.url.starts_with("wss://") {
        return Err(ApiError::TransportSelectorInvariant(
            "WSS target URL must use the wss:// scheme",
        ));
    }

    Ok(())
}

struct PrototypeQuicConnector(PrototypeCarrierConnector);

struct PrototypeWssConnector(PrototypeCarrierConnector);

impl PrototypeQuicConnector {
    fn success(transport: Box<dyn FramedDuplex>) -> Self {
        Self(PrototypeCarrierConnector::ready(
            CarrierKind::Quic,
            transport,
        ))
    }

    fn fallback(reason: FallbackReason) -> Self {
        Self(PrototypeCarrierConnector::fallback(
            CarrierKind::Quic,
            reason,
        ))
    }

    fn call_count(&self) -> usize {
        self.0.call_count()
    }

    fn observations(&self) -> Vec<ConnectorObservation> {
        self.0.observations()
    }
}

impl PrototypeWssConnector {
    fn success(transport: Box<dyn FramedDuplex>) -> Self {
        Self(PrototypeCarrierConnector::ready(
            CarrierKind::Wss,
            transport,
        ))
    }

    fn failure(error: ApiError) -> Self {
        Self(PrototypeCarrierConnector::failure(CarrierKind::Wss, error))
    }

    fn call_count(&self) -> usize {
        self.0.call_count()
    }

    fn observations(&self) -> Vec<ConnectorObservation> {
        self.0.observations()
    }
}

impl CarrierConnector for PrototypeQuicConnector {
    fn carrier(&self) -> CarrierKind {
        CarrierKind::Quic
    }

    fn connect<'a>(
        &'a self,
        target: &'a TransportTarget,
    ) -> BoxFuture<'a, ApiResult<Box<dyn FramedDuplex>>> {
        self.0.connect(target)
    }
}

impl CarrierConnector for PrototypeWssConnector {
    fn carrier(&self) -> CarrierKind {
        CarrierKind::Wss
    }

    fn connect<'a>(
        &'a self,
        target: &'a TransportTarget,
    ) -> BoxFuture<'a, ApiResult<Box<dyn FramedDuplex>>> {
        self.0.connect(target)
    }
}

fn scripted_session_fixture(
    carrier: CarrierKind,
    mode: AuthorizationMode,
    responses: Vec<Vec<u8>>,
) -> PrototypeSessionFixture {
    let mut descriptor = example_service_descriptor();
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[7_u8; 32]);
    descriptor.trust_anchors[0].key_id = "root-2026-01".to_owned();
    descriptor.trust_anchors[0].algorithm = "ed25519".to_owned();
    descriptor.trust_anchors[0].public_key =
        STANDARD.encode(signing_key.verifying_key().to_bytes());

    let prologue = descriptor.noise_prologue().expect("descriptor prologue");
    let params: snow::params::NoiseParams = NOISE_SUITE_V1.parse().expect("noise params");
    let builder = snow::Builder::new(params.clone());
    let keypair = builder.generate_keypair().expect("responder keypair");
    let responder = snow::Builder::new(params)
        .prologue(&prologue)
        .expect("prologue")
        .local_private_key(&keypair.private)
        .expect("private key")
        .build_responder()
        .expect("responder");

    let mut authorization = ServerKeyAuthorizationV1 {
        version: 1,
        key_id: descriptor.trust_anchors[0].key_id.clone(),
        not_before_unix_seconds: 1_741_000_000,
        not_after_unix_seconds: 1_743_000_000,
        environment_id: descriptor.environment_id.clone(),
        service_id: descriptor.service_id.clone(),
        service_authority: descriptor.service_authority.clone(),
        protocol_id: descriptor.protocol_id.clone(),
        server_static_public_key: keypair.public.as_slice().try_into().expect("static key"),
        signature: [0_u8; 64],
    };

    if matches!(mode, AuthorizationMode::WrongServiceId) {
        authorization.service_id = "wrong-service".to_owned();
    }

    let signature = signing_key.sign(&authorization.signed_bytes().expect("signed bytes"));
    authorization.signature = signature.to_bytes();
    if matches!(mode, AuthorizationMode::BadSignature) {
        authorization.signature[0] ^= 0xFF;
    }
    let payload = authorization.encode().expect("authorization payload");

    let state = Arc::new(Mutex::new(ResponderState {
        handshake: Some(responder),
        transport: None,
        auth_payload: payload,
        queued_outbound: VecDeque::new(),
        queued_plaintext_responses: VecDeque::from(responses),
        received_plaintexts: Vec::new(),
        handshake_completed: false,
        saw_encrypted_close: false,
        last_close_plaintext: None,
        outer_closed: false,
        last_outer_close: None,
    }));

    PrototypeSessionFixture {
        descriptor,
        transport: ScriptedNxResponderTransport {
            carrier,
            state: Arc::clone(&state),
        },
        state,
    }
}

struct ResponderState {
    handshake: Option<snow::HandshakeState>,
    transport: Option<TransportState>,
    auth_payload: Vec<u8>,
    queued_outbound: VecDeque<Vec<u8>>,
    queued_plaintext_responses: VecDeque<Vec<u8>>,
    received_plaintexts: Vec<Vec<u8>>,
    handshake_completed: bool,
    saw_encrypted_close: bool,
    last_close_plaintext: Option<CloseDirective>,
    outer_closed: bool,
    last_outer_close: Option<CloseDirective>,
}

struct ScriptedNxResponderTransport {
    carrier: CarrierKind,
    state: Arc<Mutex<ResponderState>>,
}

impl FramedDuplex for ScriptedNxResponderTransport {
    fn carrier(&self) -> CarrierKind {
        self.carrier
    }

    fn send_record<'a>(&'a mut self, record: &'a [u8]) -> BoxFuture<'a, ApiResult<()>> {
        let state = Arc::clone(&self.state);
        Box::pin(async move {
            let mut state = state.lock().expect("responder state lock poisoned");
            if let Some(mut handshake) = state.handshake.take() {
                let mut empty = [];
                handshake
                    .read_message(record, &mut empty)
                    .map_err(|_| ApiError::InnerNoiseFailure)?;

                let mut outbound = vec![0_u8; crate::MAX_RECORD_PAYLOAD_SIZE];
                let written = handshake
                    .write_message(&state.auth_payload, &mut outbound)
                    .map_err(|_| ApiError::InnerNoiseFailure)?;
                outbound.truncate(written);
                state.queued_outbound.push_back(outbound);
                state.handshake_completed = handshake.is_handshake_finished();
                state.transport = Some(
                    handshake
                        .into_transport_mode()
                        .map_err(|_| ApiError::InnerNoiseFailure)?,
                );
                return Ok(());
            }

            let mut transport = state.transport.take().ok_or(ApiError::TransportClosed)?;
            let mut plaintext = vec![0_u8; crate::MAX_RECORD_PAYLOAD_SIZE];
            let written = transport
                .read_message(record, &mut plaintext)
                .map_err(|_| ApiError::InnerNoiseFailure)?;
            plaintext.truncate(written);

            if let Some(directive) = decode_close_message(&plaintext) {
                state.saw_encrypted_close = true;
                state.last_close_plaintext = Some(directive);
            } else {
                state.received_plaintexts.push(plaintext);
                if let Some(response) = state.queued_plaintext_responses.pop_front() {
                    let mut outbound = vec![0_u8; crate::MAX_RECORD_PAYLOAD_SIZE];
                    let written = transport
                        .write_message(&response, &mut outbound)
                        .map_err(|_| ApiError::InnerNoiseFailure)?;
                    outbound.truncate(written);
                    state.queued_outbound.push_back(outbound);
                }
            }

            state.transport = Some(transport);
            drop(state);
            Ok(())
        })
    }

    fn receive_record(&mut self) -> BoxFuture<'_, ApiResult<Option<Vec<u8>>>> {
        let state = Arc::clone(&self.state);
        Box::pin(async move {
            Ok(state
                .lock()
                .expect("responder state lock poisoned")
                .queued_outbound
                .pop_front())
        })
    }

    fn close(&mut self, directive: CloseDirective) -> BoxFuture<'_, ApiResult<()>> {
        let state = Arc::clone(&self.state);
        Box::pin(async move {
            let mut state = state.lock().expect("responder state lock poisoned");
            state.outer_closed = true;
            state.last_outer_close = Some(directive);
            drop(state);
            Ok(())
        })
    }
}

fn decode_close_message(record: &[u8]) -> Option<CloseDirective> {
    if record.len() != 4 || record[0] != 1 {
        return None;
    }

    Some(CloseDirective {
        code: u16::from_be_bytes([record[1], record[2]]),
        drain: record[3] != 0,
    })
}

#[cfg(test)]
mod tests {
    use std::future::Future;

    use super::{
        AuthorizationMode, PrototypeQuicConnector, PrototypeWssConnector, scripted_session_fixture,
    };
    use crate::constants::{QUIC_ALPN_V1, WSS_SUBPROTOCOL_V1};
    use crate::selector::{TransportConnectors, TransportSelector};
    use crate::session::CacheDisposition;
    use crate::transport::{CarrierKind, FallbackReason};
    use crate::{ApiError, SnowNxClientEvaluator};

    #[test]
    fn quic_binding_reaches_secure_ready_and_transports_application_data() {
        let fixture = scripted_session_fixture(
            CarrierKind::Quic,
            AuthorizationMode::Valid,
            vec![b"pong".to_vec()],
        );
        let quic = PrototypeQuicConnector::success(Box::new(fixture.transport));
        let wss = PrototypeWssConnector::failure(ApiError::TransportClosed);
        let evaluator = SnowNxClientEvaluator::new();

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
        let evaluator = SnowNxClientEvaluator::new();

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
        let evaluator = SnowNxClientEvaluator::new();

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
        let evaluator = SnowNxClientEvaluator::new();

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
            ApiError::InvalidServiceDescriptor(
                "WSS subprotocol must match the v1 descriptor value"
            )
        );
        assert_eq!(quic.call_count(), 0);
        assert_eq!(wss.call_count(), 0);
    }

    #[test]
    fn quic_binding_stops_on_inner_trust_failure_without_trying_wss() {
        let fixture = scripted_session_fixture(
            CarrierKind::Quic,
            AuthorizationMode::WrongServiceId,
            Vec::new(),
        );
        let quic = PrototypeQuicConnector::success(Box::new(fixture.transport));
        let wss = PrototypeWssConnector::failure(ApiError::TransportClosed);
        let evaluator = SnowNxClientEvaluator::new();

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
}
