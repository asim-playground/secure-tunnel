// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::net::{Ipv6Addr, SocketAddr};

use secure_tunnel_core::{
    ApiError, ApiResult, BoxFuture, CarrierConnector, CarrierKind, CloseDirective, FallbackReason,
    FramedDuplex, TransportTarget,
};
use tokio::net::lookup_host;

use crate::config::TransportClientConfig;
use crate::framing::{encoded_record, validate_inbound_record};

/// Production raw `QUIC` connector for the v1 carrier binding.
#[derive(Debug, Clone)]
pub struct QuicConnector {
    config: TransportClientConfig,
}

impl QuicConnector {
    /// Creates a `QUIC` connector.
    #[must_use]
    pub const fn new(config: TransportClientConfig) -> Self {
        Self { config }
    }
}

impl CarrierConnector for QuicConnector {
    fn carrier(&self) -> CarrierKind {
        CarrierKind::Quic
    }

    fn connect<'a>(
        &'a self,
        target: &'a TransportTarget,
    ) -> BoxFuture<'a, ApiResult<Box<dyn FramedDuplex>>> {
        Box::pin(async move {
            let TransportTarget::Quic(target) = target else {
                return Err(ApiError::TransportSelectorInvariant(
                    "QUIC connector received a non-QUIC target",
                ));
            };
            tracing::debug!(
                event_name = "transport.adapter_connect",
                carrier = "quic",
                phase = "connect_start"
            );

            let remote = resolve_quic_addr(&target.connect_host, target.port).await?;
            let mut endpoint =
                quinn::Endpoint::client(SocketAddr::from((Ipv6Addr::UNSPECIFIED, 0)))
                    .map_err(|_| ApiError::TransportFallback(FallbackReason::OuterPathFailure))?;
            endpoint.set_default_client_config(self.config.quic_client_config(&target.alpn)?);
            let server_name = target
                .sni_override
                .as_deref()
                .unwrap_or(&target.connect_host);
            let connecting = endpoint
                .connect(remote, server_name)
                .map_err(|_| ApiError::TransportFallback(FallbackReason::OuterQuicRejected))?;
            let connection = connecting.await.map_err(map_quic_connect_error)?;
            let (send, receive) = connection
                .open_bi()
                .await
                .map_err(|_| ApiError::TransportFallback(FallbackReason::OuterQuicClosedEarly))?;
            tracing::debug!(
                event_name = "transport.adapter_connect",
                carrier = "quic",
                phase = "connect_ready"
            );

            Ok(Box::new(QuicFramedDuplex {
                endpoint,
                connection,
                send,
                receive,
            }) as Box<dyn FramedDuplex>)
        })
    }
}

async fn resolve_quic_addr(host: &str, port: u16) -> ApiResult<SocketAddr> {
    let mut addresses = lookup_host((host, port))
        .await
        .map_err(|_| ApiError::TransportFallback(FallbackReason::OuterPathFailure))?;
    addresses.next().ok_or(ApiError::TransportFallback(
        FallbackReason::OuterPathFailure,
    ))
}

fn map_quic_connect_error(error: quinn::ConnectionError) -> ApiError {
    match error {
        quinn::ConnectionError::TransportError(inner) if is_no_application_protocol(inner.code) => {
            ApiError::TransportFallback(FallbackReason::OuterQuicRejected)
        }
        quinn::ConnectionError::TransportError(inner) if is_crypto_error(inner.code) => {
            ApiError::OuterTlsFailure(CarrierKind::Quic)
        }
        quinn::ConnectionError::VersionMismatch
        | quinn::ConnectionError::TransportError(_)
        | quinn::ConnectionError::ConnectionClosed(_)
        | quinn::ConnectionError::ApplicationClosed(_) => {
            ApiError::TransportFallback(FallbackReason::OuterQuicRejected)
        }
        _ => ApiError::TransportFallback(FallbackReason::OuterPathFailure),
    }
}

fn is_no_application_protocol(code: quinn::TransportErrorCode) -> bool {
    code == quinn::TransportErrorCode::crypto(120)
}

fn is_crypto_error(code: quinn::TransportErrorCode) -> bool {
    let value = u64::from(code);
    (0x100..0x200).contains(&value)
}

struct QuicFramedDuplex {
    endpoint: quinn::Endpoint,
    connection: quinn::Connection,
    send: quinn::SendStream,
    receive: quinn::RecvStream,
}

impl FramedDuplex for QuicFramedDuplex {
    fn carrier(&self) -> CarrierKind {
        CarrierKind::Quic
    }

    fn send_record<'a>(&'a mut self, record: &'a [u8]) -> BoxFuture<'a, ApiResult<()>> {
        Box::pin(async move {
            let encoded = encoded_record(record)?;
            self.send
                .write_all(&encoded)
                .await
                .map_err(|_| ApiError::TransportClosed)
        })
    }

    fn receive_record(&mut self) -> BoxFuture<'_, ApiResult<Option<Vec<u8>>>> {
        Box::pin(async move {
            let mut length = [0_u8; 2];
            if self.receive.read_exact(&mut length).await.is_err() {
                return Ok(None);
            }
            let mut payload = vec![0_u8; usize::from(u16::from_be_bytes(length))];
            self.receive
                .read_exact(&mut payload)
                .await
                .map_err(|_| ApiError::TransportClosed)?;
            validate_inbound_record(&payload, CarrierKind::Quic)?;
            Ok(Some(payload))
        })
    }

    fn close(&mut self, _directive: CloseDirective) -> BoxFuture<'_, ApiResult<()>> {
        Box::pin(async move {
            self.connection.close(0_u32.into(), b"secure tunnel close");
            self.endpoint.close(0_u32.into(), b"secure tunnel close");
            Ok(())
        })
    }
}
