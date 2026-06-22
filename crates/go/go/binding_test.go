// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

package binding

import (
	"context"
	"encoding/base64"
	"encoding/json"
	"errors"
	"os"
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

func TestClosedClientAndConnectionRejectUse(t *testing.T) {
	config, err := DefaultClientConfig(context.Background())
	if err != nil {
		t.Fatalf("DefaultClientConfig() error = %v", err)
	}
	client, err := NewClient(context.Background(), config)
	if err != nil {
		t.Fatalf("NewClient() error = %v", err)
	}
	client.Close()

	if _, err := client.Connect(context.Background(), ConnectOptions{}); err == nil {
		t.Fatal("expected closed client connect to fail")
	}

	connection := &Connection{}
	if _, err := connection.Report(); err == nil {
		t.Fatal("expected closed connection report to fail")
	}
}

func TestConcurrentClientCloseDuringConnectValidation(t *testing.T) {
	config, err := DefaultClientConfig(context.Background())
	if err != nil {
		t.Fatalf("DefaultClientConfig() error = %v", err)
	}
	client, err := NewClient(context.Background(), config)
	if err != nil {
		t.Fatalf("NewClient() error = %v", err)
	}
	start := make(chan struct{})
	var wg sync.WaitGroup
	for range 8 {
		wg.Add(1)
		go func() {
			defer wg.Done()
			<-start
			_, _ = client.Connect(context.Background(), ConnectOptions{
				DescriptorJSON: "{}",
				NowUnixSeconds: 1_742_000_000,
			})
		}()
	}
	for range 8 {
		wg.Add(1)
		go func() {
			defer wg.Done()
			<-start
			client.Close()
		}()
	}
	close(start)
	wg.Wait()
	client.Close()
}

func TestBindingFixtureSmoke(t *testing.T) {
	fixturePath := os.Getenv("SECURE_TUNNEL_GO_FIXTURE_JSON")
	if fixturePath == "" {
		t.Skip("SECURE_TUNNEL_GO_FIXTURE_JSON is not set")
	}
	fixture := readBindingFixture(t, fixturePath)
	ctx := context.Background()
	defaults, err := DefaultClientConfig(ctx)
	if err != nil {
		t.Fatalf("DefaultClientConfig() error = %v", err)
	}
	config := ClientConfig{
		TransportPolicy: defaults.TransportPolicy,
		OuterRootCertificatesDER: decodeManyB64(
			t,
			fixture.OuterRootCertificatesDERB64,
		),
		DescriptorTrustAnchors: defaults.DescriptorTrustAnchors,
		PinnedServiceStaticPublicKeys: decodeManyB64(
			t,
			fixture.PinnedServiceStaticPublicKeysB64,
		),
	}
	client, err := NewClient(ctx, config)
	if err != nil {
		t.Fatalf("NewClient() error = %v", err)
	}
	defer client.Close()

	connection, err := client.Connect(ctx, ConnectOptions{
		DescriptorJSON: fixture.DescriptorJSON,
		NowUnixSeconds: fixture.NowUnixSeconds,
	})
	if err != nil {
		t.Fatalf("Connect() error = %v", err)
	}
	defer connection.Close()

	report, err := connection.Report()
	if err != nil {
		t.Fatalf("Report() error = %v", err)
	}
	if report.SelectedCarrier != "quic" {
		t.Fatalf("selected carrier = %q, want quic", report.SelectedCarrier)
	}
	if report.CacheState != CacheDispositionLiveProbe {
		t.Fatalf("cache state = %q, want live_probe", report.CacheState)
	}
	if len(report.Attempts) == 0 {
		t.Fatal("connect report missing attempts")
	}
	if report.TransportCache.LastSuccessfulCarrier == nil || *report.TransportCache.LastSuccessfulCarrier != CarrierQuic {
		t.Fatalf("transport cache last successful carrier = %#v, want quic", report.TransportCache.LastSuccessfulCarrier)
	}
	if report.TransportCache.HighestDescriptorSerial == nil {
		t.Fatal("transport cache missing highest descriptor serial")
	}
	if report.Attempts[0].Outcome != TransportAttemptOutcomeSecureReady {
		t.Fatalf("first attempt outcome = %q, want secure_ready", report.Attempts[0].Outcome)
	}
	reason := FallbackReasonOuterPathFailure
	nextProbe := fixture.NowUnixSeconds + 300
	cachedConnection, err := client.Connect(ctx, ConnectOptions{
		DescriptorJSON: fixture.DescriptorJSON,
		NowUnixSeconds: fixture.NowUnixSeconds,
		TransportCache: &TransportCacheSnapshot{
			LastQuicFailure:               &reason,
			NextQuicProbeAfterUnixSeconds: &nextProbe,
			HighestDescriptorSerial:       report.TransportCache.HighestDescriptorSerial,
		},
	})
	if err != nil {
		t.Fatalf("cached Connect() error = %v", err)
	}
	defer cachedConnection.Close()
	cachedReport, err := cachedConnection.Report()
	if err != nil {
		t.Fatalf("cached Report() error = %v", err)
	}
	if cachedReport.SelectedCarrier != CarrierWSS {
		t.Fatalf("cached selected carrier = %q, want wss", cachedReport.SelectedCarrier)
	}
	if cachedReport.CacheState != CacheDispositionCachedFallback {
		t.Fatalf("cached cache state = %q, want cached_fallback", cachedReport.CacheState)
	}
	if len(cachedReport.Attempts) != 1 {
		t.Fatalf("cached attempts = %d, want 1", len(cachedReport.Attempts))
	}
	if cachedReport.Attempts[0].Source != CandidateSourceCachedQuicBadNetwork {
		t.Fatalf("cached attempt source = %q, want cached_quic_bad_network", cachedReport.Attempts[0].Source)
	}
	if cachedReport.Attempts[0].Outcome != TransportAttemptOutcomeSecureReady {
		t.Fatalf("cached attempt outcome = %q, want secure_ready", cachedReport.Attempts[0].Outcome)
	}
	badRootConfig := ClientConfig{
		TransportPolicy:               defaults.TransportPolicy,
		DescriptorTrustAnchors:        defaults.DescriptorTrustAnchors,
		PinnedServiceStaticPublicKeys: config.PinnedServiceStaticPublicKeys,
	}
	badRootClient, err := NewClient(ctx, badRootConfig)
	if err != nil {
		t.Fatalf("NewClient(bad root config) error = %v", err)
	}
	defer badRootClient.Close()
	_, err = badRootClient.Connect(ctx, ConnectOptions{
		DescriptorJSON: fixture.DescriptorJSON,
		NowUnixSeconds: fixture.NowUnixSeconds,
	})
	if err == nil {
		t.Fatal("expected missing local roots to fail connect")
	}
	var connectErr *ConnectError
	if !errors.As(err, &connectErr) {
		t.Fatalf("expected ConnectError, got %T: %v", err, err)
	}
	if connectErr.Status != 8 {
		t.Fatalf("ConnectError status = %d, want 8", connectErr.Status)
	}
	if connectErr.Kind == "" || connectErr.Message == "" {
		t.Fatalf("ConnectError missing kind/message: %#v", connectErr)
	}
	if len(connectErr.Attempts) == 0 {
		t.Fatal("ConnectError missing attempts")
	}
	if connectErr.Attempts[0].Outcome == "" {
		t.Fatalf("ConnectError first attempt missing outcome: %#v", connectErr.Attempts[0])
	}
	if connectErr.Attempts[0].Outcome == TransportAttemptOutcomeFailed &&
		(connectErr.Attempts[0].FailureKind == nil || connectErr.Attempts[0].FailureMessage == nil) {
		t.Fatalf("failed attempt missing failure details: %#v", connectErr.Attempts[0])
	}
	artifacts, err := connection.SecurityArtifacts()
	if err != nil {
		t.Fatalf("SecurityArtifacts() error = %v", err)
	}
	if !containsBytes(config.PinnedServiceStaticPublicKeys, artifacts.ServiceStaticPublicKeyBytes) {
		t.Fatal("service static public key was not pinned")
	}
	_, err = connection.AuthenticateAccount(ctx, AccountAuthRequest{
		AccountID:         "go-smoke",
		CredentialPayload: []byte("credential"),
		Mode:              AccountAuthMode(99),
	})
	if err == nil {
		t.Fatal("expected invalid account auth mode to fail")
	}
	var abiErr *ABIError
	if !errors.As(err, &abiErr) {
		t.Fatalf("expected ABIError, got %T", err)
	}
	if abiErr.Status != 6 {
		t.Fatalf("ABIError status = %d, want 6", abiErr.Status)
	}
	auth, err := connection.AuthenticateAccount(ctx, AccountAuthRequest{
		AccountID:         "go-smoke",
		CredentialPayload: []byte("credential"),
		Mode:              AccountAuthModeFresh,
	})
	if err != nil {
		t.Fatalf("AuthenticateAccount() error = %v", err)
	}
	if auth.AccountID != "go-smoke" {
		t.Fatalf("account id = %q, want go-smoke", auth.AccountID)
	}
	response, err := connection.Request(ctx, decodeB64(t, fixture.SmokePingB64))
	if err != nil {
		t.Fatalf("Request() error = %v", err)
	}
	if !bytesEqual(response, decodeB64(t, fixture.SmokePongB64)) {
		t.Fatalf("unexpected smoke response: %q", string(response))
	}
	closeReport, err := connection.CloseSession(ctx, 1000, true)
	if err != nil {
		t.Fatalf("CloseSession() error = %v", err)
	}
	if closeReport.Classification != "graceful" {
		t.Fatalf("close classification = %q, want graceful", closeReport.Classification)
	}
}

type bindingFixture struct {
	DescriptorJSON                   string   `json:"descriptor_json"`
	OuterRootCertificatesDERB64      []string `json:"outer_root_certificates_der_b64"`
	PinnedServiceStaticPublicKeysB64 []string `json:"pinned_service_static_public_keys_b64"`
	NowUnixSeconds                   uint64   `json:"now_unix_seconds"`
	SmokePingB64                     string   `json:"smoke_ping_b64"`
	SmokePongB64                     string   `json:"smoke_pong_b64"`
}

func readBindingFixture(t *testing.T, path string) bindingFixture {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("read fixture: %v", err)
	}
	var fixture bindingFixture
	if err := json.Unmarshal(data, &fixture); err != nil {
		t.Fatalf("decode fixture: %v", err)
	}
	return fixture
}

func decodeManyB64(t *testing.T, values []string) [][]byte {
	t.Helper()
	decoded := make([][]byte, 0, len(values))
	for _, value := range values {
		decoded = append(decoded, decodeB64(t, value))
	}
	return decoded
}

func decodeB64(t *testing.T, value string) []byte {
	t.Helper()
	bytes, err := base64.StdEncoding.DecodeString(value)
	if err != nil {
		t.Fatalf("decode base64: %v", err)
	}
	return bytes
}

func containsBytes(values [][]byte, want []byte) bool {
	for _, value := range values {
		if bytesEqual(value, want) {
			return true
		}
	}
	return false
}

func bytesEqual(left, right []byte) bool {
	if len(left) != len(right) {
		return false
	}
	for index := range left {
		if left[index] != right[index] {
			return false
		}
	}
	return true
}
