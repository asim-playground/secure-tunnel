// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::error::{SdkError, SdkResult};
use crate::reports::Carrier;

/// Lifecycle state exposed by the SDK session object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Outer carrier is ready.
    CarrierReady,
    /// Inner Noise handshake is in progress.
    NoiseHandshake,
    /// Inner secure channel is ready for application records.
    SecureReady,
    /// Account session is established.
    AccountAuthenticated,
    /// Account and known-device authentication are established.
    KnownDeviceAuthenticated,
    /// Graceful close is in progress.
    Closing,
    /// The session is closed.
    Closed,
}

impl From<secure_tunnel_core::SessionPhase> for SessionState {
    fn from(value: secure_tunnel_core::SessionPhase) -> Self {
        match value {
            secure_tunnel_core::SessionPhase::CarrierReady => Self::CarrierReady,
            secure_tunnel_core::SessionPhase::NoiseHandshake => Self::NoiseHandshake,
            secure_tunnel_core::SessionPhase::SecureReady => Self::SecureReady,
            secure_tunnel_core::SessionPhase::AccountAuthenticated => Self::AccountAuthenticated,
            secure_tunnel_core::SessionPhase::KnownDeviceAuthenticated => {
                Self::KnownDeviceAuthenticated
            }
            secure_tunnel_core::SessionPhase::Closing => Self::Closing,
        }
    }
}

/// Report returned after a close request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseReport {
    /// Final session state observed by the SDK.
    pub final_state: SessionState,
}

/// Opaque secure tunnel session object.
#[derive(Clone)]
pub struct SecureTunnelSession {
    inner: Arc<Mutex<SessionInner>>,
}

impl std::fmt::Debug for SecureTunnelSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SecureTunnelSession")
            .field("state", &self.state())
            .finish_non_exhaustive()
    }
}

impl SecureTunnelSession {
    pub(super) fn from_selected(selected: secure_tunnel_core::SelectedTransport) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SessionInner {
                state: SessionState::SecureReady,
                selected_carrier: selected.report.carrier.into(),
                transport: Some(selected.transport),
            })),
        }
    }

    /// Returns the current session state.
    #[must_use]
    pub fn state(&self) -> SessionState {
        self.lock_inner()
            .map_or(SessionState::Closed, |inner| inner.state)
    }

    /// Returns the carrier selected for this session.
    ///
    /// # Errors
    ///
    /// Returns [`SdkErrorKind::Internal`](crate::SdkErrorKind::Internal) if the
    /// session lock is poisoned.
    pub fn selected_carrier(&self) -> SdkResult<Carrier> {
        Ok(self.lock_inner()?.selected_carrier)
    }

    /// Sends one application record over the secure channel.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError`] when the session is closed, another operation owns
    /// the transport, or the underlying framed transport fails.
    pub async fn send(&self, payload: Vec<u8>) -> SdkResult<()> {
        let mut lease = self.lease_transport(None)?;
        let result = lease
            .transport_mut()?
            .send_record(&payload)
            .await
            .map_err(|error| SdkError::from_core(&error));
        lease.restore()?;
        result
    }

    /// Receives one application record from the secure channel.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError`] when the session is closed, another operation owns
    /// the transport, or the underlying framed transport fails.
    pub async fn receive(&self) -> SdkResult<Option<Vec<u8>>> {
        let mut lease = self.lease_transport(None)?;
        let result = lease
            .transport_mut()?
            .receive_record()
            .await
            .map_err(|error| SdkError::from_core(&error));
        lease.restore()?;
        result
    }

    /// Sends one request record and waits for one response record.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError`] when send or receive fails.
    pub async fn request(&self, payload: Vec<u8>) -> SdkResult<Option<Vec<u8>>> {
        self.send(payload).await?;
        self.receive().await
    }

    /// Requests graceful encrypted close and marks the session closed.
    ///
    /// # Errors
    ///
    /// Returns [`SdkError`] when the session is already closed, another
    /// operation owns the transport, or the underlying framed transport fails.
    pub async fn close(&self, code: u16, drain: bool) -> SdkResult<CloseReport> {
        let mut lease = self.lease_transport(Some(SessionState::Closing))?;
        let result = lease
            .transport_mut()?
            .close(secure_tunnel_core::CloseDirective { code, drain })
            .await
            .map_err(|error| SdkError::from_core(&error));
        match result {
            Ok(()) => lease.finish_closed(),
            Err(error) => {
                lease.restore()?;
                Err(error)
            }
        }
    }

    fn lease_transport(
        &self,
        transient_state: Option<SessionState>,
    ) -> SdkResult<TransportLease<'_>> {
        let mut inner = self.lock_inner()?;
        if matches!(inner.state, SessionState::Closed | SessionState::Closing) {
            return Err(SdkError::closed());
        }
        let restore_state = inner.state;
        let transport = inner
            .transport
            .take()
            .ok_or_else(|| SdkError::internal("session operation is already in progress"))?;
        if let Some(state) = transient_state {
            inner.state = state;
        }
        drop(inner);
        Ok(TransportLease {
            session: self,
            restore_state,
            transport: Some(transport),
        })
    }

    fn restore_transport(
        &self,
        transport: Box<dyn secure_tunnel_core::FramedDuplex>,
        state: SessionState,
    ) -> SdkResult<()> {
        let mut inner = self.lock_inner()?;
        if inner.state != SessionState::Closed {
            inner.state = state;
            inner.transport = Some(transport);
        }
        drop(inner);
        Ok(())
    }

    fn mark_closed(&self) -> SdkResult<CloseReport> {
        let mut inner = self.lock_inner()?;
        inner.state = SessionState::Closed;
        inner.transport = None;
        drop(inner);
        Ok(CloseReport {
            final_state: SessionState::Closed,
        })
    }

    fn lock_inner(&self) -> SdkResult<std::sync::MutexGuard<'_, SessionInner>> {
        self.inner
            .lock()
            .map_err(|_| SdkError::internal("session lock is poisoned"))
    }
}

struct SessionInner {
    state: SessionState,
    selected_carrier: Carrier,
    transport: Option<Box<dyn secure_tunnel_core::FramedDuplex>>,
}

struct TransportLease<'a> {
    session: &'a SecureTunnelSession,
    restore_state: SessionState,
    transport: Option<Box<dyn secure_tunnel_core::FramedDuplex>>,
}

impl TransportLease<'_> {
    fn transport_mut(&mut self) -> SdkResult<&mut (dyn secure_tunnel_core::FramedDuplex + '_)> {
        let Some(transport) = self.transport.as_mut() else {
            return Err(SdkError::internal("session transport lease is empty"));
        };
        Ok(transport.as_mut())
    }

    fn restore(mut self) -> SdkResult<()> {
        let transport = self
            .transport
            .take()
            .ok_or_else(|| SdkError::internal("session transport lease is empty"))?;
        self.session
            .restore_transport(transport, self.restore_state)
    }

    fn finish_closed(mut self) -> SdkResult<CloseReport> {
        self.transport = None;
        self.session.mark_closed()
    }
}

impl Drop for TransportLease<'_> {
    fn drop(&mut self) {
        let Some(transport) = self.transport.take() else {
            return;
        };
        let Ok(mut inner) = self.session.inner.lock() else {
            return;
        };
        if inner.state != SessionState::Closed {
            inner.state = self.restore_state;
            inner.transport = Some(transport);
        }
    }
}
