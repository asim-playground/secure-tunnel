// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

mod fixture;
mod proxy;
mod proxy_cases;
mod server;

use secure_tunnel_core::{
    ApiError, CandidateSource, CarrierConnector, CarrierKind, FallbackReason,
    MAX_RECORD_PAYLOAD_SIZE, TransportCacheSnapshot, TransportConnectors, TransportSelector,
    TransportTarget, WSS_SUBPROTOCOL_V1, example_descriptor_trust_anchors,
};

use self::fixture::{AuthorizationMode, ServiceFixture, TestResult};
use self::server::{QuicServer, WssServer};
use crate::{ProductionTransportPorts, QuicConnector, TransportClientConfig, WssConnector};

const NOW_UNIX_SECONDS: u64 = 1_742_000_000;

#[tokio::test]
async fn custom_root_quic_adapter_reaches_secure_ready() -> TestResult<()> {
    let fixture = ServiceFixture::new()?;
    let quic = QuicServer::start(
        fixture.clone(),
        AuthorizationMode::Valid,
        vec![secure_tunnel_core::QUIC_ALPN_V1.as_bytes().to_vec()],
    )?;
    let descriptor = fixture.descriptor_for_ports(quic.port(), 9)?;
    let ports = transport_ports([quic.root_certificate_der()], &fixture);

    let selected = TransportSelector::new(300)
        .select(
            &descriptor,
            None,
            NOW_UNIX_SECONDS,
            TransportConnectors::new(Some(ports.quic()), Some(ports.wss())),
            ports.secure_ready(),
        )
        .await?;

    assert_eq!(selected.report.carrier, CarrierKind::Quic);
    assert_eq!(selected.report.fallback_reason, None);
    assert_eq!(selected.attempts.len(), 1);
    assert_eq!(selected.attempts[0].carrier, CarrierKind::Quic);
    assert!(
        !selected
            .artifacts
            .handshake_hash
            .unwrap_or_default()
            .is_empty()
    );
    Ok(())
}

#[tokio::test]
async fn custom_root_cached_fallback_wss_adapter_reaches_secure_ready() -> TestResult<()> {
    let fixture = ServiceFixture::new()?;
    let wss = WssServer::start(fixture.clone(), AuthorizationMode::Valid).await?;
    let descriptor = fixture.descriptor_for_ports(9, wss.port())?;
    let ports = transport_ports([wss.root_certificate_der()], &fixture);
    let cache = cached_quic_bad();

    let selected = TransportSelector::new(300)
        .select(
            &descriptor,
            Some(&cache),
            NOW_UNIX_SECONDS,
            TransportConnectors::new(Some(ports.quic()), Some(ports.wss())),
            ports.secure_ready(),
        )
        .await?;

    assert_eq!(selected.report.carrier, CarrierKind::Wss);
    assert_eq!(
        selected.report.fallback_reason,
        Some(FallbackReason::OuterPathFailure)
    );
    assert_eq!(selected.attempts.len(), 1);
    assert_eq!(
        selected.attempts[0].source,
        CandidateSource::CachedQuicBadNetwork
    );
    Ok(())
}

#[tokio::test]
async fn custom_root_quic_alpn_rejection_falls_back_to_wss() -> TestResult<()> {
    let fixture = ServiceFixture::new()?;
    let quic = QuicServer::start(
        fixture.clone(),
        AuthorizationMode::Valid,
        vec![b"wrong".to_vec()],
    )?;
    let wss = WssServer::start(fixture.clone(), AuthorizationMode::Valid).await?;
    let descriptor = fixture.descriptor_for_ports(quic.port(), wss.port())?;
    let ports = transport_ports(
        [quic.root_certificate_der(), wss.root_certificate_der()],
        &fixture,
    );

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
    assert_eq!(wss.connection_count(), 1);
    Ok(())
}

#[tokio::test]
async fn quic_stream_open_close_falls_back_to_wss() -> TestResult<()> {
    let fixture = ServiceFixture::new()?;
    let quic = QuicServer::start_closing_after_handshake(vec![
        secure_tunnel_core::QUIC_ALPN_V1.as_bytes().to_vec(),
    ])?;
    let wss = WssServer::start(fixture.clone(), AuthorizationMode::Valid).await?;
    let descriptor = fixture.descriptor_for_ports(quic.port(), wss.port())?;
    let ports = transport_ports(
        [quic.root_certificate_der(), wss.root_certificate_der()],
        &fixture,
    );

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
        Some(FallbackReason::OuterQuicClosedEarly)
    );
    assert_eq!(selected.attempts.len(), 2);
    assert_eq!(wss.connection_count(), 1);
    Ok(())
}

