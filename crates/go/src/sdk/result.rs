// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::ffi::CStr;

use crate::{SecureTunnelStatus, SecureTunnelStringResult, error_string};

use super::{
    SecureTunnelByteBuffer, SecureTunnelBytesResult, SecureTunnelClientResult,
    SecureTunnelConnectionResult,
};

pub(super) fn bytes_from_ptr(
    value: *const u8,
    len: usize,
) -> Result<Vec<u8>, SecureTunnelStringResult> {
    if len == 0 {
        return Ok(Vec::new());
    }
    if value.is_null() {
        return Err(crate::string_result(
            SecureTunnelStatus::SecureTunnelStatusNullPointer,
            "byte pointer must not be null when length is non-zero",
        ));
    }
    Ok(unsafe { std::slice::from_raw_parts(value, len) }.to_vec())
}

pub(super) fn bytes_success(bytes: Vec<u8>) -> SecureTunnelBytesResult {
    let mut bytes = bytes.into_boxed_slice();
    let len = bytes.len();
    let data = bytes.as_mut_ptr();
    std::mem::forget(bytes);
    SecureTunnelBytesResult {
        status: SecureTunnelStatus::SecureTunnelStatusSuccess,
        error: std::ptr::null_mut(),
        bytes: SecureTunnelByteBuffer { data, len },
    }
}

pub(super) fn bytes_error(
    status: SecureTunnelStatus,
    error: impl Into<String>,
) -> SecureTunnelBytesResult {
    SecureTunnelBytesResult {
        status,
        error: error_string(error),
        bytes: SecureTunnelByteBuffer {
            data: std::ptr::null_mut(),
            len: 0,
        },
    }
}

pub(super) fn client_error(
    status: SecureTunnelStatus,
    error: impl Into<String>,
) -> SecureTunnelClientResult {
    SecureTunnelClientResult {
        status,
        error: error_string(error),
        client: std::ptr::null_mut(),
    }
}

pub(super) fn connection_error(
    status: SecureTunnelStatus,
    error: impl Into<String>,
) -> SecureTunnelConnectionResult {
    SecureTunnelConnectionResult {
        status,
        error: error_string(error),
        connection: std::ptr::null_mut(),
    }
}

pub(super) fn take_result_message(result: SecureTunnelStringResult) -> String {
    if result.value.is_null() {
        return "no message returned".to_owned();
    }
    let message = unsafe { CStr::from_ptr(result.value).to_string_lossy().into_owned() };
    unsafe {
        crate::secure_tunnel_free_string(result.value);
    }
    message
}
