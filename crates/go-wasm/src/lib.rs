// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! WASM-exported helpers used by the Go/WASI binding.

use secure_tunnel_core::{ServiceDescriptor, example_service_descriptor};
use std::alloc::Layout;
use std::ptr;
use std::time::{SystemTime, UNIX_EPOCH};

// Initialize Talc as the global allocator for single-threaded WebAssembly.
#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: talc::wasm::WasmDynamicTalc = talc::wasm::new_wasm_dynamic_allocator();

/// Allocates memory that can be accessed from the host.
///
/// # Safety
///
/// Returns a pointer to the allocated memory.
/// The memory must be freed using `deallocate`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate(size: usize) -> *mut u8 {
    allocation_layout(size).map_or(ptr::null_mut(), |layout| unsafe {
        std::alloc::alloc(layout)
    })
}

/// Deallocates memory previously allocated with `allocate`.
///
/// # Safety
///
/// The pointer must have been allocated with `allocate`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn deallocate(ptr: *mut u8, size: usize) {
    if !ptr.is_null()
        && let Some(layout) = allocation_layout(size)
    {
        unsafe {
            std::alloc::dealloc(ptr, layout);
        }
    }
}

/// Return the stable v1 protocol identifier as a pointer/length pair.
#[unsafe(no_mangle)]
pub extern "C" fn protocol_id_v1() -> u64 {
    pack_bytes(secure_tunnel_core::PROTOCOL_ID_V1.as_bytes())
}

/// Return the v1 QUIC ALPN value as a pointer/length pair.
#[unsafe(no_mangle)]
pub extern "C" fn quic_alpn_v1() -> u64 {
    pack_bytes(secure_tunnel_core::QUIC_ALPN_V1.as_bytes())
}

/// Return the v1 WSS subprotocol value as a pointer/length pair.
#[unsafe(no_mangle)]
pub extern "C" fn wss_subprotocol_v1() -> u64 {
    pack_bytes(secure_tunnel_core::WSS_SUBPROTOCOL_V1.as_bytes())
}

/// Return a sample service descriptor JSON document as a pointer/length pair.
#[unsafe(no_mangle)]
pub extern "C" fn example_service_descriptor_json() -> u64 {
    match serde_json::to_string(&example_service_descriptor()) {
        Ok(json) => pack_bytes(json.as_bytes()),
        Err(error) => pack_error(&error.to_string()),
    }
}

/// Validate a descriptor JSON document and return "ok" or an error.
///
/// # Safety
///
/// The caller must ensure that `ptr` points to valid memory with `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn validate_service_descriptor_json(ptr: *const u8, len: usize) -> u64 {
    match decode_descriptor(ptr, len) {
        Ok(descriptor) => match descriptor.validate() {
            Ok(()) => pack_bytes(b"ok"),
            Err(error) => pack_error(&error.to_string()),
        },
        Err(error) => pack_error(&error),
    }
}

/// Decode, validate, and re-encode a descriptor JSON document.
///
/// # Safety
///
/// The caller must ensure that `ptr` points to valid memory with `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn normalize_service_descriptor_json(ptr: *const u8, len: usize) -> u64 {
    match decode_descriptor(ptr, len) {
        Ok(descriptor) => match descriptor.validate() {
            Ok(()) => match serde_json::to_string(&descriptor) {
                Ok(json) => pack_bytes(json.as_bytes()),
                Err(error) => pack_error(&error.to_string()),
            },
            Err(error) => pack_error(&error.to_string()),
        },
        Err(error) => pack_error(&error),
    }
}

/// Check whether a pointer/length pair returned by this module is an error.
///
/// # Safety
///
/// The caller must ensure the ptr/len pair was obtained from this module and
/// the memory is still valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn is_secure_tunnel_error(ptr: *const u8, len: usize) -> i32 {
    if ptr.is_null() || len < 6 {
        return 0;
    }

    let slice = unsafe { std::slice::from_raw_parts(ptr, 6.min(len)) };
    i32::from(slice == b"Error:")
}

/// Get the current timestamp in milliseconds since the UNIX epoch.
#[unsafe(no_mangle)]
pub extern "C" fn get_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

fn decode_descriptor(ptr: *const u8, len: usize) -> Result<ServiceDescriptor, String> {
    if ptr.is_null() {
        return Err("descriptor pointer is null".to_owned());
    }

    let input_bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    let input = std::str::from_utf8(input_bytes).map_err(|error| error.to_string())?;
    serde_json::from_str::<ServiceDescriptor>(input).map_err(|error| error.to_string())
}

fn allocation_layout(size: usize) -> Option<Layout> {
    Layout::from_size_align(size.max(1), 8).ok()
}

fn pack_error(message: &str) -> u64 {
    pack_bytes(format!("Error: {message}").as_bytes())
}

fn pack_bytes(bytes: &[u8]) -> u64 {
    let result_len = bytes.len();
    let result_ptr = unsafe { allocate(result_len) };
    if result_ptr.is_null() {
        return 0;
    }

    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), result_ptr, result_len);
    }

    ((result_ptr as u64) << 32) | (result_len as u64)
}
