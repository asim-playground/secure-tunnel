// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::collections::VecDeque;
use std::future::Future;
use std::sync::{Arc, Mutex};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use super::{SnowNk1ClientEvaluator, decode_close_message};
use crate::constants::{MAX_RECORD_PAYLOAD_SIZE, NOISE_SUITE_V1};
use crate::selector::{TransportConnectors, TransportSelector};
use crate::session::CloseDirective;
use crate::transport::{CarrierConnector, CarrierKind, FramedDuplex, TransportTarget};
use crate::{
    ApiError, BoxFuture, SecureReadyEvaluator, example_descriptor_trust_anchors,
    example_service_descriptor,
};

#[test]
fn secure_ready_success_exposes_handshake_hash() {
    let (descriptor, transport, state) = scripted_responder_fixture(
        CarrierKind::Quic,
        AuthorizationMode::Valid,
        vec![b"pong".to_vec()],
    );
    let evaluator = evaluator_for_descriptor(&descriptor);

    let mut secure_ready =
        block_on(evaluator.reach_secure_ready(&descriptor, 1_742_000_000, Box::new(transport)))
            .unwrap();

    let handshake_hash = secure_ready.artifacts.handshake_hash.clone().unwrap();
    assert!(!handshake_hash.is_empty());
    assert_eq!(secure_ready.artifacts.channel_binding, Some(handshake_hash));
    assert!(secure_ready.artifacts.service_static_public_key.is_some());

    block_on(secure_ready.transport.send_record(b"ping")).unwrap();
    let reply = block_on(secure_ready.transport.receive_record())
        .unwrap()
        .unwrap();
    assert_eq!(reply, b"pong");

    let guard = state.lock().unwrap();
    assert_eq!(guard.received_plaintexts, vec![b"ping".to_vec()]);
    assert!(guard.handshake_completed);
    drop(guard);
}

#[test]
fn secure_ready_rejects_wrong_service_static_public_key() {
    let (descriptor, transport, _) = scripted_responder_fixture(
        CarrierKind::Quic,
        AuthorizationMode::WrongServiceStaticKey,
        Vec::new(),
    );
    let evaluator = evaluator_for_descriptor(&descriptor);

    let Err(error) =
        block_on(evaluator.reach_secure_ready(&descriptor, 1_742_000_000, Box::new(transport)))
    else {
        panic!("secure-ready should fail on the wrong service static public key");
    };

    assert_eq!(error, ApiError::InnerNoiseFailure);
}

#[test]
fn secure_ready_rejects_descriptor_with_wrong_noise_suite() {
    let (mut descriptor, transport, _) =
        scripted_responder_fixture(CarrierKind::Quic, AuthorizationMode::Valid, Vec::new());
    descriptor.noise_suite = "Noise_NN_25519_ChaChaPoly_BLAKE2s".to_owned();
    let evaluator = evaluator_for_descriptor(&descriptor);

    let Err(error) =
        block_on(evaluator.reach_secure_ready(&descriptor, 1_742_000_000, Box::new(transport)))
    else {
        panic!("secure-ready should fail when the descriptor noise suite is invalid");
    };

    assert_eq!(
        error,
        ApiError::InvalidServiceDescriptor("noise_suite must match the v1 Noise suite identifier")
    );
}

