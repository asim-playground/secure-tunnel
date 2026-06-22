// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::reports::TransportAttemptReport;

/// Stable SDK error classes exposed above the transport and protocol internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SdkErrorKind {
    /// The descriptor could not be decoded or failed v1 validation.
    InvalidDescriptor,
    /// The descriptor or runtime adapter set cannot provide a requested carrier.
    UnavailableCarrier,
    /// The outer network path failed before the inner channel was ready.
    OuterPathFailure,
    /// The outer TLS setup failed before the inner channel was ready.
    ///
    /// Production transport adapters introduced after this facade should map
    /// TLS setup failures to this stable class.
    OuterTlsFailure,
    /// The outer proxy setup failed before the inner channel was ready.
    ///
    /// Production transport adapters introduced after this facade should map
    /// proxy setup failures to this stable class.
    OuterProxyFailure,
    /// The outer carrier protocol negotiation failed.
    OuterProtocolFailure,
    /// The outer `QUIC` carrier was rejected before the inner channel was ready.
    OuterQuicRejected,
    /// The outer `QUIC` carrier closed before the inner channel was ready.
    OuterQuicClosedEarly,
    /// Transport selection exhausted all usable candidates.
    FallbackExhausted,
    /// The inner Noise handshake failed.
    InnerNoiseFailure,
    /// The inner server trust check failed.
    InnerTrustFailure,
    /// Account or known-device authentication failed after `Secure Ready`.
    AuthFailure,
    /// An application payload exceeded the SDK limit.
    PayloadTooLarge,
    /// The caller attempted to use a closed session.
    Closed,
    /// The caller cancelled the operation.
    Cancelled,
    /// The SDK observed an invariant violation or unmapped internal failure.
    Internal,
}

/// Error returned by the product SDK facade.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{kind:?}: {message}")]
pub struct SdkError {
    kind: SdkErrorKind,
    message: String,
}

impl SdkError {
    /// Returns the stable machine-readable error class.
    #[must_use]
    pub const fn kind(&self) -> SdkErrorKind {
        self.kind
    }

    /// Returns the stable foreign-language spelling for [`Self::kind`].
    #[must_use]
    pub const fn kind_str(&self) -> &'static str {
        self.kind.as_str()
    }

    /// Returns the human-readable diagnostic message.
    #[must_use]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    pub(super) fn new(kind: SdkErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub(super) fn cancelled() -> Self {
        Self::new(SdkErrorKind::Cancelled, "operation cancelled")
    }

    pub(super) fn closed() -> Self {
        Self::new(SdkErrorKind::Closed, "secure tunnel session is closed")
    }

    pub(super) fn invalid_descriptor(message: impl Into<String>) -> Self {
        Self::new(SdkErrorKind::InvalidDescriptor, message)
    }

    pub(super) fn internal(message: impl Into<String>) -> Self {
        Self::new(SdkErrorKind::Internal, message)
    }

    pub(super) fn from_core(error: &secure_tunnel_core::ApiError) -> Self {
        use secure_tunnel_core::ApiError;

        match error {
            ApiError::InvalidServiceDescriptor(message) => Self::invalid_descriptor(*message),
            ApiError::UnavailableCarrier(carrier) | ApiError::MissingCarrierConnector(carrier) => {
                Self::new(
                    SdkErrorKind::UnavailableCarrier,
                    format!("carrier `{carrier}` is not available"),
                )
            }
            ApiError::RecordTooLarge { actual, max } => Self::new(
                SdkErrorKind::PayloadTooLarge,
                format!("payload size {actual} exceeds limit {max}"),
            ),
            ApiError::TransportPlanBlocked(message) => {
                Self::new(SdkErrorKind::FallbackExhausted, *message)
            }
            ApiError::TransportFallback(reason) => Self::from_core_fallback(*reason),
            ApiError::OuterPathFailure(carrier) => Self::new(
                SdkErrorKind::OuterPathFailure,
                format!("outer `{carrier}` path failed"),
            ),
            ApiError::OuterTlsFailure(carrier) => Self::new(
                SdkErrorKind::OuterTlsFailure,
                format!("outer `{carrier}` TLS setup failed"),
            ),
            ApiError::OuterProxyFailure(carrier) => Self::new(
                SdkErrorKind::OuterProxyFailure,
                format!("outer `{carrier}` proxy setup failed"),
            ),
            ApiError::OuterProtocolFailure(carrier) => Self::new(
                SdkErrorKind::OuterProtocolFailure,
                format!("outer `{carrier}` protocol negotiation failed"),
            ),
            ApiError::InnerNoiseFailure => Self::new(
                SdkErrorKind::InnerNoiseFailure,
                "inner Noise handshake failed",
            ),
            ApiError::InnerTrustFailure => {
                Self::new(SdkErrorKind::InnerTrustFailure, "inner trust check failed")
            }
            ApiError::PostHandshakeAuthFailure => Self::new(
                SdkErrorKind::AuthFailure,
                "post-handshake authentication failed",
            ),
            ApiError::TransportSelectionExhausted => Self::new(
                SdkErrorKind::FallbackExhausted,
                "transport selection exhausted all candidates",
            ),
            ApiError::TransportSelectionExhaustedWithFallback(reason) => Self::new(
                SdkErrorKind::FallbackExhausted,
                format!("transport selection exhausted after `{reason}`"),
            ),
            ApiError::TransportSelectorInvariant(message) => Self::internal(*message),
            ApiError::OperationCancelled => Self::cancelled(),
            ApiError::TransportClosed => Self::closed(),
        }
    }

    fn from_core_fallback(reason: secure_tunnel_core::FallbackReason) -> Self {
        let kind = match reason {
            secure_tunnel_core::FallbackReason::OuterPathFailure => SdkErrorKind::OuterPathFailure,
            secure_tunnel_core::FallbackReason::OuterQuicRejected => {
                SdkErrorKind::OuterQuicRejected
            }
            secure_tunnel_core::FallbackReason::OuterQuicClosedEarly => {
                SdkErrorKind::OuterQuicClosedEarly
            }
        };
        Self::new(
            kind,
            format!("transport attempt may fall back after `{reason}`"),
        )
    }
}

