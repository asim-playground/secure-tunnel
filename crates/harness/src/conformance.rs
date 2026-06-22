// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{HarnessError, HarnessResult, SmokeScenario};

mod managed_network;
mod scenarios;
pub use scenarios::run_conformance_scenario;

/// Conformance scenario covered by the local harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConformanceScenario {
    /// Valid `QUIC` reaches `Secure Ready`.
    QuicSuccess,
    /// `QUIC` ALPN rejection falls back to `WSS`.
    QuicRejectedWssFallback,
    /// Cached `QUIC`-bad posture attempts `WSS` first.
    CachedQuicBadWssFirst,
    /// Fallback disabled fails after the `QUIC` rejection.
    FallbackDisabled,
    /// Wrong pinned service static key fails before dialing.
    WrongServiceStaticKeyPin,
    /// Wrong descriptor trust anchor fails before dialing.
    WrongDescriptorTrustAnchor,
    /// Expired descriptor fails before dialing.
    ExpiredDescriptor,
    /// Descriptor rollback fails before dialing.
    DescriptorRollback,
    /// Authorized service key rotation succeeds.
    ServiceKeyRotationValid,
    /// Unauthorized service key rotation fails.
    ServiceKeyRotationInvalid,
    /// Stale known-device challenge fails.
    StaleDeviceChallenge,
    /// Replayed known-device challenge fails.
    ReplayedDeviceChallenge,
    /// Encrypted close is classified as graceful.
    GracefulClose,
    /// Custom outer TLS roots allow direct `QUIC`.
    CustomCaQuicSuccess,
    /// Custom outer TLS roots allow cached `WSS` first.
    CustomCaWssSuccess,
    /// Custom outer TLS roots compose with `QUIC` to `WSS` fallback.
    CustomCaQuicRejectedWssFallback,
    /// Custom outer TLS roots do not bypass inner service trust.
    CustomCaInnerTrustFailure,
    /// Wrong custom outer TLS roots surface as SDK outer TLS failure.
    CustomCaWrongRootTlsFailure,
    /// Explicit HTTP proxy carries the outer `WSS` connection.
    ProxiedWss,
}

impl ConformanceScenario {
    /// Returns the stable CLI spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::QuicSuccess => "quic-success",
            Self::QuicRejectedWssFallback => "quic-rejected-wss-fallback",
            Self::CachedQuicBadWssFirst => "cached-quic-bad-wss-first",
            Self::FallbackDisabled => "fallback-disabled",
            Self::WrongServiceStaticKeyPin => "wrong-service-static-key-pin",
            Self::WrongDescriptorTrustAnchor => "wrong-descriptor-trust-anchor",
            Self::ExpiredDescriptor => "expired-descriptor",
            Self::DescriptorRollback => "descriptor-rollback",
            Self::ServiceKeyRotationValid => "service-key-rotation-valid",
            Self::ServiceKeyRotationInvalid => "service-key-rotation-invalid",
            Self::StaleDeviceChallenge => "stale-device-challenge",
            Self::ReplayedDeviceChallenge => "replayed-device-challenge",
            Self::GracefulClose => "graceful-close",
            Self::CustomCaQuicSuccess => "custom-ca-quic-success",
            Self::CustomCaWssSuccess => "custom-ca-wss-success",
            Self::CustomCaQuicRejectedWssFallback => "custom-ca-quic-rejected-wss-fallback",
            Self::CustomCaInnerTrustFailure => "custom-ca-inner-trust-failure",
            Self::CustomCaWrongRootTlsFailure => "custom-ca-wrong-root-tls-failure",
            Self::ProxiedWss => "proxied-wss",
        }
    }
}

impl FromStr for ConformanceScenario {
    type Err = HarnessError;

    fn from_str(value: &str) -> HarnessResult<Self> {
        match value {
            "quic-success" | "quic_success" => Ok(Self::QuicSuccess),
            "quic-rejected-wss-fallback" | "quic_rejected_wss_fallback" => {
                Ok(Self::QuicRejectedWssFallback)
            }
            "cached-quic-bad-wss-first" | "cached_quic_bad_wss_first" => {
                Ok(Self::CachedQuicBadWssFirst)
            }
            "fallback-disabled" | "fallback_disabled" => Ok(Self::FallbackDisabled),
            "wrong-service-static-key-pin" | "wrong_service_static_key_pin" => {
                Ok(Self::WrongServiceStaticKeyPin)
            }
            "wrong-descriptor-trust-anchor" | "wrong_descriptor_trust_anchor" => {
                Ok(Self::WrongDescriptorTrustAnchor)
            }
            "expired-descriptor" | "expired_descriptor" => Ok(Self::ExpiredDescriptor),
            "descriptor-rollback" | "descriptor_rollback" => Ok(Self::DescriptorRollback),
            "service-key-rotation-valid" | "service_key_rotation_valid" => {
                Ok(Self::ServiceKeyRotationValid)
            }
            "service-key-rotation-invalid" | "service_key_rotation_invalid" => {
                Ok(Self::ServiceKeyRotationInvalid)
            }
            "stale-device-challenge" | "stale_device_challenge" => Ok(Self::StaleDeviceChallenge),
            "replayed-device-challenge" | "replayed_device_challenge" => {
                Ok(Self::ReplayedDeviceChallenge)
            }
            "graceful-close" | "graceful_close" => Ok(Self::GracefulClose),
            "custom-ca-quic-success" | "custom_ca_quic_success" => Ok(Self::CustomCaQuicSuccess),
            "custom-ca-wss-success" | "custom_ca_wss_success" => Ok(Self::CustomCaWssSuccess),
            "custom-ca-quic-rejected-wss-fallback" | "custom_ca_quic_rejected_wss_fallback" => {
                Ok(Self::CustomCaQuicRejectedWssFallback)
            }
            "custom-ca-inner-trust-failure" | "custom_ca_inner_trust_failure" => {
                Ok(Self::CustomCaInnerTrustFailure)
            }
            "custom-ca-wrong-root-tls-failure" | "custom_ca_wrong_root_tls_failure" => {
                Ok(Self::CustomCaWrongRootTlsFailure)
            }
            "proxied-wss" | "proxied_wss" => Ok(Self::ProxiedWss),
            _ => Err(HarnessError::Invariant("unknown conformance scenario")),
        }
    }
}

