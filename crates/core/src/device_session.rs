// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use bytes::BufMut;
use ed25519_dalek::{Signature, VerifyingKey};

use crate::app_message::{
    ApplicationMessage, FAMILY_DEVICE_AUTH, FAMILY_DEVICE_ENROLLMENT, TYPE_DEVICE_AUTH_CHALLENGE,
    TYPE_DEVICE_AUTH_FINISH, TYPE_DEVICE_AUTH_RESULT, TYPE_DEVICE_AUTH_START,
    TYPE_DEVICE_ENROLL_CHALLENGE, TYPE_DEVICE_ENROLL_FINISH, TYPE_DEVICE_ENROLL_RESULT,
    TYPE_DEVICE_ENROLL_START, map_codec_error,
};
use crate::codec::{ensure_empty, put_len_prefixed_str, take_fixed, take_u64};
use crate::error::{ApiError, ApiResult};
use crate::inner_context::{
    DEVICE_PROOF_DOMAIN_V1, Hash32, INNER_PROTOCOL_VERSION_V1, InnerChannelContext,
    PRODUCT_LABEL_V1, put_canonical_str,
};

/// Known-device reauthentication purpose code.
pub const DEVICE_PROOF_PURPOSE_KNOWN_DEVICE_REAUTH: u8 = 1;
/// New-device enrollment purpose code.
pub const DEVICE_PROOF_PURPOSE_NEW_DEVICE_ENROLLMENT: u8 = 2;

/// Device state returned by authentication or enrollment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    /// Device is active and can use the tunnel.
    Active,
    /// Device was accepted but still requires product-side approval.
    Pending,
}

impl DeviceState {
    pub(crate) const fn to_wire(self) -> u8 {
        match self {
            Self::Active => 1,
            Self::Pending => 2,
        }
    }

    pub(crate) const fn from_wire(value: u8) -> ApiResult<Self> {
        match value {
            1 => Ok(Self::Active),
            2 => Ok(Self::Pending),
            _ => Err(ApiError::PostHandshakeAuthFailure),
        }
    }
}

/// Device-proof purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceProofPurpose {
    /// Reauthenticate an enrolled returning device.
    KnownDeviceReauth,
    /// Enroll a new candidate device.
    NewDeviceEnrollment,
}

impl DeviceProofPurpose {
    /// Returns the canonical v1 purpose code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::KnownDeviceReauth => DEVICE_PROOF_PURPOSE_KNOWN_DEVICE_REAUTH,
            Self::NewDeviceEnrollment => DEVICE_PROOF_PURPOSE_NEW_DEVICE_ENROLLMENT,
        }
    }
}

/// Server-issued device challenge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceChallenge {
    /// Server challenge bytes.
    pub server_challenge: Hash32,
    /// Challenge expiry in Unix milliseconds.
    pub expires_at_unix_ms: u64,
}

impl DeviceChallenge {
    /// Returns an error when the challenge is expired for the supplied time.
    ///
    /// # Errors
    ///
    /// Returns auth failure when `now_unix_ms` is at or after the expiry.
    pub const fn ensure_fresh(&self, now_unix_ms: u64) -> ApiResult<()> {
        if now_unix_ms >= self.expires_at_unix_ms {
            return Err(ApiError::PostHandshakeAuthFailure);
        }
        Ok(())
    }
}

/// Input used to construct canonical device-proof signing bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceProofInput {
    /// Noise handshake hash.
    pub noise_handshake_hash: Hash32,
    /// Server challenge.
    pub server_challenge: Hash32,
    /// Stable service context.
    pub context: InnerChannelContext,
    /// Authenticated account context hash.
    pub account_context_hash: Hash32,
    /// Device key identifier.
    pub device_key_id: String,
    /// Proof purpose.
    pub purpose: DeviceProofPurpose,
    /// Proof expiry in Unix milliseconds.
    pub expires_at_unix_ms: u64,
}