/// Connect-specific error with transport attempt observability attached.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{error}")]
pub struct ConnectError {
    /// Stable SDK error for the terminal connect failure.
    pub error: SdkError,
    /// Attempt trace collected before the failure, if selection started.
    pub attempts: Vec<TransportAttemptReport>,
}

impl ConnectError {
    /// Creates a connect error with no transport attempts.
    #[must_use]
    pub const fn without_attempts(error: SdkError) -> Self {
        Self {
            error,
            attempts: Vec::new(),
        }
    }

    /// Creates a connect error with an attempt trace.
    #[must_use]
    pub const fn with_attempts(error: SdkError, attempts: Vec<TransportAttemptReport>) -> Self {
        Self { error, attempts }
    }

    /// Returns the stable machine-readable terminal error class.
    #[must_use]
    pub const fn kind(&self) -> SdkErrorKind {
        self.error.kind()
    }

    /// Returns the human-readable terminal diagnostic message.
    #[must_use]
    pub fn message(&self) -> String {
        self.error.message()
    }
}

/// Result alias for SDK facade operations.
pub type SdkResult<T> = Result<T, SdkError>;

/// Result alias for SDK connect operations.
pub type ConnectResult<T> = Result<T, ConnectError>;

impl SdkErrorKind {
    /// Returns the stable machine-readable SDK spelling used by non-Rust APIs.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidDescriptor => "invalid_descriptor",
            Self::UnavailableCarrier => "unavailable_carrier",
            Self::OuterPathFailure => "outer_path_failure",
            Self::OuterTlsFailure => "outer_tls_failure",
            Self::OuterProxyFailure => "outer_proxy_failure",
            Self::OuterProtocolFailure => "outer_protocol_failure",
            Self::OuterQuicRejected => "outer_quic_rejected",
            Self::OuterQuicClosedEarly => "outer_quic_closed_early",
            Self::FallbackExhausted => "fallback_exhausted",
            Self::InnerNoiseFailure => "inner_noise_failure",
            Self::InnerTrustFailure => "inner_trust_failure",
            Self::AuthFailure => "auth_failure",
            Self::PayloadTooLarge => "payload_too_large",
            Self::Closed => "closed",
            Self::Cancelled => "cancelled",
            Self::Internal => "internal",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SdkErrorKind;

    #[test]
    fn sdk_error_kind_strings_are_stable_snake_case() {
        let cases = [
            (SdkErrorKind::InvalidDescriptor, "invalid_descriptor"),
            (SdkErrorKind::UnavailableCarrier, "unavailable_carrier"),
            (SdkErrorKind::OuterPathFailure, "outer_path_failure"),
            (SdkErrorKind::OuterTlsFailure, "outer_tls_failure"),
            (SdkErrorKind::OuterProxyFailure, "outer_proxy_failure"),
            (SdkErrorKind::OuterProtocolFailure, "outer_protocol_failure"),
            (SdkErrorKind::OuterQuicRejected, "outer_quic_rejected"),
            (
                SdkErrorKind::OuterQuicClosedEarly,
                "outer_quic_closed_early",
            ),
            (SdkErrorKind::FallbackExhausted, "fallback_exhausted"),
            (SdkErrorKind::InnerNoiseFailure, "inner_noise_failure"),
            (SdkErrorKind::InnerTrustFailure, "inner_trust_failure"),
            (SdkErrorKind::AuthFailure, "auth_failure"),
            (SdkErrorKind::PayloadTooLarge, "payload_too_large"),
            (SdkErrorKind::Closed, "closed"),
            (SdkErrorKind::Cancelled, "cancelled"),
            (SdkErrorKind::Internal, "internal"),
        ];

        for (kind, expected) in cases {
            assert_eq!(kind.as_str(), expected);
        }
    }
}
