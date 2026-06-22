// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signer, SigningKey};

use super::{ConformanceReport, ConformanceScenario};
use crate::fixture::LocalServiceFixture;
use crate::server::{QuicServer, WssServer};
use crate::{
    DEVICE_KEY_ID, DEVICE_KEY_SEED, HarnessError, HarnessResult, NOW_UNIX_MS, NOW_UNIX_SECONDS,
};

/// Runs one conformance scenario.
///
/// # Errors
///
/// Returns an error when a local fixture fails before a scenario can assert the
/// expected outcome.
pub async fn run_conformance_scenario(
    scenario: ConformanceScenario,
) -> HarnessResult<ConformanceReport> {
    match scenario {
        ConformanceScenario::QuicSuccess | ConformanceScenario::CustomCaQuicSuccess => {
            success_scenario(scenario, good_alpn(), None).await
        }
        ConformanceScenario::QuicRejectedWssFallback
        | ConformanceScenario::CustomCaQuicRejectedWssFallback => {
            success_scenario(scenario, bad_alpn(), None).await
        }
        ConformanceScenario::CachedQuicBadWssFirst | ConformanceScenario::CustomCaWssSuccess => {
            success_scenario(scenario, good_alpn(), Some(quic_bad_cache())).await
        }
        ConformanceScenario::FallbackDisabled => fallback_disabled().await,
        ConformanceScenario::WrongServiceStaticKeyPin => wrong_service_static_key_pin().await,
        ConformanceScenario::WrongDescriptorTrustAnchor => wrong_descriptor_trust_anchor().await,
        ConformanceScenario::ExpiredDescriptor => expired_descriptor().await,
        ConformanceScenario::DescriptorRollback => descriptor_rollback().await,
        ConformanceScenario::ServiceKeyRotationValid => service_key_rotation_valid().await,
        ConformanceScenario::ServiceKeyRotationInvalid => service_key_rotation_invalid().await,
        ConformanceScenario::StaleDeviceChallenge => stale_device_challenge().await,
        ConformanceScenario::ReplayedDeviceChallenge => replayed_device_challenge().await,
        ConformanceScenario::GracefulClose => graceful_close().await,
        ConformanceScenario::CustomCaInnerTrustFailure => custom_ca_inner_trust_failure().await,
        ConformanceScenario::CustomCaWrongRootTlsFailure => {
            custom_ca_wrong_root_tls_failure().await
        }
        ConformanceScenario::ProxiedWss => super::managed_network::proxied_wss().await,
    }
}

async fn success_scenario(
    scenario: ConformanceScenario,
    alpn: Vec<u8>,
    cache: Option<secure_tunnel_sdk::TransportCacheSnapshot>,
) -> HarnessResult<ConformanceReport> {
    let service = LocalService::start(alpn, |_| {}).await?;
    let outcome = service.connect(service.config(), cache).await?;
    let outcome = outcome.map_err(HarnessError::Connect)?;
    Ok(success_report(
        scenario,
        outcome.report.selected_carrier,
        outcome.report.fallback_reason,
        outcome.report.attempts,
        None,
    ))
}

async fn fallback_disabled() -> HarnessResult<ConformanceReport> {
    let service = LocalService::start(bad_alpn(), |descriptor| {
        descriptor.selection_policy.allow_wss_fallback = false;
    })
    .await?;
    let result = service.connect(service.config(), None).await?;
    let error = require_connect_error(result, "fallback disabled should fail")?;
    Ok(error_report(
        ConformanceScenario::FallbackDisabled,
        error,
        secure_tunnel_sdk::SdkErrorKind::FallbackExhausted,
    ))
}

async fn wrong_service_static_key_pin() -> HarnessResult<ConformanceReport> {
    let service = LocalService::start(good_alpn(), |_| {}).await?;
    let config = service
        .config()
        .with_pinned_service_static_public_keys(vec![[9_u8; 32]]);
    let result = service.connect(config, None).await?;
    let error = require_connect_error(result, "wrong service pin should fail")?;
    Ok(error_report(
        ConformanceScenario::WrongServiceStaticKeyPin,
        error,
        secure_tunnel_sdk::SdkErrorKind::InnerTrustFailure,
    ))
}

