// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::future::Future;
use std::time::Duration;

use tokio::time::{Instant, sleep, sleep_until};

use crate::cancellation::CancellationHandle;

#[derive(Clone, Copy)]
pub(crate) struct ConnectBudget<'a> {
    cancellation: Option<&'a CancellationHandle>,
    deadline: Instant,
}

impl<'a> ConnectBudget<'a> {
    pub(crate) fn new(
        connect_timeout: Duration,
        cancellation: Option<&'a CancellationHandle>,
    ) -> Self {
        Self {
            cancellation,
            deadline: Instant::now() + connect_timeout,
        }
    }

    pub(crate) const fn guard_connector(
        self,
        inner: &'a dyn secure_tunnel_core::CarrierConnector,
    ) -> DeadlineCarrierConnector<'a> {
        DeadlineCarrierConnector {
            inner,
            budget: self,
        }
    }

    pub(crate) const fn guard_secure_ready(
        self,
        inner: &'a dyn secure_tunnel_core::SecureReadyEvaluator,
        secure_ready_timeout: Duration,
    ) -> DeadlineSecureReadyEvaluator<'a> {
        DeadlineSecureReadyEvaluator {
            inner,
            budget: self,
            secure_ready_timeout,
        }
    }
}

pub(crate) struct DeadlineCarrierConnector<'a> {
    inner: &'a dyn secure_tunnel_core::CarrierConnector,
    budget: ConnectBudget<'a>,
}

impl secure_tunnel_core::CarrierConnector for DeadlineCarrierConnector<'_> {
    fn carrier(&self) -> secure_tunnel_core::CarrierKind {
        self.inner.carrier()
    }

    fn connect<'a>(
        &'a self,
        target: &'a secure_tunnel_core::TransportTarget,
    ) -> secure_tunnel_core::BoxFuture<
        'a,
        secure_tunnel_core::ApiResult<Box<dyn secure_tunnel_core::FramedDuplex>>,
    > {
        let carrier = target.carrier();
        Box::pin(async move {
            await_connect_budget(
                self.inner.connect(target),
                self.budget,
                deadline_timeout_error(carrier),
            )
            .await
        })
    }
}

pub(crate) struct DeadlineSecureReadyEvaluator<'a> {
    inner: &'a dyn secure_tunnel_core::SecureReadyEvaluator,
    budget: ConnectBudget<'a>,
    secure_ready_timeout: Duration,
}

impl secure_tunnel_core::SecureReadyEvaluator for DeadlineSecureReadyEvaluator<'_> {
    fn reach_secure_ready(
        &self,
        descriptor: &secure_tunnel_core::ServiceDescriptor,
        now_unix_seconds: u64,
        transport: Box<dyn secure_tunnel_core::FramedDuplex>,
    ) -> secure_tunnel_core::BoxFuture<
        '_,
        secure_tunnel_core::ApiResult<secure_tunnel_core::SecureReadyTransport>,
    > {
        let descriptor = descriptor.clone();
        Box::pin(async move {
            let carrier = transport.carrier();
            await_secure_ready_budget(
                self.inner
                    .reach_secure_ready(&descriptor, now_unix_seconds, transport),
                self.budget,
                self.secure_ready_timeout,
                carrier,
            )
            .await
        })
    }
}

async fn await_connect_budget<T>(
    operation: impl Future<Output = secure_tunnel_core::ApiResult<T>> + Send,
    budget: ConnectBudget<'_>,
    timeout_error: secure_tunnel_core::ApiError,
) -> secure_tunnel_core::ApiResult<T> {
    match budget.cancellation {
        Some(cancellation) => {
            tokio::select! {
                result = operation => result,
                () = cancellation.cancelled() => Err(secure_tunnel_core::ApiError::OperationCancelled),
                () = sleep_until(budget.deadline) => Err(timeout_error),
            }
        }
        None => {
            tokio::select! {
                result = operation => result,
                () = sleep_until(budget.deadline) => Err(timeout_error),
            }
        }
    }
}

async fn await_secure_ready_budget<T>(
    operation: impl Future<Output = secure_tunnel_core::ApiResult<T>> + Send,
    budget: ConnectBudget<'_>,
    secure_ready_timeout: Duration,
    carrier: secure_tunnel_core::CarrierKind,
) -> secure_tunnel_core::ApiResult<T> {
    match budget.cancellation {
        Some(cancellation) => {
            tokio::select! {
                result = operation => result,
                () = cancellation.cancelled() => Err(secure_tunnel_core::ApiError::OperationCancelled),
                () = sleep_until(budget.deadline) => Err(deadline_timeout_error(carrier)),
                () = sleep(secure_ready_timeout) => Err(secure_ready_timeout_error(carrier)),
            }
        }
        None => {
            tokio::select! {
                result = operation => result,
                () = sleep_until(budget.deadline) => Err(deadline_timeout_error(carrier)),
                () = sleep(secure_ready_timeout) => Err(secure_ready_timeout_error(carrier)),
            }
        }
    }
}

const fn deadline_timeout_error(
    carrier: secure_tunnel_core::CarrierKind,
) -> secure_tunnel_core::ApiError {
    secure_tunnel_core::ApiError::OuterPathFailure(carrier)
}

const fn secure_ready_timeout_error(
    carrier: secure_tunnel_core::CarrierKind,
) -> secure_tunnel_core::ApiError {
    match carrier {
        secure_tunnel_core::CarrierKind::Quic => secure_tunnel_core::ApiError::TransportFallback(
            secure_tunnel_core::FallbackReason::OuterQuicClosedEarly,
        ),
        secure_tunnel_core::CarrierKind::Wss => {
            secure_tunnel_core::ApiError::OuterPathFailure(secure_tunnel_core::CarrierKind::Wss)
        }
    }
}
