// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use quinn::crypto::rustls::QuicServerConfig;
use rustls::ServerConfig;
use rustls::crypto::ring::default_provider;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use secure_tunnel_core::{ApiError, ApiResult};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::http::header::SEC_WEBSOCKET_PROTOCOL;
use tokio_tungstenite::tungstenite::protocol::Message;

use crate::fixture::LocalServiceFixture;
use crate::{HarnessError, HarnessResult, map_external};

pub struct QuicServer {
    port: u16,
    certificate: TlsFixture,
    task: JoinHandle<()>,
}

impl QuicServer {
    pub fn start(fixture: LocalServiceFixture, alpns: Vec<Vec<u8>>) -> HarnessResult<Self> {
        let certificate = TlsFixture::new()?;
        let server_config = quic_server_config(&certificate, alpns)?;
        let endpoint = map_external(quinn::Endpoint::server(
            server_config,
            SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        ))?;
        let port = map_external(endpoint.local_addr())?.port();
        let task = tokio::spawn(async move {
            while let Some(incoming) = endpoint.accept().await {
                let fixture = fixture.clone();
                tokio::spawn(async move {
                    if let Ok(connection) = incoming.await {
                        let _ = handle_quic_connection(connection, fixture).await;
                    }
                });
            }
        });

        Ok(Self {
            port,
            certificate,
            task,
        })
    }

    pub const fn port(&self) -> u16 {
        self.port
    }

    pub fn root_certificate_der(&self) -> Vec<u8> {
        self.certificate.root_der()
    }
}

impl Drop for QuicServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub struct WssServer {
    port: u16,
    certificate: TlsFixture,
    task: JoinHandle<()>,
}

impl WssServer {
    pub async fn start(fixture: LocalServiceFixture) -> HarnessResult<Self> {
        let certificate = TlsFixture::new()?;
        let server_config = wss_server_config(&certificate)?;
        let acceptor = TlsAcceptor::from(Arc::new(server_config));
        let listener =
            map_external(TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await)?;
        let port = map_external(listener.local_addr())?.port();
        let task = tokio::spawn(async move {
            loop {
                let Ok((stream, _peer)) = listener.accept().await else {
                    break;
                };
                let acceptor = acceptor.clone();
                let fixture = fixture.clone();
                tokio::spawn(async move {
                    if let Ok(tls) = acceptor.accept(stream).await {
                        let _ = Box::pin(handle_wss_connection(tls, fixture)).await;
                    }
                });
            }
        });

        Ok(Self {
            port,
            certificate,
            task,
        })
    }

    pub const fn port(&self) -> u16 {
        self.port
    }

    pub fn root_certificate_der(&self) -> Vec<u8> {
        self.certificate.root_der()
    }
}

impl Drop for WssServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn handle_quic_connection(
    connection: quinn::Connection,
    fixture: LocalServiceFixture,
) -> ApiResult<()> {
    let (mut send, mut receive) = connection
        .accept_bi()
        .await
        .map_err(|_| ApiError::OuterProtocolFailure(secure_tunnel_core::CarrierKind::Quic))?;
    let mut responder = fixture.responder()?;
    loop {
        let Some(record) = read_quic_record(&mut receive).await? else {
            return Ok(());
        };
        if let Some(outbound) = responder.process_record(&record)? {
            send.write_all(&encode_quic_record(&outbound)?)
                .await
                .map_err(|_| ApiError::TransportClosed)?;
        }
    }
}

async fn read_quic_record(receive: &mut quinn::RecvStream) -> ApiResult<Option<Vec<u8>>> {
    let mut length = [0_u8; 2];
    if receive.read_exact(&mut length).await.is_err() {
        return Ok(None);
    }
    let mut payload = vec![0_u8; usize::from(u16::from_be_bytes(length))];
    receive
        .read_exact(&mut payload)
        .await
        .map_err(|_| ApiError::TransportClosed)?;
    Ok(Some(payload))
}

