// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;
use std::time::Duration;

use quinn::crypto::rustls::QuicClientConfig;
use rustls::ClientConfig;
use rustls::crypto::CryptoProvider;
use rustls::crypto::ring::default_provider;
use rustls::pki_types::CertificateDer;
use rustls_platform_verifier::{BuilderVerifierExt, Verifier};
use secure_tunnel_core::{
    ApiError, ApiResult, CarrierKind, NoisePublicKey, TrustAnchor,
    example_descriptor_trust_anchors, obfuscated_service_static_public_key,
};

/// TLS verifier configuration shared by the production carrier adapters.
#[derive(Debug, Clone)]
pub struct TransportClientConfig {
    root_certificates_der: Option<Vec<Vec<u8>>>,
    wss_http_proxy: Option<HttpProxyConfig>,
    descriptor_trust_anchors: Vec<TrustAnchor>,
    pinned_service_static_public_keys: Vec<NoisePublicKey>,
    timeouts: TransportClientTimeouts,
}

/// Explicit HTTP proxy used by the `WSS` carrier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpProxyConfig {
    /// Plain HTTP proxy URL in the v1 `http://host:port` form.
    pub url: String,
}

/// Timeout budgets enforced by production carrier adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportClientTimeouts {
    /// Budget for `QUIC` DNS, handshake, and stream-open phases.
    pub quic_connect: Duration,
    /// Budget for the `WSS` TCP/TLS/WebSocket handshake.
    pub wss_connect: Duration,
    /// Budget for one framed record read.
    pub record_read: Duration,
    /// Budget for one framed record write.
    pub record_write: Duration,
}

impl TransportClientConfig {
    /// Creates a configuration that verifies certificates with the platform.
    #[must_use]
    pub fn platform_verifier() -> Self {
        Self {
            root_certificates_der: None,
            wss_http_proxy: None,
            descriptor_trust_anchors: Vec::new(),
            pinned_service_static_public_keys: Vec::new(),
            timeouts: TransportClientTimeouts::default(),
        }
    }

    /// Creates a configuration that augments platform trust with DER roots.
    ///
    /// The extra roots apply only to outer carrier TLS. Inner descriptor and
    /// service-static-key trust remain configured separately.
    ///
    /// Android extra-root support is currently unavailable in
    /// `rustls-platform-verifier` `0.7`; non-empty roots fail outer TLS there.
    #[must_use]
    pub fn with_root_certificate_der(root_certificates_der: Vec<Vec<u8>>) -> Self {
        Self {
            root_certificates_der: Some(root_certificates_der),
            wss_http_proxy: None,
            descriptor_trust_anchors: Vec::new(),
            pinned_service_static_public_keys: Vec::new(),
            timeouts: TransportClientTimeouts::default(),
        }
    }

    /// Sets an explicit plain HTTP `CONNECT` proxy for `WSS` only.
    #[must_use]
    pub fn with_wss_http_proxy(mut self, proxy: HttpProxyConfig) -> Self {
        self.wss_http_proxy = Some(proxy);
        self
    }

    /// Sets pinned descriptor roots for service descriptor authorization.
    #[must_use]
    pub fn with_descriptor_trust_anchors(mut self, anchors: Vec<TrustAnchor>) -> Self {
        self.descriptor_trust_anchors = anchors;
        self
    }

    /// Sets pinned service static public keys accepted by the `NK1` handshake.
    #[must_use]
    pub fn with_pinned_service_static_public_keys(mut self, keys: Vec<NoisePublicKey>) -> Self {
        self.pinned_service_static_public_keys = keys;
        self
    }

    /// Sets the adapter timeout budgets.
    #[must_use]
    pub const fn with_timeouts(mut self, timeouts: TransportClientTimeouts) -> Self {
        self.timeouts = timeouts;
        self
    }

    pub(crate) const fn timeouts(&self) -> TransportClientTimeouts {
        self.timeouts
    }

    pub(crate) const fn wss_http_proxy(&self) -> Option<&HttpProxyConfig> {
        self.wss_http_proxy.as_ref()
    }

