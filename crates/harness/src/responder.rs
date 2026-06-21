// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use secure_tunnel_core::{
    AccountAuthRequest, AccountAuthResult, AccountFreshness, ApiError, ApiResult,
    ApplicationMessage, DeviceAuthStart, DeviceChallenge, DeviceProofFinish, DeviceProofInput,
    DeviceProofPurpose, DeviceResult, DeviceState, FAMILY_DEVICE_AUTH, Hash32,
    MAX_RECORD_PAYLOAD_SIZE, ServiceDescriptor, TYPE_DEVICE_AUTH_FINISH,
    verify_device_proof_signature,
};
use snow::TransportState;

use crate::fixture::{SMOKE_PING, SMOKE_PONG, noise_params};

const ACCOUNT_CONTEXT_HASH: Hash32 = [4_u8; 32];
const DEVICE_CHALLENGE: Hash32 = [5_u8; 32];
const CHALLENGE_EXPIRES_AT_UNIX_MS: u64 = 1_760_000_010_000;
const CLOSE_MESSAGE_TYPE_V1: u8 = 1;

pub struct NoiseServiceResponder {
    descriptor: ServiceDescriptor,
    device_public_key: [u8; 32],
    handshake: Option<snow::HandshakeState>,
    handshake_hash: Option<Hash32>,
    transport: Option<TransportState>,
    account_context_hash: Option<Hash32>,
    pending_device_key_id: Option<String>,
}

impl NoiseServiceResponder {
    pub(crate) fn new(
        descriptor: ServiceDescriptor,
        server_private_key: &[u8],
        device_public_key: [u8; 32],
    ) -> ApiResult<Self> {
        let prologue = descriptor.noise_prologue()?;
        let handshake = snow::Builder::new(noise_params()?)
            .prologue(&prologue)
            .map_err(|_| ApiError::InnerNoiseFailure)?
            .local_private_key(server_private_key)
            .map_err(|_| ApiError::InnerNoiseFailure)?
            .build_responder()
            .map_err(|_| ApiError::InnerNoiseFailure)?;
        Ok(Self {
            descriptor,
            device_public_key,
            handshake: Some(handshake),
            handshake_hash: None,
            transport: None,
            account_context_hash: None,
            pending_device_key_id: None,
        })
    }

    pub(crate) fn process_record(&mut self, record: &[u8]) -> ApiResult<Option<Vec<u8>>> {
        if let Some(handshake) = self.handshake.take() {
            return self.process_handshake_record(record, handshake);
        }

        let mut transport = self.transport.take().ok_or(ApiError::TransportClosed)?;
        let mut plaintext = vec![0_u8; MAX_RECORD_PAYLOAD_SIZE];
        let written = transport
            .read_message(record, &mut plaintext)
            .map_err(|_| ApiError::InnerNoiseFailure)?;
        plaintext.truncate(written);
        let response = self.handle_plaintext(&plaintext)?;
        let outbound = response
            .as_deref()
            .map(|plaintext| encrypt_response(&mut transport, plaintext))
            .transpose()?;
        self.transport = Some(transport);
        Ok(outbound)
    }

    fn process_handshake_record(
        &mut self,
        record: &[u8],
        mut handshake: snow::HandshakeState,
    ) -> ApiResult<Option<Vec<u8>>> {
        let mut empty = [];
        handshake
            .read_message(record, &mut empty)
            .map_err(|_| ApiError::InnerNoiseFailure)?;
        let mut outbound = vec![0_u8; MAX_RECORD_PAYLOAD_SIZE];
        let written = handshake
            .write_message(&[], &mut outbound)
            .map_err(|_| ApiError::InnerNoiseFailure)?;
        outbound.truncate(written);
        self.handshake_hash = Some(fixed_32(handshake.get_handshake_hash())?);
        self.transport = Some(
            handshake
                .into_transport_mode()
                .map_err(|_| ApiError::InnerNoiseFailure)?,
        );
        Ok(Some(outbound))
    }

    fn handle_plaintext(&mut self, record: &[u8]) -> ApiResult<Option<Vec<u8>>> {
        if is_close_message(record) {
            return Ok(None);
        }
        if let Ok(request) = AccountAuthRequest::decode(record) {
            return self.handle_account_auth(request).map(Some);
        }
        if let Ok(start) = DeviceAuthStart::decode(record) {
            return self.handle_device_start(start).map(Some);
        }
        if let Ok(finish) = decode_device_auth_finish(record) {
            return self.handle_device_finish(finish).map(Some);
        }
        if record == SMOKE_PING {
            return Ok(Some(SMOKE_PONG.to_vec()));
        }
        Err(ApiError::PostHandshakeAuthFailure)
    }

