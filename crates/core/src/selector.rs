// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::error::Error;
use std::fmt;

use crate::descriptor::ServiceDescriptor;
use crate::error::{ApiError, ApiResult};
use crate::session::{
    CacheDisposition, SecureReadyArtifacts, SecureReadyReport, SecureReadyTransport,
};
use crate::transport::{
    BoxFuture, CandidateSource, CarrierConnector, CarrierKind, FramedDuplex,
    TransportCacheSnapshot, TransportCandidate,
};

/// A pair of carrier connectors keyed by the current v1 carrier set.
#[derive(Clone, Copy)]
pub struct TransportConnectors<'a> {
    /// Connector for raw `QUIC`.
    pub quic: Option<&'a dyn CarrierConnector>,
    /// Connector for `WSS`.
    pub wss: Option<&'a dyn CarrierConnector>,
}

impl<'a> TransportConnectors<'a> {
    /// Creates a connector set for the selector.
    #[must_use]
    pub const fn new(
        quic: Option<&'a dyn CarrierConnector>,
        wss: Option<&'a dyn CarrierConnector>,
    ) -> Self {
        Self { quic, wss }
    }

    fn connector_for(self, carrier: CarrierKind) -> ApiResult<&'a dyn CarrierConnector> {
        let connector = match carrier {
            CarrierKind::Quic => self.quic,
            CarrierKind::Wss => self.wss,
        }
        .ok_or(ApiError::MissingCarrierConnector(carrier))?;

        if connector.carrier() != carrier {
            return Err(ApiError::TransportSelectorInvariant(
                "connector carrier does not match the requested target",
            ));
        }

        Ok(connector)
    }
}

/// Drives a framed transport from outer-carrier establishment to `Secure Ready`.
///
/// The selector owns transport policy, but the secure-channel core owns the
/// handshake and trust work that determines whether the candidate truly reached
/// `Secure Ready`.
pub trait SecureReadyEvaluator: Send + Sync {
    /// Consumes a candidate transport and returns it only after `Secure Ready`.
    ///
    /// # Errors
    ///
    /// Returns `ApiError::TransportFallback(...)` when an outer-carrier
    /// `QUIC` failure remains eligible for `WSS` fallback, or a non-fallback
    /// error when the attempt must stop.
    fn reach_secure_ready(
        &self,
        transport: Box<dyn FramedDuplex>,
    ) -> BoxFuture<'_, ApiResult<SecureReadyTransport>>;
}

/// One recorded selector attempt for observability and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportAttemptTrace {
    /// Carrier that was attempted.
    pub carrier: CarrierKind,
    /// Why the candidate appeared in this position.
    pub source: CandidateSource,
    /// Terminal or successful outcome for the candidate.
    pub outcome: TransportAttemptOutcome,
}

/// Outcome of one selector attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportAttemptOutcome {
    /// The carrier reached `Secure Ready`.
    SecureReady,
    /// The attempt failed with a fallback-eligible `QUIC` reason.
    Fallback(crate::FallbackReason),
    /// The attempt failed and must stop transport selection.
    Failed(ApiError),
}

/// Successful selector output.
pub struct SelectedTransport {
    /// Framed transport that already reached `Secure Ready`.
    pub transport: Box<dyn FramedDuplex>,
    /// Secure-ready artifacts forwarded to later session or device work.
    pub artifacts: SecureReadyArtifacts,
    /// Higher-layer report describing the selected carrier and cache posture.
    pub report: SecureReadyReport,
    /// Updated coarse cache posture for later selection attempts.
    pub cache_snapshot: TransportCacheSnapshot,
    /// Attempt trace for observability and tests.
    pub attempts: Vec<TransportAttemptTrace>,
}

/// Selector failure with the attempt trace preserved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportSelectionError {
    /// Terminal selector cause.
    pub cause: ApiError,
    /// Attempt trace collected before the failure.
    pub attempts: Vec<TransportAttemptTrace>,
}

impl fmt::Display for TransportSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "transport selection failed: {}", self.cause)
    }
}

impl Error for TransportSelectionError {}

