// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::cancellation::CancellationHandle;
use crate::descriptor::{BootstrapDescriptor, TransportPolicyConfig};
use crate::error::{ConnectError, ConnectResult, SdkError};
use crate::planning::connect_plan_report;
use crate::ports::{TransportPorts, UnavailableTransportPorts};
use crate::reports::{
    ConnectReport, SecureChannelArtifacts, TransportAttemptReport, TransportCacheSnapshot,
};
use crate::session::SecureTunnelSession;

/// SDK client configuration shared across connect attempts.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Transport selection policy.
    pub transport_policy: TransportPolicyConfig,
}

/// Inputs for one connect attempt.
#[derive(Debug, Clone)]
pub struct ConnectOptions {
    /// Parsed bootstrap descriptor.
    pub descriptor: BootstrapDescriptor,
    /// Optional cached network posture from an earlier attempt.
    pub transport_cache: Option<TransportCacheSnapshot>,
    /// Caller-provided Unix timestamp used for deterministic selection.
    pub now_unix_seconds: u64,
    /// Optional cooperative cancellation handle.
    pub cancellation: Option<CancellationHandle>,
}

impl ConnectOptions {
    /// Creates connect options for a descriptor and timestamp.
    #[must_use]
    pub const fn new(descriptor: BootstrapDescriptor, now_unix_seconds: u64) -> Self {
        Self {
            descriptor,
            transport_cache: None,
            now_unix_seconds,
            cancellation: None,
        }
    }

    /// Adds cached network posture to the connect attempt.
    #[must_use]
    pub const fn with_transport_cache(mut self, transport_cache: TransportCacheSnapshot) -> Self {
        self.transport_cache = Some(transport_cache);
        self
    }

    /// Adds a cooperative cancellation handle to the connect attempt.
    #[must_use]
    pub fn with_cancellation(mut self, cancellation: CancellationHandle) -> Self {
        self.cancellation = Some(cancellation);
        self
    }
}

/// Successful connect output containing the session and report.
pub struct ConnectOutcome {
    /// Connected secure tunnel session.
    pub session: SecureTunnelSession,
    /// Log-safe observability report for the connect attempt.
    pub report: ConnectReport,
    /// Explicit security artifacts for integrations that need channel binding.
    pub artifacts: SecureChannelArtifacts,
}

/// Opaque SDK client object used to create tunnel sessions.
pub struct SecureTunnelClient {
    config: ClientConfig,
    ports: Arc<dyn TransportPorts>,
}

impl std::fmt::Debug for SecureTunnelClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SecureTunnelClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl SecureTunnelClient {
    /// Creates an SDK client with the default configuration.
    #[must_use]
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            ports: Arc::new(UnavailableTransportPorts::default()),
        }
    }

    /// Connects to the service described by the supplied descriptor.
    ///
    /// The method is async, but runtime-neutral. Callers choose the executor
    /// that drives the returned future.
    ///
    /// # Errors
    ///
    /// Returns [`ConnectError`] when descriptor planning fails, a required carrier
    /// adapter is unavailable, secure-ready evaluation fails, selection is
    /// exhausted, or the optional cancellation handle is cancelled. When
    /// transport selection starts, the error includes the attempt trace.
    pub async fn connect(&self, options: ConnectOptions) -> ConnectResult<ConnectOutcome> {
        Self::check_cancelled(options.cancellation.as_ref())?;
        let core_cache = options
            .transport_cache
            .as_ref()
            .map(TransportCacheSnapshot::to_core);
        let _plan = connect_plan_report(
            &options.descriptor,
            options.transport_cache.as_ref(),
            options.now_unix_seconds,
        )
        .map_err(ConnectError::without_attempts)?;
        Self::check_cancelled(options.cancellation.as_ref())?;

        let selected = secure_tunnel_core::TransportSelector::new(
            self.config.transport_policy.quic_reprobe_delay_seconds,
        )
        .select(
            options.descriptor.core_descriptor(),
            core_cache.as_ref(),
            options.now_unix_seconds,
            secure_tunnel_core::TransportConnectors::new(self.ports.quic(), self.ports.wss()),
            self.ports.secure_ready(),
        )
        .await
        .map_err(|error| {
            let attempts = error
                .attempts
                .iter()
                .map(TransportAttemptReport::from_core)
                .collect();
            ConnectError::with_attempts(SdkError::from_core(&error.cause), attempts)
        })?;

        if options
            .cancellation
            .as_ref()
            .is_some_and(CancellationHandle::is_cancelled)
        {
            let attempts = selected
                .attempts
                .iter()
                .map(TransportAttemptReport::from_core)
                .collect();
            return Err(ConnectError::with_attempts(SdkError::cancelled(), attempts));
        }
        let report = ConnectReport::from_selected(&selected);
        let artifacts = SecureChannelArtifacts::from_core(&selected.artifacts);
        let session = SecureTunnelSession::from_selected(selected);
        Ok(ConnectOutcome {
            session,
            report,
            artifacts,
        })
    }

    #[cfg(test)]
    pub(super) fn with_ports(config: ClientConfig, ports: Arc<dyn TransportPorts>) -> Self {
        Self { config, ports }
    }

    fn check_cancelled(cancellation: Option<&CancellationHandle>) -> ConnectResult<()> {
        match cancellation {
            Some(handle) if handle.is_cancelled() => {
                Err(ConnectError::without_attempts(SdkError::cancelled()))
            }
            _ => Ok(()),
        }
    }
}
