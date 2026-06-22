// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

package securetunnel

import (
	"context"
	"errors"
	"strings"
	"testing"
)

func TestDefaultClientConfigRoundTrips(t *testing.T) {
	config, err := DefaultClientConfig(context.Background())
	if err != nil {
		t.Fatalf("DefaultClientConfig() error = %v", err)
	}
	if config.TransportPolicy.ConnectTimeoutMs == 0 {
		t.Fatal("default config missing connect timeout")
	}
	if len(config.DescriptorTrustAnchors) == 0 {
		t.Fatal("default config missing descriptor trust anchors")
	}
	if len(config.PinnedServiceStaticPublicKeys) == 0 {
		t.Fatal("default config missing service static pin")
	}
	if len(config.PinnedServiceStaticPublicKeys[0]) != 32 {
		t.Fatalf("service static pin length = %d, want 32", len(config.PinnedServiceStaticPublicKeys[0]))
	}
	if config.WSSHTTPProxy != nil {
		t.Fatalf("default config WSSHTTPProxy = %#v, want nil", config.WSSHTTPProxy)
	}
}

func TestZeroValueClientConfigUsesRustDefaults(t *testing.T) {
	configJSON, err := encodeClientConfigJSON(ClientConfig{})
	if err != nil {
		t.Fatalf("encodeClientConfigJSON() error = %v", err)
	}
	if strings.Contains(string(configJSON), "transport_policy") {
		t.Fatalf("zero-value config should omit transport_policy: %s", string(configJSON))
	}
	client, err := NewClient(context.Background(), ClientConfig{})
	if err != nil {
		t.Fatalf("NewClient(zero config) error = %v", err)
	}
	client.Close()
}

func TestClientConfigProxyRoundTripsJSON(t *testing.T) {
	config := ClientConfig{
		WSSHTTPProxy: &HttpProxyConfig{URL: "http://127.0.0.1:8080"},
	}
	configJSON, err := encodeClientConfigJSON(config)
	if err != nil {
		t.Fatalf("encodeClientConfigJSON() error = %v", err)
	}
	if !strings.Contains(string(configJSON), "wss_http_proxy") {
		t.Fatalf("encoded config missing wss_http_proxy: %s", string(configJSON))
	}
	decoded, err := decodeClientConfigJSON(configJSON)
	if err != nil {
		t.Fatalf("decodeClientConfigJSON() error = %v", err)
	}
	if decoded.WSSHTTPProxy == nil || decoded.WSSHTTPProxy.URL != config.WSSHTTPProxy.URL {
		t.Fatalf("decoded WSSHTTPProxy = %#v, want %#v", decoded.WSSHTTPProxy, config.WSSHTTPProxy)
	}
}

func TestNewClientRejectsInvalidServicePin(t *testing.T) {
	config, err := DefaultClientConfig(context.Background())
	if err != nil {
		t.Fatalf("DefaultClientConfig() error = %v", err)
	}
	config.PinnedServiceStaticPublicKeys = [][]byte{{1, 2, 3}}

	client, err := NewClient(context.Background(), config)
	if err == nil {
		if client != nil {
			client.Close()
		}
		t.Fatal("expected invalid config error")
	}
	var abiErr *ABIError
	if !errors.As(err, &abiErr) {
		t.Fatalf("expected ABIError, got %T", err)
	}
	if abiErr.Status != 6 {
		t.Fatalf("ABIError status = %d, want 6", abiErr.Status)
	}
}
