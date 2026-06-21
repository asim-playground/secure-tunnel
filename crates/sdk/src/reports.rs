// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{SdkError, SdkErrorKind};

/// Outer carrier selected or attempted by the SDK.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Carrier {
    /// Raw `QUIC` over UDP.
    Quic,
    /// WebSocket over HTTPS.
    Wss,
}

impl fmt::Display for Carrier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quic => formatter.write_str("quic"),
            Self::Wss => formatter.write_str("wss"),
        }
    }
}

impl From<secure_tunnel_core::CarrierKind> for Carrier {
    fn from(value: secure_tunnel_core::CarrierKind) -> Self {
        match value {
            secure_tunnel_core::CarrierKind::Quic => Self::Quic,
            secure_tunnel_core::CarrierKind::Wss => Self::Wss,
        }
    }
}

impl From<Carrier> for secure_tunnel_core::CarrierKind {
    fn from(value: Carrier) -> Self {
        match value {
            Carrier::Quic => Self::Quic,
            Carrier::Wss => Self::Wss,
        }
    }
}

/// Why a candidate appears at its position in the transport plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateSource {
    /// The descriptor prefers this carrier on the current network.
    PreferredCarrier,
    /// The descriptor allows this carrier as a live fallback.
    FallbackCarrier,
    /// Cached network posture skipped an initial `QUIC` attempt.
    CachedQuicBadNetwork,
    /// Cached fallback expired and `QUIC` should be retried.
    QuicReprobeAfterCachedFallback,
}

impl From<secure_tunnel_core::CandidateSource> for CandidateSource {
    fn from(value: secure_tunnel_core::CandidateSource) -> Self {
        match value {
            secure_tunnel_core::CandidateSource::PreferredCarrier => Self::PreferredCarrier,
            secure_tunnel_core::CandidateSource::FallbackCarrier => Self::FallbackCarrier,
            secure_tunnel_core::CandidateSource::CachedQuicBadNetwork => Self::CachedQuicBadNetwork,
            secure_tunnel_core::CandidateSource::QuicReprobeAfterCachedFallback => {
                Self::QuicReprobeAfterCachedFallback
            }
        }
    }
}

/// Fallback-eligible outer-carrier failure classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackReason {
    /// UDP path failure before the inner secure channel is ready.
    OuterPathFailure,
    /// `QUIC` capability or selector mismatch before the inner secure channel.
    OuterQuicRejected,
    /// `QUIC` closed before the inner secure channel reached `Secure Ready`.
    OuterQuicClosedEarly,
}

impl From<secure_tunnel_core::FallbackReason> for FallbackReason {
    fn from(value: secure_tunnel_core::FallbackReason) -> Self {
        match value {
            secure_tunnel_core::FallbackReason::OuterPathFailure => Self::OuterPathFailure,
            secure_tunnel_core::FallbackReason::OuterQuicRejected => Self::OuterQuicRejected,
            secure_tunnel_core::FallbackReason::OuterQuicClosedEarly => Self::OuterQuicClosedEarly,
        }
    }
}

impl From<FallbackReason> for secure_tunnel_core::FallbackReason {
    fn from(value: FallbackReason) -> Self {
        match value {
            FallbackReason::OuterPathFailure => Self::OuterPathFailure,
            FallbackReason::OuterQuicRejected => Self::OuterQuicRejected,
            FallbackReason::OuterQuicClosedEarly => Self::OuterQuicClosedEarly,
        }
    }
}

/// Whether carrier choice came from live probing or cached network posture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheDisposition {
    /// No cached network posture affected carrier choice.
    LiveProbe,
    /// Cached network posture skipped the initial `QUIC` attempt.
    CachedFallback,
    /// Cached posture had expired and `QUIC` was retried.
    Reprobe,
}

impl From<secure_tunnel_core::CacheDisposition> for CacheDisposition {
    fn from(value: secure_tunnel_core::CacheDisposition) -> Self {
        match value {
            secure_tunnel_core::CacheDisposition::LiveProbe => Self::LiveProbe,
            secure_tunnel_core::CacheDisposition::CachedFallback => Self::CachedFallback,
            secure_tunnel_core::CacheDisposition::Reprobe => Self::Reprobe,
        }
    }
}

/// Cached network posture for transport selection.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TransportCacheSnapshot {
    /// Last carrier that successfully reached `Secure Ready`.
    pub last_successful_carrier: Option<Carrier>,
    /// Last fallback-eligible `QUIC` failure.
    pub last_quic_failure: Option<FallbackReason>,
    /// Unix timestamp after which `QUIC` should be reprobed.
    pub next_quic_probe_after_unix_seconds: Option<u64>,
    /// Highest descriptor serial accepted for this service/environment cache key.
    pub highest_descriptor_serial: Option<u64>,
}

