// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::collections::VecDeque;
use std::future::{Future, pending, ready};
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::CancellationHandle;
use crate::ports::TransportPorts;

pub(super) struct MockPorts {
    quic: MockConnector,
    wss: MockConnector,
    secure_ready: MockSecureReadyEvaluator,
}

impl MockPorts {
    pub(super) fn quic_success() -> Self {
        Self {
            quic: MockConnector::succeeds(secure_tunnel_core::CarrierKind::Quic),
            wss: MockConnector::succeeds(secure_tunnel_core::CarrierKind::Wss),
            secure_ready: MockSecureReadyEvaluator::success(),
        }
    }

    pub(super) fn quic_success_with_receives(
        receives: impl IntoIterator<Item = Option<Vec<u8>>>,
    ) -> Self {
        Self {
            quic: MockConnector::succeeds_with_receives(
                secure_tunnel_core::CarrierKind::Quic,
                receives,
            ),
            wss: MockConnector::succeeds(secure_tunnel_core::CarrierKind::Wss),
            secure_ready: MockSecureReadyEvaluator::success(),
        }
    }

    pub(super) fn quic_success_with_pending_send() -> Self {
        Self {
            quic: MockConnector::succeeds_with_pending_send(secure_tunnel_core::CarrierKind::Quic),
            wss: MockConnector::succeeds(secure_tunnel_core::CarrierKind::Wss),
            secure_ready: MockSecureReadyEvaluator::success(),
        }
    }

    pub(super) fn quic_fallback_then_wss_success() -> Self {
        Self {
            quic: MockConnector::fails(
                secure_tunnel_core::CarrierKind::Quic,
                secure_tunnel_core::ApiError::TransportFallback(
                    secure_tunnel_core::FallbackReason::OuterPathFailure,
                ),
            ),
            wss: MockConnector::succeeds(secure_tunnel_core::CarrierKind::Wss),
            secure_ready: MockSecureReadyEvaluator::success(),
        }
    }

    pub(super) fn inner_trust_failure() -> Self {
        Self {
            quic: MockConnector::succeeds(secure_tunnel_core::CarrierKind::Quic),
            wss: MockConnector::succeeds(secure_tunnel_core::CarrierKind::Wss),
            secure_ready: MockSecureReadyEvaluator::fails(
                secure_tunnel_core::ApiError::InnerTrustFailure,
            ),
        }
    }

    pub(super) fn cancel_during_quic(cancellation: CancellationHandle) -> Self {
        Self {
            quic: MockConnector::succeeds_and_cancels(
                secure_tunnel_core::CarrierKind::Quic,
                cancellation,
            ),
            wss: MockConnector::succeeds(secure_tunnel_core::CarrierKind::Wss),
            secure_ready: MockSecureReadyEvaluator::success(),
        }
    }

    pub(super) fn pending_quic() -> Self {
        Self {
            quic: MockConnector::pending(secure_tunnel_core::CarrierKind::Quic),
            wss: MockConnector::succeeds(secure_tunnel_core::CarrierKind::Wss),
            secure_ready: MockSecureReadyEvaluator::success(),
        }
    }

    pub(super) fn pending_quic_secure_ready_then_wss_success() -> Self {
        Self {
            quic: MockConnector::succeeds(secure_tunnel_core::CarrierKind::Quic),
            wss: MockConnector::succeeds(secure_tunnel_core::CarrierKind::Wss),
            secure_ready: MockSecureReadyEvaluator::pending_then_success(),
        }
    }

    pub(super) fn quic_fallback_then_pending_wss() -> Self {
        Self {
            quic: MockConnector::fails(
                secure_tunnel_core::CarrierKind::Quic,
                secure_tunnel_core::ApiError::TransportFallback(
                    secure_tunnel_core::FallbackReason::OuterPathFailure,
                ),
            ),
            wss: MockConnector::pending(secure_tunnel_core::CarrierKind::Wss),
            secure_ready: MockSecureReadyEvaluator::success(),
        }
    }

    pub(super) fn sent_records(&self) -> Vec<Vec<u8>> {
        lock(&self.quic.state.sent).clone()
    }

    pub(super) fn close_count(&self) -> usize {
        lock(&self.quic.state.closes).len()
    }

    pub(super) fn connect_count(&self) -> usize {
        *lock(&self.quic.connects) + *lock(&self.wss.connects)
    }
}

