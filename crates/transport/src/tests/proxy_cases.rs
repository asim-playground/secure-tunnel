// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use secure_tunnel_core::{
    ApiError, CarrierConnector, CarrierKind, FallbackReason, TransportConnectors, TransportSelector,
};

use super::fixture::{AuthorizationMode, ServiceFixture, TestResult};
use super::proxy::HttpProxyServer;
use super::server::{QuicServer, WssServer};
use super::{NOW_UNIX_SECONDS, cached_quic_bad, short_timeouts, transport_config, wss_target};
use crate::{HttpProxyConfig, ProductionTransportPorts, TransportClientConfig, WssConnector};

#[tokio::test]
async fn proxied_wss_adapter_reaches_secure_ready() -> TestResult<()> {
    let fixture = ServiceFixture::new()?;
    let wss = WssServer::start(fixture.clone(), AuthorizationMode::Valid).await?;
    let proxy = HttpProxyServer::start_tunnel().await?;
    let descriptor = fixture.descriptor_for_ports(9, wss.port())?;
    let config = transport_config([wss.root_certificate_der()], &fixture)
        .with_wss_http_proxy(HttpProxyConfig::new(proxy.url()));
    let ports = ProductionTransportPorts::new(config);

    let selected = TransportSelector::new(300)
        .select(
            &descriptor,
            Some(&cached_quic_bad()),
            NOW_UNIX_SECONDS,
            TransportConnectors::new(Some(ports.quic()), Some(ports.wss())),
            ports.secure_ready(),
        )
        .await?;

    assert_eq!(selected.report.carrier, CarrierKind::Wss);
    assert_eq!(selected.attempts.len(), 1);
    let expected_authority = format!("127.0.0.1:{}", wss.port());
    assert_eq!(
        proxy.last_connect_authority().as_deref(),
        Some(expected_authority.as_str())
    );
    Ok(())
}

#[tokio::test]
async fn wss_proxy_connect_rejection_maps_to_outer_proxy_failure() -> TestResult<()> {
    let fixture = ServiceFixture::new()?;
    let wss = WssServer::start(fixture, AuthorizationMode::Valid).await?;
    let proxy = HttpProxyServer::start_rejecting().await?;
    let connector = WssConnector::new(
        TransportClientConfig::with_root_certificate_der(vec![wss.root_certificate_der()])
            .with_wss_http_proxy(HttpProxyConfig::new(proxy.url()))
            .with_timeouts(short_timeouts()),
    );
    let mut target = wss_target(wss.port());
    if let secure_tunnel_core::TransportTarget::Wss(target) = &mut target {
        target.authority_override = Some("different.example:443".to_owned());
    }

    let Err(error) = connector.connect(&target).await else {
        panic!("proxy rejection should fail");
    };

    assert_eq!(error, ApiError::OuterProxyFailure(CarrierKind::Wss));
    let expected_authority = format!("127.0.0.1:{}", wss.port());
    assert_eq!(
        proxy.last_connect_authority().as_deref(),
        Some(expected_authority.as_str())
    );
    Ok(())
}

#[tokio::test]
async fn proxied_wss_bad_outer_root_maps_to_outer_tls_failure() -> TestResult<()> {
    let fixture = ServiceFixture::new()?;
    let wss = WssServer::start(fixture, AuthorizationMode::Valid).await?;
    let wrong_root = WssServer::start_stalled_after_websocket().await?;
    let proxy = HttpProxyServer::start_tunnel().await?;
    let connector = WssConnector::new(
        TransportClientConfig::with_root_certificate_der(vec![wrong_root.root_certificate_der()])
            .with_wss_http_proxy(HttpProxyConfig::new(proxy.url()))
            .with_timeouts(short_timeouts()),
    );
    let target = wss_target(wss.port());

    let Err(error) = connector.connect(&target).await else {
        panic!("bad outer WSS root should fail after CONNECT");
    };

    assert_eq!(error, ApiError::OuterTlsFailure(CarrierKind::Wss));
    let expected_authority = format!("127.0.0.1:{}", wss.port());
    assert_eq!(
        proxy.last_connect_authority().as_deref(),
        Some(expected_authority.as_str())
    );
    Ok(())
}

#[tokio::test]
async fn quic_rejection_falls_back_to_proxied_wss_with_attempt_trace() -> TestResult<()> {
    let fixture = ServiceFixture::new()?;
    let quic = QuicServer::start(
        fixture.clone(),
        AuthorizationMode::Valid,
        vec![b"wrong".to_vec()],
    )?;
    let wss = WssServer::start(fixture.clone(), AuthorizationMode::Valid).await?;
    let proxy = HttpProxyServer::start_tunnel().await?;
    let descriptor = fixture.descriptor_for_ports(quic.port(), wss.port())?;
    let config = transport_config(
        [quic.root_certificate_der(), wss.root_certificate_der()],
        &fixture,
    )
    .with_wss_http_proxy(HttpProxyConfig::new(proxy.url()));
    let ports = ProductionTransportPorts::new(config);

    let selected = TransportSelector::new(300)
        .select(
            &descriptor,
            None,
            NOW_UNIX_SECONDS,
            TransportConnectors::new(Some(ports.quic()), Some(ports.wss())),
            ports.secure_ready(),
        )
        .await?;

    assert_eq!(selected.report.carrier, CarrierKind::Wss);
    assert_eq!(
        selected.report.fallback_reason,
        Some(FallbackReason::OuterQuicRejected)
    );
    assert_eq!(selected.attempts.len(), 2);
    assert_eq!(selected.attempts[0].carrier, CarrierKind::Quic);
    assert_eq!(selected.attempts[1].carrier, CarrierKind::Wss);
    let expected_authority = format!("127.0.0.1:{}", wss.port());
    assert_eq!(
        proxy.last_connect_authority().as_deref(),
        Some(expected_authority.as_str())
    );
    Ok(())
}
