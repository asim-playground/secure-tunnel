// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use quinn::crypto::rustls::QuicClientConfig;
use rustls::crypto::ring::default_provider;
use rustls::pki_types::CertificateDer;
use rustls::{ClientConfig, RootCertStore};
use rustls_platform_verifier::BuilderVerifierExt;
use secure_tunnel_core::{
    ApiError, ApiResult, CarrierKind, NoisePublicKey, TrustAnchor,
    example_descriptor_trust_anchors, obfuscated_service_static_public_key,
};

/// TLS verifier configuration shared by the production carrier adapters.
#[derive(Debug, Clone)]
pub struct TransportClientConfig {
    root_certificates_der: Option<Vec<Vec<u8>>>,
    descriptor_trust_anchors: Vec<TrustAnchor>,
    pinned_service_static_public_keys: Vec<NoisePublicKey>,
}

impl TransportClientConfig {
    /// Creates a configuration that verifies certificates with the platform.
    #[must_use]
    pub const fn platform_verifier() -> Self {
        Self {
            root_certificates_der: None,
            descriptor_trust_anchors: Vec::new(),
            pinned_service_static_public_keys: Vec::new(),
        }
    }

    /// Creates a configuration that trusts only the supplied DER roots.
    ///
    /// This is mainly used by local integration tests until the SDK-facing
    /// custom-CA configuration lands in task `00000013`.
    #[must_use]
    pub const fn with_root_certificate_der(root_certificates_der: Vec<Vec<u8>>) -> Self {
        Self {
            root_certificates_der: Some(root_certificates_der),
            descriptor_trust_anchors: Vec::new(),
            pinned_service_static_public_keys: Vec::new(),
        }
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
        if let Some(certificates) = &self.root_certificates_der {
            client_config_with_roots(certificates, carrier)
        } else {
            let builder = ClientConfig::builder_with_provider(default_provider().into())
                .with_safe_default_protocol_versions()
                .map_err(|_| ApiError::OuterTlsFailure(carrier))?
                .with_platform_verifier()
                .map_err(|_| ApiError::OuterTlsFailure(carrier))?;
            Ok(builder.with_no_client_auth())
        }
    }
}

impl Default for TransportClientConfig {
    fn default() -> Self {
        Self {
            root_certificates_der: None,
            descriptor_trust_anchors: example_descriptor_trust_anchors(),
            pinned_service_static_public_keys: vec![obfuscated_service_static_public_key()],
        }
    }
}

fn client_config_with_roots(
    certificates: &[Vec<u8>],
    carrier: CarrierKind,
) -> ApiResult<ClientConfig> {
    let mut roots = RootCertStore::empty();
    for certificate in certificates {
        roots
            .add(CertificateDer::from(certificate.clone()))
            .map_err(|_| ApiError::OuterTlsFailure(carrier))?;
    }

    let builder = ClientConfig::builder_with_provider(default_provider().into())
        .with_safe_default_protocol_versions()
        .map_err(|_| ApiError::OuterTlsFailure(carrier))?;
    Ok(builder.with_root_certificates(roots).with_no_client_auth())
}
