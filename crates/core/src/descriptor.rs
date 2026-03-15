// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};

use crate::constants::{NOISE_SUITE_V1, PROTOCOL_ID_V1, QUIC_ALPN_V1, WSS_SUBPROTOCOL_V1};
use crate::error::{ApiError, ApiResult};
use crate::transport::{
    CandidateSource, CarrierKind, TransportCacheSnapshot, TransportCandidate, TransportTarget,
};

/// One logical descriptor with per-carrier targets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceDescriptor {
    /// Descriptor schema version.
    pub descriptor_version: u16,
    /// Monotonic descriptor instance version for one logical service.
    pub descriptor_serial: u64,
    /// RFC 3339 timestamp for the start of the validity window.
    pub not_before: String,
    /// RFC 3339 timestamp for the end of the validity window.
    pub not_after: String,
    /// Stable environment identifier.
    pub environment_id: String,
    /// Stable logical service identifier.
    pub service_id: String,
    /// Stable inner service authority bound into the Noise prologue.
    pub service_authority: String,
    /// Expected inner protocol identifier.
    pub protocol_id: String,
    /// Expected Noise suite identifier.
    pub noise_suite: String,
    /// Root keys that authorize server Noise static keys and descriptor updates.
    pub trust_anchors: Vec<TrustAnchor>,
    /// Shared transport-selection policy.
    pub selection_policy: SelectionPolicy,
    /// Per-carrier targets for one logical service.
    pub carriers: CarrierSet,
}

impl ServiceDescriptor {
    /// Returns the ordered connect plan for the current coarse network posture.
    ///
    /// This is intentionally limited to ordering and reporting. The selector
    /// state machine and network I/O remain follow-up work.
    ///
    /// # Errors
    ///
    /// Returns an error when the descriptor is internally inconsistent.
    pub fn connect_plan(
        &self,
        cache: Option<&TransportCacheSnapshot>,
        now_unix_seconds: u64,
    ) -> ApiResult<Vec<TransportCandidate>> {
        self.validate()?;

        let quic_target = self
            .carriers
            .quic
            .clone()
            .ok_or(ApiError::UnavailableCarrier(CarrierKind::Quic))?;
        let wss_target = self.carriers.wss.clone();
        let cached_quic_bad = cache
            .and_then(|snapshot| snapshot.last_quic_failure)
            .is_some();
        let cache_still_active = cache
            .and_then(|snapshot| snapshot.next_quic_probe_after_unix_seconds)
            .is_some_and(|deadline| now_unix_seconds < deadline);

        if cached_quic_bad && cache_still_active && !self.selection_policy.allow_wss_fallback {
            return Err(ApiError::TransportPlanBlocked(
                "cached QUIC-bad posture requires WSS fallback or cache expiry",
            ));
        }

        if cached_quic_bad && cache_still_active {
            return plan_for_cached_fallback(wss_target);
        }

        Ok(plan_for_live_attempt(
            &self.selection_policy,
            quic_target,
            wss_target,
            cached_quic_bad,
        ))
    }

    /// Validates the minimum structural invariants for the public descriptor.
    ///
    /// # Errors
    ///
    /// Returns an error when the descriptor omits required shared identity or
    /// carrier information.
    pub fn validate(&self) -> ApiResult<()> {
        if self.protocol_id != PROTOCOL_ID_V1 {
            return Err(ApiError::InvalidServiceDescriptor(
                "protocol_id must match the v1 protocol identifier",
            ));
        }

        if self.noise_suite != NOISE_SUITE_V1 {
            return Err(ApiError::InvalidServiceDescriptor(
                "noise_suite must match the v1 Noise suite identifier",
            ));
        }

        if self.trust_anchors.is_empty() {
            return Err(ApiError::InvalidServiceDescriptor(
                "at least one trust anchor is required",
            ));
        }

        if self.selection_policy.preferred_carrier != CarrierKind::Quic {
            return Err(ApiError::InvalidServiceDescriptor(
                "v1 requires QUIC as the preferred carrier",
            ));
        }

        if self.carriers.quic.is_none() {
            return Err(ApiError::InvalidServiceDescriptor(
                "v1 requires a QUIC carrier target",
            ));
        }

        if self.selection_policy.allow_wss_fallback && self.carriers.wss.is_none() {
            return Err(ApiError::InvalidServiceDescriptor(
                "allow_wss_fallback requires a WSS carrier target",
            ));
        }

        Ok(())
    }
}

