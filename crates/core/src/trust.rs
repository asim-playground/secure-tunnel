// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::VerifyingKey;

use crate::codec::CodecError;
use crate::descriptor::TrustAnchor;

pub fn parse_verifying_key(anchor: &TrustAnchor) -> Result<VerifyingKey, CodecError> {
    if anchor.algorithm != "ed25519" {
        return Err(CodecError::InvalidUtf8);
    }

    let decoded = STANDARD
        .decode(anchor.public_key.as_bytes())
        .map_err(|_| CodecError::InvalidUtf8)?;
    let bytes: [u8; 32] = decoded.try_into().map_err(|_| CodecError::InvalidUtf8)?;

    VerifyingKey::from_bytes(&bytes).map_err(|_| CodecError::InvalidUtf8)
}