async fn handle_wss_connection<S>(stream: S, fixture: LocalServiceFixture) -> ApiResult<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut websocket = accept_hdr_async(stream, select_wss_subprotocol)
        .await
        .map_err(|_| ApiError::OuterProtocolFailure(secure_tunnel_core::CarrierKind::Wss))?;
    let mut responder = fixture.responder()?;

    while let Some(message) = websocket.next().await {
        match message.map_err(|_| ApiError::TransportClosed)? {
            Message::Binary(record) => {
                if let Some(outbound) = responder.process_record(&record)? {
                    websocket
                        .send(Message::Binary(outbound.into()))
                        .await
                        .map_err(|_| ApiError::TransportClosed)?;
                }
            }
            Message::Close(_) => return Ok(()),
            Message::Text(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
        }
    }
    Ok(())
}

fn encode_quic_record(record: &[u8]) -> ApiResult<Vec<u8>> {
    let length = u16::try_from(record.len()).map_err(|_| ApiError::RecordTooLarge {
        actual: record.len(),
        max: usize::from(u16::MAX),
    })?;
    let mut encoded = Vec::with_capacity(2 + record.len());
    encoded.extend_from_slice(&length.to_be_bytes());
    encoded.extend_from_slice(record);
    Ok(encoded)
}

#[allow(clippy::result_large_err, clippy::unnecessary_wraps)]
fn select_wss_subprotocol(
    _request: &Request,
    mut response: Response,
) -> Result<Response, ErrorResponse> {
    let value = HeaderValue::from_static(secure_tunnel_core::WSS_SUBPROTOCOL_V1);
    response.headers_mut().insert(SEC_WEBSOCKET_PROTOCOL, value);
    Ok(response)
}

#[derive(Clone)]
struct TlsFixture {
    certificate_der: Vec<u8>,
    private_key_der: Vec<u8>,
}

impl TlsFixture {
    fn new() -> HarnessResult<Self> {
        let certified = rcgen::generate_simple_self_signed(["127.0.0.1".to_owned()])
            .map_err(HarnessError::external)?;
        Ok(Self {
            certificate_der: certified.cert.der().as_ref().to_vec(),
            private_key_der: certified.signing_key.serialize_der(),
        })
    }

    fn certificate(&self) -> CertificateDer<'static> {
        CertificateDer::from(self.certificate_der.clone())
    }

    fn private_key(&self) -> PrivateKeyDer<'static> {
        PrivatePkcs8KeyDer::from(self.private_key_der.clone()).into()
    }

    fn root_der(&self) -> Vec<u8> {
        self.certificate_der.clone()
    }
}

fn quic_server_config(
    certificate: &TlsFixture,
    alpns: Vec<Vec<u8>>,
) -> HarnessResult<quinn::ServerConfig> {
    if alpns.is_empty() {
        return Err(HarnessError::Invariant(
            "QUIC smoke server requires at least one ALPN",
        ));
    }
    let mut tls = ServerConfig::builder_with_provider(default_provider().into())
        .with_safe_default_protocol_versions()
        .map_err(HarnessError::external)?
        .with_no_client_auth()
        .with_single_cert(vec![certificate.certificate()], certificate.private_key())
        .map_err(HarnessError::external)?;
    tls.alpn_protocols = alpns;
    let crypto = QuicServerConfig::try_from(tls).map_err(HarnessError::external)?;
    Ok(quinn::ServerConfig::with_crypto(Arc::new(crypto)))
}

fn wss_server_config(certificate: &TlsFixture) -> HarnessResult<ServerConfig> {
    ServerConfig::builder_with_provider(default_provider().into())
        .with_safe_default_protocol_versions()
        .map_err(HarnessError::external)?
        .with_no_client_auth()
        .with_single_cert(vec![certificate.certificate()], certificate.private_key())
        .map_err(HarnessError::external)
}
