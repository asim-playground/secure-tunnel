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
    /**
     * Caller configuration was invalid.
     */
    SecureTunnelStatusInvalidConfig = 6,
    /**
     * Creating a runtime or opaque handle failed.
     */
    SecureTunnelStatusRuntimeFailure = 7,
    /**
     * The SDK connect operation failed.
     */
    SecureTunnelStatusConnectFailure = 8,
    /**
     * An authenticated session operation failed.
     */
    SecureTunnelStatusSessionFailure = 9,
} SecureTunnelStatus;

/**
 * Account authentication mode for the C ABI.
 *
 * Exported for named constants in generated headers. Entry points accept raw
 * integers and validate them before mapping to Rust domain enums.
 */
typedef enum SecureTunnelAccountAuthMode {
    /**
     * Authenticate with current account credentials.
     */
    SecureTunnelAccountAuthModeFresh = 0,
    /**
     * Resume a previous account session.
     */
    SecureTunnelAccountAuthModeResume = 1,
} SecureTunnelAccountAuthMode;

/**
 * Opaque SDK client handle owned by the caller.
 */
typedef struct SecureTunnelClientHandle SecureTunnelClientHandle;

/**
 * Opaque SDK connection/session handle owned by the caller.
 */
typedef struct SecureTunnelConnectionHandle SecureTunnelConnectionHandle;

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

/**
 * Result for C ABI calls that return a client handle.
 */
typedef struct SecureTunnelClientResult {
    /**
     * Operation status.
     */
    enum SecureTunnelStatus status;
    /**
     * Caller-owned error message when status is not success.
     */
    char *error;
    /**
     * Caller-owned client handle when status is success.
     */
    struct SecureTunnelClientHandle *client;
} SecureTunnelClientResult;

/**
 * Result for C ABI calls that return a connection handle.
 */
typedef struct SecureTunnelConnectionResult {
    /**
     * Operation status.
     */
    enum SecureTunnelStatus status;
    /**
     * Caller-owned error message when status is not success.
     */
    char *error;
    /**
     * Caller-owned connection handle when status is success.
     */
    struct SecureTunnelConnectionHandle *connection;
} SecureTunnelConnectionResult;

/**
 * Result for C ABI v2 connect calls that can return structured errors.
 */
typedef struct SecureTunnelConnectionResultV2 {
    /**
     * Operation status.
     */
    enum SecureTunnelStatus status;
    /**
     * Caller-owned error message when status is not success.
     */
    char *error;
    /**
     * Caller-owned structured connect error JSON when available.
     */
    char *error_details_json;
    /**
     * Caller-owned connection handle when status is success.
     */
    struct SecureTunnelConnectionHandle *connection;
} SecureTunnelConnectionResultV2;

/**
 * Caller-owned byte buffer returned by the C ABI.
 */
typedef struct SecureTunnelByteBuffer {
    /**
     * Pointer to the first byte, or null when `len` is zero.
     */
    uint8_t *data;
    /**
     * Number of bytes in `data`.
     */
    uintptr_t len;
} SecureTunnelByteBuffer;

/**
 * Result for C ABI calls that return bytes.
 */
typedef struct SecureTunnelBytesResult {
    /**
     * Operation status.
     */
    enum SecureTunnelStatus status;
    /**
     * Caller-owned error message when status is not success.
     */
    char *error;
    /**
     * Caller-owned byte buffer when status is success.
     */
    struct SecureTunnelByteBuffer bytes;
} SecureTunnelBytesResult;

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

/**
 * Returns the default Go SDK client configuration as JSON.
 */
struct SecureTunnelStringResult secure_tunnel_default_client_config_json(void);

/**
 * Creates a Secure Tunnel SDK client from JSON config.
 *
 * # Safety
 *
 * `config_json` must be null or a valid pointer to a null-terminated C string.
 */
struct SecureTunnelClientResult secure_tunnel_client_new(const char *config_json);

/**
 * Connects a client to a service descriptor.
 *
 * # Safety
 *
 * `client` must be a handle returned by `secure_tunnel_client_new`.
 * `descriptor_json` must be a valid pointer to a null-terminated C string.
 */
struct SecureTunnelConnectionResult secure_tunnel_client_connect(struct SecureTunnelClientHandle *client,
                                                                 const char *descriptor_json,
                                                                 uint64_t now_unix_seconds,
                                                                 const char *transport_cache_json);

/**
 * Connects a client to a service descriptor and returns structured failures.
 *
 * # Safety
 *
 * `client` must be a handle returned by `secure_tunnel_client_new`.
 * `descriptor_json` must be a valid pointer to a null-terminated C string.
 */
struct SecureTunnelConnectionResultV2 secure_tunnel_client_connect_v2(struct SecureTunnelClientHandle *client,
                                                                      const char *descriptor_json,
                                                                      uint64_t now_unix_seconds,
                                                                      const char *transport_cache_json);

/**
 * Returns the connection report as JSON.
 *
 * # Safety
 *
 * `connection` must be a handle returned by `secure_tunnel_client_connect`.
 */
struct SecureTunnelStringResult secure_tunnel_connection_report_json(const struct SecureTunnelConnectionHandle *connection);

/**
 * Returns secure-channel artifacts as JSON with base64 byte fields.
 *
 * # Safety
 *
 * `connection` must be a handle returned by `secure_tunnel_client_connect`.
 */
struct SecureTunnelStringResult secure_tunnel_connection_security_artifacts_json(const struct SecureTunnelConnectionHandle *connection);

/**
 * Authenticates an account session and returns an account report JSON string.
 *
 * # Safety
 *
 * `connection` must be a valid connection handle. `account_id` must be a
 * non-null C string. `credential_payload` must be null only when
 * `credential_payload_len` is zero.
 */
struct SecureTunnelStringResult secure_tunnel_connection_authenticate_account(const struct SecureTunnelConnectionHandle *connection,
                                                                              const char *account_id,
                                                                              const uint8_t *credential_payload,
                                                                              uintptr_t credential_payload_len,
                                                                              uint32_t mode);

/**
 * Sends one request payload and returns the response bytes.
 *
 * # Safety
 *
 * `connection` must be a valid connection handle. `payload` must be null only
 * when `payload_len` is zero.
 */
struct SecureTunnelBytesResult secure_tunnel_connection_request(const struct SecureTunnelConnectionHandle *connection,
                                                                const uint8_t *payload,
                                                                uintptr_t payload_len);

/**
 * Closes a session and returns the close report as JSON.
 *
 * # Safety
 *
 * `connection` must be a valid connection handle.
 */
struct SecureTunnelStringResult secure_tunnel_connection_close(const struct SecureTunnelConnectionHandle *connection,
                                                               uint16_t code,
                                                               bool drain);

/**
 * Frees a client handle returned by `secure_tunnel_client_new`.
 *
 * # Safety
 *
 * `client` must be null or a pointer returned by `secure_tunnel_client_new`.
 */
void secure_tunnel_client_free(struct SecureTunnelClientHandle *client);

/**
 * Frees a connection handle returned by `secure_tunnel_client_connect`.
 *
 * # Safety
 *
 * `connection` must be null or a pointer returned by
 * `secure_tunnel_client_connect`.
 */
void secure_tunnel_connection_free(struct SecureTunnelConnectionHandle *connection);

/**
 * Frees bytes returned in `SecureTunnelBytesResult.bytes`.
 *
 * # Safety
 *
 * `buffer` must be empty or a buffer returned by this library.
 */
void secure_tunnel_free_bytes(struct SecureTunnelByteBuffer buffer);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* SECURE_TUNNEL_FFI_H */
