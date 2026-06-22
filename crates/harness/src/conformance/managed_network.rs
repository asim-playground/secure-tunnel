// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use ed25519_dalek::SigningKey;

use super::{ConformanceReport, ConformanceScenario};
use crate::fixture::LocalServiceFixture;
use crate::proxy::HttpProxyServer;
use crate::server::{QuicServer, WssServer};
use crate::{DEVICE_KEY_SEED, HarnessError, HarnessResult, NOW_UNIX_SECONDS};

/// Runs the managed-network `WSS` HTTP proxy scenario.
///
/// # Errors
///
/// Returns an error when local fixtures fail before assertions can run.
pub(super) async fn proxied_wss() -> HarnessResult<ConformanceReport> {
    let service = ProxiedService::start().await?;
    let proxy_url = service.proxy.url();
    let config = service
        .config()
        .with_wss_http_proxy(secure_tunnel_sdk::HttpProxyConfig { url: proxy_url });
    let outcome = service
        .connect(config, Some(quic_bad_cache()))
        .await?
        .map_err(HarnessError::Connect)?;
    let expected_authority = format!("127.0.0.1:{}", service.wss.port());
    let proxied =
        service.proxy.last_connect_authority().as_deref() == Some(expected_authority.as_str());
    Ok(ConformanceReport {
        scenario: ConformanceScenario::ProxiedWss,
        ok: proxied
            && outcome.report.selected_carrier == secure_tunnel_sdk::Carrier::Wss
            && !outcome.report.attempts.is_empty(),
        selected_carrier: Some(outcome.report.selected_carrier),
        terminal_error_kind: None,
        fallback_reason: outcome.report.fallback_reason,
        close_classification: None,
        attempts: outcome.report.attempts,
    })
}

const fn quic_bad_cache() -> secure_tunnel_sdk::TransportCacheSnapshot {
    secure_tunnel_sdk::TransportCacheSnapshot {
        last_successful_carrier: Some(secure_tunnel_sdk::Carrier::Wss),
        last_quic_failure: Some(secure_tunnel_sdk::FallbackReason::OuterPathFailure),
        next_quic_probe_after_unix_seconds: Some(NOW_UNIX_SECONDS + 60),
        highest_descriptor_serial: Some(1),
    }
}

struct ProxiedService {
    fixture: LocalServiceFixture,
    quic: QuicServer,
    wss: WssServer,
    proxy: HttpProxyServer,
    descriptor: secure_tunnel_core::ServiceDescriptor,
}

impl ProxiedService {
    async fn start() -> HarnessResult<Self> {
        let signing_key = SigningKey::from_bytes(&DEVICE_KEY_SEED);
        let fixture = LocalServiceFixture::new(signing_key.verifying_key().to_bytes())?;
        let wss = WssServer::start(fixture.clone()).await?;
        let quic = QuicServer::start(
            fixture.clone(),
            vec![secure_tunnel_core::QUIC_ALPN_V1.as_bytes().to_vec()],
        )?;
        let proxy = HttpProxyServer::start_tunnel().await?;
        let descriptor = fixture.descriptor_for_ports(quic.port(), wss.port())?;
        Ok(Self {
            fixture,
            quic,
            wss,
            proxy,
            descriptor,
        })
    }

    fn config(&self) -> secure_tunnel_sdk::ClientConfig {
        secure_tunnel_sdk::ClientConfig::default()
            .with_outer_root_certificates_der(vec![
                self.quic.root_certificate_der(),
                self.wss.root_certificate_der(),
            ])
            .with_descriptor_trust_anchors(secure_tunnel_core::example_descriptor_trust_anchors())
            .with_pinned_service_static_public_keys(vec![self.fixture.server_public_key()])
    }

    async fn connect(
        &self,
        config: secure_tunnel_sdk::ClientConfig,
        cache: Option<secure_tunnel_sdk::TransportCacheSnapshot>,
    ) -> HarnessResult<Result<secure_tunnel_sdk::ConnectOutcome, secure_tunnel_sdk::ConnectError>>
    {
        let descriptor_json = serde_json::to_string(&self.descriptor)?;
        let descriptor = secure_tunnel_sdk::BootstrapDescriptor::from_json(&descriptor_json)
            .map_err(HarnessError::Sdk)?;
        let mut options = secure_tunnel_sdk::ConnectOptions::new(descriptor, NOW_UNIX_SECONDS);
        if let Some(cache) = cache {
            options = options.with_transport_cache(cache);
        }
        Ok(secure_tunnel_sdk::SecureTunnelClient::new(config)
            .connect(options)
            .await)
    }
}
