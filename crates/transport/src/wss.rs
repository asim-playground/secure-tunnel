// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use futures_util::{SinkExt, StreamExt};
use secure_tunnel_core::{
    ApiError, ApiResult, BoxFuture, CarrierConnector, CarrierKind, CloseDirective, FramedDuplex,
    MAX_RECORD_PAYLOAD_SIZE, TransportTarget,
};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::http::header::{HOST, SEC_WEBSOCKET_PROTOCOL};
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, Message, WebSocketConfig};
use tokio_tungstenite::{
    Connector, MaybeTlsStream, WebSocketStream, connect_async_tls_with_config,
};
use url::Url;

use crate::config::TransportClientConfig;
use crate::framing::{validate_inbound_record, validate_outbound_record};

/// Production `WSS` connector for the v1 carrier binding.
#[derive(Debug, Clone)]
pub struct WssConnector {
    config: TransportClientConfig,
}

impl WssConnector {
    /// Creates a `WSS` connector.
    #[must_use]
    pub const fn new(config: TransportClientConfig) -> Self {
        Self { config }
    }
}

impl CarrierConnector for WssConnector {
    fn carrier(&self) -> CarrierKind {
        CarrierKind::Wss
    }

    fn connect<'a>(
        &'a self,
        target: &'a TransportTarget,
    ) -> BoxFuture<'a, ApiResult<Box<dyn FramedDuplex>>> {
        Box::pin(async move {
            let TransportTarget::Wss(target) = target else {
                return Err(ApiError::TransportSelectorInvariant(
                    "WSS connector received a non-WSS target",
                ));
            };
            validate_wss_url(&target.url)?;
            let mut request = target
                .url
                .as_str()
                .into_client_request()
                .map_err(|_| ApiError::OuterProtocolFailure(CarrierKind::Wss))?;
            request.headers_mut().insert(
                SEC_WEBSOCKET_PROTOCOL,
                HeaderValue::from_str(&target.subprotocol)
                    .map_err(|_| ApiError::OuterProtocolFailure(CarrierKind::Wss))?,
            );
            if let Some(authority) = &target.authority_override {
                request.headers_mut().insert(
                    HOST,
                    HeaderValue::from_str(authority)
                        .map_err(|_| ApiError::OuterProtocolFailure(CarrierKind::Wss))?,
                );
            }

            let connector = Connector::Rustls(self.config.wss_client_config()?);
            let (stream, response) = Box::pin(connect_async_tls_with_config(
                request,
                Some(websocket_config()),
                false,
                Some(connector),
            ))
            .await
            .map_err(|error| map_wss_connect_error(&error))?;
            validate_selected_subprotocol(&response, &target.subprotocol)?;

            Ok(Box::new(WssFramedDuplex { stream }) as Box<dyn FramedDuplex>)
        })
    }
}

fn websocket_config() -> WebSocketConfig {
    WebSocketConfig::default()
        .max_message_size(Some(MAX_RECORD_PAYLOAD_SIZE))
        .max_frame_size(Some(MAX_RECORD_PAYLOAD_SIZE))
}

fn validate_wss_url(url: &str) -> ApiResult<()> {
    let parsed = Url::parse(url).map_err(|_| ApiError::OuterProtocolFailure(CarrierKind::Wss))?;
    if parsed.scheme() != "wss" || parsed.host_str().is_none() {
        return Err(ApiError::OuterProtocolFailure(CarrierKind::Wss));
    }
    Ok(())
}

fn validate_selected_subprotocol(
    response: &tokio_tungstenite::tungstenite::handshake::client::Response,
    expected: &str,
) -> ApiResult<()> {
    let Some(selected) = response.headers().get(SEC_WEBSOCKET_PROTOCOL) else {
        return Err(ApiError::OuterProtocolFailure(CarrierKind::Wss));
    };
    let selected = selected
        .to_str()
        .map_err(|_| ApiError::OuterProtocolFailure(CarrierKind::Wss))?;
    if selected == expected {
        Ok(())
    } else {
        Err(ApiError::OuterProtocolFailure(CarrierKind::Wss))
    }
}

const fn map_wss_connect_error(error: &tokio_tungstenite::tungstenite::Error) -> ApiError {
    use tokio_tungstenite::tungstenite::Error;

    match error {
        Error::Tls(_) => ApiError::OuterTlsFailure(CarrierKind::Wss),
        Error::ConnectionClosed | Error::AlreadyClosed | Error::Io(_) => {
            ApiError::OuterPathFailure(CarrierKind::Wss)
        }
        Error::Http(_) | Error::HttpFormat(_) | Error::Protocol(_) | Error::Url(_) => {
            ApiError::OuterProtocolFailure(CarrierKind::Wss)
        }
        _ => ApiError::OuterProtocolFailure(CarrierKind::Wss),
    }
}

struct WssFramedDuplex {
    stream: WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
}

impl FramedDuplex for WssFramedDuplex {
    fn carrier(&self) -> CarrierKind {
        CarrierKind::Wss
    }

    fn send_record<'a>(&'a mut self, record: &'a [u8]) -> BoxFuture<'a, ApiResult<()>> {
        Box::pin(async move {
            validate_outbound_record(record)?;
            self.stream
                .send(Message::Binary(record.to_vec().into()))
                .await
                .map_err(|error| map_wss_runtime_error(&error))
        })
    }

    fn receive_record(&mut self) -> BoxFuture<'_, ApiResult<Option<Vec<u8>>>> {
        Box::pin(async move {
            loop {
                let Some(message) = self.stream.next().await else {
                    return Ok(None);
                };
                match message.map_err(|error| map_wss_runtime_error(&error))? {
                    Message::Binary(payload) => {
                        validate_inbound_record(&payload, CarrierKind::Wss)?;
                        return Ok(Some(payload.to_vec()));
                    }
                    Message::Text(_) => {
                        return Err(ApiError::OuterProtocolFailure(CarrierKind::Wss));
                    }
                    Message::Close(_) => return Ok(None),
                    Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
                }
            }
        })
    }

    fn close(&mut self, directive: CloseDirective) -> BoxFuture<'_, ApiResult<()>> {
        Box::pin(async move {
            let reason = format!("secure tunnel close {}", directive.code);
            self.stream
                .close(Some(CloseFrame {
                    code: 1000.into(),
                    reason: reason.into(),
                }))
                .await
                .map_err(|error| map_wss_runtime_error(&error))
        })
    }
}

const fn map_wss_runtime_error(error: &tokio_tungstenite::tungstenite::Error) -> ApiError {
    use tokio_tungstenite::tungstenite::Error;

    match error {
        Error::ConnectionClosed | Error::AlreadyClosed | Error::Io(_) => ApiError::TransportClosed,
        Error::Tls(_) => ApiError::OuterTlsFailure(CarrierKind::Wss),
        _ => ApiError::OuterProtocolFailure(CarrierKind::Wss),
    }
}