/// One root key that authorizes descriptors and server Noise keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustAnchor {
    /// Operator-managed key identifier.
    pub key_id: String,
    /// Signature algorithm name, for example `ed25519`.
    pub algorithm: String,
    /// Public key bytes encoded for transport or config.
    pub public_key: String,
}

/// Shared transport policy for one logical service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionPolicy {
    /// Preferred first carrier.
    pub preferred_carrier: CarrierKind,
    /// Whether `WSS` may be attempted after eligible `QUIC` failures.
    pub allow_wss_fallback: bool,
}

/// Per-carrier targets for one logical service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CarrierSet {
    /// Preferred raw `QUIC` target.
    pub quic: Option<QuicTarget>,
    /// Optional `WSS` target used only when fallback policy allows it.
    pub wss: Option<WssTarget>,
}

/// `QUIC` target parameters carried by the service descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuicTarget {
    /// DNS name or IP used for the outer connection attempt.
    pub connect_host: String,
    /// UDP port for the `QUIC` endpoint.
    pub port: u16,
    /// Expected ALPN value.
    pub alpn: String,
    /// Optional SNI override when routing differs from the service authority.
    pub sni_override: Option<String>,
}

/// `WSS` target parameters carried by the service descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WssTarget {
    /// Full `WSS` URL.
    pub url: String,
    /// Expected WebSocket subprotocol.
    pub subprotocol: String,
    /// Optional Host or authority override.
    pub authority_override: Option<String>,
}

fn plan_for_cached_fallback(wss_target: Option<WssTarget>) -> ApiResult<Vec<TransportCandidate>> {
    let wss = wss_target.ok_or(ApiError::TransportPlanBlocked(
        "cached QUIC-bad posture has no WSS fallback target",
    ))?;
    Ok(vec![TransportCandidate {
        target: TransportTarget::Wss(wss),
        source: CandidateSource::CachedQuicBadNetwork,
    }])
}

fn plan_for_live_attempt(
    policy: &SelectionPolicy,
    quic_target: QuicTarget,
    wss_target: Option<WssTarget>,
    reprobe_after_cached_fallback: bool,
) -> Vec<TransportCandidate> {
    let mut plan = Vec::with_capacity(2);
    plan.push(TransportCandidate {
        target: TransportTarget::Quic(quic_target),
        source: if reprobe_after_cached_fallback {
            CandidateSource::QuicReprobeAfterCachedFallback
        } else {
            CandidateSource::PreferredCarrier
        },
    });
    if policy.allow_wss_fallback {
        if let Some(wss) = wss_target {
            plan.push(TransportCandidate {
                target: TransportTarget::Wss(wss),
                source: CandidateSource::FallbackCarrier,
            });
        }
    }
    plan
}