/// Minimal transport-selector state machine for the first prototype slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportSelector {
    quic_reprobe_delay_seconds: u64,
}

impl TransportSelector {
    /// Creates a selector with a caller-supplied `QUIC` reprobe delay.
    #[must_use]
    pub const fn new(quic_reprobe_delay_seconds: u64) -> Self {
        Self {
            quic_reprobe_delay_seconds,
        }
    }

    /// Runs the documented v1 `QUIC`-first selection flow until `Secure Ready`.
    ///
    /// # Errors
    ///
    /// Returns a `TransportSelectionError` when the descriptor is invalid, a
    /// required connector is missing, fallback is exhausted, or a non-fallback
    /// secure-channel failure occurs.
    pub async fn select(
        self,
        descriptor: &ServiceDescriptor,
        cache: Option<&TransportCacheSnapshot>,
        now_unix_seconds: u64,
        connectors: TransportConnectors<'_>,
        secure_ready: &dyn SecureReadyEvaluator,
    ) -> Result<SelectedTransport, TransportSelectionError> {
        let plan = descriptor
            .connect_plan(cache, now_unix_seconds)
            .map_err(TransportSelectionError::without_attempts)?;
        let mut state = SelectionState::new(
            plan.len(),
            plan.first().map(|candidate| candidate.source),
            cache,
        );

        for (index, candidate) in plan.iter().enumerate() {
            let transport = match self
                .connect_candidate(candidate, connectors, &mut state.attempts)
                .await?
            {
                AttemptState::Continue(reason) => {
                    state.handle_fallback(index, reason)?;
                    continue;
                }
                AttemptState::Ready(transport) => transport,
            };

            match self
                .evaluate_candidate(candidate, transport, secure_ready, &mut state.attempts)
                .await?
            {
                AttemptState::Continue(reason) => {
                    state.handle_fallback(index, reason)?;
                }
                AttemptState::Ready(secure_ready_transport) => {
                    state.attempts.push(TransportAttemptTrace {
                        carrier: candidate.target.carrier(),
                        source: candidate.source,
                        outcome: TransportAttemptOutcome::SecureReady,
                    });
                    return Ok(state.finish_success(
                        candidate.target.carrier(),
                        secure_ready_transport,
                        cache,
                        now_unix_seconds,
                        self.quic_reprobe_delay_seconds,
                    ));
                }
            }
        }

        Err(state.into_exhausted_error())
    }

    async fn connect_candidate(
        self,
        candidate: &TransportCandidate,
        connectors: TransportConnectors<'_>,
        attempts: &mut Vec<TransportAttemptTrace>,
    ) -> Result<AttemptState<Box<dyn FramedDuplex>>, TransportSelectionError> {
        let carrier = candidate.target.carrier();
        let connector = connectors
            .connector_for(carrier)
            .map_err(|cause| TransportSelectionError::with_attempts(cause, attempts))?;

        classify_attempt(
            candidate,
            attempts,
            connector.connect(&candidate.target).await,
            |transport| transport.carrier(),
        )
    }

    async fn evaluate_candidate(
        self,
        candidate: &TransportCandidate,
        transport: Box<dyn FramedDuplex>,
        secure_ready: &dyn SecureReadyEvaluator,
        attempts: &mut Vec<TransportAttemptTrace>,
    ) -> Result<AttemptState<SecureReadyTransport>, TransportSelectionError> {
        classify_attempt(
            candidate,
            attempts,
            secure_ready.reach_secure_ready(transport).await,
            SecureReadyTransport::carrier,
        )
    }
}

enum AttemptState<T> {
    Continue(crate::FallbackReason),
    Ready(T),
}

struct SelectionState {
    cache_state: CacheDisposition,
    fallback_reason: Option<crate::FallbackReason>,
    attempts: Vec<TransportAttemptTrace>,
    plan_len: usize,
}

