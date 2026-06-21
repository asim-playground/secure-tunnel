// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::codec::put_len_prefixed_str;
use crate::constants::NOISE_SUITE_V1;
use crate::error::{ApiError, ApiResult};

/// Product label bound into v1 canonical protocol bytes.
pub const PRODUCT_LABEL_V1: &str = "secure-tunnel";
/// Inner protocol version bound into v1 canonical protocol bytes.
pub const INNER_PROTOCOL_VERSION_V1: u16 = 1;
/// Domain label for the v1 Noise prologue.
pub const PROLOGUE_DOMAIN_V1: &[u8] = b"secure-tunnel-inner-prologue-v1\0";
/// Domain label for v1 device-proof signing bytes.
pub const DEVICE_PROOF_DOMAIN_V1: &[u8] = b"secure-tunnel-device-proof-v1\0";

/// Fixed-width X25519 Noise public key bytes.
pub type NoisePublicKey = [u8; 32];
/// Fixed-width descriptor hash bytes.
pub type DescriptorHash = [u8; 32];
/// Fixed-width hash bytes used for account context and channel binding.
pub type Hash32 = [u8; 32];

/// Stable public context bound into the inner Noise prologue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InnerChannelContext {
    /// Stable logical service identifier.
    pub service_id: String,
    /// Stable environment identifier.
    pub environment_id: String,
    /// Stable service authority independent of selected carrier.
    pub service_authority: String,
    /// Hash of the signed or pinned descriptor selected by the client.
    pub signed_descriptor_hash: DescriptorHash,
    /// Authorized v1 Noise suite.
    pub allowed_noise_suite: String,
}

impl InnerChannelContext {
    /// Creates a v1 context for canonical prologue construction.
    ///
    /// # Errors
    ///
    /// Returns an error when a public label is empty or invalid, or the
    /// descriptor hash is all zeroes.
    pub fn v1(
        service_id: String,
        environment_id: String,
        service_authority: String,
        signed_descriptor_hash: DescriptorHash,
    ) -> ApiResult<Self> {
        let context = Self {
            service_id,
            environment_id,
            service_authority,
            signed_descriptor_hash,
            allowed_noise_suite: NOISE_SUITE_V1.to_owned(),
        };
        context.validate()?;
        Ok(context)
    }

    /// Builds the canonical v1 Noise prologue bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when a length-prefixed field exceeds `u16::MAX`.
    pub fn prologue_bytes(&self) -> ApiResult<Vec<u8>> {
        self.validate()?;
        let mut out = Vec::with_capacity(160);
        out.extend_from_slice(PROLOGUE_DOMAIN_V1);
        put_canonical_str(&mut out, PRODUCT_LABEL_V1)?;
        out.extend_from_slice(&INNER_PROTOCOL_VERSION_V1.to_be_bytes());
        put_canonical_str(&mut out, &self.service_id)?;
        put_canonical_str(&mut out, &self.environment_id)?;
        put_canonical_str(&mut out, &self.service_authority)?;
        out.extend_from_slice(&self.signed_descriptor_hash);
        put_canonical_str(&mut out, &self.allowed_noise_suite)?;
        Ok(out)
    }

    fn validate(&self) -> ApiResult<()> {
        validate_public_label(&self.service_id)?;
        validate_public_label(&self.environment_id)?;
        validate_public_label(&self.service_authority)?;
        if self.allowed_noise_suite != NOISE_SUITE_V1 {
            return Err(ApiError::InvalidServiceDescriptor(
                "allowed_noise_suite must match the v1 Noise suite identifier",
            ));
        }
        ensure_nonzero_32(&self.signed_descriptor_hash, "signed_descriptor_hash")
    }
}

/// Parses a base64-encoded nonzero 32-byte service Noise public key.
///
/// # Errors
///
/// Returns an invalid descriptor error when the input is not base64, does not
/// decode to 32 bytes, or is all zeroes.
pub fn parse_service_static_public_key(value: &str) -> ApiResult<NoisePublicKey> {
    parse_base64_nonzero_32(
        value,
        "service_static_public_key must be base64-encoded 32-byte public key",
        "service_static_public_key must not be all zeroes",
    )
}

/// Parses a base64-encoded nonzero 32-byte descriptor hash.
///
/// # Errors
///
/// Returns an invalid descriptor error when the input is not base64, does not
/// decode to 32 bytes, or is all zeroes.
pub fn parse_signed_descriptor_hash(value: &str) -> ApiResult<DescriptorHash> {
    parse_base64_nonzero_32(
        value,
        "signed_descriptor_hash must be base64-encoded 32-byte hash",
        "signed_descriptor_hash must not be all zeroes",
    )
}

pub fn put_canonical_str(out: &mut Vec<u8>, value: &str) -> ApiResult<()> {
    put_len_prefixed_str(out, value)
        .map_err(|_| ApiError::InvalidServiceDescriptor("canonical string exceeds u16 length"))
}

fn ensure_nonzero_32(value: &[u8; 32], error_field: &'static str) -> ApiResult<()> {
    if value.iter().all(|byte| *byte == 0) {
        return Err(match error_field {
            "signed_descriptor_hash" => {
                ApiError::InvalidServiceDescriptor("signed_descriptor_hash must not be all zeroes")
            }
            "service_static_public_key" => ApiError::InvalidServiceDescriptor(
                "service_static_public_key must not be all zeroes",
            ),
            _ => ApiError::InvalidServiceDescriptor("fixed-width value must not be all zeroes"),
        });
    }
    Ok(())
}

fn parse_base64_nonzero_32(
    value: &str,
    decode_error: &'static str,
    zero_error: &'static str,
) -> ApiResult<[u8; 32]> {
    let decoded = STANDARD
        .decode(value.as_bytes())
        .map_err(|_| ApiError::InvalidServiceDescriptor(decode_error))?;
    let bytes: [u8; 32] = decoded
        .try_into()
        .map_err(|_| ApiError::InvalidServiceDescriptor(decode_error))?;
    if bytes.iter().all(|byte| *byte == 0) {
        return Err(ApiError::InvalidServiceDescriptor(zero_error));
    }
    Ok(bytes)
}

fn validate_public_label(value: &str) -> ApiResult<()> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(ApiError::InvalidServiceDescriptor(
            "public descriptor label must not be empty or contain control characters",
        ));
    }
    Ok(())
}
