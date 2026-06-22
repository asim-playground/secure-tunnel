// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

/// Outer carrier selected or attempted by the SDK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Carrier {
    /// Raw `QUIC` over UDP.
    Quic,
    /// WebSocket over HTTPS.
    Wss,
}

/// Why a candidate appears in the transport plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateSource {
    /// Descriptor-preferred carrier.
    PreferredCarrier,
    /// Live fallback carrier.
    FallbackCarrier,
    /// Cached posture skipped `QUIC`.
    CachedQuicBadNetwork,
    /// Cached fallback expired and `QUIC` is reprobed.
    QuicReprobeAfterCachedFallback,
}

/// Fallback-eligible outer-carrier failure classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackReason {
    /// UDP path failed before secure-ready.
    OuterPathFailure,
    /// `QUIC` was rejected before secure-ready.
    OuterQuicRejected,
    /// `QUIC` closed before secure-ready.
    OuterQuicClosedEarly,
}

/// Whether carrier choice came from live probing or cached posture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheDisposition {
    /// No cached posture affected selection.
    LiveProbe,
    /// Cached fallback affected selection.
    CachedFallback,
    /// Cached posture expired and was reprobed.
    Reprobe,
}

/// Terminal outcome for one carrier attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportAttemptOutcome {
    /// The carrier reached secure-ready.
    SecureReady,
    /// The carrier failed in a way that permitted fallback.
    Fallback,
    /// The carrier failed and stopped selection.
    Failed,
}

/// Account authentication mode requested by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountAuthMode {
    /// Authenticate with current account credentials.
    Fresh,
    /// Resume a previous account session.
    Resume,
}

/// Account freshness established by the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountFreshness {
    /// Fresh account authentication.
    Fresh,
    /// Resumed account session.
    Resumed,
}

/// Device state exposed by the SDK.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    /// Device can use the tunnel.
    Active,
    /// Device is accepted but awaiting approval.
    Pending,
}

/// Lifecycle state exposed by a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Outer carrier is ready.
    CarrierReady,
    /// Inner handshake is in progress.
    NoiseHandshake,
    /// Inner secure channel is ready.
    SecureReady,
    /// Account authentication completed.
    AccountAuthenticated,
    /// Known-device authentication completed.
    KnownDeviceAuthenticated,
    /// Graceful close is in progress.
    Closing,
    /// Session is closed.
    Closed,
}

/// Coarse close classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseClassification {
    /// Encrypted graceful close completed.
    Graceful,
    /// Transport closed abruptly.
    Abrupt,
    /// Close started but did not complete cleanly.
    Truncated,
}

/// SDK client configuration for generated clients.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientConfig {
    /// Seconds before retrying `QUIC` after fallback.
    pub quic_reprobe_delay_seconds: u64,
    /// Milliseconds allowed for the full connect attempt.
    pub connect_timeout_ms: u64,
    /// Milliseconds allowed for `QUIC` carrier setup phases.
    pub quic_connect_timeout_ms: u64,
    /// Milliseconds allowed for the `WSS` handshake.
    pub wss_connect_timeout_ms: u64,
    /// Milliseconds allowed for secure-ready evaluation.
    pub secure_ready_timeout_ms: u64,
    /// Milliseconds allowed for one framed read.
    pub record_read_timeout_ms: u64,
    /// Milliseconds allowed for one framed write.
    pub record_write_timeout_ms: u64,
    /// DER-encoded outer TLS roots added to platform trust.
    ///
    /// Android extra-root support is currently unavailable in the verifier
    /// dependency and fails outer carrier TLS when non-empty.
    pub outer_root_certificates_der: Vec<Vec<u8>>,
    /// Pinned descriptor roots that may authorize service descriptors.
    pub descriptor_trust_anchors: Vec<DescriptorTrustAnchor>,
    /// Accepted service static public keys.
    pub pinned_service_static_public_keys: Vec<Vec<u8>>,
}

/// Root key that authorizes descriptors for generated clients.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptorTrustAnchor {
    /// Operator-managed key identifier.
    pub key_id: String,
    /// Signature algorithm name, for example `ed25519`.
    pub algorithm: String,
    /// Public key bytes encoded for transport or config.
    pub public_key: String,
}

/// Inputs for one connect attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectOptions {
    /// JSON service descriptor.
    pub descriptor_json: String,
    /// Caller-supplied Unix timestamp.
    pub now_unix_seconds: u64,
    /// Optional cached network posture.
    pub transport_cache: Option<TransportCacheSnapshot>,
}

/// Cached transport posture.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TransportCacheSnapshot {
    /// Last carrier that reached secure-ready.
    pub last_successful_carrier: Option<Carrier>,
    /// Last fallback-eligible `QUIC` failure.
    pub last_quic_failure: Option<FallbackReason>,
    /// Unix timestamp after which `QUIC` should be reprobed.
    pub next_quic_probe_after_unix_seconds: Option<u64>,
    /// Highest descriptor serial accepted for this cache key.
    pub highest_descriptor_serial: Option<u64>,
}

/// One transport attempt for logs and smoke tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportAttemptReport {
    /// Carrier that was attempted.
    pub carrier: Carrier,
    /// Why this candidate was attempted.
    pub source: CandidateSource,
    /// Terminal attempt outcome.
    pub outcome: TransportAttemptOutcome,
    /// Fallback reason when outcome is fallback.
    pub fallback_reason: Option<FallbackReason>,
    /// Stable error class when outcome is failed.
    pub failure_kind: Option<String>,
    /// Diagnostic message when outcome is failed.
    pub failure_message: Option<String>,
}

/// Successful connect report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectReport {
    /// Carrier selected for this session.
    pub selected_carrier: Carrier,
    /// Cache influence on selection.
    pub cache_state: CacheDisposition,
    /// Fallback reason, when fallback occurred.
    pub fallback_reason: Option<FallbackReason>,
    /// Attempt trace collected during selection.
    pub attempts: Vec<TransportAttemptReport>,
    /// Updated transport cache for caller persistence.
    pub transport_cache: TransportCacheSnapshot,
}

/// Account auth request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountAuthRequest {
    /// Product account identifier.
    pub account_id: String,
    /// Opaque credential payload.
    pub credential_payload: Vec<u8>,
    /// Requested auth mode.
    pub mode: AccountAuthMode,
}

/// Account auth report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountAuthReport {
    /// Product account identifier accepted by service.
    pub account_id: String,
    /// Server-side session context identifier.
    pub session_context_id: String,
    /// Stable account context hash.
    pub account_context_hash: Vec<u8>,
    /// Established account freshness.
    pub freshness: AccountFreshness,
}