impl SelectionState {
    fn new(
        plan_len: usize,
        first_candidate_source: Option<CandidateSource>,
        cache: Option<&TransportCacheSnapshot>,
    ) -> Self {
        let cache_state = cache_disposition(first_candidate_source);

        Self {
            cache_state,
            fallback_reason: initial_fallback_reason(
                cache_state,
                cache.and_then(|snapshot| snapshot.last_quic_failure),
            ),
            attempts: Vec::with_capacity(plan_len),
            plan_len,
        }
    }

    fn handle_fallback(
        &mut self,
        index: usize,
        reason: crate::FallbackReason,
    ) -> Result<(), TransportSelectionError> {
        self.fallback_reason = Some(reason);
        if index + 1 < self.plan_len {
            return Ok(());
        }

        Err(self.clone_attempts_error(self.exhausted_cause()))
    }

    fn finish_success(
        self,
        carrier: CarrierKind,
        secure_ready_transport: SecureReadyTransport,
        previous_cache: Option<&TransportCacheSnapshot>,
        now_unix_seconds: u64,
        quic_reprobe_delay_seconds: u64,
    ) -> SelectedTransport {
        let report = SecureReadyReport {
            carrier,
            cache_state: self.cache_state,
            fallback_reason: self.fallback_reason,
        };

        SelectedTransport {
            transport: secure_ready_transport.transport,
            artifacts: secure_ready_transport.artifacts,
            report,
            cache_snapshot: updated_cache_snapshot(
                carrier,
                self.cache_state,
                self.fallback_reason,
                previous_cache,
                now_unix_seconds,
                quic_reprobe_delay_seconds,
            ),
            attempts: self.attempts,
        }
    }

    fn into_exhausted_error(self) -> TransportSelectionError {
        TransportSelectionError {
            cause: self.exhausted_cause(),
            attempts: self.attempts,
        }
    }

    const fn exhausted_cause(&self) -> ApiError {
        match self.fallback_reason {
            Some(reason) => ApiError::TransportSelectionExhaustedWithFallback(reason),
            None => ApiError::TransportSelectionExhausted,
        }
    }

    fn clone_attempts_error(&self, cause: ApiError) -> TransportSelectionError {
        TransportSelectionError {
            cause,
            attempts: self.attempts.clone(),
        }
    }
}

impl TransportSelectionError {
    const fn without_attempts(cause: ApiError) -> Self {
        Self {
            cause,
            attempts: Vec::new(),
        }
    }

    fn with_attempts(cause: ApiError, attempts: &[TransportAttemptTrace]) -> Self {
        Self {
            cause,
            attempts: attempts.to_vec(),
        }
    }
}

fn classify_attempt<T>(
    candidate: &TransportCandidate,
    attempts: &mut Vec<TransportAttemptTrace>,
    result: ApiResult<T>,
    carrier_of: impl FnOnce(&T) -> CarrierKind,
) -> Result<AttemptState<T>, TransportSelectionError> {
    let carrier = candidate.target.carrier();
    match result {
        Ok(transport) => {
            if carrier_of(&transport) != carrier {
                let cause = ApiError::TransportSelectorInvariant(
                    "attempt returned a framed transport for the wrong carrier",
                );
                attempts.push(failed_attempt(candidate.source, carrier, cause.clone()));
                return Err(TransportSelectionError::with_attempts(cause, attempts));
            }

            Ok(AttemptState::Ready(transport))
        }
        Err(ApiError::TransportFallback(reason)) if carrier == CarrierKind::Quic => {
            attempts.push(TransportAttemptTrace {
                carrier,
                source: candidate.source,
                outcome: TransportAttemptOutcome::Fallback(reason),
            });
            Ok(AttemptState::Continue(reason))
        }
        Err(ApiError::TransportFallback(_)) => {
            let cause =
                ApiError::TransportSelectorInvariant("only QUIC attempts may request WSS fallback");
            attempts.push(failed_attempt(candidate.source, carrier, cause.clone()));
            Err(TransportSelectionError::with_attempts(cause, attempts))
        }
        Err(cause) => {
            attempts.push(failed_attempt(candidate.source, carrier, cause.clone()));
            Err(TransportSelectionError::with_attempts(cause, attempts))
        }
    }
}

