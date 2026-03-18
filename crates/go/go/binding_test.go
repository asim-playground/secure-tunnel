// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

package binding

import (
	"context"
	"errors"
	"strings"
	"sync"
	"testing"
)

func TestProtocolConstants(t *testing.T) {
	if got := ProtocolID(); got != "secure-tunnel-v1" {
		t.Fatalf("ProtocolID() = %q", got)
	}
	if got := QuicALPN(); got != "secure-tunnel-v1" {
		t.Fatalf("QuicALPN() = %q", got)
	}
	if got := WSSSubprotocol(); got != "secure-tunnel-v1" {
		t.Fatalf("WSSSubprotocol() = %q", got)
	}
}

func TestExampleDescriptorValidates(t *testing.T) {
	ctx := context.Background()
	descriptor, err := ExampleServiceDescriptorJSON(ctx)
	if err != nil {
		t.Fatalf("ExampleServiceDescriptorJSON() error = %v", err)
	}
	if !strings.Contains(descriptor, `"service_id":"secure-tunnel-api"`) {
		t.Fatalf("example descriptor missing service id: %s", descriptor)
	}

	if err := ValidateServiceDescriptorJSON(ctx, descriptor); err != nil {
		t.Fatalf("ValidateServiceDescriptorJSON() error = %v", err)
	}
}

func TestNormalizeDescriptorRejectsInvalidProtocol(t *testing.T) {
	ctx := context.Background()
	descriptor := strings.Replace(
		MustExampleServiceDescriptorJSON(),
		`"protocol_id":"secure-tunnel-v1"`,
		`"protocol_id":"wrong"`,
		1,
	)

	_, err := NormalizeServiceDescriptorJSON(ctx, descriptor)
	if err == nil {
		t.Fatal("expected invalid descriptor error")
	}
	var abiErr *ABIError
	if !strings.Contains(err.Error(), "protocol_id") || !strings.Contains(err.Error(), "status 4") {
		t.Fatalf("unexpected error: %v", err)
	}
	if ok := errors.As(err, &abiErr); !ok {
		t.Fatalf("expected ABIError, got %T", err)
	}
	if abiErr.Status != 4 {
		t.Fatalf("ABIError status = %d, want 4", abiErr.Status)
	}
}

func TestValidateDescriptorRejectsInvalidWSSSubprotocol(t *testing.T) {
	ctx := context.Background()
	descriptor := strings.Replace(
		MustExampleServiceDescriptorJSON(),
		`"subprotocol":"secure-tunnel-v1"`,
		`"subprotocol":"wrong"`,
		1,
	)

	err := ValidateServiceDescriptorJSON(ctx, descriptor)
	if err == nil {
		t.Fatal("expected invalid descriptor error")
	}
	if !strings.Contains(err.Error(), "WSS subprotocol") {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestValidateRejectsEmbeddedNUL(t *testing.T) {
	err := ValidateServiceDescriptorJSON(context.Background(), "x\x00y")
	if err == nil {
		t.Fatal("expected embedded NUL input to fail")
	}
}

func TestContextCancellation(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	cancel()

	if _, err := ExampleServiceDescriptorJSON(ctx); err == nil {
		t.Fatal("expected cancelled context to fail")
	}
}

func TestConcurrentValidation(t *testing.T) {
	ctx := context.Background()
	descriptor := MustExampleServiceDescriptorJSON()

	const goroutines = 50
	var wg sync.WaitGroup
	wg.Add(goroutines)

	for range goroutines {
		go func() {
			defer wg.Done()
			if err := ValidateServiceDescriptorJSON(ctx, descriptor); err != nil {
				t.Errorf("concurrent validation failed: %v", err)
			}
		}()
	}

	wg.Wait()
}
