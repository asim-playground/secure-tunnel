// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};

use crate::constants::{NOISE_SUITE_V1, PROTOCOL_ID_V1, QUIC_ALPN_V1, WSS_SUBPROTOCOL_V1};
use crate::descriptor_auth::{authorize_descriptor_at, validate_descriptor_window};
use crate::error::{ApiError, ApiResult};
use crate::inner_context::{
    InnerChannelContext, parse_service_static_public_key, parse_signed_descriptor_hash,
};
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
    /// Authorized service Noise static public key, base64-encoded.
    pub service_static_public_key: String,
    /// Hash of the signed or pinned descriptor, base64-encoded.
    pub signed_descriptor_hash: String,
    /// Root signature over the canonical descriptor hash.
    pub descriptor_signature: DescriptorSignature,
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
        self.inner_channel_context()?.prologue_bytes()
    }

    /// Returns the authorized service Noise static public key bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the descriptor field is not valid v1 public key
    /// material.
    pub fn service_static_public_key_bytes(&self) -> ApiResult<[u8; 32]> {
        parse_service_static_public_key(&self.service_static_public_key)
    }

    /// Returns the signed or pinned descriptor hash bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the descriptor field is not valid v1 hash
    /// material.
    pub fn signed_descriptor_hash_bytes(&self) -> ApiResult<[u8; 32]> {
        parse_signed_descriptor_hash(&self.signed_descriptor_hash)
    }

    /// Returns the stable context bound into the inner Noise prologue.
    ///
    /// # Errors
    ///
    /// Returns an error when descriptor-bound context is invalid.
    pub fn inner_channel_context(&self) -> ApiResult<InnerChannelContext> {
        InnerChannelContext::v1(
            self.service_id.clone(),
            self.environment_id.clone(),
            self.service_authority.clone(),
            self.signed_descriptor_hash_bytes()?,
        )
    }

    /// Verifies descriptor freshness and root authorization at a timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error when the descriptor is structurally invalid, outside
    /// its validity window, has a mismatched canonical hash, or lacks a valid
    /// signature from one of the pinned roots.
    pub fn authorize_at(
        &self,
        now_unix_seconds: u64,
        trusted_roots: &[TrustAnchor],
    ) -> ApiResult<()> {
        authorize_descriptor_at(self, trusted_roots, now_unix_seconds)
    }

    /// Verifies descriptor freshness at a timestamp.
    ///
    /// # Errors
    ///
    /// Returns an error when the descriptor is outside its validity window.
    pub fn ensure_valid_at(&self, now_unix_seconds: u64) -> ApiResult<()> {
        validate_descriptor_window(self, now_unix_seconds)
    }

    /// Re-signs a descriptor with the built-in example root for local tests.
    ///
    /// This helper exists so integration fixtures and local smoke harnesses can
    /// mutate local ports and generated service static keys while still
    /// exercising descriptor signature verification.
    ///
    /// # Errors
    ///
    /// Returns an error when canonical descriptor bytes cannot be built.
    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn resign_with_example_key_for_testing(&mut self) -> ApiResult<()> {
        *self = crate::descriptor_auth::sign_example_descriptor(self.clone())?;
        Ok(())
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
        self.ensure_valid_at(now_unix_seconds)?;
        if cache
            .and_then(|snapshot| snapshot.highest_descriptor_serial)
            .is_some_and(|serial| self.descriptor_serial < serial)
        {
            return Err(ApiError::InvalidServiceDescriptor(
                "descriptor_serial is older than the cached accepted descriptor",
            ));
        }

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
        if self.descriptor_serial == 0 {
            return Err(ApiError::InvalidServiceDescriptor(
                "descriptor_serial must not be zero",
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

        self.service_static_public_key_bytes()?;
        self.signed_descriptor_hash_bytes()?;

        if self.trust_anchors.is_empty() {
            return Err(ApiError::InvalidServiceDescriptor(
                "at least one trust anchor is required",
            ));
        }

        for trust_anchor in &self.trust_anchors {
            validate_trust_anchor(trust_anchor)?;
        }
        validate_descriptor_signature(&self.descriptor_signature)?;

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

fn validate_descriptor_signature(signature: &DescriptorSignature) -> ApiResult<()> {
    validate_required_text("descriptor_signature.key_id", &signature.key_id)?;
    if signature.algorithm != "ed25519" {
        return Err(ApiError::InvalidServiceDescriptor(
            "descriptor_signature algorithm must be ed25519",
        ));
    }
    validate_required_text("descriptor_signature.signature", &signature.signature)
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
            "descriptor_signature.key_id" => {
                ApiError::InvalidServiceDescriptor("descriptor_signature key_id must not be empty")
            }
            "descriptor_signature.signature" => ApiError::InvalidServiceDescriptor(
                "descriptor_signature signature must not be empty",
            ),
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

/// Signature over a canonical descriptor hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescriptorSignature {
    /// Trusted root key identifier used to verify this signature.
    pub key_id: String,
    /// Signature algorithm name.
    pub algorithm: String,
    /// Signature bytes encoded with standard base64.
    pub signature: String,
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
