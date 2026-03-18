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
	"time"
)

func TestProtocolConstants(t *testing.T) {
	ctx := context.Background()

	if got, err := ProtocolID(ctx); err != nil || got != "secure-tunnel-v1" {
		t.Fatalf("ProtocolID() = %q, %v", got, err)
	}
	if got, err := QuicALPN(ctx); err != nil || got != "secure-tunnel-v1" {
		t.Fatalf("QuicALPN() = %q, %v", got, err)
	}
	if got, err := WSSSubprotocol(ctx); err != nil || got != "secure-tunnel-v1" {
		t.Fatalf("WSSSubprotocol() = %q, %v", got, err)
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
	if !errors.As(err, &abiErr) || !strings.Contains(err.Error(), "protocol_id") {
		t.Fatalf("unexpected error: %v", err)
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

func TestContextCancellation(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	cancel()

	if _, err := ProtocolID(ctx); !errors.Is(err, context.Canceled) {
		t.Fatalf("expected context.Canceled error, got %v", err)
	}
}

func TestGetWasmTimestampRecoversAfterCanceledCall(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	cancel()

	if _, err := GetWasmTimestamp(ctx); err == nil {
		t.Fatal("expected canceled context to fail")
	}

	if _, err := GetWasmTimestamp(context.Background()); err != nil {
		t.Fatalf("expected later healthy call to succeed, got %v", err)
	}
}

func TestConcurrentValidation(t *testing.T) {
	ctx := context.Background()
	descriptor := MustExampleServiceDescriptorJSON()
	const count = 50

	var wg sync.WaitGroup
	errCh := make(chan error, count)

	for range count {
		wg.Add(1)
		go func() {
			defer wg.Done()
			if err := ValidateServiceDescriptorJSON(ctx, descriptor); err != nil {
				errCh <- err
			}
		}()
	}

	wg.Wait()
	close(errCh)

	for err := range errCh {
		t.Fatalf("concurrent validation failed: %v", err)
	}
}

func TestGetWasmTimestamp(t *testing.T) {
	ctx := context.Background()
	wasmTime, err := GetWasmTimestamp(ctx)
	if err != nil {
		t.Fatalf("GetWasmTimestamp failed: %v", err)
	}

	diff := time.Since(wasmTime)
	if diff < 0 {
		diff = -diff
	}
	if diff > 2*time.Second {
		t.Errorf("time difference too large: %v", diff)
	}
}
