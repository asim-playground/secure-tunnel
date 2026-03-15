// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use bytes::{Buf, BufMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecError {
    LengthOverflow,
    UnexpectedEof,
    InvalidUtf8,
    TrailingBytes,
}

pub fn put_len_prefixed_bytes(buffer: &mut Vec<u8>, value: &[u8]) -> Result<(), CodecError> {
    let len = u16::try_from(value.len()).map_err(|_| CodecError::LengthOverflow)?;
    buffer.put_u16(len);
    buffer.extend_from_slice(value);
    Ok(())
}

pub fn put_len_prefixed_str(buffer: &mut Vec<u8>, value: &str) -> Result<(), CodecError> {
    put_len_prefixed_bytes(buffer, value.as_bytes())
}

pub fn take_u8(input: &mut &[u8]) -> Result<u8, CodecError> {
    if input.remaining() < 1 {
        return Err(CodecError::UnexpectedEof);
    }
    Ok(input.get_u8())
}

pub fn take_u16(input: &mut &[u8]) -> Result<u16, CodecError> {
    if input.remaining() < 2 {
        return Err(CodecError::UnexpectedEof);
    }
    Ok(input.get_u16())
}

pub fn take_u64(input: &mut &[u8]) -> Result<u64, CodecError> {
    if input.remaining() < 8 {
        return Err(CodecError::UnexpectedEof);
    }
    Ok(input.get_u64())
}

pub fn take_len_prefixed_bytes(input: &mut &[u8]) -> Result<Vec<u8>, CodecError> {
    let len = usize::from(take_u16(input)?);
    if input.remaining() < len {
        return Err(CodecError::UnexpectedEof);
    }

    let mut out = vec![0_u8; len];
    input.copy_to_slice(&mut out);
    Ok(out)
}

pub fn take_len_prefixed_string(input: &mut &[u8]) -> Result<String, CodecError> {
    let bytes = take_len_prefixed_bytes(input)?;
    String::from_utf8(bytes).map_err(|_| CodecError::InvalidUtf8)
}

pub fn take_fixed<const N: usize>(input: &mut &[u8]) -> Result<[u8; N], CodecError> {
    if input.remaining() < N {
        return Err(CodecError::UnexpectedEof);
    }

    let mut out = [0_u8; N];
    input.copy_to_slice(&mut out);
    Ok(out)
}

pub const fn ensure_empty(input: &[u8]) -> Result<(), CodecError> {
    if input.is_empty() {
        Ok(())
    } else {
        Err(CodecError::TrailingBytes)
    }
}