impl TransportPorts for MockPorts {
    fn quic(&self) -> Option<&dyn secure_tunnel_core::CarrierConnector> {
        Some(&self.quic)
    }

    fn wss(&self) -> Option<&dyn secure_tunnel_core::CarrierConnector> {
        Some(&self.wss)
    }

    fn secure_ready(&self) -> &dyn secure_tunnel_core::SecureReadyEvaluator {
        &self.secure_ready
    }
}

struct MockConnector {
    carrier: secure_tunnel_core::CarrierKind,
    outcomes: Mutex<VecDeque<secure_tunnel_core::ApiResult<()>>>,
    state: Arc<MockDuplexState>,
    connects: Arc<Mutex<usize>>,
    cancellation: Option<CancellationHandle>,
    pending_connect: bool,
}

impl MockConnector {
    fn succeeds(carrier: secure_tunnel_core::CarrierKind) -> Self {
        Self::succeeds_with_receives(carrier, [])
    }

    fn succeeds_with_receives(
        carrier: secure_tunnel_core::CarrierKind,
        receives: impl IntoIterator<Item = Option<Vec<u8>>>,
    ) -> Self {
        Self {
            carrier,
            outcomes: Mutex::new(VecDeque::from([Ok(())])),
            state: Arc::new(MockDuplexState::new(receives, 0)),
            connects: Arc::new(Mutex::new(0)),
            cancellation: None,
            pending_connect: false,
        }
    }

    fn succeeds_with_pending_send(carrier: secure_tunnel_core::CarrierKind) -> Self {
        Self {
            carrier,
            outcomes: Mutex::new(VecDeque::from([Ok(())])),
            state: Arc::new(MockDuplexState::new([], 1)),
            connects: Arc::new(Mutex::new(0)),
            cancellation: None,
            pending_connect: false,
        }
    }

    fn succeeds_and_cancels(
        carrier: secure_tunnel_core::CarrierKind,
        cancellation: CancellationHandle,
    ) -> Self {
        Self {
            carrier,
            outcomes: Mutex::new(VecDeque::from([Ok(())])),
            state: Arc::new(MockDuplexState::new([], 0)),
            connects: Arc::new(Mutex::new(0)),
            cancellation: Some(cancellation),
            pending_connect: false,
        }
    }

    fn fails(
        carrier: secure_tunnel_core::CarrierKind,
        error: secure_tunnel_core::ApiError,
    ) -> Self {
        Self {
            carrier,
            outcomes: Mutex::new(VecDeque::from([Err(error)])),
            state: Arc::new(MockDuplexState::new([], 0)),
            connects: Arc::new(Mutex::new(0)),
            cancellation: None,
            pending_connect: false,
        }
    }

    fn pending(carrier: secure_tunnel_core::CarrierKind) -> Self {
        Self {
            carrier,
            outcomes: Mutex::new(VecDeque::new()),
            state: Arc::new(MockDuplexState::new([], 0)),
            connects: Arc::new(Mutex::new(0)),
            cancellation: None,
            pending_connect: true,
        }
    }
}

impl secure_tunnel_core::CarrierConnector for MockConnector {
    fn carrier(&self) -> secure_tunnel_core::CarrierKind {
        self.carrier
    }

    fn connect<'a>(
        &'a self,
        target: &'a secure_tunnel_core::TransportTarget,
    ) -> secure_tunnel_core::BoxFuture<
        'a,
        secure_tunnel_core::ApiResult<Box<dyn secure_tunnel_core::FramedDuplex>>,
    > {
        *lock(&self.connects) += 1;
        let result = lock(&self.outcomes).pop_front().unwrap_or(Ok(()));
        let carrier = target.carrier();
        let state = self.state.clone();
        if let Some(cancellation) = &self.cancellation {
            cancellation.cancel();
        }
        if self.pending_connect {
            return pending_connect();
        }
        Box::pin(async move {
            result.map(|()| {
                Box::new(MockFramedDuplex { carrier, state })
                    as Box<dyn secure_tunnel_core::FramedDuplex>
            })
        })
    }
}

struct MockSecureReadyEvaluator {
    outcomes: Mutex<VecDeque<MockSecureReadyOutcome>>,
}

impl MockSecureReadyEvaluator {
    fn success() -> Self {
        Self {
            outcomes: Mutex::new(VecDeque::from([MockSecureReadyOutcome::Ready(Ok(()))])),
        }
    }