impl DeviceProofInput {
    /// Builds canonical v1 signing bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when string fields exceed v1 length bounds.
    pub fn canonical_bytes(&self) -> ApiResult<Vec<u8>> {
        let mut out = Vec::with_capacity(256);
        out.extend_from_slice(DEVICE_PROOF_DOMAIN_V1);
        put_canonical_str(&mut out, PRODUCT_LABEL_V1)?;
        out.extend_from_slice(&INNER_PROTOCOL_VERSION_V1.to_be_bytes());
        out.extend_from_slice(&self.noise_handshake_hash);
        out.extend_from_slice(&self.server_challenge);
        put_canonical_str(&mut out, &self.context.service_id)?;
        put_canonical_str(&mut out, &self.context.environment_id)?;
        put_canonical_str(&mut out, &self.context.service_authority)?;
        out.extend_from_slice(&self.context.signed_descriptor_hash);
        put_canonical_str(&mut out, &self.context.allowed_noise_suite)?;
        out.extend_from_slice(&self.account_context_hash);
        put_canonical_str(&mut out, &self.device_key_id)?;
        out.put_u8(self.purpose.code());
        out.put_u64(self.expires_at_unix_ms);
        Ok(out)
    }
}

/// Verifies an Ed25519 device proof.
///
/// # Errors
///
/// Returns auth failure when the public key or signature is malformed or does
/// not verify the canonical bytes.
pub fn verify_device_proof_signature(
    device_public_key: &[u8; 32],
    canonical_bytes: &[u8],
    signature: &[u8; 64],
) -> ApiResult<()> {
    let verifying_key = VerifyingKey::from_bytes(device_public_key)
        .map_err(|_| ApiError::PostHandshakeAuthFailure)?;
    let signature = Signature::from_bytes(signature);
    verifying_key
        .verify_strict(canonical_bytes, &signature)
        .map_err(|_| ApiError::PostHandshakeAuthFailure)
}

/// Known-device auth start message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceAuthStart {
    /// Device key identifier.
    pub device_key_id: String,
}

/// Device proof finish message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceProofFinish {
    /// Device key identifier.
    pub device_key_id: String,
    /// Server challenge bytes.
    pub server_challenge: Hash32,
    /// Proof expiry in Unix milliseconds.
    pub expires_at_unix_ms: u64,
    /// Caller-provided Ed25519 signature.
    pub signature: [u8; 64],
    /// Candidate device public key for enrollment, when applicable.
    pub candidate_device_public_key: Option<Hash32>,
}

/// Device auth or enrollment result message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceResult {
    /// Device key identifier accepted by the service.
    pub device_key_id: String,
    /// Device state.
    pub state: DeviceState,
}

/// New-device enrollment start message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceEnrollmentStart {
    /// Candidate device key identifier.
    pub device_key_id: String,
    /// Candidate device Ed25519 public key.
    pub device_public_key: Hash32,
}

impl DeviceAuthStart {
    /// Encodes this message.
    ///
    /// # Errors
    ///
    /// Returns an error when the message exceeds v1 bounds.
    pub fn encode(&self) -> ApiResult<Vec<u8>> {
        ApplicationMessage::new(
            FAMILY_DEVICE_AUTH,
            TYPE_DEVICE_AUTH_START,
            vec![self.device_key_id.as_bytes().to_vec()],
        )?
        .encode()
    }

    /// Decodes this message.
    ///
    /// # Errors
    ///
    /// Returns auth failure when bytes are malformed.
    pub fn decode(record: &[u8]) -> ApiResult<Self> {
        let message = ApplicationMessage::decode(record)?;
        message.expect_kind(FAMILY_DEVICE_AUTH, TYPE_DEVICE_AUTH_START)?;
        let [device_key_id] = take_one_field(message.fields)?;
        Ok(Self {
            device_key_id: string_field(&device_key_id)?,
        })
    }
}

impl DeviceEnrollmentStart {
    /// Encodes this message.
    ///
    /// # Errors
    ///
    /// Returns an error when the message exceeds v1 bounds.
    pub fn encode(&self) -> ApiResult<Vec<u8>> {
        ApplicationMessage::new(
            FAMILY_DEVICE_ENROLLMENT,
            TYPE_DEVICE_ENROLL_START,
            vec![
                self.device_key_id.as_bytes().to_vec(),
                self.device_public_key.to_vec(),
            ],
        )?
        .encode()
    }
}

impl DeviceChallenge {
    /// Encodes a known-device challenge.
    ///
    /// # Errors
    ///
    /// Returns an error when the message exceeds v1 bounds.
    pub fn encode_auth(&self) -> ApiResult<Vec<u8>> {
        self.encode_for(FAMILY_DEVICE_AUTH, TYPE_DEVICE_AUTH_CHALLENGE)
    }

