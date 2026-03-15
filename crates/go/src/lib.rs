// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Go FFI bindings for the bootstrap Secure Tunnel scaffold.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use secure_tunnel_core::parse;

/// Version of the FFI interface
pub const VERSION: &str = "1.0.0";

/// Error codes returned by the FFI functions
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub enum ErrorCode {
    /// Success status.
    Success = 0,
    /// Caller supplied invalid input data.
    InvalidInput = 1,
    /// Caller supplied a null pointer.
    NullPointer = 2,
    /// The scaffold parser returned an error.
    ParseError = 3,
    /// Unclassified failure.
    Unknown = 4,
}

/// Exported FFI function to parse an arithmetic expression.
///
/// # Safety
///
/// The caller must ensure:
/// * input is a valid pointer to a null-terminated C string
/// * The C string contains valid UTF-8 data
///
/// The returned string must be freed using `free_rust_string()`.
///
/// # Return value
///
/// Returns a pointer to a null-terminated C string containing either:
/// * The result of the calculation on success
/// * An error message prefixed with "Error: " on failure
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_expression(input: *const c_char) -> *mut c_char {
    if input.is_null() {
        return CString::new("Error: null pointer provided")
            .unwrap_or_default()
            .into_raw();
    }

    let Ok(input_str) = unsafe { CStr::from_ptr(input) }.to_str() else {
        return CString::new("Error: invalid UTF-8 in input")
            .unwrap_or_default()
            .into_raw();
    };

    match parse(input_str) {
        Ok(result) => CString::new(result).unwrap_or_default().into_raw(),
        Err(e) => CString::new(format!("Error: {e}"))
            .unwrap_or_default()
            .into_raw(),
    }
}

/// Helper function to free strings returned by `parse_expression()`.
///
/// # Safety
///
/// The caller must ensure:
/// * s is either null or a pointer returned by `parse_expression()`
/// * The string is not used after being freed
/// * The string is not freed more than once
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_rust_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_expression() {
        let input = CString::new("1+2").unwrap();
        let result = unsafe {
            let ptr = parse_expression(input.as_ptr());
            let output = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            free_rust_string(ptr);
            output
        };
        assert_eq!(result, "3");
    }

    #[test]
    fn test_null_input() {
        let result = unsafe {
            let ptr = parse_expression(std::ptr::null());
            let output = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            free_rust_string(ptr);
            output
        };
        assert!(result.starts_with("Error: null pointer"));
    }

    #[test]
    fn test_invalid_utf8() {
        let bytes = [0xFF, 0, 0, 0];
        let result = unsafe {
            let ptr = parse_expression(bytes.as_ptr().cast::<c_char>());
            let output = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            free_rust_string(ptr);
            output
        };
        assert!(result.starts_with("Error: invalid UTF-8"));
    }
}
