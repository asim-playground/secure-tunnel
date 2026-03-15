// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::fmt;
use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};

use crate::descriptor::{QuicTarget, WssTarget};
use crate::error::ApiResult;
use crate::session::CloseDirective;

/// Boxed future used by transport traits without forcing a runtime choice yet.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Supported outer carriers in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarrierKind {
    /// Raw `QUIC` over UDP.
    Quic,
    /// WebSocket over HTTPS.
    Wss,
}

impl fmt::Display for CarrierKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quic => formatter.write_str("quic"),
            Self::Wss => formatter.write_str("wss"),
        }
    }
}

/// One concrete carrier target to attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportTarget {
    /// Raw `QUIC` carrier.
    Quic(QuicTarget),
    /// `WSS` carrier.
    Wss(WssTarget),
}

impl TransportTarget {
    /// Returns the carrier kind for the target.
    #[must_use]
    pub const fn carrier(&self) -> CarrierKind {
        match self {
            Self::Quic(_) => CarrierKind::Quic,
            Self::Wss(_) => CarrierKind::Wss,
        }
    }
}

/// Why one carrier target appears in the current connect plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateSource {
    /// The descriptor prefers this carrier on the current network.
    PreferredCarrier,
    /// The descriptor allows this carrier as a live fallback.
    FallbackCarrier,
    /// Cached network posture says to skip the first `QUIC` attempt for now.
    CachedQuicBadNetwork,
    /// Cached fallback posture expired and `QUIC` should be retried.
    QuicReprobeAfterCachedFallback,
}

/// One candidate target in the order the selector should attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportCandidate {
    /// Concrete target for the candidate carrier.
    pub target: TransportTarget,
    /// Why the candidate is in this position.
    pub source: CandidateSource,
}

/// Coarse cached network posture that belongs to transport policy, not trust.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TransportCacheSnapshot {
    /// Cached last successful carrier for this service and network class.
    pub last_successful_carrier: Option<CarrierKind>,
    /// Most recent `QUIC` failure that still permits `WSS` fallback.
    pub last_quic_failure: Option<FallbackReason>,
    /// Deadline after which `QUIC` should be reprobed on the same network.
    pub next_quic_probe_after_unix_seconds: Option<u64>,
}

/// Fallback-eligible `QUIC` failure classes.
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

impl fmt::Display for FallbackReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OuterPathFailure => formatter.write_str("outer_path_failure"),
            Self::OuterQuicRejected => formatter.write_str("outer_quic_rejected"),
            Self::OuterQuicClosedEarly => formatter.write_str("outer_quic_closed_early"),
        }
    }
}

/// Transport-neutral framed record I/O for the secure-channel engine.
pub trait FramedDuplex: Send {
    /// Returns the carrier backing the framed duplex.
    fn carrier(&self) -> CarrierKind;

    /// Sends one complete secure-channel record.
    fn send_record<'a>(&'a mut self, record: &'a [u8]) -> BoxFuture<'a, ApiResult<()>>;

    /// Receives one complete secure-channel record.
    fn receive_record(&mut self) -> BoxFuture<'_, ApiResult<Option<Vec<u8>>>>;

    /// Requests graceful encrypted shutdown at the protocol layer.
    fn close(&mut self, directive: CloseDirective) -> BoxFuture<'_, ApiResult<()>>;
}

/// Per-carrier connector used by the future selector implementation.
pub trait CarrierConnector: Send + Sync {
    /// Carrier implemented by the connector.
    fn carrier(&self) -> CarrierKind;

    /// Establishes the outer carrier and returns framed record I/O.
    fn connect<'a>(
        &'a self,
        target: &'a TransportTarget,
    ) -> BoxFuture<'a, ApiResult<Box<dyn FramedDuplex>>>;
}
