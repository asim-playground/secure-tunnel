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
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{Instant, timeout};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::handshake::client::{Request, Response};
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::http::header::{HOST, SEC_WEBSOCKET_PROTOCOL};
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, Message, WebSocketConfig};
use tokio_tungstenite::{
    Connector, MaybeTlsStream, WebSocketStream, client_async_tls_with_config,
    connect_async_tls_with_config,
};
use url::{Host, Url};

use crate::config::{HttpProxyConfig, TransportClientConfig};
use crate::framing::{validate_inbound_record, validate_outbound_record};

const MAX_PROXY_CONNECT_RESPONSE_BYTES: usize = 8192;

type WssStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

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
            tracing::debug!(
                event_name = "transport.adapter_connect",
                carrier = "wss",
                phase = "connect_start"
            );
            let target_url = validate_wss_url(&target.url)?;
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

            let timeouts = self.config.timeouts();
            let (stream, response) =
                connect_wss_stream(&self.config, &target_url, request, timeouts.wss_connect)
                    .await?;
            validate_selected_subprotocol(&response, &target.subprotocol)?;
            tracing::debug!(
                event_name = "transport.adapter_connect",
                carrier = "wss",
                phase = "connect_ready"
            );

            Ok(Box::new(WssFramedDuplex {
                stream,
                read_timeout: timeouts.record_read,
                write_timeout: timeouts.record_write,
            }) as Box<dyn FramedDuplex>)
        })
    }
}

fn websocket_config() -> WebSocketConfig {
    WebSocketConfig::default()
        .max_message_size(Some(MAX_RECORD_PAYLOAD_SIZE))
        .max_frame_size(Some(MAX_RECORD_PAYLOAD_SIZE))
}

fn validate_wss_url(url: &str) -> ApiResult<Url> {
    let parsed = Url::parse(url).map_err(|_| ApiError::OuterProtocolFailure(CarrierKind::Wss))?;
    if parsed.scheme() != "wss" || parsed.host_str().is_none() {
        return Err(ApiError::OuterProtocolFailure(CarrierKind::Wss));
    }
    Ok(parsed)
}

async fn connect_wss_stream(
    config: &TransportClientConfig,
    target_url: &Url,
    request: Request,
    budget: std::time::Duration,
) -> ApiResult<(WssStream, Response)> {
    match config.wss_http_proxy() {
        Some(proxy) => connect_wss_via_proxy(config, proxy, target_url, request, budget).await,
        None => connect_wss_direct(config, request, budget).await,
    }
}

async fn connect_wss_direct(
    config: &TransportClientConfig,
    request: Request,
    budget: std::time::Duration,
) -> ApiResult<(WssStream, Response)> {
    let connector = Connector::Rustls(config.wss_client_config()?);
    timeout(
        budget,
        Box::pin(connect_async_tls_with_config(
            request,
            Some(websocket_config()),
            false,
            Some(connector),
        )),
    )
    .await
    .map_err(|_| ApiError::OuterPathFailure(CarrierKind::Wss))?
    .map_err(|error| map_wss_connect_error(&error))
}

async fn connect_wss_via_proxy(
    config: &TransportClientConfig,
    proxy: &HttpProxyConfig,
    target_url: &Url,
    request: Request,
    budget: std::time::Duration,
) -> ApiResult<(WssStream, Response)> {
    let deadline = Instant::now() + budget;
    let tunnel = timeout(
        remaining_budget(deadline),
        connect_proxy_tunnel(proxy, target_url),
    )
    .await
    .map_err(|_| ApiError::OuterProxyFailure(CarrierKind::Wss))??;
    let connector = Connector::Rustls(config.wss_client_config()?);
    timeout(
        remaining_budget(deadline),
        Box::pin(client_async_tls_with_config(
            request,
            tunnel,
            Some(websocket_config()),
            Some(connector),
        )),
    )
    .await
    .map_err(|_| ApiError::OuterPathFailure(CarrierKind::Wss))?
    .map_err(|error| map_wss_connect_error(&error))
}

fn remaining_budget(deadline: Instant) -> std::time::Duration {
    deadline.saturating_duration_since(Instant::now())
}

