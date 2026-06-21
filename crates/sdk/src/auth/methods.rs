// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use super::{
    AccountAuthReport, AccountAuthRequest, AccountFreshness, AccountSessionContext,
    DeviceAuthChallenge, DeviceAuthReport, DeviceEnrollmentChallenge, DeviceEnrollmentReport,
    DeviceSessionContext, PendingDeviceChallenge,
};
use crate::error::{SdkError, SdkResult};
use crate::session::{SecureTunnelSession, SessionState};

impl SecureTunnelSession {
    /// Authenticates or resumes an account over the encrypted tunnel.
    ///
    /// # Errors
    ///
    /// Returns [`SdkErrorKind::AuthFailure`](crate::SdkErrorKind::AuthFailure)
    /// when the service rejects or malforms the account response.
    pub async fn authenticate_account(
        &self,
        request: AccountAuthRequest,
    ) -> SdkResult<AccountAuthReport> {
        ensure_state(self, SessionState::SecureReady)?;
        let core_request = secure_tunnel_core::AccountAuthRequest {
            account_id: request.account_id,
            credential_payload: request.credential_payload,
            mode: request.mode.into(),
        };
        let mut lease = self.lease_transport(None)?;
        let result = send_request_decode(&mut lease, core_request.encode(), |record| {
            secure_tunnel_core::AccountAuthResult::decode(record)
        })
        .await
        .map(AccountSessionContext::from);

        match result {
            Ok(account) => {
                let report = account.report();
                lease.restore_with_auth_state(
                    SessionState::AccountAuthenticated,
                    Some(account),
                    None,
                    None,
                )?;
                Ok(report)
            }
            Err(error) => {
                lease.restore()?;
                Err(error)
            }
        }
    }

    /// Begins known-device authentication and returns bytes to sign.
    ///
    /// # Errors
    ///
    /// Returns auth failure when no account is authenticated or the service
    /// returns a malformed challenge.
    pub async fn begin_known_device_auth(
        &self,
        device_key_id: String,
    ) -> SdkResult<DeviceAuthChallenge> {
        let proof_base = self.proof_base(
            &device_key_id,
            secure_tunnel_core::DeviceProofPurpose::KnownDeviceReauth,
        )?;
        let start = secure_tunnel_core::DeviceAuthStart {
            device_key_id: device_key_id.clone(),
        };
        let mut lease = self.lease_transport(None)?;
        let result = send_request_decode(&mut lease, start.encode(), |record| {
            secure_tunnel_core::DeviceChallenge::decode_auth(record)
        })
        .await
        .and_then(|challenge| {
            let canonical_bytes = proof_base.canonical_bytes(&challenge)?;
            Ok((challenge, canonical_bytes))
        });

        match result {
            Ok((challenge, canonical_bytes)) => {
                let pending = PendingDeviceChallenge {
                    device_key_id: device_key_id.clone(),
                    server_challenge: challenge.server_challenge,
                    expires_at_unix_ms: challenge.expires_at_unix_ms,
                    purpose: secure_tunnel_core::DeviceProofPurpose::KnownDeviceReauth,
                    candidate_device_public_key: None,
                };
                let account = self.current_account()?;
                let device = self.current_device();
                lease.restore_with_auth_state(
                    SessionState::AccountAuthenticated,
                    Some(account),
                    device,
                    Some(pending),
                )?;
                Ok(DeviceAuthChallenge {
                    device_key_id,
                    server_challenge: challenge.server_challenge.to_vec(),
                    expires_at_unix_ms: challenge.expires_at_unix_ms,
                    canonical_bytes,
                })
            }
            Err(error) => {
                lease.restore()?;
                Err(error)
            }
        }
    }

    /// Finishes known-device authentication with a caller-provided signature.
    ///
    /// # Errors
    ///
    /// Returns auth failure when the challenge is stale, replayed, mismatched,
    /// or the service rejects the proof.
    pub async fn finish_known_device_auth(
        &self,
        challenge: DeviceAuthChallenge,
        signature: Vec<u8>,
        now_unix_ms: u64,
    ) -> SdkResult<DeviceAuthReport> {
        let pending = self.take_matching_pending(
            &challenge.device_key_id,
            &challenge.server_challenge,
            challenge.expires_at_unix_ms,
            secure_tunnel_core::DeviceProofPurpose::KnownDeviceReauth,
            now_unix_ms,
        )?;
        let expected_device_key_id = pending.device_key_id.clone();
        let finish = secure_tunnel_core::DeviceProofFinish {
            device_key_id: pending.device_key_id,
            server_challenge: pending.server_challenge,
            expires_at_unix_ms: pending.expires_at_unix_ms,
            signature: signature_64(signature)?,
            candidate_device_public_key: None,
        };
        let mut lease = self.lease_transport(None)?;
        let result = send_request_decode(&mut lease, finish.encode_auth(), |record| {
            secure_tunnel_core::DeviceResult::decode_auth(record)
        })
        .await
        .and_then(|result| expected_device_result(result, &expected_device_key_id))
        .map(DeviceSessionContext::from);

        match result {
            Ok(device) => {
                let report = device.auth_report();
                let account = self.current_account()?;
                lease.restore_with_auth_state(
                    SessionState::KnownDeviceAuthenticated,
                    Some(account),
                    Some(device),
                    None,
                )?;
                Ok(report)
            }
            Err(error) => {
                lease.restore()?;
                Err(error)
            }
        }
    }

