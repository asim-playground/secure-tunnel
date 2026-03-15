// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use bytes::BufMut;
use ed25519_dalek::{Signature, VerifyingKey};

use crate::codec::{
    CodecError, ensure_empty, put_len_prefixed_str, take_fixed, take_len_prefixed_string, take_u8,
    take_u64,
};
use crate::descriptor::{ServiceDescriptor, TrustAnchor};
use crate::error::{ApiError, ApiResult};

const AUTHORIZATION_VERSION_V1: u8 = 1;

/// Signed responder payload carried inside the v1 `NX` responder handshake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerKeyAuthorizationV1 {
    /// Authorization object version.
    pub version: u8,
    /// Trust-anchor key identifier expected to verify the signature.
    pub key_id: String,
    /// Inclusive start of the authorization validity window.
    pub not_before_unix_seconds: u64,
    /// Inclusive end of the authorization validity window.
    pub not_after_unix_seconds: u64,
    /// Descriptor-bound environment identifier.
    pub environment_id: String,
    /// Descriptor-bound logical service identifier.
    pub service_id: String,
    /// Descriptor-bound service authority.
    pub service_authority: String,
    /// Descriptor-bound protocol identifier.
    pub protocol_id: String,
    /// Authorized responder Noise static public key.
    pub server_static_public_key: [u8; 32],
    /// Ed25519 signature over the unsigned authorization bytes.
    pub signature: [u8; 64],
}

impl ServerKeyAuthorizationV1 {
    pub(crate) fn signed_bytes(&self) -> Result<Vec<u8>, CodecError> {
        let mut out = Vec::with_capacity(256);
        out.put_u8(self.version);
        put_len_prefixed_str(&mut out, &self.key_id)?;
        out.put_u64(self.not_before_unix_seconds);
        out.put_u64(self.not_after_unix_seconds);
        put_len_prefixed_str(&mut out, &self.environment_id)?;
        put_len_prefixed_str(&mut out, &self.service_id)?;
        put_len_prefixed_str(&mut out, &self.service_authority)?;
        put_len_prefixed_str(&mut out, &self.protocol_id)?;
        out.extend_from_slice(&self.server_static_public_key);
        Ok(out)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn encode(&self) -> Result<Vec<u8>, CodecError> {
        let mut out = self.signed_bytes()?;
        out.extend_from_slice(&self.signature);
        Ok(out)
    }

    pub(crate) fn decode(payload: &[u8]) -> Result<Self, CodecError> {
        let mut input = payload;
        let version = take_u8(&mut input)?;
        let key_id = take_len_prefixed_string(&mut input)?;
        let not_before_unix_seconds = take_u64(&mut input)?;
        let not_after_unix_seconds = take_u64(&mut input)?;
        let environment_id = take_len_prefixed_string(&mut input)?;
        let service_id = take_len_prefixed_string(&mut input)?;
        let service_authority = take_len_prefixed_string(&mut input)?;
        let protocol_id = take_len_prefixed_string(&mut input)?;
        let server_static_public_key = take_fixed::<32>(&mut input)?;
        let signature = take_fixed::<64>(&mut input)?;

        ensure_empty(input)?;

        Ok(Self {
            version,
            key_id,
            not_before_unix_seconds,
            not_after_unix_seconds,
            environment_id,
            service_id,
            service_authority,
            protocol_id,
            server_static_public_key,
            signature,
        })
    }
}

pub fn verify_server_key_authorization(
    descriptor: &ServiceDescriptor,
    now_unix_seconds: u64,
    presented_remote_static: &[u8],
    payload: &[u8],
) -> ApiResult<()> {
    let authorization =
        ServerKeyAuthorizationV1::decode(payload).map_err(|_| ApiError::InnerTrustFailure)?;

    if authorization.version != AUTHORIZATION_VERSION_V1 {
        return Err(ApiError::InnerTrustFailure);
    }

    if authorization.server_static_public_key.as_slice() != presented_remote_static {
        return Err(ApiError::InnerTrustFailure);
    }

    if now_unix_seconds < authorization.not_before_unix_seconds
        || now_unix_seconds > authorization.not_after_unix_seconds
    {
        return Err(ApiError::InnerTrustFailure);
    }

    if authorization.environment_id != descriptor.environment_id
        || authorization.service_id != descriptor.service_id
        || authorization.service_authority != descriptor.service_authority
        || authorization.protocol_id != descriptor.protocol_id
    {
        return Err(ApiError::InnerTrustFailure);
    }

    let signed_bytes = authorization
        .signed_bytes()
        .map_err(|_| ApiError::InnerTrustFailure)?;
    let signature = Signature::from_bytes(&authorization.signature);
    let trust_anchor = descriptor
        .trust_anchors
        .iter()
        .find(|anchor| anchor.key_id == authorization.key_id)
        .ok_or(ApiError::InnerTrustFailure)?;
    let verifying_key =
        parse_verifying_key(trust_anchor).map_err(|_| ApiError::InnerTrustFailure)?;

    verifying_key
        .verify_strict(&signed_bytes, &signature)
        .map_err(|_| ApiError::InnerTrustFailure)
}

fn parse_verifying_key(anchor: &TrustAnchor) -> Result<VerifyingKey, CodecError> {
    if anchor.algorithm != "ed25519" {
        return Err(CodecError::InvalidUtf8);
    }

    let decoded = STANDARD
        .decode(anchor.public_key.as_bytes())
        .map_err(|_| CodecError::InvalidUtf8)?;
    let bytes: [u8; 32] = decoded.try_into().map_err(|_| CodecError::InvalidUtf8)?;

    VerifyingKey::from_bytes(&bytes).map_err(|_| CodecError::InvalidUtf8)
}
