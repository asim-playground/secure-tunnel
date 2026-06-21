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
mod tests;
