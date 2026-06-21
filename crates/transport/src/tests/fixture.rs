// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::collections::VecDeque;
use std::error::Error;
use std::sync::{Arc, Mutex};

use secure_tunnel_core::{
    ApiError, ApiResult, CarrierKind, NOISE_SUITE_V1, QUIC_ALPN_V1, ServiceDescriptor,
    WSS_SUBPROTOCOL_V1, example_service_descriptor,
};
use snow::TransportState;

pub(super) type BoxError = Box<dyn Error + Send + Sync>;
pub(super) type TestResult<T> = Result<T, BoxError>;

#[derive(Debug, Clone, Copy)]
pub(super) enum AuthorizationMode {
    Valid,
    HandshakePayload,
}

#[derive(Clone)]
pub(super) struct ServiceFixture {
    descriptor: Arc<Mutex<ServiceDescriptor>>,
    server_private_key: Vec<u8>,
    server_public_key: [u8; 32],
}

impl ServiceFixture {
    pub(super) fn new() -> ApiResult<Self> {
        let mut descriptor = example_service_descriptor();

        let params = noise_params()?;
        let keypair = snow::Builder::new(params)
            .generate_keypair()
            .map_err(|_| ApiError::InnerNoiseFailure)?;
        let server_public_key: [u8; 32] = keypair
            .public
            .as_slice()
            .try_into()
            .map_err(|_| ApiError::InnerNoiseFailure)?;
        descriptor.service_static_public_key = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            server_public_key,
        );
        descriptor.resign_with_example_key_for_testing()?;

        Ok(Self {
            descriptor: Arc::new(Mutex::new(descriptor)),
            server_private_key: keypair.private,
            server_public_key,
        })
    }

    pub(super) fn descriptor_for_ports(
        &self,
        quic_port: u16,
        wss_port: u16,
    ) -> ApiResult<ServiceDescriptor> {
        let mut descriptor = self
            .descriptor
            .lock()
            .map_err(|_| ApiError::TransportSelectorInvariant("descriptor lock poisoned"))?
            .clone();
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
        descriptor.resign_with_example_key_for_testing()?;
        descriptor.validate()?;
        *self
            .descriptor
            .lock()
            .map_err(|_| ApiError::TransportSelectorInvariant("descriptor lock poisoned"))? =
            descriptor.clone();
        Ok(descriptor)
    }

    pub(super) fn responder(&self, mode: AuthorizationMode) -> ApiResult<NoiseResponder> {
        let descriptor = self
            .descriptor
            .lock()
            .map_err(|_| ApiError::TransportSelectorInvariant("descriptor lock poisoned"))?
            .clone();
        let prologue = descriptor.noise_prologue()?;
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
            handshake_payload: handshake_payload(mode),
            queued_plaintext_responses: VecDeque::new(),
        })
    }

    pub(super) const fn server_public_key(&self) -> [u8; 32] {
        self.server_public_key
    }
}

pub(super) struct NoiseResponder {
    handshake: Option<snow::HandshakeState>,
    transport: Option<TransportState>,
    handshake_payload: Vec<u8>,
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
                .write_message(&self.handshake_payload, &mut outbound)
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

fn noise_params() -> ApiResult<snow::params::NoiseParams> {
    NOISE_SUITE_V1
        .parse()
        .map_err(|_| ApiError::InnerNoiseFailure)
}

pub(super) fn boxed_error(message: &'static str) -> BoxError {
    Box::new(std::io::Error::other(message))
}

fn handshake_payload(mode: AuthorizationMode) -> Vec<u8> {
    if matches!(mode, AuthorizationMode::HandshakePayload) {
        b"forbidden".to_vec()
    } else {
        Vec::new()
    }
}
