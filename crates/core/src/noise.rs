// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use bytes::BufMut;
use snow::TransportState;

use crate::constants::{MAX_APPLICATION_PLAINTEXT_SIZE, MAX_RECORD_PAYLOAD_SIZE, NOISE_SUITE_V1};
use crate::descriptor::ServiceDescriptor;
use crate::error::{ApiError, ApiResult};
use crate::selector::SecureReadyEvaluator;
use crate::session::{CloseDirective, SecureReadyArtifacts, SecureReadyTransport};
use crate::transport::{BoxFuture, CarrierKind, FramedDuplex};
use crate::trust::verify_server_key_authorization;

const CLOSE_MESSAGE_TYPE_V1: u8 = 1;

/// Client-side `NX` secure-ready evaluator backed by `snow`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SnowNxClientEvaluator;

impl SnowNxClientEvaluator {
    /// Creates a client-side secure-ready evaluator.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl SecureReadyEvaluator for SnowNxClientEvaluator {
    fn reach_secure_ready(
        &self,
        descriptor: &ServiceDescriptor,
        now_unix_seconds: u64,
        mut transport: Box<dyn FramedDuplex>,
    ) -> BoxFuture<'_, ApiResult<SecureReadyTransport>> {
        let descriptor = descriptor.clone();

        Box::pin(async move {
            descriptor.validate()?;
            let prologue = descriptor.noise_prologue()?;
            let params = NOISE_SUITE_V1
                .parse()
                .map_err(|_| ApiError::InnerNoiseFailure)?;
            let builder = snow::Builder::new(params);
            let builder = builder
                .prologue(&prologue)
                .map_err(|_| ApiError::InnerNoiseFailure)?;
            let mut initiator = builder
                .build_initiator()
                .map_err(|_| ApiError::InnerNoiseFailure)?;

            let mut outbound = vec![0_u8; MAX_RECORD_PAYLOAD_SIZE];
            let first_len = initiator
                .write_message(&[], &mut outbound)
                .map_err(|_| ApiError::InnerNoiseFailure)?;
            transport
                .send_record(&outbound[..first_len])
                .await
                .map_err(|error| normalize_handshake_transport_error(transport.carrier(), error))?;

            let responder_record = match transport.receive_record().await {
                Ok(Some(record)) => record,
                Ok(None) => {
                    return Err(normalize_handshake_transport_error(
                        transport.carrier(),
                        ApiError::TransportClosed,
                    ));
                }
                Err(error) => {
                    return Err(normalize_handshake_transport_error(
                        transport.carrier(),
                        error,
                    ));
                }
            };

            let mut payload = vec![0_u8; MAX_RECORD_PAYLOAD_SIZE];
            let payload_len = initiator
                .read_message(&responder_record, &mut payload)
                .map_err(|_| ApiError::InnerNoiseFailure)?;
            let remote_static = initiator
                .get_remote_static()
                .ok_or(ApiError::InnerTrustFailure)?;

            verify_server_key_authorization(
                &descriptor,
                now_unix_seconds,
                remote_static,
                &payload[..payload_len],
            )?;

            if !initiator.is_handshake_finished() {
                return Err(ApiError::InnerNoiseFailure);
            }

            let handshake_hash = initiator.get_handshake_hash().to_vec();
            let transport_state = initiator
                .into_transport_mode()
                .map_err(|_| ApiError::InnerNoiseFailure)?;

            Ok(SecureReadyTransport {
                transport: Box::new(NoiseFramedDuplex::new(transport, transport_state)),
                artifacts: SecureReadyArtifacts {
                    handshake_hash: Some(handshake_hash.clone()),
                    channel_binding: Some(handshake_hash),
                },
            })
        })
    }
}

const fn normalize_handshake_transport_error(carrier: CarrierKind, error: ApiError) -> ApiError {
    match (carrier, error) {
        (CarrierKind::Quic, ApiError::TransportClosed) => {
            ApiError::TransportFallback(crate::FallbackReason::OuterQuicClosedEarly)
        }
        (_, error) => error,
    }
}

/// A Noise transport-mode wrapper over carrier-neutral framed I/O.
pub struct NoiseFramedDuplex {
    inner: Box<dyn FramedDuplex>,
    state: TransportState,
}