const fn cache_disposition(source: Option<CandidateSource>) -> CacheDisposition {
    match source {
        Some(CandidateSource::CachedQuicBadNetwork) => CacheDisposition::CachedFallback,
        Some(CandidateSource::QuicReprobeAfterCachedFallback) => CacheDisposition::Reprobe,
        _ => CacheDisposition::LiveProbe,
    }
}

const fn initial_fallback_reason(
    cache_state: CacheDisposition,
    cached_reason: Option<crate::FallbackReason>,
) -> Option<crate::FallbackReason> {
    match cache_state {
        CacheDisposition::CachedFallback => cached_reason,
        CacheDisposition::LiveProbe | CacheDisposition::Reprobe => None,
    }
}

fn updated_cache_snapshot(
    carrier: CarrierKind,
    cache_state: CacheDisposition,
    fallback_reason: Option<crate::FallbackReason>,
    previous_cache: Option<&TransportCacheSnapshot>,
    now_unix_seconds: u64,
    quic_reprobe_delay_seconds: u64,
) -> TransportCacheSnapshot {
    match carrier {
        CarrierKind::Quic => TransportCacheSnapshot {
            last_successful_carrier: Some(CarrierKind::Quic),
            last_quic_failure: None,
            next_quic_probe_after_unix_seconds: None,
        },
        CarrierKind::Wss => match cache_state {
            CacheDisposition::CachedFallback => TransportCacheSnapshot {
                last_successful_carrier: Some(CarrierKind::Wss),
                last_quic_failure: fallback_reason
                    .or_else(|| previous_cache.and_then(|snapshot| snapshot.last_quic_failure)),
                next_quic_probe_after_unix_seconds: previous_cache
                    .and_then(|snapshot| snapshot.next_quic_probe_after_unix_seconds),
            },
            CacheDisposition::LiveProbe | CacheDisposition::Reprobe => TransportCacheSnapshot {
                last_successful_carrier: Some(CarrierKind::Wss),
                last_quic_failure: fallback_reason,
                next_quic_probe_after_unix_seconds: fallback_reason
                    .map(|_| now_unix_seconds.saturating_add(quic_reprobe_delay_seconds)),
            },
        },
    }
}

