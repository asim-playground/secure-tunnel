// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::Notify;

/// Cooperative cancellation handle for long-running connect operations.
///
/// Foreign callers can keep a clone of this handle and call [`Self::cancel`]
/// while an async connect operation is in progress. Session operation futures
/// are safe to drop, but this first facade does not yet take cancellation
/// handles for `send`, `receive`, or `close`.
#[derive(Debug, Clone, Default)]
pub struct CancellationHandle {
    cancelled: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl CancellationHandle {
    /// Creates a cancellation handle in the active state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cooperative cancellation for operations using this handle.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    /// Returns whether cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub(crate) async fn cancelled(&self) {
        while !self.is_cancelled() {
            let notified = self.notify.notified();
            tokio::pin!(notified);
            let _ = notified.as_mut().enable();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}