/// Returns a sample descriptor with one `QUIC` target and one `WSS` fallback.
#[must_use]
pub fn example_service_descriptor() -> ServiceDescriptor {
    ServiceDescriptor {
        descriptor_version: 1,
        descriptor_serial: 1,
        not_before: "2026-03-15T00:00:00Z".to_owned(),
        not_after: "2026-06-15T00:00:00Z".to_owned(),
        environment_id: "prod".to_owned(),
        service_id: "secure-tunnel-api".to_owned(),
        service_authority: "api.example.com".to_owned(),
        protocol_id: PROTOCOL_ID_V1.to_owned(),
        noise_suite: NOISE_SUITE_V1.to_owned(),
        trust_anchors: vec![TrustAnchor {
            key_id: "root-2026-01".to_owned(),
            algorithm: "ed25519".to_owned(),
            public_key: "<base64>".to_owned(),
        }],
        selection_policy: SelectionPolicy {
            preferred_carrier: CarrierKind::Quic,
            allow_wss_fallback: true,
        },
        carriers: CarrierSet {
            quic: Some(QuicTarget {
                connect_host: "api.example.com".to_owned(),
                port: 443,
                alpn: QUIC_ALPN_V1.to_owned(),
                sni_override: None,
            }),
            wss: Some(WssTarget {
                url: "wss://api.example.com/tunnel/v1".to_owned(),
                subprotocol: WSS_SUBPROTOCOL_V1.to_owned(),
                authority_override: None,
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{CandidateSource, CarrierKind, TransportCacheSnapshot, example_service_descriptor};
    use crate::{ApiError, FallbackReason};

    #[test]
    fn connect_plan_prefers_quic_on_unknown_network() {
        let descriptor = example_service_descriptor();

        let plan = descriptor.connect_plan(None, 1_742_000_000).unwrap();

        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].target.carrier(), CarrierKind::Quic);
        assert_eq!(plan[0].source, CandidateSource::PreferredCarrier);
        assert_eq!(plan[1].target.carrier(), CarrierKind::Wss);
        assert_eq!(plan[1].source, CandidateSource::FallbackCarrier);
    }

    #[test]
    fn connect_plan_uses_only_wss_when_quic_bad_cache_is_active() {
        let descriptor = example_service_descriptor();
        let cache = TransportCacheSnapshot {
            last_successful_carrier: Some(CarrierKind::Wss),
            last_quic_failure: Some(FallbackReason::OuterPathFailure),
            next_quic_probe_after_unix_seconds: Some(2_000),
        };

        let plan = descriptor.connect_plan(Some(&cache), 1_999).unwrap();

        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].target.carrier(), CarrierKind::Wss);
        assert_eq!(plan[0].source, CandidateSource::CachedQuicBadNetwork);
    }

    #[test]
    fn connect_plan_fails_fast_when_cache_blocks_quic_and_fallback_is_disabled() {
        let mut descriptor = example_service_descriptor();
        descriptor.selection_policy.allow_wss_fallback = false;
        descriptor.carriers.wss = None;
        let cache = TransportCacheSnapshot {
            last_successful_carrier: None,
            last_quic_failure: Some(FallbackReason::OuterPathFailure),
            next_quic_probe_after_unix_seconds: Some(2_000),
        };

        let error = descriptor.connect_plan(Some(&cache), 1_999).unwrap_err();

        assert_eq!(
            error,
            ApiError::TransportPlanBlocked(
                "cached QUIC-bad posture requires WSS fallback or cache expiry"
            )
        );
    }

    #[test]
    fn connect_plan_reprobes_quic_after_cache_deadline() {
        let descriptor = example_service_descriptor();
        let cache = TransportCacheSnapshot {
            last_successful_carrier: Some(CarrierKind::Wss),
            last_quic_failure: Some(FallbackReason::OuterPathFailure),
            next_quic_probe_after_unix_seconds: Some(2_000),
        };

        let plan = descriptor.connect_plan(Some(&cache), 2_000).unwrap();

        assert_eq!(plan[0].target.carrier(), CarrierKind::Quic);
        assert_eq!(
            plan[0].source,
            CandidateSource::QuicReprobeAfterCachedFallback
        );
    }

    #[test]
    fn validate_requires_wss_target_when_fallback_enabled() {
        let mut descriptor = example_service_descriptor();
        descriptor.carriers.wss = None;

        let error = descriptor.validate().unwrap_err();

        assert_eq!(
            error,
            ApiError::InvalidServiceDescriptor("allow_wss_fallback requires a WSS carrier target")
        );
    }

    #[test]
    fn validate_requires_quic_as_preferred_carrier() {
        let mut descriptor = example_service_descriptor();
        descriptor.selection_policy.preferred_carrier = CarrierKind::Wss;

        let error = descriptor.validate().unwrap_err();

        assert_eq!(
            error,
            ApiError::InvalidServiceDescriptor("v1 requires QUIC as the preferred carrier")
        );
    }
}
