// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::Signer;
use snow::TransportState;

use crate::constants::NOISE_SUITE_V1;
use crate::descriptor::{ServiceDescriptor, example_service_descriptor};
use crate::error::{ApiError, ApiResult};
use crate::session::CloseDirective;
use crate::transport::{BoxFuture, CarrierKind, FramedDuplex};
use crate::trust::ServerKeyAuthorizationV1;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AuthorizationMode {
    Valid,
    BadSignature,
    WrongServiceId,
}

pub(super) struct PrototypeSessionFixture {
    pub(super) descriptor: ServiceDescriptor,
    pub(super) transport: ScriptedNxResponderTransport,
    pub(super) state: Arc<Mutex<ResponderState>>,
}

pub(super) fn scripted_session_fixture(
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

pub(super) struct ResponderState {
    handshake: Option<snow::HandshakeState>,
    transport: Option<TransportState>,
    auth_payload: Vec<u8>,
    queued_outbound: VecDeque<Vec<u8>>,
    queued_plaintext_responses: VecDeque<Vec<u8>>,
    pub(super) received_plaintexts: Vec<Vec<u8>>,
    pub(super) handshake_completed: bool,
    saw_encrypted_close: bool,
    last_close_plaintext: Option<CloseDirective>,
    outer_closed: bool,
    last_outer_close: Option<CloseDirective>,
}

pub(super) struct ScriptedNxResponderTransport {
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