async fn custom_ca_inner_trust_failure() -> HarnessResult<ConformanceReport> {
    let service = LocalService::start(good_alpn(), |_| {}).await?;
    let config = service
        .config()
        .with_pinned_service_static_public_keys(vec![[9_u8; 32]]);
    let result = service.connect(config, None).await?;
    let error = require_connect_error(result, "custom CA must not bypass inner trust")?;
    Ok(error_report(
        ConformanceScenario::CustomCaInnerTrustFailure,
        error,
        secure_tunnel_sdk::SdkErrorKind::InnerTrustFailure,
    ))
}

async fn custom_ca_wrong_root_tls_failure() -> HarnessResult<ConformanceReport> {
    let service = LocalService::start(good_alpn(), |_| {}).await?;
    let wrong_service = LocalService::start(good_alpn(), |_| {}).await?;
    let config = service.config().with_outer_root_certificates_der(vec![
        wrong_service.quic.root_certificate_der(),
        wrong_service.wss.root_certificate_der(),
    ]);
    let result = service.connect(config, None).await?;
    let error = require_connect_error(result, "wrong custom roots should fail outer TLS")?;
    let mut report = error_report(
        ConformanceScenario::CustomCaWrongRootTlsFailure,
        error,
        secure_tunnel_sdk::SdkErrorKind::OuterTlsFailure,
    );
    report.ok = report.ok && !report.attempts.is_empty();
    Ok(report)
}

async fn wrong_descriptor_trust_anchor() -> HarnessResult<ConformanceReport> {
    let service = LocalService::start(good_alpn(), |_| {}).await?;
    let config =
        service
            .config()
            .with_descriptor_trust_anchors(vec![secure_tunnel_core::TrustAnchor {
                key_id: "wrong-root".to_owned(),
                algorithm: "ed25519".to_owned(),
                public_key: STANDARD.encode([9_u8; 32]),
            }]);
    let result = service.connect(config, None).await?;
    let error = require_connect_error(result, "wrong descriptor root should fail")?;
    Ok(error_report(
        ConformanceScenario::WrongDescriptorTrustAnchor,
        error,
        secure_tunnel_sdk::SdkErrorKind::InvalidDescriptor,
    ))
}

async fn expired_descriptor() -> HarnessResult<ConformanceReport> {
    let service = LocalService::start(good_alpn(), |descriptor| {
        "2024-01-02T00:00:00Z".clone_into(&mut descriptor.not_after);
    })
    .await?;
    let result = service.connect(service.config(), None).await?;
    let error = require_connect_error(result, "expired descriptor should fail")?;
    Ok(error_report(
        ConformanceScenario::ExpiredDescriptor,
        error,
        secure_tunnel_sdk::SdkErrorKind::InvalidDescriptor,
    ))
}

async fn descriptor_rollback() -> HarnessResult<ConformanceReport> {
    let service = LocalService::start(good_alpn(), |_| {}).await?;
    let result = service
        .connect(service.config(), Some(rollback_cache()))
        .await?;
    let error = require_connect_error(result, "descriptor rollback should fail")?;
    Ok(error_report(
        ConformanceScenario::DescriptorRollback,
        error,
        secure_tunnel_sdk::SdkErrorKind::InvalidDescriptor,
    ))
}

async fn service_key_rotation_valid() -> HarnessResult<ConformanceReport> {
    let old_service = LocalService::start(good_alpn(), |_| {}).await?;
    let old_key = old_service.fixture.server_public_key();
    let new_service = LocalService::start(good_alpn(), |descriptor| {
        descriptor.descriptor_serial = 2;
    })
    .await?;
    let new_key = new_service.fixture.server_public_key();
    if old_key == new_key {
        return Err(HarnessError::Invariant(
            "rotation fixture reused the service key",
        ));
    }

    let config = new_service
        .config()
        .with_pinned_service_static_public_keys(vec![old_key, new_key]);
    let outcome = new_service
        .connect(config, Some(prior_descriptor_cache()))
        .await?
        .map_err(HarnessError::Connect)?;
    Ok(success_report(
        ConformanceScenario::ServiceKeyRotationValid,
        outcome.report.selected_carrier,
        outcome.report.fallback_reason,
        outcome.report.attempts,
        None,
    ))
}