impl NoiseFramedDuplex {
    /// Wraps a carrier-neutral framed transport in Noise transport mode.
    #[must_use]
    pub const fn new(inner: Box<dyn FramedDuplex>, state: TransportState) -> Self {
        Self { inner, state }
    }
}

impl FramedDuplex for NoiseFramedDuplex {
    fn carrier(&self) -> CarrierKind {
        self.inner.carrier()
    }

    fn send_record<'a>(&'a mut self, record: &'a [u8]) -> BoxFuture<'a, ApiResult<()>> {
        Box::pin(async move {
            if record.len() > MAX_APPLICATION_PLAINTEXT_SIZE {
                return Err(ApiError::RecordTooLarge {
                    actual: record.len(),
                    max: MAX_APPLICATION_PLAINTEXT_SIZE,
                });
            }

            let mut ciphertext = vec![0_u8; MAX_RECORD_PAYLOAD_SIZE];
            let written = self
                .state
                .write_message(record, &mut ciphertext)
                .map_err(|_| ApiError::InnerNoiseFailure)?;

            self.inner.send_record(&ciphertext[..written]).await
        })
    }

    fn receive_record(&mut self) -> BoxFuture<'_, ApiResult<Option<Vec<u8>>>> {
        Box::pin(async move {
            let Some(ciphertext) = self.inner.receive_record().await? else {
                return Ok(None);
            };

            let mut plaintext = vec![0_u8; MAX_RECORD_PAYLOAD_SIZE];
            let written = self
                .state
                .read_message(&ciphertext, &mut plaintext)
                .map_err(|_| ApiError::InnerNoiseFailure)?;
            plaintext.truncate(written);

            Ok(Some(plaintext))
        })
    }

    fn close(&mut self, directive: CloseDirective) -> BoxFuture<'_, ApiResult<()>> {
        Box::pin(async move {
            let close_record = encode_close_message(directive);
            self.send_record(&close_record).await?;
            self.inner.close(directive).await
        })
    }
}

fn encode_close_message(directive: CloseDirective) -> Vec<u8> {
    let mut out = Vec::with_capacity(4);
    out.put_u8(CLOSE_MESSAGE_TYPE_V1);
    out.put_u16(directive.code);
    out.put_u8(u8::from(directive.drain));
    out
}

