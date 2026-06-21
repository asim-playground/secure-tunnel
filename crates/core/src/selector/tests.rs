// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::collections::VecDeque;
use std::future::{Future, ready};
use std::sync::Mutex;

use super::{
    SecureReadyEvaluator, TransportAttemptOutcome, TransportConnectors, TransportSelector,
};
use crate::{
    ApiError, ApiResult, CarrierKind, CloseDirective, FallbackReason, FramedDuplex,
    SecureReadyArtifacts, SecureReadyTransport, ServiceDescriptor, TransportCacheSnapshot,
    TransportTarget, example_service_descriptor,
};

const VALID_NOW: u64 = 1_742_000_000;

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
    assert_eq!(selected.cache_snapshot.highest_descriptor_serial, Some(1));
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
    assert_eq!(selected.cache_snapshot.highest_descriptor_serial, Some(1));
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
        next_quic_probe_after_unix_seconds: Some(VALID_NOW + 1),
        highest_descriptor_serial: Some(1),
    };
    let quic = MockConnector::succeeds(CarrierKind::Quic);
    let wss = MockConnector::succeeds(CarrierKind::Wss);
    let evaluator = MockSecureReadyEvaluator::success();

    let selected = block_on(TransportSelector::new(300).select(
        &descriptor,
        Some(&cache),
        VALID_NOW,
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
        Some(VALID_NOW + 1)
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
        next_quic_probe_after_unix_seconds: Some(VALID_NOW),
        highest_descriptor_serial: Some(1),
    };
    let quic = MockConnector::succeeds(CarrierKind::Quic);
    let wss = MockConnector::succeeds(CarrierKind::Wss);
    let evaluator = MockSecureReadyEvaluator::success();

    let selected = block_on(TransportSelector::new(300).select(
        &descriptor,
        Some(&cache),
        VALID_NOW,
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

    fn returns_transport(carrier: CarrierKind, returned_transport_carrier: CarrierKind) -> Self {
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
        _descriptor: &ServiceDescriptor,
        _now_unix_seconds: u64,
        transport: Box<dyn FramedDuplex>,
    ) -> crate::BoxFuture<'_, ApiResult<SecureReadyTransport>> {
        let result = self.outcomes.lock().unwrap().pop_front().unwrap_or(Ok(()));
        Box::pin(async move {
            result.map(|()| SecureReadyTransport {
                transport,
                artifacts: SecureReadyArtifacts {
                    handshake_hash: Some(vec![0xAA, 0xBB]),
                    channel_binding: Some(vec![0xCC]),
                    service_static_public_key: Some(vec![0xDD; 32]),
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