/// JSON-friendly conformance result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConformanceReport {
    /// Scenario that produced this report.
    pub scenario: ConformanceScenario,
    /// True when the observed behavior matched the expectation.
    pub ok: bool,
    /// Selected carrier for success cases.
    pub selected_carrier: Option<secure_tunnel_sdk::Carrier>,
    /// Terminal error kind for expected-failure cases.
    pub terminal_error_kind: Option<secure_tunnel_sdk::SdkErrorKind>,
    /// Fallback reason, when fallback occurred.
    pub fallback_reason: Option<secure_tunnel_sdk::FallbackReason>,
    /// Close classification, when a close completed.
    pub close_classification: Option<secure_tunnel_sdk::CloseClassification>,
    /// Sanitized transport attempts.
    pub attempts: Vec<secure_tunnel_sdk::TransportAttemptReport>,
}

/// Conformance row that is defined but blocked by follow-up implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingConformanceReport {
    /// Stable scenario name.
    pub scenario: String,
    /// Why this row is pending.
    pub reason: String,
}

/// JSON-friendly conformance suite result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConformanceSuiteReport {
    /// True when all implemented scenarios passed.
    pub ok: bool,
    /// Implemented scenarios.
    pub scenarios: Vec<ConformanceReport>,
    /// Defined rows blocked by follow-up tasks.
    pub pending: Vec<PendingConformanceReport>,
}

const CURRENT_SCENARIOS: &[ConformanceScenario] = &[
    ConformanceScenario::QuicSuccess,
    ConformanceScenario::QuicRejectedWssFallback,
    ConformanceScenario::CachedQuicBadWssFirst,
    ConformanceScenario::FallbackDisabled,
    ConformanceScenario::WrongServiceStaticKeyPin,
    ConformanceScenario::WrongDescriptorTrustAnchor,
    ConformanceScenario::ExpiredDescriptor,
    ConformanceScenario::DescriptorRollback,
    ConformanceScenario::ServiceKeyRotationValid,
    ConformanceScenario::ServiceKeyRotationInvalid,
    ConformanceScenario::StaleDeviceChallenge,
    ConformanceScenario::ReplayedDeviceChallenge,
    ConformanceScenario::GracefulClose,
    ConformanceScenario::CustomCaQuicSuccess,
    ConformanceScenario::CustomCaWssSuccess,
    ConformanceScenario::CustomCaQuicRejectedWssFallback,
    ConformanceScenario::CustomCaInnerTrustFailure,
    ConformanceScenario::CustomCaWrongRootTlsFailure,
    ConformanceScenario::ProxiedWss,
];

/// Runs all implemented conformance scenarios.
///
/// # Errors
///
/// Returns an error when a local fixture fails before a scenario can assert the
/// expected outcome.
pub async fn run_conformance_suite() -> HarnessResult<ConformanceSuiteReport> {
    let mut reports = Vec::with_capacity(CURRENT_SCENARIOS.len());
    for scenario in CURRENT_SCENARIOS {
        reports.push(run_conformance_scenario(*scenario).await?);
    }
    Ok(ConformanceSuiteReport {
        ok: reports.iter().all(|report| report.ok),
        scenarios: reports,
        pending: pending_rows(),
    })
}

fn pending_rows() -> Vec<PendingConformanceReport> {
    vec![
        PendingConformanceReport {
            scenario: "abrupt-close".to_owned(),
            reason: "requires close-failure fixture beyond current local server".to_owned(),
        },
        PendingConformanceReport {
            scenario: "truncated-close".to_owned(),
            reason: "requires close-failure fixture beyond current local server".to_owned(),
        },
    ]
}

impl From<SmokeScenario> for ConformanceScenario {
    fn from(value: SmokeScenario) -> Self {
        match value {
            SmokeScenario::QuicSuccess => Self::QuicSuccess,
            SmokeScenario::WssFallback => Self::QuicRejectedWssFallback,
        }
    }
}