    fn fails(error: secure_tunnel_core::ApiError) -> Self {
        Self {
            outcomes: Mutex::new(VecDeque::from([MockSecureReadyOutcome::Ready(Err(error))])),
        }
    }

    fn pending_then_success() -> Self {
        Self {
            outcomes: Mutex::new(VecDeque::from([
                MockSecureReadyOutcome::Pending,
                MockSecureReadyOutcome::Ready(Ok(())),
            ])),
        }
    }
}

enum MockSecureReadyOutcome {
    Ready(secure_tunnel_core::ApiResult<()>),
    Pending,
}

impl secure_tunnel_core::SecureReadyEvaluator for MockSecureReadyEvaluator {
    fn reach_secure_ready(
        &self,
        _descriptor: &secure_tunnel_core::ServiceDescriptor,
        _now_unix_seconds: u64,
        transport: Box<dyn secure_tunnel_core::FramedDuplex>,
    ) -> secure_tunnel_core::BoxFuture<
        '_,
        secure_tunnel_core::ApiResult<secure_tunnel_core::SecureReadyTransport>,
    > {
        let result = lock(&self.outcomes)
            .pop_front()
            .unwrap_or(MockSecureReadyOutcome::Ready(Ok(())));
        Box::pin(async move {
            match result {
                MockSecureReadyOutcome::Ready(result) => {
                    result.map(|()| secure_tunnel_core::SecureReadyTransport {
                        transport,
                        artifacts: secure_tunnel_core::SecureReadyArtifacts {
                            handshake_hash: Some(vec![0xAA; 32]),
                            channel_binding: Some(vec![0xCC; 32]),
                            service_static_public_key: Some(vec![0xDD; 32]),
                        },
                    })
                }
                MockSecureReadyOutcome::Pending => pending().await,
            }
        })
    }
}

struct MockDuplexState {
    sent: Mutex<Vec<Vec<u8>>>,
    receives: Mutex<VecDeque<Option<Vec<u8>>>>,
    closes: Mutex<Vec<secure_tunnel_core::CloseDirective>>,
    pending_sends: Mutex<usize>,
}

impl MockDuplexState {
    fn new(receives: impl IntoIterator<Item = Option<Vec<u8>>>, pending_sends: usize) -> Self {
        Self {
            sent: Mutex::new(Vec::new()),
            receives: Mutex::new(receives.into_iter().collect()),
            closes: Mutex::new(Vec::new()),
            pending_sends: Mutex::new(pending_sends),
        }
    }
}

struct MockFramedDuplex {
    carrier: secure_tunnel_core::CarrierKind,
    state: Arc<MockDuplexState>,
}

impl secure_tunnel_core::FramedDuplex for MockFramedDuplex {
    fn carrier(&self) -> secure_tunnel_core::CarrierKind {
        self.carrier
    }

    fn send_record<'a>(
        &'a mut self,
        record: &'a [u8],
    ) -> secure_tunnel_core::BoxFuture<'a, secure_tunnel_core::ApiResult<()>> {
        lock(&self.state.sent).push(record.to_vec());
        if take_pending_send(&self.state) {
            return Box::pin(pending_send());
        }
        Box::pin(ready(Ok(())))
    }

    fn receive_record(
        &mut self,
    ) -> secure_tunnel_core::BoxFuture<'_, secure_tunnel_core::ApiResult<Option<Vec<u8>>>> {
        let record = lock(&self.state.receives).pop_front().flatten();
        Box::pin(ready(Ok(record)))
    }

    fn close(
        &mut self,
        directive: secure_tunnel_core::CloseDirective,
    ) -> secure_tunnel_core::BoxFuture<'_, secure_tunnel_core::ApiResult<()>> {
        lock(&self.state.closes).push(directive);
        Box::pin(ready(Ok(())))
    }
}

fn pending_send() -> Pin<Box<dyn Future<Output = secure_tunnel_core::ApiResult<()>> + Send>> {
    Box::pin(pending())
}

fn pending_connect() -> secure_tunnel_core::BoxFuture<
    'static,
    secure_tunnel_core::ApiResult<Box<dyn secure_tunnel_core::FramedDuplex>>,
> {
    Box::pin(pending())
}

fn take_pending_send(state: &MockDuplexState) -> bool {
    let mut pending_sends = lock(&state.pending_sends);
    if *pending_sends == 0 {
        return false;
    }
    *pending_sends -= 1;
    true
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
