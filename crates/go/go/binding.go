// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

// Package binding provides Go bindings for the Secure Tunnel C ABI.
package binding

/*
#cgo CFLAGS: -I${SRCDIR}/..
#cgo LDFLAGS: -lsecure_tunnel_ffi

#include "binding.h"
#include <stdlib.h>
*/
import "C"
import (
	"context"
	"encoding/base64"
	"encoding/json"
	"errors"
	"runtime"
	"strings"
	"sync"
	"unsafe"
)

// ProtocolID returns the v1 Secure Tunnel protocol identifier.
func ProtocolID() string {
	return C.GoString(C.secure_tunnel_protocol_id_v1())
}

// QuicALPN returns the v1 QUIC ALPN value.
func QuicALPN() string {
	return C.GoString(C.secure_tunnel_quic_alpn_v1())
}

// WSSSubprotocol returns the v1 WebSocket subprotocol value.
func WSSSubprotocol() string {
	return C.GoString(C.secure_tunnel_wss_subprotocol_v1())
}

// Client is an opaque Go SDK client backed by the Rust C ABI.
type Client struct {
	mu  sync.RWMutex
	ptr *C.SecureTunnelClientHandle
}

// Connection is an opaque secure tunnel session backed by the Rust C ABI.
type Connection struct {
	mu  sync.RWMutex
	ptr *C.SecureTunnelConnectionHandle
}

// ExampleServiceDescriptorJSON returns a sample descriptor JSON document.
func ExampleServiceDescriptorJSON(ctx context.Context) (string, error) {
	if err := ctx.Err(); err != nil {
		return "", err
	}
	return unwrapStringResult(C.secure_tunnel_example_service_descriptor_json())
}

// ValidateServiceDescriptorJSON validates a descriptor JSON document.
func ValidateServiceDescriptorJSON(ctx context.Context, descriptorJSON string) error {
	if err := ctx.Err(); err != nil {
		return err
	}
	if strings.ContainsRune(descriptorJSON, '\x00') {
		return errors.New("descriptor JSON contains NUL byte")
	}

	cJSON := C.CString(descriptorJSON)
	defer C.free(unsafe.Pointer(cJSON))

	_, err := unwrapStringResult(C.secure_tunnel_validate_service_descriptor_json(cJSON))
	return err
}

// NormalizeServiceDescriptorJSON validates and re-encodes a descriptor JSON document.
func NormalizeServiceDescriptorJSON(ctx context.Context, descriptorJSON string) (string, error) {
	if err := ctx.Err(); err != nil {
		return "", err
	}
	if strings.ContainsRune(descriptorJSON, '\x00') {
		return "", errors.New("descriptor JSON contains NUL byte")
	}

	cJSON := C.CString(descriptorJSON)
	defer C.free(unsafe.Pointer(cJSON))

	return unwrapStringResult(C.secure_tunnel_normalize_service_descriptor_json(cJSON))
}

// MustExampleServiceDescriptorJSON is like ExampleServiceDescriptorJSON but panics on error.
func MustExampleServiceDescriptorJSON() string {
	json, err := ExampleServiceDescriptorJSON(context.Background())
	if err != nil {
		panic(err)
	}
	return json
}

// DefaultClientConfig returns the Rust SDK default client configuration.
func DefaultClientConfig(ctx context.Context) (ClientConfig, error) {
	if err := ctx.Err(); err != nil {
		return ClientConfig{}, err
	}
	jsonValue, err := unwrapStringResult(C.secure_tunnel_default_client_config_json())
	if err != nil {
		return ClientConfig{}, err
	}
	return decodeClientConfigJSON([]byte(jsonValue))
}

// NewClient creates a Go SDK client.
func NewClient(ctx context.Context, config ClientConfig) (*Client, error) {
	if err := ctx.Err(); err != nil {
		return nil, err
	}
	configJSON, err := encodeClientConfigJSON(config)
	if err != nil {
		return nil, err
	}
	cJSON := C.CString(string(configJSON))
	defer C.free(unsafe.Pointer(cJSON))
	result := C.secure_tunnel_client_new(cJSON)
	if result.error != nil {
		defer C.secure_tunnel_free_string(result.error)
	}
	if status := int32(result.status); status != 0 {
		return nil, &ABIError{Status: status, msg: cStringOrDefault(result.error)}
	}
	client := &Client{ptr: result.client}
	runtime.SetFinalizer(client, (*Client).Close)
	return client, nil
}

