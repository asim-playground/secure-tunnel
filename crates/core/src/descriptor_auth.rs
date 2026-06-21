// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, Signer, SigningKey};
use sha2::{Digest, Sha256};

use crate::descriptor::{CarrierSet, DescriptorSignature, ServiceDescriptor, TrustAnchor};
use crate::error::{ApiError, ApiResult};
use crate::inner_context::{parse_service_static_public_key, put_canonical_str};
use crate::transport::CarrierKind;
use crate::trust::parse_verifying_key;

const DESCRIPTOR_BODY_DOMAIN_V1: &[u8] = b"secure-tunnel-descriptor-body-v1\0";
const DESCRIPTOR_SIGNATURE_DOMAIN_V1: &[u8] = b"secure-tunnel-descriptor-signature-v1\0";
const EXAMPLE_DESCRIPTOR_SIGNING_KEY: [u8; 32] = [7_u8; 32];

pub fn authorize_descriptor_at(
    descriptor: &ServiceDescriptor,
    trusted_roots: &[TrustAnchor],
    now_unix_seconds: u64,
) -> ApiResult<()> {
    descriptor.validate()?;
    validate_descriptor_window(descriptor, now_unix_seconds)?;
    verify_descriptor_hash(descriptor)?;
    verify_descriptor_signature(descriptor, trusted_roots)
}

pub fn validate_descriptor_window(
    descriptor: &ServiceDescriptor,
    now_unix_seconds: u64,
) -> ApiResult<()> {
    let not_before = parse_rfc3339_utc_seconds(&descriptor.not_before)?;
    let not_after = parse_rfc3339_utc_seconds(&descriptor.not_after)?;
    if not_before >= not_after {
        return Err(ApiError::InvalidServiceDescriptor(
            "descriptor validity window must be ordered",
        ));
    }
    if now_unix_seconds < not_before || now_unix_seconds >= not_after {
        return Err(ApiError::InvalidServiceDescriptor(
            "descriptor is outside its validity window",
        ));
    }
    Ok(())
}

fn verify_descriptor_hash(descriptor: &ServiceDescriptor) -> ApiResult<()> {
    let expected = descriptor.signed_descriptor_hash_bytes()?;
    if descriptor_body_hash(descriptor)? != expected {
        return Err(ApiError::InvalidServiceDescriptor(
            "signed_descriptor_hash must match the canonical descriptor body",
        ));
    }
    Ok(())
}

pub fn sign_example_descriptor(mut descriptor: ServiceDescriptor) -> ApiResult<ServiceDescriptor> {
    let signing_key = SigningKey::from_bytes(&EXAMPLE_DESCRIPTOR_SIGNING_KEY);
    descriptor.trust_anchors = vec![TrustAnchor {
        key_id: "root-2026-01".to_owned(),
        algorithm: "ed25519".to_owned(),
        public_key: STANDARD.encode(signing_key.verifying_key().to_bytes()),
    }];
    descriptor.signed_descriptor_hash = STANDARD.encode(descriptor_body_hash(&descriptor)?);
    descriptor.descriptor_signature = DescriptorSignature {
        key_id: "root-2026-01".to_owned(),
        algorithm: "ed25519".to_owned(),
        signature: STANDARD.encode(
            signing_key
                .sign(&descriptor_signature_input(
                    &descriptor.signed_descriptor_hash_bytes()?,
                ))
                .to_bytes(),
        ),
    };
    Ok(descriptor)
}

pub fn example_trust_anchors() -> Vec<TrustAnchor> {
    let signing_key = SigningKey::from_bytes(&EXAMPLE_DESCRIPTOR_SIGNING_KEY);
    vec![TrustAnchor {
        key_id: "root-2026-01".to_owned(),
        algorithm: "ed25519".to_owned(),
        public_key: STANDARD.encode(signing_key.verifying_key().to_bytes()),
    }]
}