const fn failed_attempt(
    source: CandidateSource,
    carrier: CarrierKind,
    cause: ApiError,
) -> TransportAttemptTrace {
    TransportAttemptTrace {
        carrier,
        source,
        outcome: TransportAttemptOutcome::Failed(cause),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::future::{Future, ready};
    use std::sync::Mutex;

    use super::{
        SecureReadyEvaluator, TransportAttemptOutcome, TransportConnectors, TransportSelector,
    };
    use crate::{
        ApiError, ApiResult, CarrierKind, CloseDirective, FallbackReason, FramedDuplex,
        SecureReadyArtifacts, SecureReadyTransport, TransportCacheSnapshot, TransportTarget,
        example_service_descriptor,
    };

    #[test]
    fn selector_prefers_quic_on_unknown_network() {
        let descriptor = example_service_descriptor();
        let quic = MockConnector::succeeds(CarrierKind::Quic);
        let wss = MockConnector::succeeds(CarrierKind::Wss);
        let evaluator = MockSecureReadyEvaluator::success();

        let selected = block_on(TransportSelector::new(300).select(
            &descriptor,
            None,
            1_742_000_000,
            TransportConnectors::new(Some(&quic), Some(&wss)),
            &evaluator,
        ))
        .unwrap();

        assert_eq!(selected.transport.carrier(), CarrierKind::Quic);
        assert_eq!(selected.artifacts.handshake_hash, Some(vec![0xAA, 0xBB]));
        assert_eq!(selected.report.carrier, CarrierKind::Quic);
        assert_eq!(
            selected.report.cache_state,
            crate::CacheDisposition::LiveProbe
        );
        assert_eq!(selected.report.fallback_reason, None);
        assert_eq!(
            selected.cache_snapshot.last_successful_carrier,
            Some(CarrierKind::Quic)
        );
        assert_eq!(selected.cache_snapshot.last_quic_failure, None);
        assert_eq!(
            selected.cache_snapshot.next_quic_probe_after_unix_seconds,
            None
        );
        assert_eq!(selected.attempts.len(), 1);
        assert_eq!(
            selected.attempts[0].outcome,
            TransportAttemptOutcome::SecureReady
        );
        assert_eq!(quic.call_count(), 1);
        assert_eq!(wss.call_count(), 0);
    }

    #[test]
    fn selector_falls_back_to_wss_after_quic_outer_failure() {
        let descriptor = example_service_descriptor();
        let quic = MockConnector::fails(
            CarrierKind::Quic,
            ApiError::TransportFallback(FallbackReason::OuterPathFailure),
        );
        let wss = MockConnector::succeeds(CarrierKind::Wss);
        let evaluator = MockSecureReadyEvaluator::success();

        let selected = block_on(TransportSelector::new(300).select(
            &descriptor,
            None,
            1_742_000_000,
            TransportConnectors::new(Some(&quic), Some(&wss)),
            &evaluator,
        ))
        .unwrap();

        assert_eq!(selected.report.carrier, CarrierKind::Wss);
        assert_eq!(
            selected.report.cache_state,
            crate::CacheDisposition::LiveProbe
        );
        assert_eq!(
            selected.report.fallback_reason,
            Some(FallbackReason::OuterPathFailure)
        );
        assert_eq!(
            selected.cache_snapshot.last_successful_carrier,
            Some(CarrierKind::Wss)
        );
        assert_eq!(
            selected.cache_snapshot.last_quic_failure,
            Some(FallbackReason::OuterPathFailure)
        );
        assert_eq!(
            selected.cache_snapshot.next_quic_probe_after_unix_seconds,
            Some(1_742_000_300)
        );
        assert_eq!(
            selected.attempts[0].outcome,
            TransportAttemptOutcome::Fallback(FallbackReason::OuterPathFailure)
        );
        assert_eq!(
            selected.attempts[1].outcome,
            TransportAttemptOutcome::SecureReady
        );
        assert_eq!(quic.call_count(), 1);
        assert_eq!(wss.call_count(), 1);
    }

    #[test]
    fn selector_uses_cached_fallback_without_extending_deadline() {
        let descriptor = example_service_descriptor();
        let cache = TransportCacheSnapshot {
            last_successful_carrier: Some(CarrierKind::Wss),
            last_quic_failure: Some(FallbackReason::OuterPathFailure),
            next_quic_probe_after_unix_seconds: Some(2_000),
        };
        let quic = MockConnector::succeeds(CarrierKind::Quic);
        let wss = MockConnector::succeeds(CarrierKind::Wss);
        let evaluator = MockSecureReadyEvaluator::success();

        let selected = block_on(TransportSelector::new(300).select(
            &descriptor,
            Some(&cache),
            1_999,
            TransportConnectors::new(Some(&quic), Some(&wss)),
            &evaluator,
        ))
        .unwrap();

        assert_eq!(selected.report.carrier, CarrierKind::Wss);
        assert_eq!(
            selected.report.cache_state,
            crate::CacheDisposition::CachedFallback
        );
        assert_eq!(
            selected.report.fallback_reason,
            Some(FallbackReason::OuterPathFailure)
        );
        assert_eq!(selected.attempts.len(), 1);
        assert_eq!(
            selected.attempts[0].source,
            crate::CandidateSource::CachedQuicBadNetwork
        );
        assert_eq!(
            selected.attempts[0].outcome,
            TransportAttemptOutcome::SecureReady
        );
        assert_eq!(
            selected.cache_snapshot.next_quic_probe_after_unix_seconds,
            Some(2_000)
        );
        assert_eq!(quic.call_count(), 0);
        assert_eq!(wss.call_count(), 1);
    }

    #[test]
    fn selector_reprobes_quic_after_cache_expiry() {
        let descriptor = example_service_descriptor();
        let cache = TransportCacheSnapshot {
            last_successful_carrier: Some(CarrierKind::Wss),
            last_quic_failure: Some(FallbackReason::OuterPathFailure),
            next_quic_probe_after_unix_seconds: Some(2_000),
        };
        let quic = MockConnector::succeeds(CarrierKind::Quic);
        let wss = MockConnector::succeeds(CarrierKind::Wss);
        let evaluator = MockSecureReadyEvaluator::success();

        let selected = block_on(TransportSelector::new(300).select(
            &descriptor,
            Some(&cache),
            2_000,
            TransportConnectors::new(Some(&quic), Some(&wss)),
            &evaluator,
        ))
        .unwrap();

        assert_eq!(selected.report.carrier, CarrierKind::Quic);
        assert_eq!(
            selected.report.cache_state,
            crate::CacheDisposition::Reprobe
        );
        assert_eq!(selected.report.fallback_reason, None);
        assert_eq!(
            selected.attempts[0].source,
            crate::CandidateSource::QuicReprobeAfterCachedFallback
        );
        assert_eq!(quic.call_count(), 1);
        assert_eq!(wss.call_count(), 0);
    }

    #[test]
    fn selector_does_not_fallback_after_inner_trust_failure() {
        let descriptor = example_service_descriptor();
        let quic = MockConnector::succeeds(CarrierKind::Quic);
        let wss = MockConnector::succeeds(CarrierKind::Wss);
        let evaluator = MockSecureReadyEvaluator::fails(ApiError::InnerTrustFailure);

        let result = block_on(TransportSelector::new(300).select(
            &descriptor,
            None,
            1_742_000_000,
            TransportConnectors::new(Some(&quic), Some(&wss)),
            &evaluator,
        ));
        let Err(error) = result else {
            panic!("selector should stop on inner trust failure");
        };

        assert_eq!(error.cause, ApiError::InnerTrustFailure);
        assert_eq!(error.attempts.len(), 1);
        assert_eq!(
            error.attempts[0].outcome,
            TransportAttemptOutcome::Failed(ApiError::InnerTrustFailure)
        );
        assert_eq!(quic.call_count(), 1);
        assert_eq!(wss.call_count(), 0);
    }

    #[test]
    fn selector_surfaces_fallback_reason_when_no_wss_candidate_exists() {
        let mut descriptor = example_service_descriptor();
        descriptor.selection_policy.allow_wss_fallback = false;
        descriptor.carriers.wss = None;

        let quic = MockConnector::fails(
            CarrierKind::Quic,
            ApiError::TransportFallback(FallbackReason::OuterPathFailure),
        );
        let evaluator = MockSecureReadyEvaluator::success();

        let result = block_on(TransportSelector::new(300).select(
            &descriptor,
            None,
            1_742_000_000,
            TransportConnectors::new(Some(&quic), None),
            &evaluator,
        ));
        let Err(error) = result else {
            panic!("selector should surface exhausted fallback without WSS");
        };

        assert_eq!(
            error.cause,
            ApiError::TransportSelectionExhaustedWithFallback(FallbackReason::OuterPathFailure)
        );
        assert_eq!(
            error.attempts[0].outcome,
            TransportAttemptOutcome::Fallback(FallbackReason::OuterPathFailure)
        );
    }

    #[test]
    fn selector_rejects_transport_with_mismatched_carrier() {
        let descriptor = example_service_descriptor();
        let quic = MockConnector::returns_transport(CarrierKind::Quic, CarrierKind::Wss);
        let wss = MockConnector::succeeds(CarrierKind::Wss);
        let evaluator = MockSecureReadyEvaluator::success();

        let result = block_on(TransportSelector::new(300).select(
            &descriptor,
            None,
            1_742_000_000,
            TransportConnectors::new(Some(&quic), Some(&wss)),
            &evaluator,
        ));
        let Err(error) = result else {
            panic!("selector should reject a mismatched returned carrier");
        };

        assert_eq!(
            error.cause,
            ApiError::TransportSelectorInvariant(
                "attempt returned a framed transport for the wrong carrier"
            )
        );
        assert_eq!(
            error.attempts[0].outcome,
            TransportAttemptOutcome::Failed(ApiError::TransportSelectorInvariant(
                "attempt returned a framed transport for the wrong carrier"
            ))
        );
        assert_eq!(quic.call_count(), 1);
        assert_eq!(wss.call_count(), 0);
    }

    fn block_on<F>(future: F) -> F::Output
    where
        F: Future,
    {
        futures::executor::block_on(future)
    }

    struct MockConnector {
        carrier: CarrierKind,
        outcomes: Mutex<VecDeque<ApiResult<()>>>,
        calls: Mutex<Vec<CarrierKind>>,
        returned_transport_carrier: CarrierKind,
    }

    impl MockConnector {
        fn succeeds(carrier: CarrierKind) -> Self {
            Self {
                carrier,
                outcomes: Mutex::new(VecDeque::from([Ok(())])),
                calls: Mutex::new(Vec::new()),
                returned_transport_carrier: carrier,
            }
        }

        fn fails(carrier: CarrierKind, error: ApiError) -> Self {
            Self {
                carrier,
                outcomes: Mutex::new(VecDeque::from([Err(error)])),
                calls: Mutex::new(Vec::new()),
                returned_transport_carrier: carrier,
            }
        }

        fn returns_transport(
            carrier: CarrierKind,
            returned_transport_carrier: CarrierKind,
        ) -> Self {
            Self {
                carrier,
                outcomes: Mutex::new(VecDeque::from([Ok(())])),
                calls: Mutex::new(Vec::new()),
                returned_transport_carrier,
            }
        }

        fn call_count(&self) -> usize {
            self.calls.lock().unwrap().len()
        }
    }

    impl crate::CarrierConnector for MockConnector {
        fn carrier(&self) -> CarrierKind {
            self.carrier
        }

        fn connect<'a>(
            &'a self,
            target: &'a TransportTarget,
        ) -> crate::BoxFuture<'a, ApiResult<Box<dyn FramedDuplex>>> {
            self.calls.lock().unwrap().push(target.carrier());
            let result = self.outcomes.lock().unwrap().pop_front().unwrap_or(Ok(()));
            let carrier = self.returned_transport_carrier;
            Box::pin(async move {
                result.map(|()| Box::new(MockFramedDuplex { carrier }) as Box<dyn FramedDuplex>)
            })
        }
    }

    struct MockSecureReadyEvaluator {
        outcomes: Mutex<VecDeque<ApiResult<()>>>,
    }

    impl MockSecureReadyEvaluator {
        fn success() -> Self {
            Self {
                outcomes: Mutex::new(VecDeque::from([Ok(())])),
            }
        }

        fn fails(error: ApiError) -> Self {
            Self {
                outcomes: Mutex::new(VecDeque::from([Err(error)])),
            }
        }
    }

    impl SecureReadyEvaluator for MockSecureReadyEvaluator {
        fn reach_secure_ready(
            &self,
            transport: Box<dyn FramedDuplex>,
        ) -> crate::BoxFuture<'_, ApiResult<SecureReadyTransport>> {
            let result = self.outcomes.lock().unwrap().pop_front().unwrap_or(Ok(()));
            Box::pin(async move {
                result.map(|()| SecureReadyTransport {
                    transport,
                    artifacts: SecureReadyArtifacts {
                        handshake_hash: Some(vec![0xAA, 0xBB]),
                        channel_binding: Some(vec![0xCC]),
                    },
                })
            })
        }
    }

    struct MockFramedDuplex {
        carrier: CarrierKind,
    }

    impl FramedDuplex for MockFramedDuplex {
        fn carrier(&self) -> CarrierKind {
            self.carrier
        }

        fn send_record<'a>(&'a mut self, _record: &'a [u8]) -> crate::BoxFuture<'a, ApiResult<()>> {
            Box::pin(ready(Ok(())))
        }

        fn receive_record(&mut self) -> crate::BoxFuture<'_, ApiResult<Option<Vec<u8>>>> {
            Box::pin(ready(Ok(None)))
        }

        fn close(&mut self, _directive: CloseDirective) -> crate::BoxFuture<'_, ApiResult<()>> {
            Box::pin(ready(Ok(())))
        }
    }
}
