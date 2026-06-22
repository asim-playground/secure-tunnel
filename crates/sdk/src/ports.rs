// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

pub(super) trait TransportPorts: Send + Sync {
    fn quic(&self) -> Option<&dyn secure_tunnel_core::CarrierConnector>;
    fn wss(&self) -> Option<&dyn secure_tunnel_core::CarrierConnector>;
    fn secure_ready(&self) -> &dyn secure_tunnel_core::SecureReadyEvaluator;
}

pub(super) struct ProductionTransportPorts {
    inner: secure_tunnel_transport::ProductionTransportPorts,
}

impl ProductionTransportPorts {
    pub(super) fn new(config: &crate::ClientConfig) -> Self {
        let mut transport_config = config
            .outer_root_certificates_der
            .as_ref()
            .map_or_else(
                secure_tunnel_transport::TransportClientConfig::platform_verifier,
                |certificates| {
                    secure_tunnel_transport::TransportClientConfig::with_root_certificate_der(
                        certificates.clone(),
                    )
                },
            )
            .with_descriptor_trust_anchors(config.descriptor_trust_anchors.clone())
            .with_pinned_service_static_public_keys(
                config.pinned_service_static_public_keys.clone(),
            )
            .with_timeouts(secure_tunnel_transport::TransportClientTimeouts {
                quic_connect: std::time::Duration::from_millis(
                    config.transport_policy.quic_connect_timeout_ms,
                ),
                wss_connect: std::time::Duration::from_millis(
                    config.transport_policy.wss_connect_timeout_ms,
                ),
                record_read: std::time::Duration::from_millis(
                    config.transport_policy.record_read_timeout_ms,
                ),
                record_write: std::time::Duration::from_millis(
                    config.transport_policy.record_write_timeout_ms,
                ),
            });
        if let Some(proxy) = &config.wss_http_proxy {
            transport_config = transport_config.with_wss_http_proxy(
                secure_tunnel_transport::HttpProxyConfig::new(proxy.url.clone()),
            );
        }
        Self {
            inner: secure_tunnel_transport::ProductionTransportPorts::new(transport_config),
        }
    }
}

impl TransportPorts for ProductionTransportPorts {
    fn quic(&self) -> Option<&dyn secure_tunnel_core::CarrierConnector> {
        Some(self.inner.quic())
    }

    fn wss(&self) -> Option<&dyn secure_tunnel_core::CarrierConnector> {
        Some(self.inner.wss())
    }

    fn secure_ready(&self) -> &dyn secure_tunnel_core::SecureReadyEvaluator {
        self.inner.secure_ready()
    }
}
