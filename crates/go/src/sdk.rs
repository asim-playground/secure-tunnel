// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use std::ffi::c_char;
use std::sync::Arc;

mod json;
mod result;

use self::json::{
    GoAccountAuthReportJson, GoClientConfigJson, GoConnectErrorJson, GoSecureChannelArtifactsJson,
    decode_config, decode_transport_cache_json, encode_json_string,
};
use self::result::{
    bytes_error, bytes_from_ptr, bytes_success, client_error, connection_error_v2,
    connection_error_with_details_v2, take_result_message,
};

use crate::{SecureTunnelStatus, SecureTunnelStringResult, c_string_to_string};

/// Caller-owned byte buffer returned by the C ABI.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SecureTunnelByteBuffer {
    /// Pointer to the first byte, or null when `len` is zero.
    pub data: *mut u8,
    /// Number of bytes in `data`.
    pub len: usize,
}

/// Result for C ABI calls that return bytes.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SecureTunnelBytesResult {
    /// Operation status.
    pub status: SecureTunnelStatus,
    /// Caller-owned error message when status is not success.
    pub error: *mut c_char,
    /// Caller-owned byte buffer when status is success.
    pub bytes: SecureTunnelByteBuffer,
}

/// Result for C ABI calls that return a client handle.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SecureTunnelClientResult {
    /// Operation status.
    pub status: SecureTunnelStatus,
    /// Caller-owned error message when status is not success.
    pub error: *mut c_char,
    /// Caller-owned client handle when status is success.
    pub client: *mut SecureTunnelClientHandle,
}

/// Result for C ABI calls that return a connection handle.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SecureTunnelConnectionResult {
    /// Operation status.
    pub status: SecureTunnelStatus,
    /// Caller-owned error message when status is not success.
    pub error: *mut c_char,
    /// Caller-owned connection handle when status is success.
    pub connection: *mut SecureTunnelConnectionHandle,
}

/// Result for C ABI v2 connect calls that can return structured errors.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SecureTunnelConnectionResultV2 {
    /// Operation status.
    pub status: SecureTunnelStatus,
    /// Caller-owned error message when status is not success.
    pub error: *mut c_char,
    /// Caller-owned structured connect error JSON when available.
    pub error_details_json: *mut c_char,
    /// Caller-owned connection handle when status is success.
    pub connection: *mut SecureTunnelConnectionHandle,
}

/// Account authentication mode for the C ABI.
///
/// Exported for named constants in generated headers. Entry points accept raw
/// integers and validate them before mapping to Rust domain enums.
#[repr(C)]
#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SecureTunnelAccountAuthMode {
    /// Authenticate with current account credentials.
    SecureTunnelAccountAuthModeFresh = 0,
    /// Resume a previous account session.
    SecureTunnelAccountAuthModeResume = 1,
}

/// Opaque SDK client handle owned by the caller.
pub struct SecureTunnelClientHandle {
    client: secure_tunnel_sdk::SecureTunnelClient,
    runtime: Arc<tokio::runtime::Runtime>,
}

/// Opaque SDK connection/session handle owned by the caller.
pub struct SecureTunnelConnectionHandle {
    session: secure_tunnel_sdk::SecureTunnelSession,
    report: secure_tunnel_sdk::ConnectReport,
    artifacts: secure_tunnel_sdk::SecureChannelArtifacts,
    runtime: Arc<tokio::runtime::Runtime>,
}

/// Returns the default Go SDK client configuration as JSON.
#[unsafe(no_mangle)]
pub extern "C" fn secure_tunnel_default_client_config_json() -> SecureTunnelStringResult {
    let config = GoClientConfigJson::from_sdk_default();
    encode_json_string(config, SecureTunnelStatus::SecureTunnelStatusInvalidJson)
}

/// Creates a Secure Tunnel SDK client from JSON config.
///
/// # Safety
///
/// `config_json` must be null or a valid pointer to a null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn secure_tunnel_client_new(
    config_json: *const c_char,
) -> SecureTunnelClientResult {
    let config = if config_json.is_null() {
        Ok(secure_tunnel_sdk::ClientConfig::default())
    } else {
        decode_config(unsafe { c_string_to_string(config_json, "config_json") })
    };
    let config = match config {
        Ok(config) => config,
        Err(error) => return client_error(error.status, take_result_message(error)),
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => Arc::new(runtime),
        Err(error) => {
            return client_error(
                SecureTunnelStatus::SecureTunnelStatusRuntimeFailure,
                error.to_string(),
            );
        }
    };
    let handle = SecureTunnelClientHandle {
        client: secure_tunnel_sdk::SecureTunnelClient::new(config),
        runtime,
    };
    SecureTunnelClientResult {
        status: SecureTunnelStatus::SecureTunnelStatusSuccess,
        error: std::ptr::null_mut(),
        client: Box::into_raw(Box::new(handle)),
    }
}

