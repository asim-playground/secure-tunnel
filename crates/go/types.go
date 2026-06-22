// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

package securetunnel

import (
	"encoding/json"
	"fmt"
)

// Version of the binding package.
const Version = "0.1.0"

// ABIError represents an error returned by the Secure Tunnel C ABI.
type ABIError struct {
	Status int32
	msg    string
}

func (e *ABIError) Error() string {
	return fmt.Sprintf("secure tunnel ABI status %d: %s", e.Status, e.msg)
}

// ConnectError represents a failed connect with structured SDK details.
type ConnectError struct {
	Status   int32                    `json:"-"`
	Kind     string                   `json:"kind"`
	Message  string                   `json:"message"`
	Attempts []TransportAttemptReport `json:"attempts"`
}

func (e *ConnectError) Error() string {
	if e.Kind == "" {
		return fmt.Sprintf("secure tunnel connect status %d: %s", e.Status, e.Message)
	}
	return fmt.Sprintf("secure tunnel connect %s: %s", e.Kind, e.Message)
}

// TransportPolicyConfig controls carrier selection and operation budgets.
type TransportPolicyConfig struct {
	QuicReprobeDelaySeconds uint64 `json:"quic_reprobe_delay_seconds"`
	ConnectTimeoutMs        uint64 `json:"connect_timeout_ms"`
	QuicConnectTimeoutMs    uint64 `json:"quic_connect_timeout_ms"`
	WssConnectTimeoutMs     uint64 `json:"wss_connect_timeout_ms"`
	SecureReadyTimeoutMs    uint64 `json:"secure_ready_timeout_ms"`
	RecordReadTimeoutMs     uint64 `json:"record_read_timeout_ms"`
	RecordWriteTimeoutMs    uint64 `json:"record_write_timeout_ms"`
}

// DescriptorTrustAnchor authorizes signed Secure Tunnel descriptors.
type DescriptorTrustAnchor struct {
	KeyID     string `json:"key_id"`
	Algorithm string `json:"algorithm"`
	PublicKey string `json:"public_key"`
}

// ClientConfig is the Go SDK client configuration.
type ClientConfig struct {
	TransportPolicy               TransportPolicyConfig
	OuterRootCertificatesDER      [][]byte
	DescriptorTrustAnchors        []DescriptorTrustAnchor
	PinnedServiceStaticPublicKeys [][]byte
}

// ConnectOptions supplies one connect attempt.
type ConnectOptions struct {
	DescriptorJSON string
	NowUnixSeconds uint64
	TransportCache *TransportCacheSnapshot
}

// Carrier is an outer transport carrier selected or attempted by the SDK.
type Carrier string

const (
	// CarrierQuic is raw QUIC over UDP.
	CarrierQuic Carrier = "quic"
	// CarrierWSS is WebSocket over HTTPS.
	CarrierWSS Carrier = "wss"
)

// CandidateSource describes why a carrier candidate appeared in the plan.
type CandidateSource string

const (
	CandidateSourcePreferredCarrier               CandidateSource = "preferred_carrier"
	CandidateSourceFallbackCarrier                CandidateSource = "fallback_carrier"
	CandidateSourceCachedQuicBadNetwork           CandidateSource = "cached_quic_bad_network"
	CandidateSourceQuicReprobeAfterCachedFallback CandidateSource = "quic_reprobe_after_cached_fallback"
)

// FallbackReason is a fallback-eligible outer-carrier failure class.
type FallbackReason string

const (
	FallbackReasonOuterPathFailure     FallbackReason = "outer_path_failure"
	FallbackReasonOuterQuicRejected    FallbackReason = "outer_quic_rejected"
	FallbackReasonOuterQuicClosedEarly FallbackReason = "outer_quic_closed_early"
)

// CacheDisposition describes whether carrier choice used live probing or cache.
type CacheDisposition string

const (
	CacheDispositionLiveProbe      CacheDisposition = "live_probe"
	CacheDispositionCachedFallback CacheDisposition = "cached_fallback"
	CacheDispositionReprobe        CacheDisposition = "reprobe"
)

// TransportCacheSnapshot is caller-persisted coarse network posture.
type TransportCacheSnapshot struct {
	LastSuccessfulCarrier         *Carrier        `json:"last_successful_carrier"`
	LastQuicFailure               *FallbackReason `json:"last_quic_failure"`
	NextQuicProbeAfterUnixSeconds *uint64         `json:"next_quic_probe_after_unix_seconds"`
	HighestDescriptorSerial       *uint64         `json:"highest_descriptor_serial"`
}