    fn handle_account_auth(&mut self, request: AccountAuthRequest) -> ApiResult<Vec<u8>> {
        self.account_context_hash = Some(ACCOUNT_CONTEXT_HASH);
        AccountAuthResult {
            account_id: request.account_id,
            session_context_id: "session-smoke".to_owned(),
            account_context_hash: ACCOUNT_CONTEXT_HASH,
            freshness: AccountFreshness::Fresh,
        }
        .encode()
    }

    fn handle_device_start(&mut self, start: DeviceAuthStart) -> ApiResult<Vec<u8>> {
        self.pending_device_key_id = Some(start.device_key_id);
        DeviceChallenge {
            server_challenge: DEVICE_CHALLENGE,
            expires_at_unix_ms: CHALLENGE_EXPIRES_AT_UNIX_MS,
        }
        .encode_auth()
    }

    fn handle_device_finish(&mut self, finish: DeviceProofFinish) -> ApiResult<Vec<u8>> {
        let Some(expected_key_id) = self.pending_device_key_id.take() else {
            return Err(ApiError::PostHandshakeAuthFailure);
        };
        if expected_key_id != finish.device_key_id || finish.server_challenge != DEVICE_CHALLENGE {
            return Err(ApiError::PostHandshakeAuthFailure);
        }
        let input = DeviceProofInput {
            noise_handshake_hash: self
                .handshake_hash
                .ok_or(ApiError::PostHandshakeAuthFailure)?,
            server_challenge: finish.server_challenge,
            context: self.descriptor.inner_channel_context()?,
            account_context_hash: self
                .account_context_hash
                .ok_or(ApiError::PostHandshakeAuthFailure)?,
            device_key_id: finish.device_key_id.clone(),
            purpose: DeviceProofPurpose::KnownDeviceReauth,
            expires_at_unix_ms: finish.expires_at_unix_ms,
        };
        verify_device_proof_signature(
            &self.device_public_key,
            &input.canonical_bytes()?,
            &finish.signature,
        )?;
        DeviceResult {
            device_key_id: finish.device_key_id,
            state: DeviceState::Active,
        }
        .encode_auth()
    }
}

fn encrypt_response(transport: &mut TransportState, plaintext: &[u8]) -> ApiResult<Vec<u8>> {
    let mut outbound = vec![0_u8; MAX_RECORD_PAYLOAD_SIZE];
    let written = transport
        .write_message(plaintext, &mut outbound)
        .map_err(|_| ApiError::InnerNoiseFailure)?;
    outbound.truncate(written);
    Ok(outbound)
}

fn decode_device_auth_finish(record: &[u8]) -> ApiResult<DeviceProofFinish> {
    let message = ApplicationMessage::decode(record)?;
    message.expect_kind(FAMILY_DEVICE_AUTH, TYPE_DEVICE_AUTH_FINISH)?;
    let [device_key_id, server_challenge, expires_at, signature] =
        take_four_fields(message.fields)?;
    Ok(DeviceProofFinish {
        device_key_id: String::from_utf8(device_key_id)
            .map_err(|_| ApiError::PostHandshakeAuthFailure)?,
        server_challenge: fixed_32(&server_challenge)?,
        expires_at_unix_ms: u64::from_be_bytes(fixed_8(&expires_at)?),
        signature: fixed_64(&signature)?,
        candidate_device_public_key: None,
    })
}

fn take_four_fields(fields: Vec<Vec<u8>>) -> ApiResult<[Vec<u8>; 4]> {
    fields
        .try_into()
        .map_err(|_| ApiError::PostHandshakeAuthFailure)
}

fn fixed_8(bytes: &[u8]) -> ApiResult<[u8; 8]> {
    bytes
        .try_into()
        .map_err(|_| ApiError::PostHandshakeAuthFailure)
}

fn fixed_32(bytes: &[u8]) -> ApiResult<[u8; 32]> {
    bytes
        .try_into()
        .map_err(|_| ApiError::PostHandshakeAuthFailure)
}

fn fixed_64(bytes: &[u8]) -> ApiResult<[u8; 64]> {
    bytes
        .try_into()
        .map_err(|_| ApiError::PostHandshakeAuthFailure)
}

const fn is_close_message(record: &[u8]) -> bool {
    record.len() == 4 && record[0] == CLOSE_MESSAGE_TYPE_V1
}
