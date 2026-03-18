// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

package binding

import (
	"context"
	"crypto/rand"
	_ "embed"
	"errors"
	"fmt"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
	"github.com/tetratelabs/wazero/imports/wasi_snapshot_preview1"
)

// Version of the binding package
const Version = "1.0.0"

var (
	runtimeOnce sync.Once
	runtime     wazero.Runtime
	runtimeMu   sync.RWMutex

	compiledModuleOnce sync.Once
	compiledModule     wazero.CompiledModule
	compiledModuleErr  error

	moduleNameCounter uint64

	//go:embed secure_tunnel_go_wasm.wasm
	wasmBytes []byte
)

// ABIError represents an error returned by the WASM ABI.
type ABIError struct {
	msg string
}

func (e *ABIError) Error() string {
	return e.msg
}

// ProtocolID returns the v1 Secure Tunnel protocol identifier.
func ProtocolID(ctx context.Context) (string, error) {
	return callStringExport(ctx, "protocol_id_v1")
}

// QuicALPN returns the v1 QUIC ALPN value.
func QuicALPN(ctx context.Context) (string, error) {
	return callStringExport(ctx, "quic_alpn_v1")
}

// WSSSubprotocol returns the v1 WebSocket subprotocol value.
func WSSSubprotocol(ctx context.Context) (string, error) {
	return callStringExport(ctx, "wss_subprotocol_v1")
}

// ExampleServiceDescriptorJSON returns a sample descriptor JSON document.
func ExampleServiceDescriptorJSON(ctx context.Context) (string, error) {
	return callStringExport(ctx, "example_service_descriptor_json")
}

// ValidateServiceDescriptorJSON validates a descriptor JSON document.
func ValidateServiceDescriptorJSON(ctx context.Context, descriptorJSON string) error {
	if strings.ContainsRune(descriptorJSON, '\x00') {
		return errors.New("descriptor JSON contains NUL byte")
	}
	_, err := callStringExportWithInput(ctx, "validate_service_descriptor_json", descriptorJSON)
	return err
}

// NormalizeServiceDescriptorJSON validates and re-encodes a descriptor JSON document.
func NormalizeServiceDescriptorJSON(ctx context.Context, descriptorJSON string) (string, error) {
	if strings.ContainsRune(descriptorJSON, '\x00') {
		return "", errors.New("descriptor JSON contains NUL byte")
	}
	return callStringExportWithInput(ctx, "normalize_service_descriptor_json", descriptorJSON)
}

// MustExampleServiceDescriptorJSON is like ExampleServiceDescriptorJSON but panics on error.
func MustExampleServiceDescriptorJSON() string {
	descriptor, err := ExampleServiceDescriptorJSON(context.Background())
	if err != nil {
		panic(err)
	}
	return descriptor
}

// GetWasmTimestamp returns the current timestamp as reported by the WASM module.
func GetWasmTimestamp(ctx context.Context) (time.Time, error) {
	module, err := instantiateModule(ctx)
	if err != nil {
		return time.Time{}, err
	}
	defer module.Close(ctx) //nolint:errcheck

	getTimestamp := module.ExportedFunction("get_timestamp_ms")
	if getTimestamp == nil {
		return time.Time{}, errors.New("get_timestamp_ms function not exported from WASM module")
	}

	results, err := getTimestamp.Call(ctx)
	if err != nil {
		return time.Time{}, fmt.Errorf("failed to call get_timestamp_ms: %w", err)
	}

	return time.Unix(0, int64(results[0])*int64(time.Millisecond)), nil
}

// Close cleans up the WASM runtime.
func Close(ctx context.Context) error {
	runtimeMu.Lock()
	defer runtimeMu.Unlock()

	if runtime != nil {
		err := runtime.Close(ctx)
		runtime = nil
		return err
	}
	return nil
}

func callStringExport(ctx context.Context, name string) (string, error) {
	module, err := instantiateModule(ctx)
	if err != nil {
		return "", err
	}
	defer module.Close(ctx) //nolint:errcheck

	exported := module.ExportedFunction(name)
	if exported == nil {
		return "", fmt.Errorf("%s function not exported from WASM module", name)
	}

	results, err := exported.Call(ctx)
	if err != nil {
		return "", fmt.Errorf("failed to call %s: %w", name, err)
	}

	return readPackedString(ctx, module, results[0])
}