fn verify_descriptor_signature(
    descriptor: &ServiceDescriptor,
    trusted_roots: &[TrustAnchor],
) -> ApiResult<()> {
    if trusted_roots.is_empty() {
        return Err(ApiError::InvalidServiceDescriptor(
            "at least one pinned descriptor trust anchor is required",
        ));
    }

    let signature = &descriptor.descriptor_signature;
    validate_signature_algorithm(signature.algorithm.as_str())?;
    let root = trusted_roots
        .iter()
        .find(|anchor| anchor.key_id == signature.key_id && anchor.algorithm == signature.algorithm)
        .ok_or(ApiError::InvalidServiceDescriptor(
            "descriptor signature key must match a pinned trust anchor",
        ))?;
    if !descriptor.trust_anchors.iter().any(|anchor| anchor == root) {
        return Err(ApiError::InvalidServiceDescriptor(
            "descriptor trust anchors must include the pinned signing root",
        ));
    }

    let verifying_key = parse_verifying_key(root).map_err(|_| {
        ApiError::InvalidServiceDescriptor(
            "pinned descriptor trust anchor must be a valid Ed25519 key",
        )
    })?;
    let signature_bytes = STANDARD
        .decode(signature.signature.as_bytes())
        .map_err(|_| ApiError::InvalidServiceDescriptor("descriptor_signature must be base64"))?;
    let signature_bytes: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| ApiError::InvalidServiceDescriptor("descriptor_signature must be 64 bytes"))?;
    let signature = Signature::from_bytes(&signature_bytes);
    verifying_key
        .verify_strict(
            &descriptor_signature_input(&descriptor.signed_descriptor_hash_bytes()?),
            &signature,
        )
        .map_err(|_| ApiError::InvalidServiceDescriptor("descriptor_signature is invalid"))
}

fn descriptor_body_hash(descriptor: &ServiceDescriptor) -> ApiResult<[u8; 32]> {
    let hash = Sha256::digest(descriptor_body_bytes(descriptor)?);
    Ok(hash.into())
}

fn descriptor_signature_input(descriptor_hash: &[u8; 32]) -> Vec<u8> {
    let mut input = Vec::with_capacity(DESCRIPTOR_SIGNATURE_DOMAIN_V1.len() + 32);
    input.extend_from_slice(DESCRIPTOR_SIGNATURE_DOMAIN_V1);
    input.extend_from_slice(descriptor_hash);
    input
}

fn descriptor_body_bytes(descriptor: &ServiceDescriptor) -> ApiResult<Vec<u8>> {
    let mut out = Vec::with_capacity(512);
    out.extend_from_slice(DESCRIPTOR_BODY_DOMAIN_V1);
    out.extend_from_slice(&descriptor.descriptor_version.to_be_bytes());
    out.extend_from_slice(&descriptor.descriptor_serial.to_be_bytes());
    put_canonical_str(&mut out, &descriptor.not_before)?;
    put_canonical_str(&mut out, &descriptor.not_after)?;
    put_canonical_str(&mut out, &descriptor.environment_id)?;
    put_canonical_str(&mut out, &descriptor.service_id)?;
    put_canonical_str(&mut out, &descriptor.service_authority)?;
    put_canonical_str(&mut out, &descriptor.protocol_id)?;
    put_canonical_str(&mut out, &descriptor.noise_suite)?;
    out.extend_from_slice(&parse_service_static_public_key(
        &descriptor.service_static_public_key,
    )?);
    put_trust_anchors(&mut out, &descriptor.trust_anchors)?;
    put_selection_policy(&mut out, descriptor.selection_policy.preferred_carrier);
    out.push(u8::from(descriptor.selection_policy.allow_wss_fallback));
    put_carriers(&mut out, &descriptor.carriers)?;
    Ok(out)
}

fn put_trust_anchors(out: &mut Vec<u8>, anchors: &[TrustAnchor]) -> ApiResult<()> {
    put_count(out, anchors.len())?;
    for anchor in anchors {
        put_canonical_str(out, &anchor.key_id)?;
        put_canonical_str(out, &anchor.algorithm)?;
        put_canonical_str(out, &anchor.public_key)?;
    }
    Ok(())
}

