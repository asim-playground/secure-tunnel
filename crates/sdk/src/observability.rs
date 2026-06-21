// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};

use crate::error::SdkErrorKind;
use crate::reports::{
    CacheDisposition, Carrier, FallbackReason, TransportAttemptOutcome, TransportAttemptReport,
};

/// Stable structured event names emitted by the SDK and suitable for OTEL mapping.
pub mod event_names {
    /// Descriptor validation and authorization result.
    pub const DESCRIPTOR_VALIDATION: &str = "descriptor.validation";
    /// One outer carrier attempt.
    pub const TRANSPORT_ATTEMPT: &str = "transport.attempt";
    /// Fallback between outer carriers.
    pub const TRANSPORT_FALLBACK: &str = "transport.fallback";
    /// Inner channel reached `Secure Ready`.
    pub const TRANSPORT_SECURE_READY: &str = "transport.secure_ready";
    /// Non-fallback inner channel failure.
    pub const TRANSPORT_INNER_FAILURE: &str = "transport.inner_failure";
    /// Account authentication result.
    pub const AUTH_ACCOUNT: &str = "auth.account";
    /// Device authentication or enrollment result.
    pub const AUTH_DEVICE: &str = "auth.device";
    /// Session close result.
    pub const SESSION_CLOSE: &str = "session.close";
}

/// Stable metric names for future backend integrations.
pub mod metric_names {
    /// Count of attempted outer carriers.
    pub const TRANSPORT_ATTEMPT_TOTAL: &str = "transport_attempt_total";
    /// Count of fallback transitions.
    pub const TRANSPORT_FALLBACK_TOTAL: &str = "transport_fallback_total";
    /// Count of secure-ready successes.
    pub const TRANSPORT_SECURE_READY_TOTAL: &str = "transport_secure_ready_total";
    /// Count of non-fallback inner failures.
    pub const TRANSPORT_INNER_FAILURE_TOTAL: &str = "transport_inner_failure_total";
    /// Count of cache decisions.
    pub const TRANSPORT_CACHE_DECISION_TOTAL: &str = "transport_cache_decision_total";
    /// Count of descriptor validation outcomes.
    pub const DESCRIPTOR_VALIDATION_TOTAL: &str = "descriptor_validation_total";
    /// Count of session close classifications.
    pub const SESSION_CLOSE_TOTAL: &str = "session_close_total";
    /// Count of QUIC address-validation outcomes.
    pub const QUIC_ADDRESS_VALIDATION_TOTAL: &str = "quic_address_validation_total";
    /// Count of QUIC retry outcomes.
    pub const QUIC_RETRY_TOTAL: &str = "quic_retry_total";
}

/// Coarse telemetry outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryOutcome {
    /// Operation succeeded.
    Success,
    /// Carrier reached secure-ready.
    SecureReady,
    /// Operation used fallback.
    Fallback,
    /// Operation failed.
    Failure,
}

/// Coarse non-sensitive failure class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    /// Descriptor validation, signature, freshness, or rollback failure.
    InvalidDescriptor,
    /// No configured or descriptor-available carrier.
    UnavailableCarrier,
    /// Outer path failure.
    OuterPathFailure,
    /// Outer TLS failure.
    OuterTlsFailure,
    /// Outer proxy failure.
    OuterProxyFailure,
    /// Outer protocol failure.
    OuterProtocolFailure,
    /// QUIC rejected before secure-ready.
    OuterQuicRejected,
    /// QUIC closed before secure-ready.
    OuterQuicClosedEarly,
    /// Fallback candidates were exhausted.
    FallbackExhausted,
    /// Inner Noise failure.
    InnerNoiseFailure,
    /// Inner trust failure.
    InnerTrustFailure,
    /// Post-handshake authentication failure.
    PostHandshakeAuthFailure,
    /// Payload exceeded limits.
    PayloadTooLarge,
    /// Session or transport was closed.
    Closed,
    /// Caller cancellation.
    Cancelled,
    /// Internal invariant failure.
    Internal,
}

