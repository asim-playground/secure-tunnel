#ifndef SECURE_TUNNEL_FFI_H
#define SECURE_TUNNEL_FFI_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Status code for C ABI calls.
 */
typedef enum SecureTunnelStatus {
    /**
     * The operation succeeded.
     */
    SecureTunnelStatusSuccess = 0,
    /**
     * The caller supplied a null pointer.
     */
    SecureTunnelStatusNullPointer = 1,
    /**
     * The caller supplied bytes that were not valid UTF-8.
     */
    SecureTunnelStatusInvalidUtf8 = 2,
    /**
     * JSON decoding or encoding failed.
     */
    SecureTunnelStatusInvalidJson = 3,
    /**
     * The descriptor decoded but failed Secure Tunnel validation.
     */
    SecureTunnelStatusInvalidDescriptor = 4,
    /**
     * The Rust side could not allocate a caller-owned C string.
     */
    SecureTunnelStatusAllocationFailure = 5,
} SecureTunnelStatus;

/**
 * Result for FFI calls that return a caller-owned string or an error message.
 *
 * When `status` is `SecureTunnelStatusSuccess`, `value` is the returned
 * payload or null for operations with no payload. When `status` is any other
 * value, `value` is a caller-owned error message when allocation succeeds.
 * Free every non-null `value` with `secure_tunnel_free_string`.
 */
typedef struct SecureTunnelStringResult {
    /**
     * Operation status.
     */
    enum SecureTunnelStatus status;
    /**
     * Returned string payload or error message.
     */
    char *value;
} SecureTunnelStringResult;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * Returns the C ABI version implemented by this library.
 */
const char *secure_tunnel_ffi_version(void);

/**
 * Returns the stable v1 protocol identifier.
 */
const char *secure_tunnel_protocol_id_v1(void);

/**
 * Returns the v1 QUIC ALPN value.
 */
const char *secure_tunnel_quic_alpn_v1(void);

/**
 * Returns the v1 WSS subprotocol value.
 */
const char *secure_tunnel_wss_subprotocol_v1(void);

/**
 * Returns a sample service descriptor as JSON.
 */
struct SecureTunnelStringResult secure_tunnel_example_service_descriptor_json(void);

/**
 * Validates a service descriptor JSON string.
 *
 * # Safety
 *
 * `descriptor_json` must be a valid pointer to a null-terminated C string.
 * The pointer may be null, in which case a `NullPointer` result is returned.
 */
struct SecureTunnelStringResult secure_tunnel_validate_service_descriptor_json(const char *descriptor_json);

/**
 * Decodes, validates, and re-encodes a service descriptor JSON string.
 *
 * # Safety
 *
 * `descriptor_json` must be a valid pointer to a null-terminated C string.
 * The pointer may be null, in which case a `NullPointer` result is returned.
 */
struct SecureTunnelStringResult secure_tunnel_normalize_service_descriptor_json(const char *descriptor_json);

/**
 * Frees a string returned in `SecureTunnelStringResult.value`.
 *
 * # Safety
 *
 * `value` must be null or a pointer returned by this library. Do not free a
 * static string returned by one of the protocol constant functions.
 */
void secure_tunnel_free_string(char *value);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* SECURE_TUNNEL_FFI_H */