    /// Encodes an enrollment challenge.
    ///
    /// # Errors
    ///
    /// Returns an error when the message exceeds v1 bounds.
    pub fn encode_enrollment(&self) -> ApiResult<Vec<u8>> {
        self.encode_for(FAMILY_DEVICE_ENROLLMENT, TYPE_DEVICE_ENROLL_CHALLENGE)
    }

    /// Decodes a known-device challenge.
    ///
    /// # Errors
    ///
    /// Returns auth failure when bytes are malformed.
    pub fn decode_auth(record: &[u8]) -> ApiResult<Self> {
        Self::decode_for(record, FAMILY_DEVICE_AUTH, TYPE_DEVICE_AUTH_CHALLENGE)
    }

    /// Decodes an enrollment challenge.
    ///
    /// # Errors
    ///
    /// Returns auth failure when bytes are malformed.
    pub fn decode_enrollment(record: &[u8]) -> ApiResult<Self> {
        Self::decode_for(
            record,
            FAMILY_DEVICE_ENROLLMENT,
            TYPE_DEVICE_ENROLL_CHALLENGE,
        )
    }

    fn encode_for(&self, family: u8, message_type: u8) -> ApiResult<Vec<u8>> {
        ApplicationMessage::new(
            family,
            message_type,
            vec![
                self.server_challenge.to_vec(),
                self.expires_at_unix_ms.to_be_bytes().to_vec(),
            ],
        )?
        .encode()
    }

    fn decode_for(record: &[u8], family: u8, message_type: u8) -> ApiResult<Self> {
        let message = ApplicationMessage::decode(record)?;
        message.expect_kind(family, message_type)?;
        let [challenge, expiry] = take_two_fields(message.fields)?;
        let mut challenge_input = challenge.as_slice();
        let server_challenge = take_fixed::<32>(&mut challenge_input).map_err(map_codec_error)?;
        ensure_empty(challenge_input).map_err(map_codec_error)?;
        let mut expiry_input = expiry.as_slice();
        let expires_at_unix_ms = take_u64(&mut expiry_input).map_err(map_codec_error)?;
        ensure_empty(expiry_input).map_err(map_codec_error)?;
        Ok(Self {
            server_challenge,
            expires_at_unix_ms,
        })
    }
}

impl DeviceProofFinish {
    /// Encodes a known-device proof finish message.
    ///
    /// # Errors
    ///
    /// Returns an error when the message exceeds v1 bounds.
    pub fn encode_auth(&self) -> ApiResult<Vec<u8>> {
        ApplicationMessage::new(
            FAMILY_DEVICE_AUTH,
            TYPE_DEVICE_AUTH_FINISH,
            proof_finish_fields(self, false),
        )?
        .encode()
    }

    /// Encodes an enrollment proof finish message.
    ///
    /// # Errors
    ///
    /// Returns an error when the message exceeds v1 bounds.
    pub fn encode_enrollment(&self) -> ApiResult<Vec<u8>> {
        ApplicationMessage::new(
            FAMILY_DEVICE_ENROLLMENT,
            TYPE_DEVICE_ENROLL_FINISH,
            proof_finish_fields(self, true),
        )?
        .encode()
    }
}

impl DeviceResult {
    /// Encodes a known-device auth result.
    ///
    /// # Errors
    ///
    /// Returns an error when the message exceeds v1 bounds.
    pub fn encode_auth(&self) -> ApiResult<Vec<u8>> {
        self.encode_for(FAMILY_DEVICE_AUTH, TYPE_DEVICE_AUTH_RESULT)
    }

    /// Encodes an enrollment result.
    ///
    /// # Errors
    ///
    /// Returns an error when the message exceeds v1 bounds.
    pub fn encode_enrollment(&self) -> ApiResult<Vec<u8>> {
        self.encode_for(FAMILY_DEVICE_ENROLLMENT, TYPE_DEVICE_ENROLL_RESULT)
    }

    /// Decodes a known-device auth result.
    ///
    /// # Errors
    ///
    /// Returns auth failure when bytes are malformed.
    pub fn decode_auth(record: &[u8]) -> ApiResult<Self> {
        Self::decode_for(record, FAMILY_DEVICE_AUTH, TYPE_DEVICE_AUTH_RESULT)
    }

