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
use crate::error::{ConnectError, ConnectResult, SdkError, SdkErrorKind};
use crate::observability::{FailureClass, TelemetryOutcome, event_names};
use crate::planning::connect_plan_report;
use crate::ports::{ProductionTransportPorts, TransportPorts};
use crate::reports::{
    ConnectReport, SecureChannelArtifacts, TransportAttemptOutcome, TransportAttemptReport,
    TransportCacheSnapshot,
};
use crate::session::SecureTunnelSession;

/// SDK client configuration shared across connect attempts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Transport selection policy.
    pub transport_policy: TransportPolicyConfig,
    /// Optional DER-encoded outer TLS roots for local or managed deployments.
    pub outer_root_certificates_der: Option<Vec<Vec<u8>>>,
    /// Pinned descriptor roots that may authorize service descriptors.
    pub descriptor_trust_anchors: Vec<secure_tunnel_core::TrustAnchor>,
    /// Pinned service static public keys accepted for the inner `NK1` channel.
    pub pinned_service_static_public_keys: Vec<secure_tunnel_core::NoisePublicKey>,
}

impl ClientConfig {
    /// Sets pinned descriptor roots for service descriptor authorization.
    #[must_use]
    pub fn with_descriptor_trust_anchors(
        mut self,
        anchors: Vec<secure_tunnel_core::TrustAnchor>,
    ) -> Self {
        self.descriptor_trust_anchors = anchors;
        self
    }

    /// Sets pinned service static public keys accepted by the `NK1` handshake.
    #[must_use]
    pub fn with_pinned_service_static_public_keys(
        mut self,
        keys: Vec<secure_tunnel_core::NoisePublicKey>,
    ) -> Self {
        self.pinned_service_static_public_keys = keys;
        self
    }