// Connect creates a secure tunnel session.
func (c *Client) Connect(ctx context.Context, options ConnectOptions) (*Connection, error) {
	if err := ctx.Err(); err != nil {
		return nil, err
	}
	if c == nil {
		return nil, errors.New("secure tunnel client is closed")
	}
	if strings.ContainsRune(options.DescriptorJSON, '\x00') {
		return nil, errors.New("descriptor JSON contains NUL byte")
	}
	c.mu.RLock()
	defer c.mu.RUnlock()
	defer runtime.KeepAlive(c)
	if c.ptr == nil {
		return nil, errors.New("secure tunnel client is closed")
	}
	cJSON := C.CString(options.DescriptorJSON)
	defer C.free(unsafe.Pointer(cJSON))
	var cCacheJSON *C.char
	if options.TransportCache != nil {
		cacheJSON, err := json.Marshal(options.TransportCache)
		if err != nil {
			return nil, err
		}
		cCacheJSON = C.CString(string(cacheJSON))
		defer C.free(unsafe.Pointer(cCacheJSON))
	}
	result := C.secure_tunnel_client_connect_v2(
		c.ptr,
		cJSON,
		C.uint64_t(options.NowUnixSeconds),
		cCacheJSON,
	)
	if result.error != nil {
		defer C.secure_tunnel_free_string(result.error)
	}
	if result.error_details_json != nil {
		defer C.secure_tunnel_free_string(result.error_details_json)
	}
	if status := int32(result.status); status != 0 {
		return nil, connectResultError(status, result)
	}
	connection := &Connection{ptr: result.connection}
	runtime.SetFinalizer(connection, (*Connection).Close)
	return connection, nil
}

// Close frees the Rust client handle.
func (c *Client) Close() {
	if c == nil {
		return
	}
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.ptr == nil {
		return
	}
	ptr := c.ptr
	c.ptr = nil
	runtime.SetFinalizer(c, nil)
	C.secure_tunnel_client_free(ptr)
	runtime.KeepAlive(c)
}

// Report returns the connect report.
func (c *Connection) Report() (ConnectReport, error) {
	if c == nil {
		return ConnectReport{}, errors.New("secure tunnel connection is closed")
	}
	c.mu.RLock()
	defer c.mu.RUnlock()
	defer runtime.KeepAlive(c)
	if c.ptr == nil {
		return ConnectReport{}, errors.New("secure tunnel connection is closed")
	}
	jsonValue, err := unwrapStringResult(C.secure_tunnel_connection_report_json(c.ptr))
	if err != nil {
		return ConnectReport{}, err
	}
	var report ConnectReport
	if err := json.Unmarshal([]byte(jsonValue), &report); err != nil {
		return ConnectReport{}, err
	}
	return report, nil
}

// SecurityArtifacts returns explicit secure-channel artifacts.
func (c *Connection) SecurityArtifacts() (SecureChannelArtifacts, error) {
	if c == nil {
		return SecureChannelArtifacts{}, errors.New("secure tunnel connection is closed")
	}
	c.mu.RLock()
	defer c.mu.RUnlock()
	defer runtime.KeepAlive(c)
	if c.ptr == nil {
		return SecureChannelArtifacts{}, errors.New("secure tunnel connection is closed")
	}
	jsonValue, err := unwrapStringResult(
		C.secure_tunnel_connection_security_artifacts_json(c.ptr),
	)
	if err != nil {
		return SecureChannelArtifacts{}, err
	}
	var artifacts SecureChannelArtifacts
	if err := json.Unmarshal([]byte(jsonValue), &artifacts); err != nil {
		return SecureChannelArtifacts{}, err
	}
	if artifacts.ServiceStaticPublicKeyB64 != nil {
		serviceKey, err := base64.StdEncoding.DecodeString(*artifacts.ServiceStaticPublicKeyB64)
		if err != nil {
			return SecureChannelArtifacts{}, err
		}
		artifacts.ServiceStaticPublicKeyBytes = serviceKey
	}
	return artifacts, nil
}

// AuthenticateAccount authenticates an account session.
func (c *Connection) AuthenticateAccount(
	ctx context.Context,
	request AccountAuthRequest,
) (AccountAuthReport, error) {
	if err := ctx.Err(); err != nil {
		return AccountAuthReport{}, err
	}
	if c == nil {
		return AccountAuthReport{}, errors.New("secure tunnel connection is closed")
	}
	if strings.ContainsRune(request.AccountID, '\x00') {
		return AccountAuthReport{}, errors.New("account ID contains NUL byte")
	}
	c.mu.RLock()
	defer c.mu.RUnlock()
	defer runtime.KeepAlive(c)
	if c.ptr == nil {
		return AccountAuthReport{}, errors.New("secure tunnel connection is closed")
	}
	cAccountID := C.CString(request.AccountID)
	defer C.free(unsafe.Pointer(cAccountID))
	payload, payloadLen := bytePointer(request.CredentialPayload)
	result := C.secure_tunnel_connection_authenticate_account(
		c.ptr,
		cAccountID,
		payload,
		payloadLen,
		C.uint32_t(request.Mode),
	)
	jsonValue, err := unwrapStringResult(result)
	if err != nil {
		return AccountAuthReport{}, err
	}
	var report AccountAuthReport
	if err := json.Unmarshal([]byte(jsonValue), &report); err != nil {
		return AccountAuthReport{}, err
	}
	return report, nil
}