impl From<SdkErrorKind> for FailureClass {
    fn from(value: SdkErrorKind) -> Self {
        match value {
            SdkErrorKind::InvalidDescriptor => Self::InvalidDescriptor,
            SdkErrorKind::UnavailableCarrier => Self::UnavailableCarrier,
            SdkErrorKind::OuterPathFailure => Self::OuterPathFailure,
            SdkErrorKind::OuterTlsFailure => Self::OuterTlsFailure,
            SdkErrorKind::OuterProxyFailure => Self::OuterProxyFailure,
            SdkErrorKind::OuterProtocolFailure => Self::OuterProtocolFailure,
            SdkErrorKind::OuterQuicRejected => Self::OuterQuicRejected,
            SdkErrorKind::OuterQuicClosedEarly => Self::OuterQuicClosedEarly,
            SdkErrorKind::FallbackExhausted => Self::FallbackExhausted,
            SdkErrorKind::InnerNoiseFailure => Self::InnerNoiseFailure,
            SdkErrorKind::InnerTrustFailure => Self::InnerTrustFailure,
            SdkErrorKind::AuthFailure => Self::PostHandshakeAuthFailure,
            SdkErrorKind::PayloadTooLarge => Self::PayloadTooLarge,
            SdkErrorKind::Closed => Self::Closed,
            SdkErrorKind::Cancelled => Self::Cancelled,
            SdkErrorKind::Internal => Self::Internal,
        }
    }
}

/// Session close classification for routine logs and metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseClassification {
    /// Encrypted close completed.
    Graceful,
    /// Carrier closed without an encrypted close.
    Abrupt,
    /// Encrypted close started but did not complete cleanly.
    Truncated,
}

/// Authentication stage for redacted auth telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthStage {
    /// Account authentication.
    Account,
    /// Known-device authentication.
    Device,
    /// New-device enrollment.
    DeviceEnrollment,
}

/// Redacted telemetry event snapshot for tests and foreign SDK mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryEvent {
    /// Stable event name.
    pub name: String,
    /// Event outcome.
    pub outcome: Option<TelemetryOutcome>,
    /// Carrier label, when relevant.
    pub carrier: Option<Carrier>,
    /// Cache state, when relevant.
    pub cache_state: Option<CacheDisposition>,
    /// Fallback reason, when relevant.
    pub fallback_reason: Option<FallbackReason>,
    /// Failure class, when relevant.
    pub failure_class: Option<FailureClass>,
    /// Close classification, when relevant.
    pub close_classification: Option<CloseClassification>,
    /// Auth stage, when relevant.
    pub auth_stage: Option<AuthStage>,
}

impl TelemetryEvent {
    /// Creates an event with only a stable name.
    #[must_use]
    pub fn named(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            outcome: None,
            carrier: None,
            cache_state: None,
            fallback_reason: None,
            failure_class: None,
            close_classification: None,
            auth_stage: None,
        }
    }

    /// Creates the redacted event for one transport attempt report.
    #[must_use]
    pub fn from_transport_attempt(value: &TransportAttemptReport) -> Self {
        let mut event = Self::named(event_names::TRANSPORT_ATTEMPT);
        event.carrier = Some(value.carrier);
        match &value.outcome {
            TransportAttemptOutcome::SecureReady => {
                event.outcome = Some(TelemetryOutcome::SecureReady);
            }
            TransportAttemptOutcome::Fallback { reason } => {
                event.outcome = Some(TelemetryOutcome::Fallback);
                event.fallback_reason = Some(*reason);
            }
            TransportAttemptOutcome::Failed { kind, .. } => {
                event.outcome = Some(TelemetryOutcome::Failure);
                event.failure_class = Some((*kind).into());
            }
        }
        event
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_attempt_event_excludes_diagnostic_message() {
        let report = TransportAttemptReport {
            carrier: Carrier::Quic,
            source: crate::CandidateSource::PreferredCarrier,
            outcome: TransportAttemptOutcome::Failed {
                kind: SdkErrorKind::InnerTrustFailure,
                message: "contains host.example and maybe payload".to_owned(),
            },
        };

        let event = TelemetryEvent::from_transport_attempt(&report);
        let json = serde_json::to_string(&event).expect("event serializes");

        assert_eq!(event.failure_class, Some(FailureClass::InnerTrustFailure));
        assert!(!json.contains("host.example"));
        assert!(!json.contains("payload"));
    }
}