#[tokio::test]
async fn malformed_wss_target_fails_before_io() -> TestResult<()> {
    let connector = WssConnector::new(TransportClientConfig::with_root_certificate_der(Vec::new()));
    let target = TransportTarget::Wss(secure_tunnel_core::WssTarget {
        url: "https://127.0.0.1/tunnel".to_owned(),
        subprotocol: WSS_SUBPROTOCOL_V1.to_owned(),
        authority_override: None,
    });

    let Err(error) = connector.connect(&target).await else {
        panic!("malformed WSS target should fail");
    };

    assert_eq!(error, ApiError::OuterProtocolFailure(CarrierKind::Wss));
    Ok(())
}

#[tokio::test]
async fn wrong_custom_root_quic_fails_as_outer_tls_failure() -> TestResult<()> {
    let fixture = ServiceFixture::new()?;
    let quic = QuicServer::start(
        fixture,
        AuthorizationMode::Valid,
        vec![secure_tunnel_core::QUIC_ALPN_V1.as_bytes().to_vec()],
    )?;
    let wrong_root = QuicServer::start_closing_after_handshake(vec![
        secure_tunnel_core::QUIC_ALPN_V1.as_bytes().to_vec(),
    ])?;
    let connector = QuicConnector::new(
        TransportClientConfig::with_root_certificate_der(vec![wrong_root.root_certificate_der()])
            .with_timeouts(short_timeouts()),
    );
    let target = TransportTarget::Quic(secure_tunnel_core::QuicTarget {
        connect_host: "127.0.0.1".to_owned(),
        port: quic.port(),
        alpn: secure_tunnel_core::QUIC_ALPN_V1.to_owned(),
        sni_override: None,
    });

    let Err(error) = connector.connect(&target).await else {
        panic!("wrong custom QUIC root should fail");
    };

    assert_eq!(error, ApiError::OuterTlsFailure(CarrierKind::Quic));
    Ok(())
}

#[tokio::test]
async fn wrong_custom_root_wss_fails_as_outer_tls_failure() -> TestResult<()> {
    let fixture = ServiceFixture::new()?;
    let wss = WssServer::start(fixture, AuthorizationMode::Valid).await?;
    let wrong_root = WssServer::start_stalled_after_websocket().await?;
    let connector = WssConnector::new(
        TransportClientConfig::with_root_certificate_der(vec![wrong_root.root_certificate_der()])
            .with_timeouts(short_timeouts()),
    );
    let target = TransportTarget::Wss(secure_tunnel_core::WssTarget {
        url: format!("wss://127.0.0.1:{}/tunnel", wss.port()),
        subprotocol: WSS_SUBPROTOCOL_V1.to_owned(),
        authority_override: None,
    });

    let Err(error) = connector.connect(&target).await else {
        panic!("wrong custom WSS root should fail");
    };

    assert_eq!(error, ApiError::OuterTlsFailure(CarrierKind::Wss));
    Ok(())
}

#[tokio::test]
async fn oversized_wss_message_fails_at_adapter_limit() -> TestResult<()> {
    let wss = WssServer::start_oversized_message().await?;
    let connector = WssConnector::new(TransportClientConfig::with_root_certificate_der(vec![
        wss.root_certificate_der(),
    ]));
    let target = TransportTarget::Wss(secure_tunnel_core::WssTarget {
        url: format!("wss://127.0.0.1:{}/tunnel", wss.port()),
        subprotocol: WSS_SUBPROTOCOL_V1.to_owned(),
        authority_override: None,
    });

    let mut transport = connector.connect(&target).await?;
    let Err(error) = transport.receive_record().await else {
        panic!("oversized WSS message should fail");
    };

    assert_eq!(
        error,
        ApiError::OuterProtocolFailure(CarrierKind::Wss),
        "payloads larger than {MAX_RECORD_PAYLOAD_SIZE} bytes must fail before secure-ready"
    );
    Ok(())
}

#[tokio::test]
async fn stalled_wss_secure_ready_is_bounded_by_record_timeout() -> TestResult<()> {
    let wss = WssServer::start_stalled_after_websocket().await?;
    let connector = WssConnector::new(
        TransportClientConfig::with_root_certificate_der(vec![wss.root_certificate_der()])
            .with_timeouts(crate::TransportClientTimeouts {
                quic_connect: std::time::Duration::from_millis(100),
                wss_connect: std::time::Duration::from_millis(100),
                record_read: std::time::Duration::from_millis(20),
                record_write: std::time::Duration::from_millis(100),
            }),
    );
    let target = TransportTarget::Wss(secure_tunnel_core::WssTarget {
        url: format!("wss://127.0.0.1:{}/tunnel", wss.port()),
        subprotocol: WSS_SUBPROTOCOL_V1.to_owned(),
        authority_override: None,
    });

    let mut transport = connector.connect(&target).await?;
    let started = std::time::Instant::now();
    let Err(error) = transport.receive_record().await else {
        panic!("stalled WSS secure-ready read should time out");
    };

    assert_eq!(error, ApiError::TransportClosed);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "stalled WSS record reads must be bounded"
    );
    Ok(())
}

