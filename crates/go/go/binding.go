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
	"errors"
	"fmt"
	"strings"
	"unsafe"
)

// Version of the binding package
const Version = "1.0.0"

// ABIError represents an error returned by the Secure Tunnel C ABI.
type ABIError struct {
	Status int32
	msg    string
}

func (e *ABIError) Error() string {
	return fmt.Sprintf("secure tunnel ABI status %d: %s", e.Status, e.msg)
}

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