/// Connects a client to a service descriptor.
///
/// # Safety
///
/// `client` must be a handle returned by `secure_tunnel_client_new`.
/// `descriptor_json` must be a valid pointer to a null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn secure_tunnel_client_connect(
    client: *mut SecureTunnelClientHandle,
    descriptor_json: *const c_char,
    now_unix_seconds: u64,
    transport_cache_json: *const c_char,
) -> SecureTunnelConnectionResult {
    let result = unsafe {
        secure_tunnel_client_connect_v2(
            client,
            descriptor_json,
            now_unix_seconds,
            transport_cache_json,
        )
    };
    if !result.error_details_json.is_null() {
        unsafe {
            crate::secure_tunnel_free_string(result.error_details_json);
        }
    }
    SecureTunnelConnectionResult {
        status: result.status,
        error: result.error,
        connection: result.connection,
    }
}

/// Connects a client to a service descriptor and returns structured failures.
///
/// # Safety
///
/// `client` must be a handle returned by `secure_tunnel_client_new`.
/// `descriptor_json` must be a valid pointer to a null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn secure_tunnel_client_connect_v2(
    client: *mut SecureTunnelClientHandle,
    descriptor_json: *const c_char,
    now_unix_seconds: u64,
    transport_cache_json: *const c_char,
) -> SecureTunnelConnectionResultV2 {
    let Some(client) = (unsafe { client.as_ref() }) else {
        return connection_error_v2(
            SecureTunnelStatus::SecureTunnelStatusNullPointer,
            "client must not be null",
        );
    };
    let descriptor_json = match unsafe { c_string_to_string(descriptor_json, "descriptor_json") } {
        Ok(value) => value,
        Err(error) => return connection_error_v2(error.status, take_result_message(error)),
    };
    let descriptor = match secure_tunnel_sdk::BootstrapDescriptor::from_json(&descriptor_json) {
        Ok(descriptor) => descriptor,
        Err(error) => {
            return connection_error_with_details_v2(
                SecureTunnelStatus::SecureTunnelStatusInvalidDescriptor,
                error.message(),
                GoConnectErrorJson::from_sdk_error(&error),
            );
        }
    };
    let transport_cache_json = if transport_cache_json.is_null() {
        None
    } else {
        match unsafe { c_string_to_string(transport_cache_json, "transport_cache_json") } {
            Ok(value) => Some(value),
            Err(error) => return connection_error_v2(error.status, take_result_message(error)),
        }
    };
    let transport_cache = match decode_transport_cache_json(transport_cache_json) {
        Ok(value) => value,
        Err(error) => return connection_error_v2(error.status, take_result_message(error)),
    };
    let mut options = secure_tunnel_sdk::ConnectOptions::new(descriptor, now_unix_seconds);
    if let Some(transport_cache) = transport_cache {
        options = options.with_transport_cache(transport_cache);
    }
    let outcome = client.runtime.block_on(client.client.connect(options));
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(error) => {
            return connection_error_with_details_v2(
                SecureTunnelStatus::SecureTunnelStatusConnectFailure,
                error.message(),
                GoConnectErrorJson::from_connect_error(&error),
            );
        }
    };
    let handle = SecureTunnelConnectionHandle {
        session: outcome.session,
        report: outcome.report,
        artifacts: outcome.artifacts,
        runtime: Arc::clone(&client.runtime),
    };
    SecureTunnelConnectionResultV2 {
        status: SecureTunnelStatus::SecureTunnelStatusSuccess,
        error: std::ptr::null_mut(),
        error_details_json: std::ptr::null_mut(),
        connection: Box::into_raw(Box::new(handle)),
    }
}

/// Returns the connection report as JSON.
///
/// # Safety
///
/// `connection` must be a handle returned by `secure_tunnel_client_connect`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn secure_tunnel_connection_report_json(
    connection: *const SecureTunnelConnectionHandle,
) -> SecureTunnelStringResult {
    let Some(connection) = (unsafe { connection.as_ref() }) else {
        return crate::string_result(
            SecureTunnelStatus::SecureTunnelStatusNullPointer,
            "connection must not be null",
        );
    };
    encode_json_string(
        &connection.report,
        SecureTunnelStatus::SecureTunnelStatusInvalidJson,
    )
}

/// Returns secure-channel artifacts as JSON with base64 byte fields.
///
/// # Safety
///
/// `connection` must be a handle returned by `secure_tunnel_client_connect`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn secure_tunnel_connection_security_artifacts_json(
    connection: *const SecureTunnelConnectionHandle,
) -> SecureTunnelStringResult {
    let Some(connection) = (unsafe { connection.as_ref() }) else {
        return crate::string_result(
            SecureTunnelStatus::SecureTunnelStatusNullPointer,
            "connection must not be null",
        );
    };
    encode_json_string(
        GoSecureChannelArtifactsJson::from(&connection.artifacts),
        SecureTunnelStatus::SecureTunnelStatusInvalidJson,
    )
}

