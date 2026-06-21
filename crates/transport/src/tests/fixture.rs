// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::collections::VecDeque;
use std::error::Error;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signer, SigningKey};
use secure_tunnel_core::{
    ApiError, ApiResult, CarrierKind, NOISE_SUITE_V1, QUIC_ALPN_V1, ServerKeyAuthorizationV1,
    ServiceDescriptor, WSS_SUBPROTOCOL_V1, example_service_descriptor,
};
use snow::TransportState;

pub(super) type BoxError = Box<dyn Error + Send + Sync>;
pub(super) type TestResult<T> = Result<T, BoxError>;

#[derive(Debug, Clone, Copy)]
pub(super) enum AuthorizationMode {
    Valid,
    BadSignature,
}

#[derive(Clone)]
pub(super) struct ServiceFixture {
    descriptor: ServiceDescriptor,
    server_private_key: Vec<u8>,
    server_public_key: [u8; 32],
}

impl ServiceFixture {
    pub(super) fn new() -> ApiResult<Self> {
        let mut descriptor = example_service_descriptor();
        let signing_key = signing_key();
        descriptor.trust_anchors[0].key_id = "root-2026-01".to_owned();
        descriptor.trust_anchors[0].algorithm = "ed25519".to_owned();
        descriptor.trust_anchors[0].public_key =
            STANDARD.encode(signing_key.verifying_key().to_bytes());

        let params = noise_params()?;
        let keypair = snow::Builder::new(params)
            .generate_keypair()
            .map_err(|_| ApiError::InnerNoiseFailure)?;
        let server_public_key = keypair
            .public
            .as_slice()
            .try_into()
            .map_err(|_| ApiError::InnerNoiseFailure)?;

        Ok(Self {
            descriptor,
            server_private_key: keypair.private,
            server_public_key,
        })
    }

    pub(super) fn descriptor_for_ports(
        &self,
        quic_port: u16,
        wss_port: u16,
    ) -> ApiResult<ServiceDescriptor> {
        let mut descriptor = self.descriptor.clone();
        let quic = descriptor
            .carriers
            .quic
            .as_mut()
            .ok_or(ApiError::UnavailableCarrier(CarrierKind::Quic))?;
        quic.connect_host = "127.0.0.1".to_owned();
        quic.port = quic_port;
        quic.alpn = QUIC_ALPN_V1.to_owned();
        quic.sni_override = None;

        let wss = descriptor
            .carriers
            .wss
            .as_mut()
            .ok_or(ApiError::UnavailableCarrier(CarrierKind::Wss))?;
        wss.url = format!("wss://127.0.0.1:{wss_port}/tunnel");
        wss.subprotocol = WSS_SUBPROTOCOL_V1.to_owned();
        wss.authority_override = None;
        descriptor.validate()?;
        Ok(descriptor)
    }

    pub(super) fn responder(&self, mode: AuthorizationMode) -> ApiResult<NoiseResponder> {
        let prologue = self.descriptor.noise_prologue()?;
        let responder = snow::Builder::new(noise_params()?)
            .prologue(&prologue)
            .map_err(|_| ApiError::InnerNoiseFailure)?
            .local_private_key(&self.server_private_key)
            .map_err(|_| ApiError::InnerNoiseFailure)?
            .build_responder()
            .map_err(|_| ApiError::InnerNoiseFailure)?;

        Ok(NoiseResponder {
            handshake: Some(responder),
            transport: None,
            auth_payload: self.authorization(mode)?,
            queued_plaintext_responses: VecDeque::new(),
        })
    }