#[tokio::test]
async fn wss_control_frames_do_not_extend_record_read_timeout() -> TestResult<()> {
    let wss = WssServer::start_pinging_after_websocket().await?;
    let connector = WssConnector::new(
        TransportClientConfig::with_root_certificate_der(vec![wss.root_certificate_der()])
            .with_timeouts(crate::TransportClientTimeouts {
                quic_connect: std::time::Duration::from_millis(100),
                wss_connect: std::time::Duration::from_millis(100),
                record_read: std::time::Duration::from_millis(20),
                record_write: std::time::Duration::from_millis(100),
            }),
    );
    let target = TransportTarget::Wss(secure_tunnel_core::WssTarget {
        url: format!("wss://127.0.0.1:{}/tunnel", wss.port()),
        subprotocol: WSS_SUBPROTOCOL_V1.to_owned(),
        authority_override: None,
    });

    let mut transport = connector.connect(&target).await?;
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(200),
        transport.receive_record(),
    )
    .await
    .expect("logical record read timeout should fire before test timeout");
    let Err(error) = result else {
        panic!("ping-only WSS stream should time out before any record arrives");
    };

    assert_eq!(error, ApiError::TransportClosed);
    Ok(())
}

#[tokio::test]
async fn inner_trust_failure_does_not_fallback_to_wss() -> TestResult<()> {
    let fixture = ServiceFixture::new()?;
    let quic = QuicServer::start(
        fixture.clone(),
        AuthorizationMode::HandshakePayload,
        vec![secure_tunnel_core::QUIC_ALPN_V1.as_bytes().to_vec()],
    )?;
    let wss = WssServer::start(fixture.clone(), AuthorizationMode::Valid).await?;
    let descriptor = fixture.descriptor_for_ports(quic.port(), wss.port())?;
    let ports = transport_ports(
        [quic.root_certificate_der(), wss.root_certificate_der()],
        &fixture,
    );

    let Err(error) = TransportSelector::new(300)
        .select(
            &descriptor,
            None,
            NOW_UNIX_SECONDS,
            TransportConnectors::new(Some(ports.quic()), Some(ports.wss())),
            ports.secure_ready(),
        )
        .await
    else {
        panic!("selector should stop on inner trust failure");
    };

    assert_eq!(error.cause, ApiError::InnerTrustFailure);
    assert_eq!(error.attempts.len(), 1);
    assert_eq!(wss.connection_count(), 0);
    Ok(())
}

fn transport_ports(
    certificates: impl IntoIterator<Item = Vec<u8>>,
    fixture: &ServiceFixture,
) -> ProductionTransportPorts {
    ProductionTransportPorts::new(transport_config(certificates, fixture))
}

fn transport_config(
    certificates: impl IntoIterator<Item = Vec<u8>>,
    fixture: &ServiceFixture,
) -> TransportClientConfig {
    TransportClientConfig::with_root_certificate_der(certificates.into_iter().collect())
        .with_descriptor_trust_anchors(example_descriptor_trust_anchors())
        .with_pinned_service_static_public_keys(vec![fixture.server_public_key()])
}

fn cached_quic_bad() -> TransportCacheSnapshot {
    TransportCacheSnapshot {
        last_successful_carrier: Some(CarrierKind::Wss),
        last_quic_failure: Some(FallbackReason::OuterPathFailure),
        next_quic_probe_after_unix_seconds: Some(NOW_UNIX_SECONDS + 60),
        highest_descriptor_serial: Some(1),
    }
}

const fn short_timeouts() -> crate::TransportClientTimeouts {
    crate::TransportClientTimeouts {
        quic_connect: std::time::Duration::from_millis(100),
        wss_connect: std::time::Duration::from_millis(100),
        record_read: std::time::Duration::from_millis(100),
        record_write: std::time::Duration::from_millis(100),
    }
}

fn wss_target(port: u16) -> TransportTarget {
    TransportTarget::Wss(secure_tunnel_core::WssTarget {
        url: format!("wss://127.0.0.1:{port}/tunnel"),
        subprotocol: WSS_SUBPROTOCOL_V1.to_owned(),
        authority_override: None,
    })
}
