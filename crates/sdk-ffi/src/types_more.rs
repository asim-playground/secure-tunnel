// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use crate::types::{
    AccountFreshness, CacheDisposition, CandidateSource, Carrier, CloseClassification,
    FallbackReason, SessionState, TransportAttemptOutcome,
};

/// Known-device authentication challenge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceAuthChallenge {
    /// Device key identifier being authenticated.
    pub device_key_id: String,
    /// Server challenge bytes.
    pub server_challenge: Vec<u8>,
    /// Challenge expiry in Unix milliseconds.
    pub expires_at_unix_ms: u64,
    /// Canonical bytes the caller must sign.
    pub canonical_bytes: Vec<u8>,
}

/// Known-device authentication report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceAuthReport {
    /// Device key identifier accepted by the service.
    pub device_key_id: String,
    /// Device state after authentication.
    pub state: crate::types::DeviceState,
}

/// Report returned after close.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseReport {
    /// Final session state.
    pub final_state: SessionState,
    /// Coarse close classification.
    pub classification: CloseClassification,
}

/// Explicit security artifacts for integrations that need channel binding.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SecureChannelArtifacts {
    /// Secure-channel transcript binding, when available.
    pub handshake_hash: Option<Vec<u8>>,
    /// Additional channel-binding bytes, when available.
    pub channel_binding: Option<Vec<u8>>,
    /// Authenticated service Noise static public key, when available.
    pub service_static_public_key: Option<Vec<u8>>,
}

pub const fn carrier(value: secure_tunnel_sdk::Carrier) -> Carrier {
    match value {
        secure_tunnel_sdk::Carrier::Quic => Carrier::Quic,
        secure_tunnel_sdk::Carrier::Wss => Carrier::Wss,
    }
}

pub const fn candidate_source(value: secure_tunnel_sdk::CandidateSource) -> CandidateSource {
    match value {
        secure_tunnel_sdk::CandidateSource::PreferredCarrier => CandidateSource::PreferredCarrier,
        secure_tunnel_sdk::CandidateSource::FallbackCarrier => CandidateSource::FallbackCarrier,
        secure_tunnel_sdk::CandidateSource::CachedQuicBadNetwork => {
            CandidateSource::CachedQuicBadNetwork
        }
        secure_tunnel_sdk::CandidateSource::QuicReprobeAfterCachedFallback => {
            CandidateSource::QuicReprobeAfterCachedFallback
        }
    }
}

pub const fn fallback_reason(value: secure_tunnel_sdk::FallbackReason) -> FallbackReason {
    match value {
        secure_tunnel_sdk::FallbackReason::OuterPathFailure => FallbackReason::OuterPathFailure,
        secure_tunnel_sdk::FallbackReason::OuterQuicRejected => FallbackReason::OuterQuicRejected,
        secure_tunnel_sdk::FallbackReason::OuterQuicClosedEarly => {
            FallbackReason::OuterQuicClosedEarly
        }
    }
}

pub const fn cache_state(value: secure_tunnel_sdk::CacheDisposition) -> CacheDisposition {
    match value {
        secure_tunnel_sdk::CacheDisposition::LiveProbe => CacheDisposition::LiveProbe,
        secure_tunnel_sdk::CacheDisposition::CachedFallback => CacheDisposition::CachedFallback,
        secure_tunnel_sdk::CacheDisposition::Reprobe => CacheDisposition::Reprobe,
    }
}

pub const fn session_state(value: secure_tunnel_sdk::SessionState) -> SessionState {
    match value {
        secure_tunnel_sdk::SessionState::CarrierReady => SessionState::CarrierReady,
        secure_tunnel_sdk::SessionState::NoiseHandshake => SessionState::NoiseHandshake,
        secure_tunnel_sdk::SessionState::SecureReady => SessionState::SecureReady,
        secure_tunnel_sdk::SessionState::AccountAuthenticated => SessionState::AccountAuthenticated,
        secure_tunnel_sdk::SessionState::KnownDeviceAuthenticated => {
            SessionState::KnownDeviceAuthenticated
        }
        secure_tunnel_sdk::SessionState::Closing => SessionState::Closing,
        secure_tunnel_sdk::SessionState::Closed => SessionState::Closed,
    }
}

pub const fn close_classification(
    value: secure_tunnel_sdk::CloseClassification,
) -> CloseClassification {
    match value {
        secure_tunnel_sdk::CloseClassification::Graceful => CloseClassification::Graceful,
        secure_tunnel_sdk::CloseClassification::Abrupt => CloseClassification::Abrupt,
        secure_tunnel_sdk::CloseClassification::Truncated => CloseClassification::Truncated,
    }
}

pub const fn account_freshness(value: secure_tunnel_sdk::AccountFreshness) -> AccountFreshness {
    match value {
        secure_tunnel_sdk::AccountFreshness::Fresh => AccountFreshness::Fresh,
        secure_tunnel_sdk::AccountFreshness::Resumed => AccountFreshness::Resumed,
    }
}

pub const fn attempt_outcome(
    value: &secure_tunnel_sdk::TransportAttemptOutcome,
) -> TransportAttemptOutcome {
    match value {
        secure_tunnel_sdk::TransportAttemptOutcome::SecureReady => {
            TransportAttemptOutcome::SecureReady
        }
        secure_tunnel_sdk::TransportAttemptOutcome::Fallback { .. } => {
            TransportAttemptOutcome::Fallback
        }
        secure_tunnel_sdk::TransportAttemptOutcome::Failed { .. } => {
            TransportAttemptOutcome::Failed
        }
    }
}