async fn service_key_rotation_invalid() -> HarnessResult<ConformanceReport> {
    let old_service = LocalService::start(good_alpn(), |_| {}).await?;
    let new_service = LocalService::start(good_alpn(), |_| {}).await?;
    let config = new_service
        .config()
        .with_pinned_service_static_public_keys(vec![old_service.fixture.server_public_key()]);
    let result = new_service.connect(config, None).await?;
    let error = require_connect_error(result, "unpinned rotated service key should fail")?;
    Ok(error_report(
        ConformanceScenario::ServiceKeyRotationInvalid,
        error,
        secure_tunnel_sdk::SdkErrorKind::InnerTrustFailure,
    ))
}

async fn stale_device_challenge() -> HarnessResult<ConformanceReport> {
    let service = LocalService::start(good_alpn(), |_| {}).await?;
    let (outcome, signing_key) = authenticated_account(&service).await?;
    let challenge = outcome
        .session
        .begin_known_device_auth(DEVICE_KEY_ID.to_owned())
        .await?;
    let signature = signing_key
        .sign(&challenge.canonical_bytes)
        .to_bytes()
        .to_vec();
    let result = outcome
        .session
        .finish_known_device_auth(challenge, signature, u64::MAX)
        .await;
    let error = require_sdk_error(result, "stale device challenge should fail")?;
    Ok(sdk_error_report(
        ConformanceScenario::StaleDeviceChallenge,
        &error,
        secure_tunnel_sdk::SdkErrorKind::AuthFailure,
        outcome.report.selected_carrier,
        outcome.report.fallback_reason,
        outcome.report.attempts,
    ))
}

async fn replayed_device_challenge() -> HarnessResult<ConformanceReport> {
    let service = LocalService::start(good_alpn(), |_| {}).await?;
    let (outcome, signing_key) = authenticated_account(&service).await?;
    let challenge = outcome
        .session
        .begin_known_device_auth(DEVICE_KEY_ID.to_owned())
        .await?;
    let signature = signing_key
        .sign(&challenge.canonical_bytes)
        .to_bytes()
        .to_vec();
    outcome
        .session
        .finish_known_device_auth(challenge.clone(), signature.clone(), NOW_UNIX_MS)
        .await?;
    let result = outcome
        .session
        .finish_known_device_auth(challenge, signature, NOW_UNIX_MS)
        .await;
    let error = require_sdk_error(result, "replayed device challenge should fail")?;
    Ok(sdk_error_report(
        ConformanceScenario::ReplayedDeviceChallenge,
        &error,
        secure_tunnel_sdk::SdkErrorKind::AuthFailure,
        outcome.report.selected_carrier,
        outcome.report.fallback_reason,
        outcome.report.attempts,
    ))
}

async fn graceful_close() -> HarnessResult<ConformanceReport> {
    let service = LocalService::start(good_alpn(), |_| {}).await?;
    let outcome = service
        .connect(service.config(), None)
        .await?
        .map_err(HarnessError::Connect)?;
    let close = outcome.session.close(1000, true).await?;
    Ok(success_report(
        ConformanceScenario::GracefulClose,
        outcome.report.selected_carrier,
        outcome.report.fallback_reason,
        outcome.report.attempts,
        Some(close.classification),
    ))
}

async fn authenticated_account(
    service: &LocalService,
) -> HarnessResult<(secure_tunnel_sdk::ConnectOutcome, SigningKey)> {
    let signing_key = SigningKey::from_bytes(&DEVICE_KEY_SEED);
    let outcome = service
        .connect(service.config(), None)
        .await?
        .map_err(HarnessError::Connect)?;
    outcome
        .session
        .authenticate_account(secure_tunnel_sdk::AccountAuthRequest {
            account_id: "acct-smoke".to_owned(),
            credential_payload: b"local-smoke-credential".to_vec(),
            mode: secure_tunnel_sdk::AccountAuthMode::Fresh,
        })
        .await?;
    Ok((outcome, signing_key))
}

fn require_connect_error(
    result: Result<secure_tunnel_sdk::ConnectOutcome, secure_tunnel_sdk::ConnectError>,
    message: &'static str,
) -> HarnessResult<secure_tunnel_sdk::ConnectError> {
    match result {
        Ok(_) => Err(HarnessError::Invariant(message)),
        Err(error) => Ok(error),
    }
}