async fn connect_proxy_tunnel(proxy: &HttpProxyConfig, target_url: &Url) -> ApiResult<TcpStream> {
    let parsed_proxy = parse_http_proxy(proxy)?;
    let target_authority = authority_for_url(target_url, target_url.port_or_known_default())?;
    let mut stream = TcpStream::connect((parsed_proxy.host.as_str(), parsed_proxy.port))
        .await
        .map_err(|_| ApiError::OuterProxyFailure(CarrierKind::Wss))?;
    let request =
        format!("CONNECT {target_authority} HTTP/1.1\r\nHost: {target_authority}\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|_| ApiError::OuterProxyFailure(CarrierKind::Wss))?;
    read_proxy_connect_response(&mut stream).await?;
    Ok(stream)
}

struct ParsedHttpProxy {
    host: String,
    port: u16,
}

fn parse_http_proxy(proxy: &HttpProxyConfig) -> ApiResult<ParsedHttpProxy> {
    let url = Url::parse(&proxy.url).map_err(|_| ApiError::OuterProxyFailure(CarrierKind::Wss))?;
    if url.scheme() != "http"
        || url.username() != ""
        || url.password().is_some()
        || !matches!(url.path(), "" | "/")
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ApiError::OuterProxyFailure(CarrierKind::Wss));
    }
    let host = host_for_connect(&url)?;
    let port = url
        .port()
        .ok_or(ApiError::OuterProxyFailure(CarrierKind::Wss))?;
    Ok(ParsedHttpProxy { host, port })
}

fn host_for_connect(url: &Url) -> ApiResult<String> {
    match url
        .host()
        .ok_or(ApiError::OuterProxyFailure(CarrierKind::Wss))?
    {
        Host::Domain(domain) => Ok(domain.to_owned()),
        Host::Ipv4(address) => Ok(address.to_string()),
        Host::Ipv6(address) => Ok(address.to_string()),
    }
}

fn authority_for_url(url: &Url, port: Option<u16>) -> ApiResult<String> {
    let port = port.ok_or(ApiError::OuterProxyFailure(CarrierKind::Wss))?;
    let host = match url
        .host()
        .ok_or(ApiError::OuterProxyFailure(CarrierKind::Wss))?
    {
        Host::Domain(domain) => domain.to_owned(),
        Host::Ipv4(address) => address.to_string(),
        Host::Ipv6(address) => format!("[{address}]"),
    };
    Ok(format!("{host}:{port}"))
}

async fn read_proxy_connect_response(stream: &mut TcpStream) -> ApiResult<()> {
    let mut response = Vec::with_capacity(128);
    let mut byte = [0_u8; 1];
    while !response.ends_with(b"\r\n\r\n") {
        if response.len() >= MAX_PROXY_CONNECT_RESPONSE_BYTES {
            return Err(ApiError::OuterProxyFailure(CarrierKind::Wss));
        }
        let read = stream
            .read(&mut byte)
            .await
            .map_err(|_| ApiError::OuterProxyFailure(CarrierKind::Wss))?;
        if read == 0 {
            return Err(ApiError::OuterProxyFailure(CarrierKind::Wss));
        }
        response.push(byte[0]);
    }
    validate_proxy_connect_response(&response)
}

fn validate_proxy_connect_response(response: &[u8]) -> ApiResult<()> {
    let response =
        std::str::from_utf8(response).map_err(|_| ApiError::OuterProxyFailure(CarrierKind::Wss))?;
    let status_line = response
        .split("\r\n")
        .next()
        .ok_or(ApiError::OuterProxyFailure(CarrierKind::Wss))?;
    let mut parts = status_line.split_whitespace();
    let version = parts
        .next()
        .ok_or(ApiError::OuterProxyFailure(CarrierKind::Wss))?;
    let status = parts
        .next()
        .ok_or(ApiError::OuterProxyFailure(CarrierKind::Wss))?;
    if matches!(version, "HTTP/1.0" | "HTTP/1.1") && status == "200" {
        Ok(())
    } else {
        Err(ApiError::OuterProxyFailure(CarrierKind::Wss))
    }
}