func callStringExportWithInput(ctx context.Context, name string, input string) (string, error) {
	module, err := instantiateModule(ctx)
	if err != nil {
		return "", err
	}
	defer module.Close(ctx) //nolint:errcheck

	allocate := module.ExportedFunction("allocate")
	deallocate := module.ExportedFunction("deallocate")
	exported := module.ExportedFunction(name)
	if allocate == nil || deallocate == nil || exported == nil {
		return "", errors.New("required functions not exported from WASM module")
	}

	inputBytes := []byte(input)
	inputLen := uint64(len(inputBytes))
	allocResults, err := allocate.Call(ctx, inputLen)
	if err != nil {
		return "", fmt.Errorf("failed to allocate memory: %w", err)
	}
	inputPtr := allocResults[0]
	defer deallocate.Call(ctx, inputPtr, inputLen) //nolint:errcheck

	if !module.Memory().Write(uint32(inputPtr), inputBytes) {
		return "", errors.New("failed to write to WASM memory")
	}

	results, err := exported.Call(ctx, inputPtr, inputLen)
	if err != nil {
		return "", fmt.Errorf("failed to call %s: %w", name, err)
	}

	return readPackedString(ctx, module, results[0])
}

func instantiateModule(ctx context.Context) (api.Module, error) {
	if err := ctx.Err(); err != nil {
		return nil, err
	}

	rt, compiled, err := initRuntime()
	if err != nil {
		return nil, err
	}

	moduleConfig := wazero.NewModuleConfig().
		WithName(uniqueModuleName()).
		WithArgs("secure_tunnel_go_wasm").
		WithSysWalltime().
		WithSysNanotime().
		WithRandSource(rand.Reader)

	module, err := rt.InstantiateModule(ctx, compiled, moduleConfig)
	if err != nil {
		return nil, fmt.Errorf("failed to instantiate WASM module: %w", err)
	}
	return module, nil
}

func initRuntime() (wazero.Runtime, wazero.CompiledModule, error) {
	initCtx := context.Background()

	runtimeOnce.Do(func() {
		runtime = wazero.NewRuntime(initCtx)
		wasi_snapshot_preview1.MustInstantiate(initCtx, runtime)
	})

	runtimeMu.RLock()
	rt := runtime
	runtimeMu.RUnlock()

	if rt == nil {
		return nil, nil, errors.New("failed to initialize runtime")
	}

	compiledModuleOnce.Do(func() {
		compiledModule, compiledModuleErr = rt.CompileModule(initCtx, wasmBytes)
	})

	return rt, compiledModule, compiledModuleErr
}

func readPackedString(ctx context.Context, module api.Module, packed uint64) (string, error) {
	if packed == 0 {
		return "", errors.New("allocation error in WASM module")
	}

	ptr := uint32(packed >> 32)
	length := uint32(packed & 0xFFFFFFFF)
	defer module.ExportedFunction("deallocate").Call(ctx, uint64(ptr), uint64(length)) //nolint:errcheck

	resultBytes, ok := module.Memory().Read(ptr, length)
	if !ok {
		return "", errors.New("failed to read result from WASM memory")
	}
	result := string(resultBytes)

	errorCheck := module.ExportedFunction("is_secure_tunnel_error")
	if errorCheck == nil {
		return "", errors.New("is_secure_tunnel_error function not exported from WASM module")
	}
	errorResults, err := errorCheck.Call(ctx, uint64(ptr), uint64(length))
	if err != nil {
		return "", fmt.Errorf("failed to check result status: %w", err)
	}
	if errorResults[0] != 0 {
		return "", &ABIError{msg: strings.TrimPrefix(result, "Error: ")}
	}

	return result, nil
}

func uniqueModuleName() string {
	return fmt.Sprintf("secure_tunnel_go_wasm_%d", atomic.AddUint64(&moduleNameCounter, 1))
}
