// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use thiserror::Error;

use crate::{CarrierKind, FallbackReason};

/// Error type shared by the initial transport-neutral API surface.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ApiError {
    /// The descriptor is structurally inconsistent with the documented v1 shape.
    #[error("invalid service descriptor: {0}")]
    InvalidServiceDescriptor(&'static str),
    /// The descriptor or caller requested a carrier that is unavailable.
    #[error("carrier `{0}` is not available in this descriptor")]
    UnavailableCarrier(CarrierKind),
    /// A record exceeded the documented v1 size budget.
    #[error("record payload size {actual} exceeds v1 limit {max}")]
    RecordTooLarge {
        /// Observed payload size.
        actual: usize,
        /// Configured v1 maximum payload size.
        max: usize,
    },
    /// The descriptor and cached network posture leave no valid transport plan.
    #[error("transport planning is blocked: {0}")]
    TransportPlanBlocked(&'static str),
    /// The caller did not provide a connector for a required carrier.
    #[error("carrier connector `{0}` is not configured")]
    MissingCarrierConnector(CarrierKind),
    /// The outer carrier failed in a way that still permits fallback.
    #[error("transport attempt may fall back after `{0}`")]
    TransportFallback(FallbackReason),
    /// Secure-channel processing failed during the inner Noise handshake.
    #[error("secure-ready failed during the inner Noise handshake")]
    InnerNoiseFailure,
    /// Secure-channel processing failed during server trust validation.
    #[error("secure-ready failed during trust validation")]
    InnerTrustFailure,
    /// Secure-channel processing failed after `Secure Ready`.
    #[error("secure-ready failed during post-handshake authentication")]
    PostHandshakeAuthFailure,
    /// The selector exhausted all transport candidates without success.
    #[error("transport selection exhausted all candidates")]
    TransportSelectionExhausted,
    /// The selector exhausted all transport candidates after a fallback-eligible `QUIC` failure.
    #[error("transport selection exhausted all candidates after `{0}`")]
    TransportSelectionExhaustedWithFallback(FallbackReason),
    /// The selector observed an impossible state transition.
    #[error("transport selector invariant violated: {0}")]
    TransportSelectorInvariant(&'static str),
    /// The framed transport is already closed.
    #[error("framed transport is closed")]
    TransportClosed,
}

/// Result alias for the public API surface.
pub type ApiResult<T> = Result<T, ApiError>;
