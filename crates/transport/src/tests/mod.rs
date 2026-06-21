// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

mod fixture;
mod server;

use secure_tunnel_core::{
    ApiError, CandidateSource, CarrierConnector, CarrierKind, FallbackReason,
    MAX_RECORD_PAYLOAD_SIZE, TransportCacheSnapshot, TransportConnectors, TransportSelector,
    TransportTarget, WSS_SUBPROTOCOL_V1,
};

use self::fixture::{AuthorizationMode, ServiceFixture, TestResult};
use self::server::{QuicServer, WssServer};
use crate::{ProductionTransportPorts, TransportClientConfig, WssConnector};

const NOW_UNIX_SECONDS: u64 = 1_742_000_000;

#[tokio::test]
async fn quic_adapter_reaches_secure_ready() -> TestResult<()> {
    let fixture = ServiceFixture::new()?;
    let quic = QuicServer::start(
        fixture.clone(),
        AuthorizationMode::Valid,
        vec![secure_tunnel_core::QUIC_ALPN_V1.as_bytes().to_vec()],
    )?;
    let descriptor = fixture.descriptor_for_ports(quic.port(), 9)?;
    let ports = transport_ports([quic.root_certificate_der()]);

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
async fn cached_fallback_wss_adapter_reaches_secure_ready() -> TestResult<()> {
    let fixture = ServiceFixture::new()?;
    let wss = WssServer::start(fixture.clone(), AuthorizationMode::Valid).await?;
    let descriptor = fixture.descriptor_for_ports(9, wss.port())?;
    let ports = transport_ports([wss.root_certificate_der()]);
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
async fn quic_alpn_rejection_falls_back_to_wss() -> TestResult<()> {
    let fixture = ServiceFixture::new()?;
    let quic = QuicServer::start(
        fixture.clone(),
        AuthorizationMode::Valid,
        vec![b"wrong".to_vec()],
    )?;
    let wss = WssServer::start(fixture.clone(), AuthorizationMode::Valid).await?;
    let descriptor = fixture.descriptor_for_ports(quic.port(), wss.port())?;
    let ports = transport_ports([quic.root_certificate_der(), wss.root_certificate_der()]);

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
    let ports = transport_ports([quic.root_certificate_der(), wss.root_certificate_der()]);

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
async fn inner_trust_failure_does_not_fallback_to_wss() -> TestResult<()> {
    let fixture = ServiceFixture::new()?;
    let quic = QuicServer::start(
        fixture.clone(),
        AuthorizationMode::BadSignature,
        vec![secure_tunnel_core::QUIC_ALPN_V1.as_bytes().to_vec()],
    )?;
    let wss = WssServer::start(fixture.clone(), AuthorizationMode::Valid).await?;
    let descriptor = fixture.descriptor_for_ports(quic.port(), wss.port())?;
    let ports = transport_ports([quic.root_certificate_der(), wss.root_certificate_der()]);

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

fn transport_ports(certificates: impl IntoIterator<Item = Vec<u8>>) -> ProductionTransportPorts {
    ProductionTransportPorts::new(TransportClientConfig::with_root_certificate_der(
        certificates.into_iter().collect(),
    ))
}

fn cached_quic_bad() -> TransportCacheSnapshot {
    TransportCacheSnapshot {
        last_successful_carrier: Some(CarrierKind::Wss),
        last_quic_failure: Some(FallbackReason::OuterPathFailure),
        next_quic_probe_after_unix_seconds: Some(NOW_UNIX_SECONDS + 60),
    }
}
