// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use crate::convert::{
    account_report, close_report, connect_report, sdk_account_mode, sdk_cache, security_artifacts,
};
use crate::error::{FfiResult, IntoFfiResult, internal_error};
use crate::types::{
    AccountAuthReport, AccountAuthRequest, ClientConfig, ConnectOptions, ConnectReport,
    SessionState,
};
use crate::types_more::{
    CloseReport, DeviceAuthChallenge, DeviceAuthReport, SecureChannelArtifacts, session_state,
};

/// Opaque generated-binding client object.
pub struct SecureTunnelClient {
    client: secure_tunnel_sdk::SecureTunnelClient,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl SecureTunnelClient {
    /// Creates a generated-binding SDK client.
    ///
    /// # Errors
    ///
    /// Returns an error when the configuration is invalid or the internal async
    /// runtime cannot be created.
    pub fn new(config: ClientConfig) -> FfiResult<Self> {
        let client =
            secure_tunnel_sdk::SecureTunnelClient::new(crate::convert::sdk_config(config)?);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|error| internal_error(error.to_string()))?;
        Ok(Self {
            client,
            runtime: Arc::new(runtime),
        })
    }

    /// Connects to the service described by the supplied descriptor.
    ///
    /// # Errors
    ///
    /// Returns an error when descriptor validation, transport selection,
    /// handshake, or service-static-key authorization fails.
    pub fn connect(&self, options: ConnectOptions) -> FfiResult<Arc<SecureTunnelConnection>> {
        let ConnectOptions {
            descriptor_json,
            now_unix_seconds,
            transport_cache,
        } = options;
        let descriptor =
            secure_tunnel_sdk::BootstrapDescriptor::from_json(&descriptor_json).into_ffi()?;
        let mut sdk_options = secure_tunnel_sdk::ConnectOptions::new(descriptor, now_unix_seconds);
        if let Some(cache) = transport_cache.as_ref().map(sdk_cache) {
            sdk_options = sdk_options.with_transport_cache(cache);
        }
        let outcome = self.client.connect(sdk_options);
        let outcome = self.runtime.block_on(outcome).into_ffi()?;
        Ok(Arc::new(SecureTunnelConnection {
            session: outcome.session,
            report: connect_report(&outcome.report),
            artifacts: security_artifacts(&outcome.artifacts),
            runtime: Arc::clone(&self.runtime),
        }))
    }
}

/// Opaque generated-binding connection/session object.
pub struct SecureTunnelConnection {
    session: secure_tunnel_sdk::SecureTunnelSession,
    report: ConnectReport,
    artifacts: SecureChannelArtifacts,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl SecureTunnelConnection {
    /// Returns the connect report.
    #[must_use]
    pub fn report(&self) -> ConnectReport {
        self.report.clone()
    }

    /// Returns explicit channel-binding/security artifacts for this session.
    #[must_use]
    pub fn security_artifacts(&self) -> SecureChannelArtifacts {
        self.artifacts.clone()
    }

    /// Returns the current session state.
    #[must_use]
    pub fn state(&self) -> SessionState {
        session_state(self.session.state())
    }

    /// Authenticates the account session.
    ///
    /// # Errors
    ///
    /// Returns an error when the service rejects the account credentials or the
    /// secure session is no longer usable.
    pub fn authenticate_account(
        &self,
        request: AccountAuthRequest,
    ) -> FfiResult<AccountAuthReport> {
        let sdk_request = secure_tunnel_sdk::AccountAuthRequest {
            account_id: request.account_id,
            credential_payload: request.credential_payload,
            mode: sdk_account_mode(request.mode),
        };
        let future = self.session.authenticate_account(sdk_request);
        self.runtime_block_on(future).map(account_report)
    }

    /// Begins known-device authentication.
    ///
    /// # Errors
    ///
    /// Returns an error when the service rejects the device key id or the secure
    /// session is no longer usable.
    pub fn begin_known_device_auth(&self, device_key_id: String) -> FfiResult<DeviceAuthChallenge> {
        let future = self.session.begin_known_device_auth(device_key_id);
        self.runtime_block_on(future)
            .map(|challenge| DeviceAuthChallenge {
                device_key_id: challenge.device_key_id,
                server_challenge: challenge.server_challenge,
                expires_at_unix_ms: challenge.expires_at_unix_ms,
                canonical_bytes: challenge.canonical_bytes,
            })
    }

    /// Finishes known-device authentication.
    ///
    /// # Errors
    ///
    /// Returns an error when the signature, challenge, expiry, or session state
    /// is invalid.
    pub fn finish_known_device_auth(
        &self,
        challenge: DeviceAuthChallenge,
        signature: Vec<u8>,
        now_unix_ms: u64,
    ) -> FfiResult<DeviceAuthReport> {
        let sdk_challenge = secure_tunnel_sdk::DeviceAuthChallenge {
            device_key_id: challenge.device_key_id,
            server_challenge: challenge.server_challenge,
            expires_at_unix_ms: challenge.expires_at_unix_ms,
            canonical_bytes: challenge.canonical_bytes,
        };
        let future = self
            .session
            .finish_known_device_auth(sdk_challenge, signature, now_unix_ms);
        self.runtime_block_on(future)
            .map(|report| DeviceAuthReport {
                device_key_id: report.device_key_id,
                state: match report.state {
                    secure_tunnel_sdk::DeviceState::Active => crate::types::DeviceState::Active,
                    secure_tunnel_sdk::DeviceState::Pending => crate::types::DeviceState::Pending,
                },
            })
    }

    /// Sends one request and returns the response payload.
    ///
    /// # Errors
    ///
    /// Returns an error when the request cannot be written, the response cannot
    /// be read, or the service closes without an application response.
    pub fn request(&self, payload: Vec<u8>) -> FfiResult<Vec<u8>> {
        let future = self.session.request(payload);
        self.runtime_block_on(future)?
            .ok_or_else(|| internal_error("missing application response"))
    }

    /// Closes the session gracefully.
    ///
    /// # Errors
    ///
    /// Returns an error when the close message cannot be sent or the transport
    /// fails before the close report is produced.
    pub fn close(&self, code: u16, drain: bool) -> FfiResult<CloseReport> {
        let future = self.session.close(code, drain);
        self.runtime_block_on(future)
            .map(|report| close_report(&report))
    }

    fn runtime_block_on<T>(
        &self,
        future: impl std::future::Future<Output = secure_tunnel_sdk::SdkResult<T>>,
    ) -> FfiResult<T> {
        self.runtime.block_on(future).into_ffi()
    }
}