// Request sends one application request and returns one response.
func (c *Connection) Request(ctx context.Context, payload []byte) ([]byte, error) {
	if err := ctx.Err(); err != nil {
		return nil, err
	}
	if c == nil {
		return nil, errors.New("secure tunnel connection is closed")
	}
	c.mu.RLock()
	defer c.mu.RUnlock()
	defer runtime.KeepAlive(c)
	if c.ptr == nil {
		return nil, errors.New("secure tunnel connection is closed")
	}
	data, dataLen := bytePointer(payload)
	result := C.secure_tunnel_connection_request(c.ptr, data, dataLen)
	if result.error != nil {
		defer C.secure_tunnel_free_string(result.error)
	}
	if status := int32(result.status); status != 0 {
		return nil, &ABIError{Status: status, msg: cStringOrDefault(result.error)}
	}
	defer C.secure_tunnel_free_bytes(result.bytes)
	return copyByteBuffer(result.bytes)
}

// CloseSession closes the secure tunnel session gracefully.
func (c *Connection) CloseSession(
	ctx context.Context,
	code uint16,
	drain bool,
) (CloseReport, error) {
	if err := ctx.Err(); err != nil {
		return CloseReport{}, err
	}
	if c == nil {
		return CloseReport{}, errors.New("secure tunnel connection is closed")
	}
	c.mu.RLock()
	defer c.mu.RUnlock()
	defer runtime.KeepAlive(c)
	if c.ptr == nil {
		return CloseReport{}, errors.New("secure tunnel connection is closed")
	}
	jsonValue, err := unwrapStringResult(
		C.secure_tunnel_connection_close(c.ptr, C.uint16_t(code), C.bool(drain)),
	)
	if err != nil {
		return CloseReport{}, err
	}
	var report CloseReport
	if err := json.Unmarshal([]byte(jsonValue), &report); err != nil {
		return CloseReport{}, err
	}
	return report, nil
}

// Close frees the Rust connection handle.
func (c *Connection) Close() {
	if c == nil {
		return
	}
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.ptr == nil {
		return
	}
	ptr := c.ptr
	c.ptr = nil
	runtime.SetFinalizer(c, nil)
	C.secure_tunnel_connection_free(ptr)
	runtime.KeepAlive(c)
}

func cStringOrDefault(value *C.char) string {
	if value == nil {
		return "no message returned"
	}
	return C.GoString(value)
}

func bytePointer(value []byte) (*C.uint8_t, C.size_t) {
	if len(value) == 0 {
		return nil, 0
	}
	return (*C.uint8_t)(unsafe.Pointer(&value[0])), C.size_t(len(value))
}

func copyByteBuffer(buffer C.SecureTunnelByteBuffer) ([]byte, error) {
	if buffer.len == 0 {
		return []byte{}, nil
	}
	if buffer.data == nil {
		return nil, errors.New("secure tunnel ABI returned null byte buffer with non-zero length")
	}
	maxInt := int(^uint(0) >> 1)
	if buffer.len > C.size_t(maxInt) {
		return nil, errors.New("secure tunnel ABI returned byte buffer too large for Go")
	}
	length := int(buffer.len)
	value := make([]byte, length)
	copy(value, unsafe.Slice((*byte)(unsafe.Pointer(buffer.data)), length))
	return value, nil
}

func unwrapStringResult(result C.SecureTunnelStringResult) (string, error) {
	var value string
	if result.value != nil {
		value = C.GoString(result.value)
		C.secure_tunnel_free_string(result.value)
	}

	status := int32(result.status)
	if status != 0 {
		if value == "" {
			value = "no message returned"
		}
		return "", &ABIError{Status: status, msg: value}
	}

	return value, nil
}

func connectResultError(status int32, result C.SecureTunnelConnectionResultV2) error {
	message := cStringOrDefault(result.error)
	if result.error_details_json == nil {
		return &ABIError{Status: status, msg: message}
	}
	var connectErr ConnectError
	if err := json.Unmarshal([]byte(C.GoString(result.error_details_json)), &connectErr); err != nil {
		return &ABIError{Status: status, msg: message}
	}
	connectErr.Status = status
	if connectErr.Message == "" {
		connectErr.Message = message
	}
	return &connectErr
}
