// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_util::{SinkExt, StreamExt};
use quinn::crypto::rustls::QuicServerConfig;
use rustls::ServerConfig;
use rustls::crypto::ring::default_provider;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use secure_tunnel_core::{ApiResult, MAX_RECORD_PAYLOAD_SIZE};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::http::header::SEC_WEBSOCKET_PROTOCOL;
use tokio_tungstenite::tungstenite::protocol::Message;

use super::fixture::{AuthorizationMode, BoxError, ServiceFixture, TestResult, boxed_error};
use crate::framing::encoded_record;

pub(super) struct QuicServer {
    port: u16,
    certificate: TlsFixture,
    task: JoinHandle<()>,
}

impl QuicServer {
    pub(super) fn start(
        fixture: ServiceFixture,
        mode: AuthorizationMode,
        alpns: Vec<Vec<u8>>,
    ) -> TestResult<Self> {
        let certificate = TlsFixture::new()?;
        let server_config = quic_server_config(&certificate, alpns)?;
        let endpoint =
            quinn::Endpoint::server(server_config, SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))?;
        let port = endpoint.local_addr()?.port();
        let task = tokio::spawn(async move {
            while let Some(incoming) = endpoint.accept().await {
                let fixture = fixture.clone();
                tokio::spawn(async move {
                    if let Ok(connection) = incoming.await {
                        let _ = handle_quic_connection(connection, fixture, mode).await;
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

    pub(super) fn start_closing_after_handshake(alpns: Vec<Vec<u8>>) -> TestResult<Self> {
        let certificate = TlsFixture::new()?;
        let server_config = quic_server_config(&certificate, alpns)?;
        let endpoint =
            quinn::Endpoint::server(server_config, SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))?;
        let port = endpoint.local_addr()?.port();
        let task = tokio::spawn(async move {
            while let Some(incoming) = endpoint.accept().await {
                tokio::spawn(async move {
                    if let Ok(connection) = incoming.await {
                        connection.close(0_u32.into(), b"test close before secure ready");
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

    pub(super) const fn port(&self) -> u16 {
        self.port
    }

    pub(super) fn root_certificate_der(&self) -> Vec<u8> {
        self.certificate.root_der()
    }
}

impl Drop for QuicServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub(super) struct WssServer {
    port: u16,
    certificate: TlsFixture,
    connection_count: Arc<AtomicUsize>,
    task: JoinHandle<()>,
}

impl WssServer {
    pub(super) async fn start(
        fixture: ServiceFixture,
        mode: AuthorizationMode,
    ) -> TestResult<Self> {
        let certificate = TlsFixture::new()?;
        let server_config = wss_server_config(&certificate)?;
        let acceptor = TlsAcceptor::from(Arc::new(server_config));
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
        let port = listener.local_addr()?.port();
        let connection_count = Arc::new(AtomicUsize::new(0));
        let task_count = Arc::clone(&connection_count);
        let task = tokio::spawn(async move {
            loop {
                let Ok((stream, _peer)) = listener.accept().await else {
                    break;
                };
                let acceptor = acceptor.clone();
                let fixture = fixture.clone();
                let task_count = Arc::clone(&task_count);
                tokio::spawn(async move {
                    if let Ok(tls) = acceptor.accept(stream).await {
                        task_count.fetch_add(1, Ordering::SeqCst);
                        let _ = Box::pin(handle_wss_connection(tls, fixture, mode)).await;
                    }
                });
            }
        });

        Ok(Self {
            port,
            certificate,
            connection_count,
            task,
        })
    }

    pub(super) async fn start_oversized_message() -> TestResult<Self> {
        let certificate = TlsFixture::new()?;
        let server_config = wss_server_config(&certificate)?;
        let acceptor = TlsAcceptor::from(Arc::new(server_config));
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
        let port = listener.local_addr()?.port();
        let connection_count = Arc::new(AtomicUsize::new(0));
        let task_count = Arc::clone(&connection_count);
        let task = tokio::spawn(async move {
            loop {
                let Ok((stream, _peer)) = listener.accept().await else {
                    break;
                };
                let acceptor = acceptor.clone();
                let task_count = Arc::clone(&task_count);
                tokio::spawn(async move {
                    if let Ok(tls) = acceptor.accept(stream).await {
                        task_count.fetch_add(1, Ordering::SeqCst);
                        let _ = Box::pin(handle_oversized_wss_connection(tls)).await;
                    }
                });
            }
        });

        Ok(Self {
            port,
            certificate,
            connection_count,
            task,
        })
    }

    pub(super) async fn start_stalled_after_websocket() -> TestResult<Self> {
        let certificate = TlsFixture::new()?;
        let server_config = wss_server_config(&certificate)?;
        let acceptor = TlsAcceptor::from(Arc::new(server_config));
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
        let port = listener.local_addr()?.port();
        let connection_count = Arc::new(AtomicUsize::new(0));
        let task_count = Arc::clone(&connection_count);
        let task = tokio::spawn(async move {
            loop {
                let Ok((stream, _peer)) = listener.accept().await else {
                    break;
                };
                let acceptor = acceptor.clone();
                let task_count = Arc::clone(&task_count);
                tokio::spawn(async move {
                    if let Ok(tls) = acceptor.accept(stream).await {
                        task_count.fetch_add(1, Ordering::SeqCst);
                        let _ = Box::pin(handle_stalled_wss_connection(tls)).await;
                    }
                });
            }
        });

        Ok(Self {
            port,
            certificate,
            connection_count,
            task,
        })
    }

    pub(super) async fn start_pinging_after_websocket() -> TestResult<Self> {
        let certificate = TlsFixture::new()?;
        let server_config = wss_server_config(&certificate)?;
        let acceptor = TlsAcceptor::from(Arc::new(server_config));
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
        let port = listener.local_addr()?.port();
        let connection_count = Arc::new(AtomicUsize::new(0));
        let task_count = Arc::clone(&connection_count);
        let task = tokio::spawn(async move {
            loop {
                let Ok((stream, _peer)) = listener.accept().await else {
                    break;
                };
                let acceptor = acceptor.clone();
                let task_count = Arc::clone(&task_count);
                tokio::spawn(async move {
                    if let Ok(tls) = acceptor.accept(stream).await {
                        task_count.fetch_add(1, Ordering::SeqCst);
                        let _ = Box::pin(handle_pinging_wss_connection(tls)).await;
                    }
                });
            }
        });

        Ok(Self {
            port,
            certificate,
            connection_count,
            task,
        })
    }

    pub(super) const fn port(&self) -> u16 {
        self.port
    }

    pub(super) fn root_certificate_der(&self) -> Vec<u8> {
        self.certificate.root_der()
    }

    pub(super) fn connection_count(&self) -> usize {
        self.connection_count.load(Ordering::SeqCst)
    }
}

impl Drop for WssServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn handle_quic_connection(
    connection: quinn::Connection,
    fixture: ServiceFixture,
    mode: AuthorizationMode,
) -> ApiResult<()> {
    let (mut send, mut receive) = connection.accept_bi().await.map_err(|_| {
        secure_tunnel_core::ApiError::OuterProtocolFailure(secure_tunnel_core::CarrierKind::Quic)
    })?;
    let mut responder = fixture.responder(mode)?;
    loop {
        let Some(record) = read_quic_record(&mut receive).await? else {
            return Ok(());
        };
        if let Some(outbound) = responder.process_record(&record)? {
            let encoded = encoded_record(&outbound)?;
            send.write_all(&encoded)
                .await
                .map_err(|_| secure_tunnel_core::ApiError::TransportClosed)?;
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
        .map_err(|_| secure_tunnel_core::ApiError::TransportClosed)?;
    Ok(Some(payload))
}

async fn handle_wss_connection<S>(
    stream: S,
    fixture: ServiceFixture,
    mode: AuthorizationMode,
) -> ApiResult<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut websocket = accept_hdr_async(stream, select_wss_subprotocol)
        .await
        .map_err(|_| {
            secure_tunnel_core::ApiError::OuterProtocolFailure(secure_tunnel_core::CarrierKind::Wss)
        })?;
    let mut responder = fixture.responder(mode)?;

    while let Some(message) = websocket.next().await {
        match message.map_err(|_| secure_tunnel_core::ApiError::TransportClosed)? {
            Message::Binary(record) => {
                if let Some(outbound) = responder.process_record(&record)? {
                    websocket
                        .send(Message::Binary(outbound.into()))
                        .await
                        .map_err(|_| secure_tunnel_core::ApiError::TransportClosed)?;
                }
            }
            Message::Close(_) => return Ok(()),
            Message::Text(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
        }
    }
    Ok(())
}

async fn handle_oversized_wss_connection<S>(stream: S) -> ApiResult<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut websocket = accept_hdr_async(stream, select_wss_subprotocol)
        .await
        .map_err(|_| {
            secure_tunnel_core::ApiError::OuterProtocolFailure(secure_tunnel_core::CarrierKind::Wss)
        })?;
    websocket
        .send(Message::Binary(
            vec![0_u8; MAX_RECORD_PAYLOAD_SIZE + 1].into(),
        ))
        .await
        .map_err(|_| secure_tunnel_core::ApiError::TransportClosed)
}

async fn handle_stalled_wss_connection<S>(stream: S) -> ApiResult<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let _websocket = accept_hdr_async(stream, select_wss_subprotocol)
        .await
        .map_err(|_| {
            secure_tunnel_core::ApiError::OuterProtocolFailure(secure_tunnel_core::CarrierKind::Wss)
        })?;
    std::future::pending::<()>().await;
    Ok(())
}

async fn handle_pinging_wss_connection<S>(stream: S) -> ApiResult<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut websocket = accept_hdr_async(stream, select_wss_subprotocol)
        .await
        .map_err(|_| {
            secure_tunnel_core::ApiError::OuterProtocolFailure(secure_tunnel_core::CarrierKind::Wss)
        })?;
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        websocket
            .send(Message::Ping(Vec::new().into()))
            .await
            .map_err(|_| secure_tunnel_core::ApiError::TransportClosed)?;
    }
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
    fn new() -> TestResult<Self> {
        let certified = rcgen::generate_simple_self_signed(["127.0.0.1".to_owned()])?;
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
) -> TestResult<quinn::ServerConfig> {
    if alpns.is_empty() {
        return Err(boxed_error("QUIC test server requires at least one ALPN"));
    }
    let mut tls = ServerConfig::builder_with_provider(default_provider().into())
        .with_safe_default_protocol_versions()?
        .with_no_client_auth()
        .with_single_cert(vec![certificate.certificate()], certificate.private_key())?;
    tls.alpn_protocols = alpns;
    let crypto = QuicServerConfig::try_from(tls).map_err(map_boxed)?;
    Ok(quinn::ServerConfig::with_crypto(Arc::new(crypto)))
}

fn wss_server_config(certificate: &TlsFixture) -> TestResult<ServerConfig> {
    ServerConfig::builder_with_provider(default_provider().into())
        .with_safe_default_protocol_versions()?
        .with_no_client_auth()
        .with_single_cert(vec![certificate.certificate()], certificate.private_key())
        .map_err(map_boxed)
}

fn map_boxed(error: impl std::error::Error + Send + Sync + 'static) -> BoxError {
    Box::new(error)
}