#[cfg(test)]
fn decode_close_message(record: &[u8]) -> Option<CloseDirective> {
    if record.len() != 4 || record[0] != CLOSE_MESSAGE_TYPE_V1 {
        return None;
    }

    Some(CloseDirective {
        code: u16::from_be_bytes([record[1], record[2]]),
        drain: record[3] != 0,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::future::Future;
    use std::sync::{Arc, Mutex};

    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use ed25519_dalek::{Signer, SigningKey};

    use super::{SnowNxClientEvaluator, decode_close_message};
    use crate::constants::{MAX_RECORD_PAYLOAD_SIZE, NOISE_SUITE_V1};
    use crate::descriptor::example_service_descriptor;
    use crate::selector::{TransportConnectors, TransportSelector};
    use crate::session::CloseDirective;
    use crate::transport::{CarrierConnector, CarrierKind, FramedDuplex, TransportTarget};
    use crate::{ApiError, BoxFuture, SecureReadyEvaluator, ServerKeyAuthorizationV1};

    #[test]
    fn secure_ready_success_exposes_handshake_hash() {
        let (descriptor, transport, state) = scripted_responder_fixture(
            CarrierKind::Quic,
            AuthorizationMode::Valid,
            vec![b"pong".to_vec()],
        );
        let evaluator = SnowNxClientEvaluator::new();

        let mut secure_ready =
            block_on(evaluator.reach_secure_ready(&descriptor, 1_742_000_000, Box::new(transport)))
                .unwrap();

        let handshake_hash = secure_ready.artifacts.handshake_hash.clone().unwrap();
        assert!(!handshake_hash.is_empty());
        assert_eq!(secure_ready.artifacts.channel_binding, Some(handshake_hash));

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
    fn secure_ready_rejects_bad_server_key_authorization() {
        let (descriptor, transport, _) = scripted_responder_fixture(
            CarrierKind::Quic,
            AuthorizationMode::WrongServiceId,
            Vec::new(),
        );
        let evaluator = SnowNxClientEvaluator::new();

        let Err(error) =
            block_on(evaluator.reach_secure_ready(&descriptor, 1_742_000_000, Box::new(transport)))
        else {
            panic!("secure-ready should fail on a bad authorization payload");
        };

        assert_eq!(error, ApiError::InnerTrustFailure);
    }

    #[test]
    fn secure_ready_rejects_descriptor_with_wrong_noise_suite() {
        let (mut descriptor, transport, _) =
            scripted_responder_fixture(CarrierKind::Quic, AuthorizationMode::Valid, Vec::new());
        descriptor.noise_suite = "Noise_NN_25519_ChaChaPoly_BLAKE2s".to_owned();
        let evaluator = SnowNxClientEvaluator::new();

        let Err(error) =
            block_on(evaluator.reach_secure_ready(&descriptor, 1_742_000_000, Box::new(transport)))
        else {
            panic!("secure-ready should fail when the descriptor noise suite is invalid");
        };

        assert_eq!(
            error,
            ApiError::InvalidServiceDescriptor(
                "noise_suite must match the v1 Noise suite identifier"
            )
        );
    }

    #[test]
    fn selector_does_not_fallback_on_inner_trust_failure_with_real_evaluator() {
        let (descriptor, quic_transport, _) = scripted_responder_fixture(
            CarrierKind::Quic,
            AuthorizationMode::BadSignature,
            Vec::new(),
        );
        let (_, wss_transport, _) =
            scripted_responder_fixture(CarrierKind::Wss, AuthorizationMode::Valid, Vec::new());
        let quic = ConnectOnce::new(CarrierKind::Quic, Box::new(quic_transport));
        let wss = ConnectOnce::new(CarrierKind::Wss, Box::new(wss_transport));
        let evaluator = SnowNxClientEvaluator::new();

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
        let evaluator = SnowNxClientEvaluator::new();

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
        let evaluator = SnowNxClientEvaluator::new();

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
        BadSignature,
        WrongServiceId,
    }

    fn scripted_responder_fixture(
        carrier: CarrierKind,
        mode: AuthorizationMode,
        responses: Vec<Vec<u8>>,
    ) -> (
        crate::ServiceDescriptor,
        ScriptedNxResponderTransport,
        Arc<Mutex<ResponderState>>,
    ) {
        let mut descriptor = example_service_descriptor();
        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        descriptor.trust_anchors[0].key_id = "root-2026-01".to_owned();
        descriptor.trust_anchors[0].algorithm = "ed25519".to_owned();
        descriptor.trust_anchors[0].public_key =
            STANDARD.encode(signing_key.verifying_key().to_bytes());

        let prologue = descriptor.noise_prologue().unwrap();
        let params: snow::params::NoiseParams = NOISE_SUITE_V1.parse().unwrap();
        let builder = snow::Builder::new(params.clone());
        let keypair = builder.generate_keypair().unwrap();
        let responder = snow::Builder::new(params)
            .prologue(&prologue)
            .unwrap()
            .local_private_key(&keypair.private)
            .unwrap()
            .build_responder()
            .unwrap();

        let mut authorization = ServerKeyAuthorizationV1 {
            version: 1,
            key_id: descriptor.trust_anchors[0].key_id.clone(),
            not_before_unix_seconds: 1_741_000_000,
            not_after_unix_seconds: 1_743_000_000,
            environment_id: descriptor.environment_id.clone(),
            service_id: descriptor.service_id.clone(),
            service_authority: descriptor.service_authority.clone(),
            protocol_id: descriptor.protocol_id.clone(),
            server_static_public_key: keypair.public.as_slice().try_into().unwrap(),
            signature: [0_u8; 64],
        };

        if matches!(mode, AuthorizationMode::WrongServiceId) {
            authorization.service_id = "wrong-service".to_owned();
        }

        let signature = signing_key.sign(&authorization.signed_bytes().unwrap());
        authorization.signature = signature.to_bytes();
        if matches!(mode, AuthorizationMode::BadSignature) {
            authorization.signature[0] ^= 0xFF;
        }
        let payload = authorization.encode().unwrap();

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

        (
            descriptor,
            ScriptedNxResponderTransport {
                carrier,
                state: state.clone(),
            },
            state,
        )
    }

    struct ResponderState {
        handshake: Option<snow::HandshakeState>,
        transport: Option<snow::TransportState>,
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
}