    fn authorization(&self, mode: AuthorizationMode) -> ApiResult<Vec<u8>> {
        let signing_key = signing_key();
        let mut authorization = ServerKeyAuthorizationV1 {
            version: 1,
            key_id: self.descriptor.trust_anchors[0].key_id.clone(),
            not_before_unix_seconds: 1_741_000_000,
            not_after_unix_seconds: 1_743_000_000,
            environment_id: self.descriptor.environment_id.clone(),
            service_id: self.descriptor.service_id.clone(),
            service_authority: self.descriptor.service_authority.clone(),
            protocol_id: self.descriptor.protocol_id.clone(),
            server_static_public_key: self.server_public_key,
            signature: [0_u8; 64],
        };
        let signature = signing_key.sign(&signed_authorization_bytes(&authorization)?);
        authorization.signature = signature.to_bytes();
        if matches!(mode, AuthorizationMode::BadSignature) {
            authorization.signature[0] ^= 0xFF;
        }
        encode_authorization(&authorization)
    }
}

pub(super) struct NoiseResponder {
    handshake: Option<snow::HandshakeState>,
    transport: Option<TransportState>,
    auth_payload: Vec<u8>,
    queued_plaintext_responses: VecDeque<Vec<u8>>,
}

impl NoiseResponder {
    pub(super) fn process_record(&mut self, record: &[u8]) -> ApiResult<Option<Vec<u8>>> {
        if let Some(mut handshake) = self.handshake.take() {
            let mut empty = [];
            handshake
                .read_message(record, &mut empty)
                .map_err(|_| ApiError::InnerNoiseFailure)?;
            let mut outbound = vec![0_u8; secure_tunnel_core::MAX_RECORD_PAYLOAD_SIZE];
            let written = handshake
                .write_message(&self.auth_payload, &mut outbound)
                .map_err(|_| ApiError::InnerNoiseFailure)?;
            outbound.truncate(written);
            self.transport = Some(
                handshake
                    .into_transport_mode()
                    .map_err(|_| ApiError::InnerNoiseFailure)?,
            );
            return Ok(Some(outbound));
        }

        let mut transport = self.transport.take().ok_or(ApiError::TransportClosed)?;
        let mut plaintext = vec![0_u8; secure_tunnel_core::MAX_RECORD_PAYLOAD_SIZE];
        let written = transport
            .read_message(record, &mut plaintext)
            .map_err(|_| ApiError::InnerNoiseFailure)?;
        plaintext.truncate(written);
        self.transport = Some(transport);
        Ok(self.queued_plaintext_responses.pop_front())
    }
}

pub(super) fn boxed_error(message: &'static str) -> BoxError {
    Box::new(std::io::Error::other(message))
}

fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[7_u8; 32])
}

fn noise_params() -> ApiResult<snow::params::NoiseParams> {
    NOISE_SUITE_V1
        .parse()
        .map_err(|_| ApiError::InnerNoiseFailure)
}

fn encode_authorization(authorization: &ServerKeyAuthorizationV1) -> ApiResult<Vec<u8>> {
    let mut out = signed_authorization_bytes(authorization)?;
    out.extend_from_slice(&authorization.signature);
    Ok(out)
}

fn signed_authorization_bytes(authorization: &ServerKeyAuthorizationV1) -> ApiResult<Vec<u8>> {
    let mut out = Vec::with_capacity(256);
    out.push(authorization.version);
    put_len_prefixed_str(&mut out, &authorization.key_id)?;
    out.extend_from_slice(&authorization.not_before_unix_seconds.to_be_bytes());
    out.extend_from_slice(&authorization.not_after_unix_seconds.to_be_bytes());
    put_len_prefixed_str(&mut out, &authorization.environment_id)?;
    put_len_prefixed_str(&mut out, &authorization.service_id)?;
    put_len_prefixed_str(&mut out, &authorization.service_authority)?;
    put_len_prefixed_str(&mut out, &authorization.protocol_id)?;
    out.extend_from_slice(&authorization.server_static_public_key);
    Ok(out)
}

fn put_len_prefixed_str(out: &mut Vec<u8>, value: &str) -> ApiResult<()> {
    let length = u16::try_from(value.len()).map_err(|_| ApiError::InnerTrustFailure)?;
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(value.as_bytes());
    Ok(())
}
