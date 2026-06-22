// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! C ABI for Swift, Go, and other foreign-language callers.

mod sdk;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use secure_tunnel_core::{
    PROTOCOL_ID_V1, QUIC_ALPN_V1, ServiceDescriptor, WSS_SUBPROTOCOL_V1, example_service_descriptor,
};

static FFI_VERSION: &[u8] = b"1\0";
static PROTOCOL_ID: &[u8] = b"secure-tunnel-v1\0";
static QUIC_ALPN: &[u8] = b"secure-tunnel-v1\0";
static WSS_SUBPROTOCOL: &[u8] = b"secure-tunnel-v1\0";

/// Status code for C ABI calls.
#[repr(C)]
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SecureTunnelStatus {
    /// The operation succeeded.
    SecureTunnelStatusSuccess = 0,
    /// The caller supplied a null pointer.
    SecureTunnelStatusNullPointer = 1,
    /// The caller supplied bytes that were not valid UTF-8.
    SecureTunnelStatusInvalidUtf8 = 2,
    /// JSON decoding or encoding failed.
    SecureTunnelStatusInvalidJson = 3,
    /// The descriptor decoded but failed Secure Tunnel validation.
    SecureTunnelStatusInvalidDescriptor = 4,
    /// The Rust side could not allocate a caller-owned C string.
    SecureTunnelStatusAllocationFailure = 5,
    /// Caller configuration was invalid.
    SecureTunnelStatusInvalidConfig = 6,
    /// Creating a runtime or opaque handle failed.
    SecureTunnelStatusRuntimeFailure = 7,
    /// The SDK connect operation failed.
    SecureTunnelStatusConnectFailure = 8,
    /// An authenticated session operation failed.
    SecureTunnelStatusSessionFailure = 9,
}

/// Result for FFI calls that return a caller-owned string or an error message.
///
/// When `status` is `SecureTunnelStatusSuccess`, `value` is the returned
/// payload or null for operations with no payload. When `status` is any other
/// value, `value` is a caller-owned error message when allocation succeeds.
/// Free every non-null `value` with `secure_tunnel_free_string`.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SecureTunnelStringResult {
    /// Operation status.
    pub status: SecureTunnelStatus,
    /// Returned string payload or error message.
    pub value: *mut c_char,
}

/// Returns the C ABI version implemented by this library.
#[unsafe(no_mangle)]
pub extern "C" fn secure_tunnel_ffi_version() -> *const c_char {
    FFI_VERSION.as_ptr().cast()
}

/// Returns the stable v1 protocol identifier.
#[unsafe(no_mangle)]
pub extern "C" fn secure_tunnel_protocol_id_v1() -> *const c_char {
    debug_assert_eq!(
        PROTOCOL_ID_V1.as_bytes(),
        &PROTOCOL_ID[..PROTOCOL_ID.len() - 1]
    );
    PROTOCOL_ID.as_ptr().cast()
}

/// Returns the v1 QUIC ALPN value.
#[unsafe(no_mangle)]
pub extern "C" fn secure_tunnel_quic_alpn_v1() -> *const c_char {
    debug_assert_eq!(QUIC_ALPN_V1.as_bytes(), &QUIC_ALPN[..QUIC_ALPN.len() - 1]);
    QUIC_ALPN.as_ptr().cast()
}

/// Returns the v1 WSS subprotocol value.
#[unsafe(no_mangle)]
pub extern "C" fn secure_tunnel_wss_subprotocol_v1() -> *const c_char {
    debug_assert_eq!(
        WSS_SUBPROTOCOL_V1.as_bytes(),
        &WSS_SUBPROTOCOL[..WSS_SUBPROTOCOL.len() - 1]
    );
    WSS_SUBPROTOCOL.as_ptr().cast()
}

/// Returns a sample service descriptor as JSON.
#[unsafe(no_mangle)]
pub extern "C" fn secure_tunnel_example_service_descriptor_json() -> SecureTunnelStringResult {
    encode_json(&example_service_descriptor())
}

/// Validates a service descriptor JSON string.
///
/// # Safety
///
/// `descriptor_json` must be a valid pointer to a null-terminated C string.
/// The pointer may be null, in which case a `NullPointer` result is returned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn secure_tunnel_validate_service_descriptor_json(
    descriptor_json: *const c_char,
) -> SecureTunnelStringResult {
    match decode_descriptor(descriptor_json) {
        Ok(descriptor) => match descriptor.validate() {
            Ok(()) => SecureTunnelStringResult {
                status: SecureTunnelStatus::SecureTunnelStatusSuccess,
                value: std::ptr::null_mut(),
            },
            Err(error) => string_result(
                SecureTunnelStatus::SecureTunnelStatusInvalidDescriptor,
                error.to_string(),
            ),
        },
        Err(result) => result,
    }
}

/// Decodes, validates, and re-encodes a service descriptor JSON string.
///
/// # Safety
///
/// `descriptor_json` must be a valid pointer to a null-terminated C string.
/// The pointer may be null, in which case a `NullPointer` result is returned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn secure_tunnel_normalize_service_descriptor_json(
    descriptor_json: *const c_char,
) -> SecureTunnelStringResult {
    match decode_descriptor(descriptor_json) {
        Ok(descriptor) => match descriptor.validate() {
            Ok(()) => encode_json(&descriptor),
            Err(error) => string_result(
                SecureTunnelStatus::SecureTunnelStatusInvalidDescriptor,
                error.to_string(),
            ),
        },
        Err(result) => result,
    }
}