type transportAttemptReportJSON struct {
	Carrier Carrier         `json:"carrier"`
	Source  CandidateSource `json:"source"`
	Outcome json.RawMessage `json:"outcome"`
}

// TransportAttemptOutcome is a terminal attempt result.
type TransportAttemptOutcome string

const (
	TransportAttemptOutcomeSecureReady TransportAttemptOutcome = "secure_ready"
	TransportAttemptOutcomeFallback    TransportAttemptOutcome = "fallback"
	TransportAttemptOutcomeFailed      TransportAttemptOutcome = "failed"
)

// TransportAttemptReport records one carrier attempt for observability.
type TransportAttemptReport struct {
	Carrier        Carrier                 `json:"carrier"`
	Source         CandidateSource         `json:"source"`
	Outcome        TransportAttemptOutcome `json:"outcome"`
	FallbackReason *FallbackReason         `json:"fallback_reason,omitempty"`
	FailureKind    *string                 `json:"failure_kind,omitempty"`
	FailureMessage *string                 `json:"failure_message,omitempty"`
}

func (r *TransportAttemptReport) UnmarshalJSON(data []byte) error {
	var value transportAttemptReportJSON
	if err := json.Unmarshal(data, &value); err != nil {
		return err
	}
	r.Carrier = value.Carrier
	r.Source = value.Source
	return r.decodeOutcome(value.Outcome)
}

func (r *TransportAttemptReport) decodeOutcome(data json.RawMessage) error {
	var unit string
	if err := json.Unmarshal(data, &unit); err == nil {
		r.Outcome = TransportAttemptOutcome(unit)
		return nil
	}
	var tagged map[string]json.RawMessage
	if err := json.Unmarshal(data, &tagged); err != nil {
		return err
	}
	if _, ok := tagged[string(TransportAttemptOutcomeSecureReady)]; ok {
		r.Outcome = TransportAttemptOutcomeSecureReady
		return nil
	}
	if payload, ok := tagged[string(TransportAttemptOutcomeFallback)]; ok {
		var fallback struct {
			Reason FallbackReason `json:"reason"`
		}
		if err := json.Unmarshal(payload, &fallback); err != nil {
			return err
		}
		r.Outcome = TransportAttemptOutcomeFallback
		r.FallbackReason = &fallback.Reason
		return nil
	}
	if payload, ok := tagged[string(TransportAttemptOutcomeFailed)]; ok {
		var failed struct {
			Kind    string `json:"kind"`
			Message string `json:"message"`
		}
		if err := json.Unmarshal(payload, &failed); err != nil {
			return err
		}
		r.Outcome = TransportAttemptOutcomeFailed
		r.FailureKind = &failed.Kind
		r.FailureMessage = &failed.Message
		return nil
	}
	return fmt.Errorf("unknown transport attempt outcome: %s", string(data))
}

// ConnectReport is the JSON-friendly SDK connect report.
type ConnectReport struct {
	SelectedCarrier Carrier                  `json:"selected_carrier"`
	CacheState      CacheDisposition         `json:"cache_state"`
	FallbackReason  *FallbackReason          `json:"fallback_reason"`
	Attempts        []TransportAttemptReport `json:"attempts"`
	TransportCache  TransportCacheSnapshot   `json:"transport_cache"`
}

// SecureChannelArtifacts exposes channel-binding material for integrations.
type SecureChannelArtifacts struct {
	HandshakeHashB64            *string `json:"handshake_hash_b64"`
	ChannelBindingB64           *string `json:"channel_binding_b64"`
	ServiceStaticPublicKeyB64   *string `json:"service_static_public_key_b64"`
	ServiceStaticPublicKeyBytes []byte  `json:"-"`
}

// AccountAuthMode selects account authentication mode.
type AccountAuthMode int

const (
	// AccountAuthModeFresh authenticates with fresh account credentials.
	AccountAuthModeFresh AccountAuthMode = iota
	// AccountAuthModeResume resumes a previous account session.
	AccountAuthModeResume
)

// AccountAuthRequest is an account authentication request.
type AccountAuthRequest struct {
	AccountID         string
	CredentialPayload []byte
	Mode              AccountAuthMode
}

// AccountAuthReport is returned after account authentication succeeds.
type AccountAuthReport struct {
	AccountID             string `json:"account_id"`
	SessionContextID      string `json:"session_context_id"`
	AccountContextHashB64 string `json:"account_context_hash_b64"`
	Freshness             string `json:"freshness"`
}

// CloseReport is returned after graceful session close.
type CloseReport struct {
	FinalState     string `json:"final_state"`
	Classification string `json:"classification"`
}