    /// Begins new-device enrollment and returns bytes to sign.
    ///
    /// # Errors
    ///
    /// Returns auth failure when the account is not freshly authenticated or the
    /// service returns a malformed challenge.
    pub async fn begin_device_enrollment(
        &self,
        device_key_id: String,
        device_public_key: Vec<u8>,
    ) -> SdkResult<DeviceEnrollmentChallenge> {
        let device_public_key = hash32_vec(device_public_key)?;
        self.ensure_fresh_account()?;
        let proof_base = self.proof_base(
            &device_key_id,
            secure_tunnel_core::DeviceProofPurpose::NewDeviceEnrollment,
        )?;
        let start = secure_tunnel_core::DeviceEnrollmentStart {
            device_key_id: device_key_id.clone(),
            device_public_key,
        };
        let mut lease = self.lease_transport(None)?;
        let result = send_request_decode(&mut lease, start.encode(), |record| {
            secure_tunnel_core::DeviceChallenge::decode_enrollment(record)
        })
        .await
        .and_then(|challenge| {
            let canonical_bytes = proof_base.canonical_bytes(&challenge)?;
            Ok((challenge, canonical_bytes))
        });

        match result {
            Ok((challenge, canonical_bytes)) => {
                let pending = PendingDeviceChallenge {
                    device_key_id: device_key_id.clone(),
                    server_challenge: challenge.server_challenge,
                    expires_at_unix_ms: challenge.expires_at_unix_ms,
                    purpose: secure_tunnel_core::DeviceProofPurpose::NewDeviceEnrollment,
                    candidate_device_public_key: Some(device_public_key),
                };
                let account = self.current_account()?;
                let device = self.current_device();
                lease.restore_with_auth_state(
                    SessionState::AccountAuthenticated,
                    Some(account),
                    device,
                    Some(pending),
                )?;
                Ok(DeviceEnrollmentChallenge {
                    device_key_id,
                    device_public_key: device_public_key.to_vec(),
                    server_challenge: challenge.server_challenge.to_vec(),
                    expires_at_unix_ms: challenge.expires_at_unix_ms,
                    canonical_bytes,
                })
            }
            Err(error) => {
                lease.restore()?;
                Err(error)
            }
        }
    }

    /// Finishes new-device enrollment with a caller-provided signature.
    ///
    /// # Errors
    ///
    /// Returns auth failure when the challenge is stale, replayed, mismatched,
    /// or the service rejects the proof.
    pub async fn finish_device_enrollment(
        &self,
        challenge: DeviceEnrollmentChallenge,
        signature: Vec<u8>,
        now_unix_ms: u64,
    ) -> SdkResult<DeviceEnrollmentReport> {
        let pending = self.take_matching_pending(
            &challenge.device_key_id,
            &challenge.server_challenge,
            challenge.expires_at_unix_ms,
            secure_tunnel_core::DeviceProofPurpose::NewDeviceEnrollment,
            now_unix_ms,
        )?;
        let expected_device_key_id = pending.device_key_id.clone();
        let finish = secure_tunnel_core::DeviceProofFinish {
            device_key_id: pending.device_key_id,
            server_challenge: pending.server_challenge,
            expires_at_unix_ms: pending.expires_at_unix_ms,
            signature: signature_64(signature)?,
            candidate_device_public_key: pending.candidate_device_public_key,
        };
        let mut lease = self.lease_transport(None)?;
        let result = send_request_decode(&mut lease, finish.encode_enrollment(), |record| {
            secure_tunnel_core::DeviceResult::decode_enrollment(record)
        })
        .await
        .and_then(|result| expected_device_result(result, &expected_device_key_id))
        .map(DeviceSessionContext::from);

        match result {
            Ok(device) => {
                let report = device.enrollment_report();
                let account = self.current_account()?;
                lease.restore_with_auth_state(
                    SessionState::KnownDeviceAuthenticated,
                    Some(account),
                    Some(device),
                    None,
                )?;
                Ok(report)
            }
            Err(error) => {
                lease.restore()?;
                Err(error)
            }
        }
    }

    fn proof_base(
        &self,
        device_key_id: &str,
        purpose: secure_tunnel_core::DeviceProofPurpose,
    ) -> SdkResult<ProofBase> {
        let inner = self.lock_inner()?;
        let account = inner.account.clone().ok_or_else(auth_failure)?;
        let handshake_hash = hash32_option(inner.artifacts.handshake_hash.as_ref())?;
        let context = inner
            .descriptor
            .core_descriptor()
            .inner_channel_context()
            .map_err(|error| SdkError::from_core(&error))?;
        drop(inner);
        Ok(ProofBase {
            noise_handshake_hash: handshake_hash,
            context,
            account_context_hash: account.account_context_hash,
            device_key_id: device_key_id.to_owned(),
            purpose,
        })
    }

