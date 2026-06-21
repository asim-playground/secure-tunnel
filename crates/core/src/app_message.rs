// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use bytes::BufMut;

use crate::codec::{CodecError, ensure_empty, put_len_prefixed_bytes, take_len_prefixed_bytes};
use crate::constants::MAX_APPLICATION_PLAINTEXT_SIZE;
use crate::error::{ApiError, ApiResult};

/// v1 encrypted application-message envelope version.
pub const APP_MESSAGE_VERSION_V1: u8 = 1;

/// Account-session message family.
pub const FAMILY_ACCOUNT: u8 = 1;
/// Known-device authentication message family.
pub const FAMILY_DEVICE_AUTH: u8 = 2;
/// New-device enrollment message family.
pub const FAMILY_DEVICE_ENROLLMENT: u8 = 3;

/// Account login or resume request message type.
pub const TYPE_ACCOUNT_AUTH_REQUEST: u8 = 1;
/// Account login or resume result message type.
pub const TYPE_ACCOUNT_AUTH_RESULT: u8 = 2;
/// Device authentication start message type.
pub const TYPE_DEVICE_AUTH_START: u8 = 1;
/// Device authentication challenge message type.
pub const TYPE_DEVICE_AUTH_CHALLENGE: u8 = 2;
/// Device authentication finish message type.
pub const TYPE_DEVICE_AUTH_FINISH: u8 = 3;
/// Device authentication result message type.
pub const TYPE_DEVICE_AUTH_RESULT: u8 = 4;
/// Device enrollment start message type.
pub const TYPE_DEVICE_ENROLL_START: u8 = 1;
/// Device enrollment challenge message type.
pub const TYPE_DEVICE_ENROLL_CHALLENGE: u8 = 2;
/// Device enrollment finish message type.
pub const TYPE_DEVICE_ENROLL_FINISH: u8 = 3;
/// Device enrollment result message type.
pub const TYPE_DEVICE_ENROLL_RESULT: u8 = 4;

/// Decoded v1 encrypted application-message envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationMessage {
    /// Message family.
    pub family: u8,
    /// Family-local message type.
    pub message_type: u8,
    /// Length-prefixed message fields.
    pub fields: Vec<Vec<u8>>,
}

impl ApplicationMessage {
    /// Creates an application message from field bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the field count or encoded size exceeds v1 limits.
    pub fn new(family: u8, message_type: u8, fields: Vec<Vec<u8>>) -> ApiResult<Self> {
        let message = Self {
            family,
            message_type,
            fields,
        };
        ensure_message_size(&message)?;
        Ok(message)
    }

    /// Encodes this message into v1 wire bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the encoded message exceeds v1 limits.
    pub fn encode(&self) -> ApiResult<Vec<u8>> {
        ensure_message_size(self)?;
        let field_count =
            u8::try_from(self.fields.len()).map_err(|_| ApiError::RecordTooLarge {
                actual: self.fields.len(),
                max: usize::from(u8::MAX),
            })?;
        let mut out = Vec::new();
        out.put_u8(APP_MESSAGE_VERSION_V1);
        out.put_u8(self.family);
        out.put_u8(self.message_type);
        out.put_u8(field_count);
        for field in &self.fields {
            put_len_prefixed_bytes(&mut out, field).map_err(map_codec_error)?;
        }
        ensure_payload_len(out.len())?;
        Ok(out)
    }

    /// Decodes v1 wire bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the envelope version, field layout, or size is
    /// invalid.
    pub fn decode(record: &[u8]) -> ApiResult<Self> {
        ensure_payload_len(record.len())?;
        if record.len() < 4 {
            return Err(ApiError::PostHandshakeAuthFailure);
        }
        let mut input = record;
        let version = take_byte(&mut input)?;
        if version != APP_MESSAGE_VERSION_V1 {
            return Err(ApiError::PostHandshakeAuthFailure);
        }
        let family = take_byte(&mut input)?;
        let message_type = take_byte(&mut input)?;
        let field_count = take_byte(&mut input)?;
        let mut fields = Vec::with_capacity(usize::from(field_count));
        for _ in 0..field_count {
            fields.push(take_len_prefixed_bytes(&mut input).map_err(map_codec_error)?);
        }
        ensure_empty(input).map_err(map_codec_error)?;
        Self::new(family, message_type, fields)
    }

    /// Verifies the family and type for a decoded message.
    ///
    /// # Errors
    ///
    /// Returns post-handshake auth failure when the message is not the expected
    /// protocol response.
    pub const fn expect_kind(&self, family: u8, message_type: u8) -> ApiResult<()> {
        if self.family != family || self.message_type != message_type {
            return Err(ApiError::PostHandshakeAuthFailure);
        }
        Ok(())
    }
}

fn ensure_message_size(message: &ApplicationMessage) -> ApiResult<()> {
    if message.fields.len() > usize::from(u8::MAX) {
        return Err(ApiError::RecordTooLarge {
            actual: message.fields.len(),
            max: usize::from(u8::MAX),
        });
    }
    let mut size = 4_usize;
    for field in &message.fields {
        size = size.saturating_add(2).saturating_add(field.len());
    }
    ensure_payload_len(size)
}

const fn ensure_payload_len(len: usize) -> ApiResult<()> {
    if len > MAX_APPLICATION_PLAINTEXT_SIZE {
        return Err(ApiError::RecordTooLarge {
            actual: len,
            max: MAX_APPLICATION_PLAINTEXT_SIZE,
        });
    }
    Ok(())
}

fn take_byte(input: &mut &[u8]) -> ApiResult<u8> {
    crate::codec::take_u8(input).map_err(map_codec_error)
}

pub const fn map_codec_error(_error: CodecError) -> ApiError {
    ApiError::PostHandshakeAuthFailure
}