impl TransportCacheSnapshot {
    pub(super) fn to_core(&self) -> secure_tunnel_core::TransportCacheSnapshot {
        secure_tunnel_core::TransportCacheSnapshot {
            last_successful_carrier: self.last_successful_carrier.map(Into::into),
            last_quic_failure: self.last_quic_failure.map(Into::into),
            next_quic_probe_after_unix_seconds: self.next_quic_probe_after_unix_seconds,
            highest_descriptor_serial: self.highest_descriptor_serial,
        }
    }

    pub(super) fn from_core(value: &secure_tunnel_core::TransportCacheSnapshot) -> Self {
        Self {
            last_successful_carrier: value.last_successful_carrier.map(Into::into),
            last_quic_failure: value.last_quic_failure.map(Into::into),
            next_quic_probe_after_unix_seconds: value.next_quic_probe_after_unix_seconds,
            highest_descriptor_serial: value.highest_descriptor_serial,
        }
    }
}

/// One carrier candidate in the deterministic connect plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportCandidateReport {
    /// Carrier to attempt.
    pub carrier: Carrier,
    /// Why the carrier appears in this plan position.
    pub source: CandidateSource,
}

impl TransportCandidateReport {
    pub(super) fn from_core(value: &secure_tunnel_core::TransportCandidate) -> Self {
        Self {
            carrier: value.target.carrier().into(),
            source: value.source.into(),
        }
    }
}

/// Terminal outcome for one carrier attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportAttemptOutcome {
    /// The carrier reached `Secure Ready`.
    SecureReady,
    /// The carrier failed in a way that permitted fallback.
    Fallback {
        /// Normalized fallback reason.
        reason: FallbackReason,
    },
    /// The carrier failed and stopped selection.
    Failed {
        /// Stable SDK error class.
        kind: SdkErrorKind,
        /// Human-readable diagnostic message.
        message: String,
    },
}

/// One recorded transport attempt for observability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportAttemptReport {
    /// Carrier that was attempted.
    pub carrier: Carrier,
    /// Why this candidate was attempted.
    pub source: CandidateSource,
    /// Attempt outcome.
    pub outcome: TransportAttemptOutcome,
}

impl TransportAttemptReport {
    pub(super) fn from_core(value: &secure_tunnel_core::TransportAttemptTrace) -> Self {
        let outcome = match &value.outcome {
            secure_tunnel_core::TransportAttemptOutcome::SecureReady => {
                TransportAttemptOutcome::SecureReady
            }
            secure_tunnel_core::TransportAttemptOutcome::Fallback(reason) => {
                TransportAttemptOutcome::Fallback {
                    reason: (*reason).into(),
                }
            }
            secure_tunnel_core::TransportAttemptOutcome::Failed(error) => {
                let sdk_error = SdkError::from_core(error);
                TransportAttemptOutcome::Failed {
                    kind: sdk_error.kind(),
                    message: sdk_error.message(),
                }
            }
        };

        Self {
            carrier: value.carrier.into(),
            source: value.source.into(),
            outcome,
        }
    }
}

/// Successful connect report suitable for foreign callers and routine logs.
///
/// Security/session correlation bytes are intentionally excluded from this
/// record. Use [`SecureChannelArtifacts`] when a caller explicitly needs
/// transcript or channel-binding material.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectReport {
    /// Carrier selected for this session.
    pub selected_carrier: Carrier,
    /// Whether carrier choice came from live probing or cached posture.
    pub cache_state: CacheDisposition,
    /// Fallback reason, when fallback occurred.
    pub fallback_reason: Option<FallbackReason>,
    /// Attempt trace collected during selection.
    pub attempts: Vec<TransportAttemptReport>,
    /// Updated coarse transport cache for the caller to persist if desired.
    pub transport_cache: TransportCacheSnapshot,
}

impl ConnectReport {
    pub(super) fn from_selected(value: &secure_tunnel_core::SelectedTransport) -> Self {
        Self {
            selected_carrier: value.report.carrier.into(),
            cache_state: value.report.cache_state.into(),
            fallback_reason: value.report.fallback_reason.map(Into::into),
            attempts: value
                .attempts
                .iter()
                .map(TransportAttemptReport::from_core)
                .collect(),
            transport_cache: TransportCacheSnapshot::from_core(&value.cache_snapshot),
        }
    }
}

/// Security artifacts produced by the inner secure channel.
///
/// These bytes are useful for explicit security integrations, but they are not
/// part of [`ConnectReport`] because they can correlate sessions and should not
/// be emitted to routine logs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SecureChannelArtifacts {
    /// Secure-channel transcript binding, when available.
    pub handshake_hash: Option<Vec<u8>>,
    /// Additional channel-binding bytes, when available.
    pub channel_binding: Option<Vec<u8>>,
    /// Authenticated service Noise static public key, when available.
    pub service_static_public_key: Option<Vec<u8>>,
}

impl SecureChannelArtifacts {
    pub(super) fn from_core(value: &secure_tunnel_core::SecureReadyArtifacts) -> Self {
        Self {
            handshake_hash: value.handshake_hash.clone(),
            channel_binding: value.channel_binding.clone(),
            service_static_public_key: value.service_static_public_key.clone(),
        }
    }
}
