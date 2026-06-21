// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};

/// Account authentication mode requested by the SDK caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountAuthMode {
    /// Authenticate with fresh account credentials.
    Fresh,
    /// Resume a previously established account session.
    Resume,
}

/// Account freshness established by the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountFreshness {
    /// The account was freshly authenticated with current credentials.
    Fresh,
    /// The account session was resumed from opaque session material.
    Resumed,
}

/// Account authentication request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountAuthRequest {
    /// Product account identifier.
    pub account_id: String,
    /// Opaque credential or resume payload.
    pub credential_payload: Vec<u8>,
    /// Requested account authentication mode.
    pub mode: AccountAuthMode,
}

/// Account authentication report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountAuthReport {
    /// Product account identifier accepted by the service.
    pub account_id: String,
    /// Server-side account session context identifier.
    pub session_context_id: String,
    /// Stable account context hash bound into later device proof bytes.
    pub account_context_hash: Vec<u8>,
    /// Established account freshness.
    pub freshness: AccountFreshness,
}

/// Known-device authentication challenge returned by the service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceAuthChallenge {
    /// Device key identifier being authenticated.
    pub device_key_id: String,
    /// Server challenge bytes.
    pub server_challenge: Vec<u8>,
    /// Challenge expiry in Unix milliseconds.
    pub expires_at_unix_ms: u64,
    /// Canonical bytes the caller must sign with the device key.
    pub canonical_bytes: Vec<u8>,
}

/// Known-device authentication report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceAuthReport {
    /// Device key identifier accepted by the service.
    pub device_key_id: String,
    /// Device state after authentication.
    pub state: DeviceState,
}

/// New-device enrollment challenge returned by the service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceEnrollmentChallenge {
    /// Candidate device key identifier.
    pub device_key_id: String,
    /// Candidate device public key supplied by the caller.
    pub device_public_key: Vec<u8>,
    /// Server challenge bytes.
    pub server_challenge: Vec<u8>,
    /// Challenge expiry in Unix milliseconds.
    pub expires_at_unix_ms: u64,
    /// Canonical bytes the caller must sign with the candidate device key.
    pub canonical_bytes: Vec<u8>,
}

/// New-device enrollment report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceEnrollmentReport {
    /// Device key identifier accepted by the service.
    pub device_key_id: String,
    /// Device state after enrollment.
    pub state: DeviceState,
}

/// Device state exposed by the SDK.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceState {
    /// Device is active and can use the tunnel.
    Active,
    /// Device was accepted but still needs product-side approval.
    Pending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AccountSessionContext {
    pub(crate) account_id: String,
    pub(crate) session_context_id: String,
    pub(crate) account_context_hash: [u8; 32],
    pub(crate) freshness: AccountFreshness,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeviceSessionContext {
    pub(crate) device_key_id: String,
    pub(crate) state: DeviceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingDeviceChallenge {
    pub(crate) device_key_id: String,
    pub(crate) server_challenge: [u8; 32],
    pub(crate) expires_at_unix_ms: u64,
    pub(crate) purpose: secure_tunnel_core::DeviceProofPurpose,
    pub(crate) candidate_device_public_key: Option<[u8; 32]>,
}

impl AccountSessionContext {
    pub(crate) fn report(&self) -> AccountAuthReport {
        AccountAuthReport {
            account_id: self.account_id.clone(),
            session_context_id: self.session_context_id.clone(),
            account_context_hash: self.account_context_hash.to_vec(),
            freshness: self.freshness,
        }
    }
}

impl DeviceSessionContext {
    pub(crate) fn auth_report(&self) -> DeviceAuthReport {
        DeviceAuthReport {
            device_key_id: self.device_key_id.clone(),
            state: self.state,
        }
    }

    pub(crate) fn enrollment_report(&self) -> DeviceEnrollmentReport {
        DeviceEnrollmentReport {
            device_key_id: self.device_key_id.clone(),
            state: self.state,
        }
    }
}

impl From<secure_tunnel_core::AccountAuthMode> for AccountAuthMode {
    fn from(value: secure_tunnel_core::AccountAuthMode) -> Self {
        match value {
            secure_tunnel_core::AccountAuthMode::Fresh => Self::Fresh,
            secure_tunnel_core::AccountAuthMode::Resume => Self::Resume,
        }
    }
}

impl From<AccountAuthMode> for secure_tunnel_core::AccountAuthMode {
    fn from(value: AccountAuthMode) -> Self {
        match value {
            AccountAuthMode::Fresh => Self::Fresh,
            AccountAuthMode::Resume => Self::Resume,
        }
    }
}

impl From<secure_tunnel_core::AccountFreshness> for AccountFreshness {
    fn from(value: secure_tunnel_core::AccountFreshness) -> Self {
        match value {
            secure_tunnel_core::AccountFreshness::Fresh => Self::Fresh,
            secure_tunnel_core::AccountFreshness::Resumed => Self::Resumed,
        }
    }
}

impl From<AccountFreshness> for secure_tunnel_core::AccountFreshness {
    fn from(value: AccountFreshness) -> Self {
        match value {
            AccountFreshness::Fresh => Self::Fresh,
            AccountFreshness::Resumed => Self::Resumed,
        }
    }
}

impl From<secure_tunnel_core::DeviceState> for DeviceState {
    fn from(value: secure_tunnel_core::DeviceState) -> Self {
        match value {
            secure_tunnel_core::DeviceState::Active => Self::Active,
            secure_tunnel_core::DeviceState::Pending => Self::Pending,
        }
    }
}

impl From<DeviceState> for secure_tunnel_core::DeviceState {
    fn from(value: DeviceState) -> Self {
        match value {
            DeviceState::Active => Self::Active,
            DeviceState::Pending => Self::Pending,
        }
    }
}

impl From<secure_tunnel_core::AccountAuthResult> for AccountSessionContext {
    fn from(value: secure_tunnel_core::AccountAuthResult) -> Self {
        Self {
            account_id: value.account_id,
            session_context_id: value.session_context_id,
            account_context_hash: value.account_context_hash,
            freshness: value.freshness.into(),
        }
    }
}

impl From<secure_tunnel_core::DeviceResult> for DeviceSessionContext {
    fn from(value: secure_tunnel_core::DeviceResult) -> Self {
        Self {
            device_key_id: value.device_key_id,
            state: value.state.into(),
        }
    }
}