fn validate_selected_subprotocol(response: &Response, expected: &str) -> ApiResult<()> {
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

fn map_wss_connect_error(error: &tokio_tungstenite::tungstenite::Error) -> ApiError {
    use tokio_tungstenite::tungstenite::Error;

    match error {
        Error::Tls(_) => ApiError::OuterTlsFailure(CarrierKind::Wss),
        Error::Io(inner) if is_tls_io_error(inner) => ApiError::OuterTlsFailure(CarrierKind::Wss),
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
    stream: WssStream,
    read_timeout: std::time::Duration,
    write_timeout: std::time::Duration,
}

impl FramedDuplex for WssFramedDuplex {
    fn carrier(&self) -> CarrierKind {
        CarrierKind::Wss
    }

    fn send_record<'a>(&'a mut self, record: &'a [u8]) -> BoxFuture<'a, ApiResult<()>> {
        Box::pin(async move {
            validate_outbound_record(record)?;
            timeout(
                self.write_timeout,
                self.stream.send(Message::Binary(record.to_vec().into())),
            )
            .await
            .map_err(|_| ApiError::TransportClosed)?
            .map_err(|error| map_wss_runtime_error(&error))
        })
    }

    fn receive_record(&mut self) -> BoxFuture<'_, ApiResult<Option<Vec<u8>>>> {
        Box::pin(async move {
            timeout(self.read_timeout, receive_wss_record(&mut self.stream))
                .await
                .map_err(|_| ApiError::TransportClosed)?
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

async fn receive_wss_record(stream: &mut WssStream) -> ApiResult<Option<Vec<u8>>> {
    loop {
        let Some(message) = stream.next().await else {
            return Ok(None);
        };
        match message.map_err(|error| map_wss_runtime_error(&error))? {
            Message::Binary(payload) => {
                validate_inbound_record(&payload, CarrierKind::Wss)?;
                return Ok(Some(payload.to_vec()));
            }
            Message::Text(_) => return Err(ApiError::OuterProtocolFailure(CarrierKind::Wss)),
            Message::Close(_) => return Ok(None),
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
        }
    }
}

fn map_wss_runtime_error(error: &tokio_tungstenite::tungstenite::Error) -> ApiError {
    use tokio_tungstenite::tungstenite::Error;

    match error {
        Error::Tls(_) => ApiError::OuterTlsFailure(CarrierKind::Wss),
        Error::Io(inner) if is_tls_io_error(inner) => ApiError::OuterTlsFailure(CarrierKind::Wss),
        Error::ConnectionClosed | Error::AlreadyClosed | Error::Io(_) => ApiError::TransportClosed,
        _ => ApiError::OuterProtocolFailure(CarrierKind::Wss),
    }
}

fn is_tls_io_error(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::InvalidData
}

#[cfg(test)]
mod tests {
    use tokio_tungstenite::tungstenite::Error;

    use crate::HttpProxyConfig;

    use super::{map_wss_runtime_error, parse_http_proxy, websocket_config};

    #[test]
    fn connection_closed_runtime_errors_map_to_transport_closed() {
        assert_eq!(
            map_wss_runtime_error(&Error::ConnectionClosed),
            secure_tunnel_core::ApiError::TransportClosed
        );
        assert_eq!(
            map_wss_runtime_error(&Error::AlreadyClosed),
            secure_tunnel_core::ApiError::TransportClosed
        );
        assert_eq!(
            map_wss_runtime_error(&Error::Io(std::io::Error::other("closed"))),
            secure_tunnel_core::ApiError::TransportClosed
        );
    }

    #[test]
    fn websocket_config_caps_frames_and_messages_to_record_limit() {
        let config = websocket_config();

        assert_eq!(
            config.max_message_size,
            Some(secure_tunnel_core::MAX_RECORD_PAYLOAD_SIZE)
        );
        assert_eq!(
            config.max_frame_size,
            Some(secure_tunnel_core::MAX_RECORD_PAYLOAD_SIZE)
        );
    }

    #[test]
    fn http_proxy_config_accepts_only_plain_explicit_connect_url() {
        assert!(parse_http_proxy(&HttpProxyConfig::new("http://127.0.0.1:8080")).is_ok());
        for url in [
            "https://127.0.0.1:8080",
            "http://127.0.0.1",
            "http://user@127.0.0.1:8080",
            "http://127.0.0.1:8080/proxy",
            "http://127.0.0.1:8080?x=1",
            "http://127.0.0.1:8080#frag",
            "socks5://127.0.0.1:1080",
        ] {
            assert!(
                parse_http_proxy(&HttpProxyConfig::new(url)).is_err(),
                "proxy URL should be rejected: {url}"
            );
        }
    }
}