#[test]
fn selector_does_not_fallback_on_inner_trust_failure_with_real_evaluator() {
    let (descriptor, quic_transport, _) = scripted_responder_fixture(
        CarrierKind::Quic,
        AuthorizationMode::HandshakePayload,
        Vec::new(),
    );
    let (_, wss_transport, _) =
        scripted_responder_fixture(CarrierKind::Wss, AuthorizationMode::Valid, Vec::new());
    let quic = ConnectOnce::new(CarrierKind::Quic, Box::new(quic_transport));
    let wss = ConnectOnce::new(CarrierKind::Wss, Box::new(wss_transport));
    let evaluator = evaluator_for_descriptor(&descriptor);

    let result = block_on(TransportSelector::new(300).select(
        &descriptor,
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
}

#[test]
fn selector_falls_back_on_quic_transport_closed_during_secure_ready_handshake() {
    let (descriptor, wss_transport, _) =
        scripted_responder_fixture(CarrierKind::Wss, AuthorizationMode::Valid, Vec::new());
    let quic = ConnectOnce::new(
        CarrierKind::Quic,
        Box::new(HandshakeCloseTransport {
            carrier: CarrierKind::Quic,
        }),
    );
    let wss = ConnectOnce::new(CarrierKind::Wss, Box::new(wss_transport));
    let evaluator = evaluator_for_descriptor(&descriptor);

    let selected = block_on(TransportSelector::new(300).select(
        &descriptor,
        None,
        1_742_000_000,
        TransportConnectors::new(Some(&quic), Some(&wss)),
        &evaluator,
    ))
    .unwrap();

    assert_eq!(selected.report.carrier, CarrierKind::Wss);
    assert_eq!(quic.call_count(), 1);
    assert_eq!(wss.call_count(), 1);
}

#[test]
fn noise_transport_close_sends_encrypted_close() {
    let (descriptor, transport, state) =
        scripted_responder_fixture(CarrierKind::Quic, AuthorizationMode::Valid, Vec::new());
    let evaluator = evaluator_for_descriptor(&descriptor);

    let mut secure_ready =
        block_on(evaluator.reach_secure_ready(&descriptor, 1_742_000_000, Box::new(transport)))
            .unwrap();
    let directive = CloseDirective {
        code: 7,
        drain: true,
    };

    block_on(secure_ready.transport.close(directive)).unwrap();

    let guard = state.lock().unwrap();
    assert_eq!(guard.last_close_plaintext, Some(directive));
    assert!(guard.saw_encrypted_close);
    assert!(guard.outer_closed);
    assert_eq!(guard.last_outer_close, Some(directive));
    drop(guard);
}

fn block_on<F>(future: F) -> F::Output
where
    F: Future,
{
    futures::executor::block_on(future)
}

#[derive(Clone, Copy)]
enum AuthorizationMode {
    Valid,
    WrongServiceStaticKey,
    HandshakePayload,
}

fn scripted_responder_fixture(
    carrier: CarrierKind,
    mode: AuthorizationMode,
    responses: Vec<Vec<u8>>,
) -> (
    crate::ServiceDescriptor,
    ScriptedNk1ResponderTransport,
    Arc<Mutex<ResponderState>>,
) {
    let mut descriptor = example_service_descriptor();

    let params: snow::params::NoiseParams = NOISE_SUITE_V1.parse().unwrap();
    let builder = snow::Builder::new(params.clone());
    let keypair = builder.generate_keypair().unwrap();
    descriptor.service_static_public_key = STANDARD.encode(&keypair.public);
    if matches!(mode, AuthorizationMode::WrongServiceStaticKey) {
        descriptor.service_static_public_key = STANDARD.encode([8_u8; 32]);
    }
    descriptor.resign_with_example_key_for_testing().unwrap();
    let prologue = descriptor.noise_prologue().unwrap();
    let responder = snow::Builder::new(params)
        .prologue(&prologue)
        .unwrap()
        .local_private_key(&keypair.private)
        .unwrap()
        .build_responder()
        .unwrap();
    let handshake_payload = if matches!(mode, AuthorizationMode::HandshakePayload) {
        b"forbidden".to_vec()
    } else {
        Vec::new()
    };

    let state = Arc::new(Mutex::new(ResponderState {
        handshake: Some(responder),
        transport: None,
        handshake_payload,
        queued_outbound: VecDeque::new(),
        queued_plaintext_responses: VecDeque::from(responses),
        received_plaintexts: Vec::new(),
        handshake_completed: false,
        saw_encrypted_close: false,
        last_close_plaintext: None,
        outer_closed: false,
        last_outer_close: None,
    }));

    (
        descriptor,
        ScriptedNk1ResponderTransport {
            carrier,
            state: state.clone(),
        },
        state,
    )
}

fn evaluator_for_descriptor(descriptor: &crate::ServiceDescriptor) -> SnowNk1ClientEvaluator {
    SnowNk1ClientEvaluator::with_pinned_trust(
        example_descriptor_trust_anchors(),
        vec![descriptor.service_static_public_key_bytes().unwrap()],
    )
}

struct ResponderState {
    handshake: Option<snow::HandshakeState>,
    transport: Option<snow::TransportState>,
    handshake_payload: Vec<u8>,
    queued_outbound: VecDeque<Vec<u8>>,
    queued_plaintext_responses: VecDeque<Vec<u8>>,
    received_plaintexts: Vec<Vec<u8>>,
    handshake_completed: bool,
    saw_encrypted_close: bool,
    last_close_plaintext: Option<CloseDirective>,
    outer_closed: bool,
    last_outer_close: Option<CloseDirective>,
}

struct ScriptedNk1ResponderTransport {
    carrier: CarrierKind,
    state: Arc<Mutex<ResponderState>>,
}

impl FramedDuplex for ScriptedNk1ResponderTransport {
    fn carrier(&self) -> CarrierKind {
        self.carrier
    }

    fn send_record<'a>(&'a mut self, record: &'a [u8]) -> BoxFuture<'a, crate::ApiResult<()>> {
        let state = self.state.clone();
        Box::pin(async move {
            let mut state = state.lock().unwrap();
            if let Some(mut handshake) = state.handshake.take() {
                let mut empty = [];
                handshake
                    .read_message(record, &mut empty)
                    .map_err(|_| ApiError::InnerNoiseFailure)?;

                let mut outbound = vec![0_u8; MAX_RECORD_PAYLOAD_SIZE];
                let written = handshake
                    .write_message(&state.handshake_payload, &mut outbound)
                    .map_err(|_| ApiError::InnerNoiseFailure)?;
                outbound.truncate(written);
                state.queued_outbound.push_back(outbound);
                state.handshake_completed = handshake.is_handshake_finished();
                state.transport = Some(
                    handshake
                        .into_transport_mode()
                        .map_err(|_| ApiError::InnerNoiseFailure)?,
                );
                drop(state);
                return Ok(());
            }

            let mut transport = state.transport.take().ok_or(ApiError::TransportClosed)?;
            let mut plaintext = vec![0_u8; MAX_RECORD_PAYLOAD_SIZE];
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
                    let mut outbound = vec![0_u8; MAX_RECORD_PAYLOAD_SIZE];
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

    fn receive_record(&mut self) -> BoxFuture<'_, crate::ApiResult<Option<Vec<u8>>>> {
        let state = self.state.clone();
        Box::pin(async move { Ok(state.lock().unwrap().queued_outbound.pop_front()) })
    }

    fn close(&mut self, directive: CloseDirective) -> BoxFuture<'_, crate::ApiResult<()>> {
        let state = self.state.clone();
        Box::pin(async move {
            let mut state = state.lock().unwrap();
            state.outer_closed = true;
            state.last_outer_close = Some(directive);
            drop(state);
            Ok(())
        })
    }
}

struct ConnectOnce {
    carrier: CarrierKind,
    transport: Mutex<Option<Box<dyn FramedDuplex>>>,
    calls: Mutex<usize>,
}

impl ConnectOnce {
    fn new(carrier: CarrierKind, transport: Box<dyn FramedDuplex>) -> Self {
        Self {
            carrier,
            transport: Mutex::new(Some(transport)),
            calls: Mutex::new(0),
        }
    }

    fn call_count(&self) -> usize {
        *self.calls.lock().unwrap()
    }
}

impl CarrierConnector for ConnectOnce {
    fn carrier(&self) -> CarrierKind {
        self.carrier
    }

    fn connect<'a>(
        &'a self,
        _target: &'a TransportTarget,
    ) -> BoxFuture<'a, crate::ApiResult<Box<dyn FramedDuplex>>> {
        *self.calls.lock().unwrap() += 1;
        let transport = self.transport.lock().unwrap().take();
        Box::pin(async move { transport.ok_or(ApiError::TransportClosed) })
    }
}

struct HandshakeCloseTransport {
    carrier: CarrierKind,
}

impl FramedDuplex for HandshakeCloseTransport {
    fn carrier(&self) -> CarrierKind {
        self.carrier
    }

    fn send_record<'a>(&'a mut self, _record: &'a [u8]) -> BoxFuture<'a, crate::ApiResult<()>> {
        Box::pin(async move { Ok(()) })
    }

    fn receive_record(&mut self) -> BoxFuture<'_, crate::ApiResult<Option<Vec<u8>>>> {
        Box::pin(async move { Err(ApiError::TransportClosed) })
    }

    fn close(&mut self, _directive: CloseDirective) -> BoxFuture<'_, crate::ApiResult<()>> {
        Box::pin(async move { Ok(()) })
    }
}
