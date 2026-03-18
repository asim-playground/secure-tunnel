// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};

use crate::codec::put_len_prefixed_str;
use crate::constants::{NOISE_SUITE_V1, PROTOCOL_ID_V1, QUIC_ALPN_V1, WSS_SUBPROTOCOL_V1};
use crate::error::{ApiError, ApiResult};
use crate::transport::{
    CandidateSource, CarrierKind, TransportCacheSnapshot, TransportCandidate, TransportTarget,
};
use crate::trust::parse_verifying_key;

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
    /// Builds the canonical Noise prologue for this descriptor.
    ///
    /// # Errors
    ///
    /// Returns an error when any bound field exceeds the v1 encoding limit.
    pub fn noise_prologue(&self) -> ApiResult<Vec<u8>> {
        let mut prologue = Vec::with_capacity(128);

        put_prologue_field(&mut prologue, &self.protocol_id)?;
        put_prologue_field(&mut prologue, &self.environment_id)?;
        put_prologue_field(&mut prologue, &self.service_id)?;
        put_prologue_field(&mut prologue, &self.service_authority)?;

        Ok(prologue)
    }

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
        if self.descriptor_version != 1 {
            return Err(ApiError::InvalidServiceDescriptor(
                "descriptor_version must be 1 for v1 descriptors",
            ));
        }

        validate_required_text("not_before", &self.not_before)?;
        validate_required_text("not_after", &self.not_after)?;
        validate_required_text("environment_id", &self.environment_id)?;
        validate_required_text("service_id", &self.service_id)?;
        validate_required_text("service_authority", &self.service_authority)?;

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

        for trust_anchor in &self.trust_anchors {
            validate_trust_anchor(trust_anchor)?;
        }

        if self.selection_policy.preferred_carrier != CarrierKind::Quic {
            return Err(ApiError::InvalidServiceDescriptor(
                "v1 requires QUIC as the preferred carrier",
            ));
        }

        let quic_target = self
            .carriers
            .quic
            .as_ref()
            .ok_or(ApiError::InvalidServiceDescriptor(
                "v1 requires a QUIC carrier target",
            ))?;
        validate_quic_target(quic_target)?;

        if self.selection_policy.allow_wss_fallback && self.carriers.wss.is_none() {
            return Err(ApiError::InvalidServiceDescriptor(
                "allow_wss_fallback requires a WSS carrier target",
            ));
        }

        if let Some(wss_target) = &self.carriers.wss {
            validate_wss_target(wss_target)?;
        }

        Ok(())
    }
}

fn validate_required_text(field: &str, value: &str) -> ApiResult<()> {
    if value.trim().is_empty() {
        return Err(match field {
            "not_before" => ApiError::InvalidServiceDescriptor("not_before must not be empty"),
            "not_after" => ApiError::InvalidServiceDescriptor("not_after must not be empty"),
            "environment_id" => {
                ApiError::InvalidServiceDescriptor("environment_id must not be empty")
            }
            "service_id" => ApiError::InvalidServiceDescriptor("service_id must not be empty"),
            "service_authority" => {
                ApiError::InvalidServiceDescriptor("service_authority must not be empty")
            }
            _ => ApiError::InvalidServiceDescriptor("required descriptor field must not be empty"),
        });
    }
    Ok(())
}

fn validate_trust_anchor(trust_anchor: &TrustAnchor) -> ApiResult<()> {
    if trust_anchor.key_id.trim().is_empty() {
        return Err(ApiError::InvalidServiceDescriptor(
            "trust anchor key_id must not be empty",
        ));
    }
    if trust_anchor.algorithm != "ed25519" {
        return Err(ApiError::InvalidServiceDescriptor(
            "trust anchor algorithm must be ed25519",
        ));
    }
    if parse_verifying_key(trust_anchor).is_err() {
        return Err(ApiError::InvalidServiceDescriptor(
            "trust anchor public_key must be a valid Ed25519 verifying key",
        ));
    }
    Ok(())
}

fn validate_quic_target(target: &QuicTarget) -> ApiResult<()> {
    if target.connect_host.trim().is_empty() {
        return Err(ApiError::InvalidServiceDescriptor(
            "QUIC connect_host must not be empty",
        ));
    }
    if target.port == 0 {
        return Err(ApiError::InvalidServiceDescriptor(
            "QUIC port must not be zero",
        ));
    }
    if target.alpn != QUIC_ALPN_V1 {
        return Err(ApiError::InvalidServiceDescriptor(
            "QUIC ALPN must match the v1 descriptor value",
        ));
    }
    if target
        .sni_override
        .as_ref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(ApiError::InvalidServiceDescriptor(
            "QUIC sni_override must not be empty when present",
        ));
    }
    Ok(())
}

fn validate_wss_target(target: &WssTarget) -> ApiResult<()> {
    if !wss_url_has_authority(&target.url) {
        return Err(ApiError::InvalidServiceDescriptor(
            "WSS target URL must use wss:// with a non-empty authority",
        ));
    }
    if target.subprotocol != WSS_SUBPROTOCOL_V1 {
        return Err(ApiError::InvalidServiceDescriptor(
            "WSS subprotocol must match the v1 descriptor value",
        ));
    }
    if target
        .authority_override
        .as_ref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(ApiError::InvalidServiceDescriptor(
            "WSS authority_override must not be empty when present",
        ));
    }
    Ok(())
}

fn wss_url_has_authority(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("wss://") else {
        return false;
    };
    let authority = rest.split('/').next().unwrap_or_default();
    !authority.is_empty()
        && !authority.starts_with(['?', '#'])
        && !authority.chars().any(char::is_whitespace)
}

fn put_prologue_field(buffer: &mut Vec<u8>, value: &str) -> ApiResult<()> {
    put_len_prefixed_str(buffer, value)
        .map_err(|_| ApiError::InvalidServiceDescriptor("prologue field exceeds u16 length"))
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
    if policy.allow_wss_fallback
        && let Some(wss) = wss_target
    {
        plan.push(TransportCandidate {
            target: TransportTarget::Wss(wss),
            source: CandidateSource::FallbackCarrier,
        });
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
            public_key: "11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=".to_owned(),
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
