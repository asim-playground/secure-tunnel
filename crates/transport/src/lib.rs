// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Production carrier adapters for Secure Tunnel v1.
//!
//! This crate owns the side-effecting outer transport layer. The core crate
//! remains transport-neutral and receives only framed secure-channel records.

mod config;
mod framing;
mod quic;
mod wss;

pub use config::{TransportClientConfig, TransportClientTimeouts};
pub use quic::QuicConnector;
pub use wss::WssConnector;

/// Production connector set for the v1 `QUIC` plus `WSS` carrier policy.
pub struct ProductionTransportPorts {
    quic: QuicConnector,
    wss: WssConnector,
    secure_ready: secure_tunnel_core::SnowNk1ClientEvaluator,
}

impl ProductionTransportPorts {
    /// Creates production carrier adapters with platform TLS verification.
    #[must_use]
    pub fn new(config: TransportClientConfig) -> Self {
        let secure_ready = secure_tunnel_core::SnowNk1ClientEvaluator::with_pinned_trust(
            config.descriptor_trust_anchors(),
            config.pinned_service_static_public_keys(),
        );
        Self {
            quic: QuicConnector::new(config.clone()),
            wss: WssConnector::new(config),
            secure_ready,
        }
    }

    /// Returns the raw `QUIC` connector.
    #[must_use]
    pub const fn quic(&self) -> &QuicConnector {
        &self.quic
    }

    /// Returns the `WSS` connector.
    #[must_use]
    pub const fn wss(&self) -> &WssConnector {
        &self.wss
    }

    /// Returns the secure-ready evaluator used after carrier establishment.
    #[must_use]
    pub const fn secure_ready(&self) -> &secure_tunnel_core::SnowNk1ClientEvaluator {
        &self.secure_ready
    }
}

impl Default for ProductionTransportPorts {
    fn default() -> Self {
        Self::new(TransportClientConfig::default())
    }
}

#[cfg(test)]
mod tests;
