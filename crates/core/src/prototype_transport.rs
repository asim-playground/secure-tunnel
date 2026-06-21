// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Test-only prototype QUIC and WSS transport bindings.
//!
//! This module keeps the task-00000012 prototype local to `crates/core` by
//! combining the real `SnowNk1ClientEvaluator` with in-memory carrier adapters
//! that validate carrier-specific selector values, record connection metrics,
//! and expose one bidirectional framed channel per successful carrier.

use std::sync::{Arc, Mutex};

use crate::constants::{QUIC_ALPN_V1, WSS_SUBPROTOCOL_V1};
use crate::error::{ApiError, ApiResult};
use crate::transport::{
    BoxFuture, CarrierConnector, CarrierKind, FallbackReason, FramedDuplex, TransportTarget,
};

mod scripted_responder;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConnectorObservation {
    carrier: CarrierKind,
    target_summary: String,
    selector_field: &'static str,
    selector_value: String,
    logical_record_channels: usize,
    outcome: ConnectorOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConnectorOutcome {
    Established,
    Fallback(FallbackReason),
    Failed(ApiError),
}

enum ConnectorPlan {
    Ready {
        transport: Option<Box<dyn FramedDuplex>>,
    },
    Fallback(FallbackReason),
    Failure(ApiError),
}

struct PrototypeCarrierConnector {
    carrier: CarrierKind,
    plan: Mutex<Option<ConnectorPlan>>,
    observations: Arc<Mutex<Vec<ConnectorObservation>>>,
}

impl PrototypeCarrierConnector {
    fn ready(carrier: CarrierKind, transport: Box<dyn FramedDuplex>) -> Self {
        Self {
            carrier,
            plan: Mutex::new(Some(ConnectorPlan::Ready {
                transport: Some(transport),
            })),
            observations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn fallback(carrier: CarrierKind, reason: FallbackReason) -> Self {
        Self {
            carrier,
            plan: Mutex::new(Some(ConnectorPlan::Fallback(reason))),
            observations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn failure(carrier: CarrierKind, error: ApiError) -> Self {
        Self {
            carrier,
            plan: Mutex::new(Some(ConnectorPlan::Failure(error))),
            observations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn call_count(&self) -> usize {
        self.observations
            .lock()
            .expect("observation lock poisoned")
            .len()
    }

    fn observations(&self) -> Vec<ConnectorObservation> {
        self.observations
            .lock()
            .expect("observation lock poisoned")
            .clone()
    }

    fn connect<'a>(
        &'a self,
        target: &'a TransportTarget,
    ) -> BoxFuture<'a, ApiResult<Box<dyn FramedDuplex>>> {
        let attempt = ConnectorAttempt::new(self.carrier, target);

        Box::pin(async move { self.connect_attempt(&attempt, target) })
    }

    fn connect_attempt(
        &self,
        attempt: &ConnectorAttempt,
        target: &TransportTarget,
    ) -> ApiResult<Box<dyn FramedDuplex>> {
        if target.carrier() != self.carrier {
            return Err(self.observe_error(
                attempt,
                ApiError::TransportSelectorInvariant(
                    "prototype connector received the wrong carrier target",
                ),
            ));
        }

        validate_target(target).map_err(|error| self.observe_error(attempt, error))?;

        match self.take_plan() {
            ConnectorPlan::Ready { mut transport } => {
                let transport = transport
                    .take()
                    .ok_or(ApiError::TransportClosed)
                    .map_err(|error| self.observe_error(attempt, error))?;
                self.observe_success(attempt, 1);
                Ok(transport)
            }
            ConnectorPlan::Fallback(reason) => {
                Err(self.observe_error(attempt, ApiError::TransportFallback(reason)))
            }
            ConnectorPlan::Failure(error) => Err(self.observe_error(attempt, error)),
        }
    }

    fn take_plan(&self) -> ConnectorPlan {
        self.plan
            .lock()
            .expect("plan lock poisoned")
            .take()
            .unwrap_or(ConnectorPlan::Failure(ApiError::TransportClosed))
    }

    fn observe_success(&self, attempt: &ConnectorAttempt, logical_record_channels: usize) {
        self.record_attempt(
            attempt.observation(logical_record_channels, ConnectorOutcome::Established),
        );
    }

    fn observe_error(&self, attempt: &ConnectorAttempt, error: ApiError) -> ApiError {
        self.record_attempt(attempt.observation(0, connector_outcome_for_error(&error)));
        error
    }

    fn record_attempt(&self, observation: ConnectorObservation) {
        record_observation(&self.observations, observation);
    }
}

fn record_observation(
    observations: &Arc<Mutex<Vec<ConnectorObservation>>>,
    observation: ConnectorObservation,
) {
    observations
        .lock()
        .expect("observation lock poisoned")
        .push(observation);
}

fn connector_observation(
    carrier: CarrierKind,
    target_summary: String,
    selector_field: &'static str,
    selector_value: String,
    logical_record_channels: usize,
    outcome: ConnectorOutcome,
) -> ConnectorObservation {
    ConnectorObservation {
        carrier,
        target_summary,
        selector_field,
        selector_value,
        logical_record_channels,
        outcome,
    }
}

fn connector_outcome_for_error(error: &ApiError) -> ConnectorOutcome {
    match error {
        ApiError::TransportFallback(reason) => ConnectorOutcome::Fallback(*reason),
        _ => ConnectorOutcome::Failed(error.clone()),
    }
}

struct ConnectorAttempt {
    carrier: CarrierKind,
    target_summary: String,
    selector_field: &'static str,
    selector_value: String,
}

impl ConnectorAttempt {
    fn new(carrier: CarrierKind, target: &TransportTarget) -> Self {
        let (selector_field, selector_value) = selector_metadata(target);

        Self {
            carrier,
            target_summary: target_summary(target),
            selector_field,
            selector_value,
        }
    }

    fn observation(
        &self,
        logical_record_channels: usize,
        outcome: ConnectorOutcome,
    ) -> ConnectorObservation {
        connector_observation(
            self.carrier,
            self.target_summary.clone(),
            self.selector_field,
            self.selector_value.clone(),
            logical_record_channels,
            outcome,
        )
    }
}

fn target_summary(target: &TransportTarget) -> String {
    match target {
        TransportTarget::Quic(quic) => format!("quic://{}:{}", quic.connect_host, quic.port),
        TransportTarget::Wss(wss) => wss.url.clone(),
    }
}

fn selector_metadata(target: &TransportTarget) -> (&'static str, String) {
    match target {
        TransportTarget::Quic(quic) => ("alpn", quic.alpn.clone()),
        TransportTarget::Wss(wss) => ("subprotocol", wss.subprotocol.clone()),
    }
}

fn validate_target(target: &TransportTarget) -> ApiResult<()> {
    match target {
        TransportTarget::Quic(quic_target) => validate_quic_target(quic_target),
        TransportTarget::Wss(wss_target) => validate_wss_target(wss_target),
    }
}

fn validate_quic_target(target: &crate::descriptor::QuicTarget) -> ApiResult<()> {
    if target.alpn != QUIC_ALPN_V1 {
        return Err(ApiError::TransportFallback(
            FallbackReason::OuterQuicRejected,
        ));
    }

    if target.connect_host.is_empty() || target.port == 0 {
        return Err(ApiError::TransportFallback(
            FallbackReason::OuterPathFailure,
        ));
    }

    Ok(())
}

fn validate_wss_target(target: &crate::descriptor::WssTarget) -> ApiResult<()> {
    if target.subprotocol != WSS_SUBPROTOCOL_V1 {
        return Err(ApiError::TransportSelectorInvariant(
            "WSS subprotocol must match the v1 descriptor value",
        ));
    }

    if !target.url.starts_with("wss://") {
        return Err(ApiError::TransportSelectorInvariant(
            "WSS target URL must use the wss:// scheme",
        ));
    }

    Ok(())
}

struct PrototypeQuicConnector(PrototypeCarrierConnector);

struct PrototypeWssConnector(PrototypeCarrierConnector);

impl PrototypeQuicConnector {
    fn success(transport: Box<dyn FramedDuplex>) -> Self {
        Self(PrototypeCarrierConnector::ready(
            CarrierKind::Quic,
            transport,
        ))
    }

    fn fallback(reason: FallbackReason) -> Self {
        Self(PrototypeCarrierConnector::fallback(
            CarrierKind::Quic,
            reason,
        ))
    }

    fn call_count(&self) -> usize {
        self.0.call_count()
    }

    fn observations(&self) -> Vec<ConnectorObservation> {
        self.0.observations()
    }
}

impl PrototypeWssConnector {
    fn success(transport: Box<dyn FramedDuplex>) -> Self {
        Self(PrototypeCarrierConnector::ready(
            CarrierKind::Wss,
            transport,
        ))
    }

    fn failure(error: ApiError) -> Self {
        Self(PrototypeCarrierConnector::failure(CarrierKind::Wss, error))
    }

    fn call_count(&self) -> usize {
        self.0.call_count()
    }

    fn observations(&self) -> Vec<ConnectorObservation> {
        self.0.observations()
    }
}

impl CarrierConnector for PrototypeQuicConnector {
    fn carrier(&self) -> CarrierKind {
        CarrierKind::Quic
    }

    fn connect<'a>(
        &'a self,
        target: &'a TransportTarget,
    ) -> BoxFuture<'a, ApiResult<Box<dyn FramedDuplex>>> {
        self.0.connect(target)
    }
}

impl CarrierConnector for PrototypeWssConnector {
    fn carrier(&self) -> CarrierKind {
        CarrierKind::Wss
    }

    fn connect<'a>(
        &'a self,
        target: &'a TransportTarget,
    ) -> BoxFuture<'a, ApiResult<Box<dyn FramedDuplex>>> {
        self.0.connect(target)
    }
}

#[cfg(test)]
mod tests;