/// Frees a string returned in `SecureTunnelStringResult.value`.
///
/// # Safety
///
/// `value` must be null or a pointer returned by this library. Do not free a
/// static string returned by one of the protocol constant functions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn secure_tunnel_free_string(value: *mut c_char) {
    if !value.is_null() {
        unsafe {
            drop(CString::from_raw(value));
        }
    }
}

fn decode_descriptor(
    descriptor_json: *const c_char,
) -> Result<ServiceDescriptor, SecureTunnelStringResult> {
    if descriptor_json.is_null() {
        return Err(string_result(
            SecureTunnelStatus::SecureTunnelStatusNullPointer,
            "descriptor_json must not be null",
        ));
    }

    let input = unsafe { CStr::from_ptr(descriptor_json) }
        .to_str()
        .map_err(|error| {
            string_result(
                SecureTunnelStatus::SecureTunnelStatusInvalidUtf8,
                error.to_string(),
            )
        })?;

    serde_json::from_str::<ServiceDescriptor>(input).map_err(|error| {
        string_result(
            SecureTunnelStatus::SecureTunnelStatusInvalidJson,
            error.to_string(),
        )
    })
}

fn encode_json(descriptor: &ServiceDescriptor) -> SecureTunnelStringResult {
    match serde_json::to_string(descriptor) {
        Ok(json) => string_result(SecureTunnelStatus::SecureTunnelStatusSuccess, json),
        Err(error) => string_result(
            SecureTunnelStatus::SecureTunnelStatusInvalidJson,
            error.to_string(),
        ),
    }
}

pub(crate) fn string_result(
    status: SecureTunnelStatus,
    value: impl Into<String>,
) -> SecureTunnelStringResult {
    CString::new(value.into()).map_or_else(
        |_| SecureTunnelStringResult {
            status: SecureTunnelStatus::SecureTunnelStatusAllocationFailure,
            value: std::ptr::null_mut(),
        },
        |value| SecureTunnelStringResult {
            status,
            value: value.into_raw(),
        },
    )
}

pub(crate) fn error_string(value: impl Into<String>) -> *mut c_char {
    CString::new(value.into()).map_or(std::ptr::null_mut(), CString::into_raw)
}

pub(crate) unsafe fn c_string_to_string(
    value: *const c_char,
    name: &str,
) -> Result<String, SecureTunnelStringResult> {
    if value.is_null() {
        return Err(string_result(
            SecureTunnelStatus::SecureTunnelStatusNullPointer,
            format!("{name} must not be null"),
        ));
    }
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map(str::to_owned)
        .map_err(|error| {
            string_result(
                SecureTunnelStatus::SecureTunnelStatusInvalidUtf8,
                error.to_string(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn take_result(result: SecureTunnelStringResult) -> (SecureTunnelStatus, Option<String>) {
        if result.value.is_null() {
            return (result.status, None);
        }
        let value = unsafe { CStr::from_ptr(result.value).to_string_lossy().into_owned() };
        unsafe {
            secure_tunnel_free_string(result.value);
        }
        (result.status, Some(value))
    }

    #[test]
    fn protocol_constants_are_static_c_strings() {
        let protocol_id = unsafe { CStr::from_ptr(secure_tunnel_protocol_id_v1()) }
            .to_str()
            .expect("protocol id is UTF-8");
        let wss_subprotocol = unsafe { CStr::from_ptr(secure_tunnel_wss_subprotocol_v1()) }
            .to_str()
            .expect("subprotocol is UTF-8");

        assert_eq!(protocol_id, PROTOCOL_ID_V1);
        assert_eq!(wss_subprotocol, WSS_SUBPROTOCOL_V1);
    }

    #[test]
    fn example_descriptor_json_validates() {
        let (status, value) = take_result(secure_tunnel_example_service_descriptor_json());
        assert_eq!(status, SecureTunnelStatus::SecureTunnelStatusSuccess);
        let json = CString::new(value.expect("example json")).expect("json has no NUL");

        let (status, value) =
            take_result(unsafe { secure_tunnel_validate_service_descriptor_json(json.as_ptr()) });

        assert_eq!(status, SecureTunnelStatus::SecureTunnelStatusSuccess);
        assert_eq!(value, None);
    }

    #[test]
    fn validation_rejects_null_descriptor() {
        let (status, value) = take_result(unsafe {
            secure_tunnel_validate_service_descriptor_json(std::ptr::null())
        });

        assert_eq!(status, SecureTunnelStatus::SecureTunnelStatusNullPointer);
        assert!(value.expect("message").contains("must not be null"));
    }

    #[test]
    fn normalization_rejects_invalid_descriptor_shape() {
        let mut descriptor = example_service_descriptor();
        descriptor.protocol_id = "wrong".to_owned();
        let json = serde_json::to_string(&descriptor).expect("descriptor encodes");
        let c_json = CString::new(json).expect("json has no NUL");

        let (status, value) = take_result(unsafe {
            secure_tunnel_normalize_service_descriptor_json(c_json.as_ptr())
        });

        assert_eq!(
            status,
            SecureTunnelStatus::SecureTunnelStatusInvalidDescriptor
        );
        assert!(value.expect("message").contains("protocol_id"));
    }
}