fn require_sdk_error<T>(
    result: Result<T, secure_tunnel_sdk::SdkError>,
    message: &'static str,
) -> HarnessResult<secure_tunnel_sdk::SdkError> {
    match result {
        Ok(_) => Err(HarnessError::Invariant(message)),
        Err(error) => Ok(error),
    }
}

const fn success_report(
    scenario: ConformanceScenario,
    carrier: secure_tunnel_sdk::Carrier,
    fallback_reason: Option<secure_tunnel_sdk::FallbackReason>,
    attempts: Vec<secure_tunnel_sdk::TransportAttemptReport>,
    close_classification: Option<secure_tunnel_sdk::CloseClassification>,
) -> ConformanceReport {
    ConformanceReport {
        scenario,
        ok: true,
        selected_carrier: Some(carrier),
        terminal_error_kind: None,
        fallback_reason,
        close_classification,
        attempts,
    }
}

fn error_report(
    scenario: ConformanceScenario,
    error: secure_tunnel_sdk::ConnectError,
    expected: secure_tunnel_sdk::SdkErrorKind,
) -> ConformanceReport {
    let ok = error.kind() == expected;
    ConformanceReport {
        scenario,
        ok,
        selected_carrier: None,
        terminal_error_kind: Some(error.kind()),
        fallback_reason: None,
        close_classification: None,
        attempts: error.attempts,
    }
}

fn sdk_error_report(
    scenario: ConformanceScenario,
    error: &secure_tunnel_sdk::SdkError,
    expected: secure_tunnel_sdk::SdkErrorKind,
    carrier: secure_tunnel_sdk::Carrier,
    fallback_reason: Option<secure_tunnel_sdk::FallbackReason>,
    attempts: Vec<secure_tunnel_sdk::TransportAttemptReport>,
) -> ConformanceReport {
    ConformanceReport {
        scenario,
        ok: error.kind() == expected,
        selected_carrier: Some(carrier),
        terminal_error_kind: Some(error.kind()),
        fallback_reason,
        close_classification: None,
        attempts,
    }
}

fn good_alpn() -> Vec<u8> {
    secure_tunnel_core::QUIC_ALPN_V1.as_bytes().to_vec()
}

fn bad_alpn() -> Vec<u8> {
    b"wrong-alpn".to_vec()
}

const fn quic_bad_cache() -> secure_tunnel_sdk::TransportCacheSnapshot {
    secure_tunnel_sdk::TransportCacheSnapshot {
        last_successful_carrier: Some(secure_tunnel_sdk::Carrier::Wss),
        last_quic_failure: Some(secure_tunnel_sdk::FallbackReason::OuterPathFailure),
        next_quic_probe_after_unix_seconds: Some(NOW_UNIX_SECONDS + 60),
        highest_descriptor_serial: Some(1),
    }
}

const fn rollback_cache() -> secure_tunnel_sdk::TransportCacheSnapshot {
    secure_tunnel_sdk::TransportCacheSnapshot {
        highest_descriptor_serial: Some(2),
        ..quic_bad_cache()
    }
}

const fn prior_descriptor_cache() -> secure_tunnel_sdk::TransportCacheSnapshot {
    secure_tunnel_sdk::TransportCacheSnapshot {
        last_successful_carrier: None,
        last_quic_failure: None,
        next_quic_probe_after_unix_seconds: None,
        highest_descriptor_serial: Some(1),
    }
}

struct LocalService {
    fixture: LocalServiceFixture,
    quic: QuicServer,
    wss: WssServer,
    descriptor: secure_tunnel_core::ServiceDescriptor,
}

impl LocalService {
    async fn start(
        quic_alpn: Vec<u8>,
        mutate: impl FnOnce(&mut secure_tunnel_core::ServiceDescriptor),
    ) -> HarnessResult<Self> {
        let signing_key = SigningKey::from_bytes(&DEVICE_KEY_SEED);
        let fixture = LocalServiceFixture::new(signing_key.verifying_key().to_bytes())?;
        let wss = WssServer::start(fixture.clone()).await?;
        let quic = QuicServer::start(fixture.clone(), vec![quic_alpn])?;
        let descriptor = fixture.descriptor_for_ports_with(quic.port(), wss.port(), mutate)?;
        Ok(Self {
            fixture,
            quic,
            wss,
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