    /// Sets DER-encoded outer TLS roots for local or managed-network use.
    #[must_use]
    pub fn with_outer_root_certificates_der(mut self, certificates: Vec<Vec<u8>>) -> Self {
        self.outer_root_certificates_der = Some(certificates);
        self
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            transport_policy: TransportPolicyConfig::default(),
            outer_root_certificates_der: None,
            descriptor_trust_anchors: secure_tunnel_core::example_descriptor_trust_anchors(),
            pinned_service_static_public_keys: vec![
                secure_tunnel_core::obfuscated_service_static_public_key(),
            ],
        }
    }
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
        let ports = Arc::new(ProductionTransportPorts::new(&config));
        Self { config, ports }
    }

    /// Connects to the service described by the supplied descriptor.
    ///
    /// The default client uses production transport adapters backed by Tokio
    /// DNS, socket, `QUIC`, and WebSocket I/O. Callers must drive this future
    /// inside a Tokio runtime.
    ///
    /// # Errors
    ///
    /// Returns [`ConnectError`] when descriptor planning fails, a required carrier
    /// adapter is unavailable, secure-ready evaluation fails, selection is
    /// exhausted, or the optional cancellation handle is cancelled. When
    /// transport selection starts, the error includes the attempt trace.
    pub async fn connect(&self, options: ConnectOptions) -> ConnectResult<ConnectOutcome> {
        Self::check_cancelled(options.cancellation.as_ref())?;
        let service_id = options.descriptor.service_id();
        let environment_id = options.descriptor.environment_id();
        if let Err(error) =
            self.authorize_descriptor_for_network(&options.descriptor, options.now_unix_seconds)
        {
            trace_descriptor_validation(&service_id, &environment_id, error.kind());
            return Err(error);
        }
        tracing::info!(
            event_name = event_names::DESCRIPTOR_VALIDATION,
            outcome = ?TelemetryOutcome::Success,
            service_id = %service_id,
            environment_id = %environment_id,
        );
        let core_cache = options
            .transport_cache
            .as_ref()
            .map(TransportCacheSnapshot::to_core);
        if let Err(error) = connect_plan_report(
            &options.descriptor,
            options.transport_cache.as_ref(),
            options.now_unix_seconds,
        ) {
            let error = ConnectError::without_attempts(error);
            trace_descriptor_validation(&service_id, &environment_id, error.kind());
            return Err(error);
        }
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
            let attempts: Vec<TransportAttemptReport> = error
                .attempts
                .iter()
                .map(TransportAttemptReport::from_core)
                .collect();
            let error = SdkError::from_core(&error.cause);
            trace_attempts(&attempts);
            trace_terminal_failure(error.kind());
            ConnectError::with_attempts(error, attempts)
        })?;

        if options
            .cancellation
            .as_ref()
            .is_some_and(CancellationHandle::is_cancelled)
        {
            let attempts: Vec<TransportAttemptReport> = selected
                .attempts
                .iter()
                .map(TransportAttemptReport::from_core)
                .collect();
            trace_attempts(&attempts);
            trace_terminal_failure(SdkErrorKind::Cancelled);
            return Err(ConnectError::with_attempts(SdkError::cancelled(), attempts));
        }
        let report = ConnectReport::from_selected(&selected);
        trace_attempts(&report.attempts);
        tracing::info!(
            event_name = event_names::TRANSPORT_SECURE_READY,
            outcome = ?TelemetryOutcome::SecureReady,
            carrier = ?report.selected_carrier,
            cache_state = ?report.cache_state,
            fallback_reason = ?report.fallback_reason,
        );
        let artifacts = SecureChannelArtifacts::from_core(&selected.artifacts);
        let session = SecureTunnelSession::from_selected(selected, options.descriptor);
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

    fn authorize_descriptor_for_network(
        &self,
        descriptor: &BootstrapDescriptor,
        now_unix_seconds: u64,
    ) -> ConnectResult<()> {
        let core_descriptor = descriptor.core_descriptor();
        core_descriptor
            .authorize_at(now_unix_seconds, &self.config.descriptor_trust_anchors)
            .map_err(|error| ConnectError::without_attempts(SdkError::from_core(&error)))?;
        let service_static_key = core_descriptor
            .service_static_public_key_bytes()
            .map_err(|error| ConnectError::without_attempts(SdkError::from_core(&error)))?;
        if self
            .config
            .pinned_service_static_public_keys
            .iter()
            .all(|pinned| pinned != &service_static_key)
        {
            return Err(ConnectError::without_attempts(SdkError::from_core(
                &secure_tunnel_core::ApiError::InnerTrustFailure,
            )));
        }
        Ok(())
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

fn trace_descriptor_validation(service_id: &str, environment_id: &str, kind: SdkErrorKind) {
    tracing::warn!(
        event_name = event_names::DESCRIPTOR_VALIDATION,
        outcome = ?TelemetryOutcome::Failure,
        failure_class = ?FailureClass::from(kind),
        service_id = %service_id,
        environment_id = %environment_id,
    );
}

fn trace_attempts(attempts: &[TransportAttemptReport]) {
    for attempt in attempts {
        match &attempt.outcome {
            TransportAttemptOutcome::SecureReady => tracing::info!(
                event_name = event_names::TRANSPORT_ATTEMPT,
                outcome = ?TelemetryOutcome::SecureReady,
                carrier = ?attempt.carrier,
                source = ?attempt.source,
            ),
            TransportAttemptOutcome::Fallback { reason } => {
                tracing::warn!(
                    event_name = event_names::TRANSPORT_ATTEMPT,
                    outcome = ?TelemetryOutcome::Fallback,
                    carrier = ?attempt.carrier,
                    source = ?attempt.source,
                    fallback_reason = ?reason,
                );
                tracing::warn!(
                    event_name = event_names::TRANSPORT_FALLBACK,
                    outcome = ?TelemetryOutcome::Fallback,
                    from = ?attempt.carrier,
                    to = ?crate::Carrier::Wss,
                    reason = ?reason,
                );
            }
            TransportAttemptOutcome::Failed { kind, .. } => tracing::warn!(
                event_name = event_names::TRANSPORT_ATTEMPT,
                outcome = ?TelemetryOutcome::Failure,
                carrier = ?attempt.carrier,
                source = ?attempt.source,
                failure_class = ?FailureClass::from(*kind),
            ),
        }
    }
}

fn trace_terminal_failure(kind: SdkErrorKind) {
    let failure_class = FailureClass::from(kind);
    if matches!(
        failure_class,
        FailureClass::InnerNoiseFailure | FailureClass::InnerTrustFailure
    ) {
        tracing::warn!(
            event_name = event_names::TRANSPORT_INNER_FAILURE,
            outcome = ?TelemetryOutcome::Failure,
            failure_class = ?failure_class,
        );
    }
}