    fn ensure_fresh_account(&self) -> SdkResult<()> {
        let account = self.current_account()?;
        if account.freshness != AccountFreshness::Fresh {
            return Err(auth_failure());
        }
        Ok(())
    }

    fn current_account(&self) -> SdkResult<AccountSessionContext> {
        self.lock_inner()?.account.clone().ok_or_else(auth_failure)
    }

    fn current_device(&self) -> Option<DeviceSessionContext> {
        self.lock_inner()
            .ok()
            .and_then(|inner| inner.device.clone())
    }

    fn take_matching_pending(
        &self,
        device_key_id: &str,
        server_challenge: &[u8],
        expires_at_unix_ms: u64,
        purpose: secure_tunnel_core::DeviceProofPurpose,
        now_unix_ms: u64,
    ) -> SdkResult<PendingDeviceChallenge> {
        let challenge = secure_tunnel_core::DeviceChallenge {
            server_challenge: hash32_slice(server_challenge)?,
            expires_at_unix_ms,
        };
        challenge
            .ensure_fresh(now_unix_ms)
            .map_err(|error| SdkError::from_core(&error))?;
        let mut inner = self.lock_inner()?;
        let pending = inner
            .pending_device_challenge
            .take()
            .ok_or_else(auth_failure)?;
        drop(inner);
        if pending.device_key_id != device_key_id
            || pending.server_challenge != challenge.server_challenge
            || pending.expires_at_unix_ms != expires_at_unix_ms
            || pending.purpose != purpose
        {
            return Err(auth_failure());
        }
        Ok(pending)
    }
}

struct ProofBase {
    noise_handshake_hash: [u8; 32],
    context: secure_tunnel_core::InnerChannelContext,
    account_context_hash: [u8; 32],
    device_key_id: String,
    purpose: secure_tunnel_core::DeviceProofPurpose,
}

impl ProofBase {
    fn canonical_bytes(
        &self,
        challenge: &secure_tunnel_core::DeviceChallenge,
    ) -> SdkResult<Vec<u8>> {
        secure_tunnel_core::DeviceProofInput {
            noise_handshake_hash: self.noise_handshake_hash,
            server_challenge: challenge.server_challenge,
            context: self.context.clone(),
            account_context_hash: self.account_context_hash,
            device_key_id: self.device_key_id.clone(),
            purpose: self.purpose,
            expires_at_unix_ms: challenge.expires_at_unix_ms,
        }
        .canonical_bytes()
        .map_err(|error| SdkError::from_core(&error))
    }
}

async fn send_request_decode<T>(
    lease: &mut crate::session::TransportLease<'_>,
    encoded_request: secure_tunnel_core::ApiResult<Vec<u8>>,
    decode: impl FnOnce(&[u8]) -> secure_tunnel_core::ApiResult<T>,
) -> SdkResult<T> {
    let request = encoded_request.map_err(|error| SdkError::from_core(&error))?;
    lease
        .transport_mut()?
        .send_record(&request)
        .await
        .map_err(|error| SdkError::from_core(&error))?;
    let response = lease
        .transport_mut()?
        .receive_record()
        .await
        .map_err(|error| SdkError::from_core(&error))?
        .ok_or_else(auth_failure)?;
    decode(&response).map_err(|error| SdkError::from_core(&error))
}

fn ensure_state(session: &SecureTunnelSession, expected: SessionState) -> SdkResult<()> {
    if session.state() != expected {
        return Err(auth_failure());
    }
    Ok(())
}

fn expected_device_result(
    result: secure_tunnel_core::DeviceResult,
    expected_device_key_id: &str,
) -> SdkResult<secure_tunnel_core::DeviceResult> {
    if result.device_key_id == expected_device_key_id {
        Ok(result)
    } else {
        Err(auth_failure())
    }
}

fn signature_64(value: Vec<u8>) -> SdkResult<[u8; 64]> {
    value.try_into().map_err(|_| auth_failure())
}

fn hash32_vec(value: Vec<u8>) -> SdkResult<[u8; 32]> {
    value.try_into().map_err(|_| auth_failure())
}

fn hash32_slice(value: &[u8]) -> SdkResult<[u8; 32]> {
    value.try_into().map_err(|_| auth_failure())
}

fn hash32_option(value: Option<&Vec<u8>>) -> SdkResult<[u8; 32]> {
    value
        .and_then(|bytes| bytes.as_slice().try_into().ok())
        .ok_or_else(auth_failure)
}

fn auth_failure() -> SdkError {
    SdkError::from_core(&secure_tunnel_core::ApiError::PostHandshakeAuthFailure)
}
