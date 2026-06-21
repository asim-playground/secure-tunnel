// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use crate::types::TransportAttemptReport;

/// Error object exposed through generated foreign-language bindings.
#[derive(Debug, thiserror::Error)]
#[error("{kind}: {message}")]
pub struct SecureTunnelError {
    kind: String,
    message: String,
    attempts: Vec<TransportAttemptReport>,
}

impl SecureTunnelError {
    pub(crate) fn new(kind: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
            attempts: Vec::new(),
        }
    }

    pub(crate) fn with_attempts(
        kind: impl Into<String>,
        message: impl Into<String>,
        attempts: Vec<TransportAttemptReport>,
    ) -> Self {
        Self {
            kind: kind.into(),
            message: message.into(),
            attempts,
        }
    }

    /// Returns the stable SDK error class.
    #[must_use]
    pub fn kind(&self) -> String {
        self.kind.clone()
    }

    /// Returns the human-readable diagnostic message.
    #[must_use]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    /// Returns the connect attempt trace attached to this error, if available.
    #[must_use]
    pub fn attempts(&self) -> Vec<TransportAttemptReport> {
        self.attempts.clone()
    }
}

pub type FfiResult<T> = Result<T, Arc<SecureTunnelError>>;

pub trait IntoFfiResult<T> {
    fn into_ffi(self) -> FfiResult<T>;
}

impl<T> IntoFfiResult<T> for secure_tunnel_sdk::SdkResult<T> {
    fn into_ffi(self) -> FfiResult<T> {
        self.map_err(|error| Arc::new(error_from_sdk(&error)))
    }
}

impl<T> IntoFfiResult<T> for secure_tunnel_sdk::ConnectResult<T> {
    fn into_ffi(self) -> FfiResult<T> {
        self.map_err(|error| {
            Arc::new(SecureTunnelError::with_attempts(
                format!("{:?}", error.kind()),
                error.message(),
                error
                    .attempts
                    .iter()
                    .map(crate::convert::attempt_report)
                    .collect(),
            ))
        })
    }
}

pub fn internal_error(message: impl Into<String>) -> Arc<SecureTunnelError> {
    Arc::new(SecureTunnelError::new("Internal", message))
}

pub fn invalid_config(message: impl Into<String>) -> Arc<SecureTunnelError> {
    Arc::new(SecureTunnelError::new("InvalidConfig", message))
}

fn error_from_sdk(error: &secure_tunnel_sdk::SdkError) -> SecureTunnelError {
    SecureTunnelError::new(format!("{:?}", error.kind()), error.message())
}
