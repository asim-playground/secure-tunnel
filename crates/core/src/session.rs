// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use crate::transport::{CarrierKind, FallbackReason, FramedDuplex};

/// Higher-layer view of a successful secure channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecureReadyReport {
    /// Outer carrier selected for this session.
    pub carrier: CarrierKind,
    /// Whether transport choice came from live probing or cached posture.
    pub cache_state: CacheDisposition,
    /// Normalized fallback reason, when fallback occurred.
    pub fallback_reason: Option<FallbackReason>,
}

/// Secure-channel artifacts produced when a transport reaches `Secure Ready`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SecureReadyArtifacts {
    /// Handshake hash `h` or equivalent transcript binding, when available.
    pub handshake_hash: Option<Vec<u8>>,
    /// Additional channel-binding bytes reserved for later session work.
    pub channel_binding: Option<Vec<u8>>,
}

/// Successful secure-ready transition with the surviving transport and artifacts.
pub struct SecureReadyTransport {
    /// Framed transport that reached `Secure Ready`.
    pub transport: Box<dyn FramedDuplex>,
    /// Secure-ready artifacts exposed to later session or device flows.
    pub artifacts: SecureReadyArtifacts,
}

impl SecureReadyTransport {
    /// Returns the carrier backing the secure-ready transport.
    #[must_use]
    pub fn carrier(&self) -> CarrierKind {
        self.transport.carrier()
    }
}

/// Whether transport choice came from a live probe or cached posture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheDisposition {
    /// No cached network posture affected carrier choice.
    LiveProbe,
    /// Cached network posture skipped the initial `QUIC` attempt.
    CachedFallback,
    /// Cached posture had expired and `QUIC` was retried.
    Reprobe,
}

/// Secure-channel lifecycle state exposed above the transport adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    /// Outer carrier established and selector value confirmed.
    CarrierReady,
    /// Noise handshake and trust validation are in progress.
    NoiseHandshake,
    /// Noise transport mode is active.
    SecureReady,
    /// Account session is established inside Noise transport.
    AccountAuthenticated,
    /// Account session and known-device status are both established.
    KnownDeviceAuthenticated,
    /// Graceful encrypted shutdown is in progress.
    Closing,
}

/// Initial close directive that belongs in the core protocol, not the carrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloseDirective {
    /// Application-level reason code for encrypted close.
    pub code: u16,
    /// Whether the peer should try to drain before tearing down the carrier.
    pub drain: bool,
}