/// Authenticates an account session and returns an account report JSON string.
///
/// # Safety
///
/// `connection` must be a valid connection handle. `account_id` must be a
/// non-null C string. `credential_payload` must be null only when
/// `credential_payload_len` is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn secure_tunnel_connection_authenticate_account(
    connection: *const SecureTunnelConnectionHandle,
    account_id: *const c_char,
    credential_payload: *const u8,
    credential_payload_len: usize,
    mode: u32,
) -> SecureTunnelStringResult {
    let Some(connection) = (unsafe { connection.as_ref() }) else {
        return crate::string_result(
            SecureTunnelStatus::SecureTunnelStatusNullPointer,
            "connection must not be null",
        );
    };
    let account_id = match unsafe { c_string_to_string(account_id, "account_id") } {
        Ok(value) => value,
        Err(error) => return error,
    };
    let credential_payload = match bytes_from_ptr(credential_payload, credential_payload_len) {
        Ok(value) => value,
        Err(error) => return error,
    };
    let mode = match mode {
        0 => secure_tunnel_sdk::AccountAuthMode::Fresh,
        1 => secure_tunnel_sdk::AccountAuthMode::Resume,
        _ => {
            return crate::string_result(
                SecureTunnelStatus::SecureTunnelStatusInvalidConfig,
                "account auth mode must be 0 or 1",
            );
        }
    };
    let request = secure_tunnel_sdk::AccountAuthRequest {
        account_id,
        credential_payload,
        mode,
    };
    let report = connection
        .runtime
        .block_on(connection.session.authenticate_account(request));
    match report {
        Ok(report) => encode_json_string(
            GoAccountAuthReportJson::from(report),
            SecureTunnelStatus::SecureTunnelStatusInvalidJson,
        ),
        Err(error) => crate::string_result(
            SecureTunnelStatus::SecureTunnelStatusSessionFailure,
            error.to_string(),
        ),
    }
}

/// Sends one request payload and returns the response bytes.
///
/// # Safety
///
/// `connection` must be a valid connection handle. `payload` must be null only
/// when `payload_len` is zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn secure_tunnel_connection_request(
    connection: *const SecureTunnelConnectionHandle,
    payload: *const u8,
    payload_len: usize,
) -> SecureTunnelBytesResult {
    let Some(connection) = (unsafe { connection.as_ref() }) else {
        return bytes_error(
            SecureTunnelStatus::SecureTunnelStatusNullPointer,
            "connection must not be null",
        );
    };
    let payload = match bytes_from_ptr(payload, payload_len) {
        Ok(value) => value,
        Err(error) => return bytes_error(error.status, take_result_message(error)),
    };
    match connection
        .runtime
        .block_on(connection.session.request(payload))
    {
        Ok(Some(response)) => bytes_success(response),
        Ok(None) => bytes_error(
            SecureTunnelStatus::SecureTunnelStatusSessionFailure,
            "missing application response",
        ),
        Err(error) => bytes_error(
            SecureTunnelStatus::SecureTunnelStatusSessionFailure,
            error.to_string(),
        ),
    }
}

/// Closes a session and returns the close report as JSON.
///
/// # Safety
///
/// `connection` must be a valid connection handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn secure_tunnel_connection_close(
    connection: *const SecureTunnelConnectionHandle,
    code: u16,
    drain: bool,
) -> SecureTunnelStringResult {
    let Some(connection) = (unsafe { connection.as_ref() }) else {
        return crate::string_result(
            SecureTunnelStatus::SecureTunnelStatusNullPointer,
            "connection must not be null",
        );
    };
    match connection
        .runtime
        .block_on(connection.session.close(code, drain))
    {
        Ok(report) => encode_json_string(report, SecureTunnelStatus::SecureTunnelStatusInvalidJson),
        Err(error) => crate::string_result(
            SecureTunnelStatus::SecureTunnelStatusSessionFailure,
            error.to_string(),
        ),
    }
}

/// Frees a client handle returned by `secure_tunnel_client_new`.
///
/// # Safety
///
/// `client` must be null or a pointer returned by `secure_tunnel_client_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn secure_tunnel_client_free(client: *mut SecureTunnelClientHandle) {
    if !client.is_null() {
        unsafe {
            drop(Box::from_raw(client));
        }
    }
}

/// Frees a connection handle returned by `secure_tunnel_client_connect`.
///
/// # Safety
///
/// `connection` must be null or a pointer returned by
/// `secure_tunnel_client_connect`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn secure_tunnel_connection_free(
    connection: *mut SecureTunnelConnectionHandle,
) {
    if !connection.is_null() {
        unsafe {
            drop(Box::from_raw(connection));
        }
    }
}

/// Frees bytes returned in `SecureTunnelBytesResult.bytes`.
///
/// # Safety
///
/// `buffer` must be empty or a buffer returned by this library.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn secure_tunnel_free_bytes(buffer: SecureTunnelByteBuffer) {
    if !buffer.data.is_null() {
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                buffer.data,
                buffer.len,
            )));
        }
    }
}