    /// Decodes an enrollment result.
    ///
    /// # Errors
    ///
    /// Returns auth failure when bytes are malformed.
    pub fn decode_enrollment(record: &[u8]) -> ApiResult<Self> {
        Self::decode_for(record, FAMILY_DEVICE_ENROLLMENT, TYPE_DEVICE_ENROLL_RESULT)
    }

    fn encode_for(&self, family: u8, message_type: u8) -> ApiResult<Vec<u8>> {
        ApplicationMessage::new(
            family,
            message_type,
            vec![
                self.device_key_id.as_bytes().to_vec(),
                vec![self.state.to_wire()],
            ],
        )?
        .encode()
    }

    fn decode_for(record: &[u8], family: u8, message_type: u8) -> ApiResult<Self> {
        let message = ApplicationMessage::decode(record)?;
        message.expect_kind(family, message_type)?;
        let [device_key_id, state] = take_two_fields(message.fields)?;
        if state.len() != 1 {
            return Err(ApiError::PostHandshakeAuthFailure);
        }
        Ok(Self {
            device_key_id: string_field(&device_key_id)?,
            state: DeviceState::from_wire(state[0])?,
        })
    }
}

fn proof_finish_fields(finish: &DeviceProofFinish, include_public_key: bool) -> Vec<Vec<u8>> {
    let mut fields = vec![
        finish.device_key_id.as_bytes().to_vec(),
        finish.server_challenge.to_vec(),
        finish.expires_at_unix_ms.to_be_bytes().to_vec(),
        finish.signature.to_vec(),
    ];
    if include_public_key {
        fields.push(
            finish
                .candidate_device_public_key
                .unwrap_or([0_u8; 32])
                .to_vec(),
        );
    }
    fields
}

fn string_field(field: &[u8]) -> ApiResult<String> {
    let mut encoded = Vec::new();
    put_len_prefixed_str(
        &mut encoded,
        std::str::from_utf8(field).map_err(|_| ApiError::PostHandshakeAuthFailure)?,
    )
    .map_err(map_codec_error)?;
    let mut input = encoded.as_slice();
    let value = crate::codec::take_len_prefixed_string(&mut input).map_err(map_codec_error)?;
    ensure_empty(input).map_err(map_codec_error)?;
    Ok(value)
}

fn take_one_field(fields: Vec<Vec<u8>>) -> ApiResult<[Vec<u8>; 1]> {
    fields
        .try_into()
        .map_err(|_| ApiError::PostHandshakeAuthFailure)
}

fn take_two_fields(fields: Vec<Vec<u8>>) -> ApiResult<[Vec<u8>; 2]> {
    fields
        .try_into()
        .map_err(|_| ApiError::PostHandshakeAuthFailure)
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{Signer, SigningKey};

    use super::*;

    #[test]
    fn canonical_device_proof_changes_with_handshake_hash() {
        let mut input = proof_input([1_u8; 32]);
        let first = input.canonical_bytes().unwrap();
        input.noise_handshake_hash = [2_u8; 32];
        let second = input.canonical_bytes().unwrap();

        assert_ne!(first, second);
    }

    #[test]
    fn ed25519_device_proof_verifies() {
        let signing_key = SigningKey::from_bytes(&[5_u8; 32]);
        let bytes = proof_input([1_u8; 32]).canonical_bytes().unwrap();
        let signature = signing_key.sign(&bytes).to_bytes();

        verify_device_proof_signature(&signing_key.verifying_key().to_bytes(), &bytes, &signature)
            .unwrap();
    }

    fn proof_input(noise_handshake_hash: Hash32) -> DeviceProofInput {
        DeviceProofInput {
            noise_handshake_hash,
            server_challenge: [2_u8; 32],
            context: InnerChannelContext::v1(
                "svc".to_owned(),
                "prod".to_owned(),
                "api.example.com".to_owned(),
                [3_u8; 32],
            )
            .unwrap(),
            account_context_hash: [4_u8; 32],
            device_key_id: "device-1".to_owned(),
            purpose: DeviceProofPurpose::KnownDeviceReauth,
            expires_at_unix_ms: 1_760_000_000_000,
        }
    }
}