fn put_carriers(out: &mut Vec<u8>, carriers: &CarrierSet) -> ApiResult<()> {
    if let Some(quic) = &carriers.quic {
        out.push(1);
        put_canonical_str(out, &quic.connect_host)?;
        out.extend_from_slice(&quic.port.to_be_bytes());
        put_canonical_str(out, &quic.alpn)?;
        put_optional_str(out, quic.sni_override.as_deref())?;
    } else {
        out.push(0);
    }
    if let Some(wss) = &carriers.wss {
        out.push(1);
        put_canonical_str(out, &wss.url)?;
        put_canonical_str(out, &wss.subprotocol)?;
        put_optional_str(out, wss.authority_override.as_deref())?;
    } else {
        out.push(0);
    }
    Ok(())
}

fn put_selection_policy(out: &mut Vec<u8>, carrier: CarrierKind) {
    out.push(match carrier {
        CarrierKind::Quic => 1,
        CarrierKind::Wss => 2,
    });
}

fn put_optional_str(out: &mut Vec<u8>, value: Option<&str>) -> ApiResult<()> {
    match value {
        Some(value) => {
            out.push(1);
            put_canonical_str(out, value)?;
        }
        None => out.push(0),
    }
    Ok(())
}

fn put_count(out: &mut Vec<u8>, count: usize) -> ApiResult<()> {
    let count = u16::try_from(count)
        .map_err(|_| ApiError::InvalidServiceDescriptor("canonical count exceeds u16 length"))?;
    out.extend_from_slice(&count.to_be_bytes());
    Ok(())
}

fn parse_rfc3339_utc_seconds(value: &str) -> ApiResult<u64> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return Err(ApiError::InvalidServiceDescriptor(
            "descriptor timestamps must use YYYY-MM-DDTHH:MM:SSZ",
        ));
    }
    let year = parse_digits(&bytes[0..4])?;
    let month = parse_digits(&bytes[5..7])?;
    let day = parse_digits(&bytes[8..10])?;
    let hour = parse_digits(&bytes[11..13])?;
    let minute = parse_digits(&bytes[14..16])?;
    let second = parse_digits(&bytes[17..19])?;
    validate_date_time(year, month, day, hour, minute, second)?;
    let days = days_from_civil(i64::from(year), i64::from(month), i64::from(day));
    if days < 0 {
        return Err(ApiError::InvalidServiceDescriptor(
            "descriptor timestamps must be after 1970-01-01",
        ));
    }
    let seconds = days
        .checked_mul(86_400)
        .and_then(|value| value.checked_add(i64::from(hour) * 3_600))
        .and_then(|value| value.checked_add(i64::from(minute) * 60))
        .and_then(|value| value.checked_add(i64::from(second)))
        .ok_or(ApiError::InvalidServiceDescriptor(
            "descriptor timestamp is out of range",
        ))?;
    u64::try_from(seconds)
        .map_err(|_| ApiError::InvalidServiceDescriptor("descriptor timestamp is out of range"))
}

fn parse_digits(bytes: &[u8]) -> ApiResult<u32> {
    let mut value = 0_u32;
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return Err(ApiError::InvalidServiceDescriptor(
                "descriptor timestamp contains non-digit characters",
            ));
        }
        value = value
            .checked_mul(10)
            .and_then(|acc| acc.checked_add(u32::from(*byte - b'0')))
            .ok_or(ApiError::InvalidServiceDescriptor(
                "descriptor timestamp is out of range",
            ))?;
    }
    Ok(value)
}

fn validate_date_time(
    year: u32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> ApiResult<()> {
    if year < 1970
        || !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return Err(ApiError::InvalidServiceDescriptor(
            "descriptor timestamp has invalid date or time fields",
        ));
    }
    Ok(())
}

const fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

const fn is_leap_year(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

const fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn validate_signature_algorithm(algorithm: &str) -> ApiResult<()> {
    if algorithm == "ed25519" {
        Ok(())
    } else {
        Err(ApiError::InvalidServiceDescriptor(
            "descriptor_signature algorithm must be ed25519",
        ))
    }
}
