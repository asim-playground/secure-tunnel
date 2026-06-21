// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use crate::app_message::{
    ApplicationMessage, FAMILY_ACCOUNT, TYPE_ACCOUNT_AUTH_REQUEST, TYPE_ACCOUNT_AUTH_RESULT,
    map_codec_error,
};
use crate::codec::{put_len_prefixed_str, take_fixed, take_len_prefixed_string, take_u8};
use crate::error::{ApiError, ApiResult};
use crate::inner_context::Hash32;

/// Account session freshness returned by the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountFreshness {
    /// The account was freshly authenticated with current credentials.
    Fresh,
    /// The account session was resumed from an opaque token.
    Resumed,
}

impl AccountFreshness {
    /// Returns true when this state can enroll new devices by default.
    #[must_use]
    pub const fn permits_device_enrollment(self) -> bool {
        matches!(self, Self::Fresh)
    }

    pub(crate) const fn to_wire(self) -> u8 {
        match self {
            Self::Fresh => 1,
            Self::Resumed => 2,
        }
    }

    pub(crate) const fn from_wire(value: u8) -> ApiResult<Self> {
        match value {
            1 => Ok(Self::Fresh),
            2 => Ok(Self::Resumed),
            _ => Err(ApiError::PostHandshakeAuthFailure),
        }
    }
}

/// Account authentication mode requested by the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountAuthMode {
    /// Authenticate with fresh account credentials.
    Fresh,
    /// Resume a previously established account session.
    Resume,
}

impl AccountAuthMode {
    pub(crate) const fn to_wire(self) -> u8 {
        match self {
            Self::Fresh => 1,
            Self::Resume => 2,
        }
    }

    pub(crate) const fn from_wire(value: u8) -> ApiResult<Self> {
        match value {
            1 => Ok(Self::Fresh),
            2 => Ok(Self::Resume),
            _ => Err(ApiError::PostHandshakeAuthFailure),
        }
    }
}

/// Account authentication request carried after `Secure Ready`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountAuthRequest {
    /// Product account identifier.
    pub account_id: String,
    /// Opaque credential or resume payload.
    pub credential_payload: Vec<u8>,
    /// Requested authentication mode.
    pub mode: AccountAuthMode,
}

impl AccountAuthRequest {
    /// Encodes this request as one application message.
    ///
    /// # Errors
    ///
    /// Returns an error when fields are too large for v1 encoding.
    pub fn encode(&self) -> ApiResult<Vec<u8>> {
        let mode = vec![self.mode.to_wire()];
        ApplicationMessage::new(
            FAMILY_ACCOUNT,
            TYPE_ACCOUNT_AUTH_REQUEST,
            vec![
                self.account_id.as_bytes().to_vec(),
                mode,
                self.credential_payload.clone(),
            ],
        )?
        .encode()
    }

    /// Decodes an account auth request.
    ///
    /// # Errors
    ///
    /// Returns post-handshake auth failure when bytes are malformed.
    pub fn decode(record: &[u8]) -> ApiResult<Self> {
        let message = ApplicationMessage::decode(record)?;
        message.expect_kind(FAMILY_ACCOUNT, TYPE_ACCOUNT_AUTH_REQUEST)?;
        let [account_id, mode, credential_payload] = take_three_fields(message.fields)?;
        let mut mode_input = mode.as_slice();
        let mode = AccountAuthMode::from_wire(take_u8(&mut mode_input).map_err(map_codec_error)?)?;
        crate::codec::ensure_empty(mode_input).map_err(map_codec_error)?;
        Ok(Self {
            account_id: string_field(&account_id)?,
            credential_payload,
            mode,
        })
    }
}

/// Account authentication result returned by the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountAuthResult {
    /// Product account identifier pinned by the service.
    pub account_id: String,
    /// Server-side account session context identifier.
    pub session_context_id: String,
    /// Stable hash bound into device proof bytes.
    pub account_context_hash: Hash32,
    /// Established account session freshness.
    pub freshness: AccountFreshness,
}

impl AccountAuthResult {
    /// Encodes this result as one application message.
    ///
    /// # Errors
    ///
    /// Returns an error when fields are too large for v1 encoding.
    pub fn encode(&self) -> ApiResult<Vec<u8>> {
        let freshness = vec![self.freshness.to_wire()];
        ApplicationMessage::new(
            FAMILY_ACCOUNT,
            TYPE_ACCOUNT_AUTH_RESULT,
            vec![
                self.account_id.as_bytes().to_vec(),
                self.session_context_id.as_bytes().to_vec(),
                self.account_context_hash.to_vec(),
                freshness,
            ],
        )?
        .encode()
    }

    /// Decodes an account auth result.
    ///
    /// # Errors
    ///
    /// Returns post-handshake auth failure when bytes are malformed.
    pub fn decode(record: &[u8]) -> ApiResult<Self> {
        let message = ApplicationMessage::decode(record)?;
        message.expect_kind(FAMILY_ACCOUNT, TYPE_ACCOUNT_AUTH_RESULT)?;
        let [
            account_id,
            session_context_id,
            account_context_hash,
            freshness,
        ] = take_four_fields(message.fields)?;
        let mut hash_input = account_context_hash.as_slice();
        let account_context_hash = take_fixed::<32>(&mut hash_input).map_err(map_codec_error)?;
        crate::codec::ensure_empty(hash_input).map_err(map_codec_error)?;
        let mut freshness_input = freshness.as_slice();
        let freshness =
            AccountFreshness::from_wire(take_u8(&mut freshness_input).map_err(map_codec_error)?)?;
        crate::codec::ensure_empty(freshness_input).map_err(map_codec_error)?;
        Ok(Self {
            account_id: string_field(&account_id)?,
            session_context_id: string_field(&session_context_id)?,
            account_context_hash,
            freshness,
        })
    }
}

fn string_field(field: &[u8]) -> ApiResult<String> {
    let mut encoded = Vec::new();
    put_len_prefixed_str(
        &mut encoded,
        std::str::from_utf8(field).map_err(|_| ApiError::PostHandshakeAuthFailure)?,
    )
    .map_err(map_codec_error)?;
    let mut input = encoded.as_slice();
    let value = take_len_prefixed_string(&mut input).map_err(map_codec_error)?;
    crate::codec::ensure_empty(input).map_err(map_codec_error)?;
    Ok(value)
}

fn take_three_fields(fields: Vec<Vec<u8>>) -> ApiResult<[Vec<u8>; 3]> {
    fields
        .try_into()
        .map_err(|_| ApiError::PostHandshakeAuthFailure)
}

fn take_four_fields(fields: Vec<Vec<u8>>) -> ApiResult<[Vec<u8>; 4]> {
    fields
        .try_into()
        .map_err(|_| ApiError::PostHandshakeAuthFailure)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_request_round_trips() {
        let request = AccountAuthRequest {
            account_id: "acct-1".to_owned(),
            credential_payload: vec![1, 2, 3],
            mode: AccountAuthMode::Fresh,
        };

        assert_eq!(
            AccountAuthRequest::decode(&request.encode().unwrap()).unwrap(),
            request
        );
    }

    #[test]
    fn account_result_round_trips() {
        let result = AccountAuthResult {
            account_id: "acct-1".to_owned(),
            session_context_id: "ctx-1".to_owned(),
            account_context_hash: [4_u8; 32],
            freshness: AccountFreshness::Resumed,
        };

        assert_eq!(
            AccountAuthResult::decode(&result.encode().unwrap()).unwrap(),
            result
        );
    }
}