    pub(crate) fn descriptor_trust_anchors(&self) -> Vec<TrustAnchor> {
        self.descriptor_trust_anchors.clone()
    }

    pub(crate) fn pinned_service_static_public_keys(&self) -> Vec<NoisePublicKey> {
        self.pinned_service_static_public_keys.clone()
    }

    pub(crate) fn quic_client_config(&self, alpn: &str) -> ApiResult<quinn::ClientConfig> {
        let mut tls = self.rustls_client_config(CarrierKind::Quic)?;
        tls.alpn_protocols = vec![alpn.as_bytes().to_vec()];
        let crypto = QuicClientConfig::try_from(tls)
            .map_err(|_| ApiError::OuterTlsFailure(CarrierKind::Quic))?;
        Ok(quinn::ClientConfig::new(Arc::new(crypto)))
    }

    pub(crate) fn wss_client_config(&self) -> ApiResult<Arc<ClientConfig>> {
        self.rustls_client_config(CarrierKind::Wss).map(Arc::new)
    }

    fn rustls_client_config(&self, carrier: CarrierKind) -> ApiResult<ClientConfig> {
        self.root_certificates_der.as_ref().map_or_else(
            || platform_client_config(carrier),
            |certificates| client_config_with_extra_roots(certificates, carrier),
        )
    }
}

impl Default for TransportClientConfig {
    fn default() -> Self {
        Self {
            root_certificates_der: None,
            wss_http_proxy: None,
            descriptor_trust_anchors: example_descriptor_trust_anchors(),
            pinned_service_static_public_keys: vec![obfuscated_service_static_public_key()],
            timeouts: TransportClientTimeouts::default(),
        }
    }
}

impl HttpProxyConfig {
    /// Creates an explicit HTTP proxy configuration.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

impl Default for TransportClientTimeouts {
    fn default() -> Self {
        Self {
            quic_connect: Duration::from_secs(2),
            wss_connect: Duration::from_secs(5),
            record_read: Duration::from_secs(30),
            record_write: Duration::from_secs(30),
        }
    }
}

fn platform_client_config(carrier: CarrierKind) -> ApiResult<ClientConfig> {
    let builder = ClientConfig::builder_with_provider(default_provider().into())
        .with_safe_default_protocol_versions()
        .map_err(|_| ApiError::OuterTlsFailure(carrier))?
        .with_platform_verifier()
        .map_err(|_| ApiError::OuterTlsFailure(carrier))?;
    Ok(builder.with_no_client_auth())
}

#[cfg(not(target_os = "android"))]
fn client_config_with_extra_roots(
    certificates: &[Vec<u8>],
    carrier: CarrierKind,
) -> ApiResult<ClientConfig> {
    if certificates.is_empty() {
        return platform_client_config(carrier);
    }

    let extra_roots: Vec<CertificateDer<'static>> = certificates
        .iter()
        .map(|certificate| CertificateDer::from(certificate.clone()))
        .collect();
    let provider: Arc<CryptoProvider> = default_provider().into();
    let verifier = Verifier::new_with_extra_roots(extra_roots, Arc::clone(&provider))
        .map_err(|_| ApiError::OuterTlsFailure(carrier))?;
    let builder = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|_| ApiError::OuterTlsFailure(carrier))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier));
    Ok(builder.with_no_client_auth())
}

#[cfg(target_os = "android")]
fn client_config_with_extra_roots(
    certificates: &[Vec<u8>],
    carrier: CarrierKind,
) -> ApiResult<ClientConfig> {
    if certificates.is_empty() {
        platform_client_config(carrier)
    } else {
        Err(ApiError::OuterTlsFailure(carrier))
    }
}

#[cfg(test)]
mod tests {
    use super::TransportClientConfig;

    #[test]
    fn empty_extra_roots_use_platform_verifier() {
        assert!(
            TransportClientConfig::with_root_certificate_der(Vec::new())
                .wss_client_config()
                .is_ok()
        );
    }

    #[test]
    fn malformed_extra_roots_fail_tls_config() {
        assert!(
            TransportClientConfig::with_root_certificate_der(vec![b"not der".to_vec()])
                .wss_client_config()
                .is_err()
        );
    }
}
