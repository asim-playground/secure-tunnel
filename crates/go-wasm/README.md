# Deprecated Go WASM Bindings

This scaffold is deprecated and is not a supported Secure Tunnel SDK target.
Native Go over the manual C ABI in `crates/go` is the supported Go SDK path.
The WASM scaffold remains only as legacy experimental code until a future task
either deletes it or proves a concrete product need.

This directory contains a Go binding for Secure Tunnel protocol metadata and
descriptor validation that uses WebAssembly (WASM) with WASI support instead of
a native Rust library.

## Features

- Uses WebAssembly (WASM) compiled from Rust code
- Runs in Go using the wazero runtime (zero dependencies)
- Supports WASI for system interfaces (time, etc.)
- Provides the same descriptor/protocol API shape as the native Go bindings

## Requirements

- Rust with the `wasm32-wasip1` target installed
- Go 1.21 or newer

## Building

```bash
# Install the `wasm32-wasip1` target if needed
mise run go-wasm:setup

# Build the WASM module
mise run go-wasm:build-wasm-release

# Run the tests
mise run go-wasm:test
```

## Usage

```go
package main

import (
    "context"
    "fmt"
    "log"

    "github.com/ab/cd/go-wasm/go/binding"
)

func main() {
    ctx := context.Background()
    
    // Inspect protocol metadata
    protocolID, err := binding.ProtocolID(ctx)
    if err != nil {
        log.Fatalf("ProtocolID error: %v", err)
    }
    fmt.Println(protocolID)

    // Validate a service descriptor
    descriptor := binding.MustExampleServiceDescriptorJSON()
    err = binding.ValidateServiceDescriptorJSON(ctx, descriptor)
    if err != nil {
        log.Fatalf("descriptor validation error: %v", err)
    }
    
    // Get timestamp from the WASM module (uses WASI)
    timestamp, err := binding.GetWasmTimestamp(ctx)
    if err != nil {
        log.Fatalf("GetWasmTimestamp error: %v", err)
    }
    fmt.Println("WASM Timestamp:", timestamp)
    
    // Clean up when done
    binding.Close(ctx)
}
```

## How It Works

1. Rust code is compiled to WebAssembly with WASI support
2. The Go binding uses wazero to load and execute the WASM module
3. Memory management is handled through exported functions
4. The API matches the native Go bindings for ease of use

## Benefits Over Native Bindings

- No need for CGO or native library compilation
- Portable across all platforms supported by Go
- Isolated execution in a sandboxed environment
- Easier deployment without platform-specific binaries
