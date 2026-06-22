// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

package securetunnel

import (
	"encoding/base64"
	"encoding/json"
)

type clientConfigJSON struct {
	TransportPolicy                  *TransportPolicyConfig  `json:"transport_policy,omitempty"`
	OuterRootCertificatesDERB64      []string                `json:"outer_root_certificates_der_b64,omitempty"`
	WSSHTTPProxy                     *HttpProxyConfig        `json:"wss_http_proxy,omitempty"`
	DescriptorTrustAnchors           []DescriptorTrustAnchor `json:"descriptor_trust_anchors,omitempty"`
	PinnedServiceStaticPublicKeysB64 []string                `json:"pinned_service_static_public_keys_b64,omitempty"`
}

func encodeClientConfigJSON(config ClientConfig) ([]byte, error) {
	var transportPolicy *TransportPolicyConfig
	if config.TransportPolicy != (TransportPolicyConfig{}) {
		transportPolicy = &config.TransportPolicy
	}
	value := clientConfigJSON{
		TransportPolicy:                  transportPolicy,
		OuterRootCertificatesDERB64:      encodeMany(config.OuterRootCertificatesDER),
		WSSHTTPProxy:                     config.WSSHTTPProxy,
		DescriptorTrustAnchors:           config.DescriptorTrustAnchors,
		PinnedServiceStaticPublicKeysB64: encodeMany(config.PinnedServiceStaticPublicKeys),
	}
	return json.Marshal(value)
}

func decodeClientConfigJSON(data []byte) (ClientConfig, error) {
	var value clientConfigJSON
	if err := json.Unmarshal(data, &value); err != nil {
		return ClientConfig{}, err
	}
	outerRoots, err := decodeMany(value.OuterRootCertificatesDERB64)
	if err != nil {
		return ClientConfig{}, err
	}
	servicePins, err := decodeMany(value.PinnedServiceStaticPublicKeysB64)
	if err != nil {
		return ClientConfig{}, err
	}
	var transportPolicy TransportPolicyConfig
	if value.TransportPolicy != nil {
		transportPolicy = *value.TransportPolicy
	}
	return ClientConfig{
		TransportPolicy:               transportPolicy,
		OuterRootCertificatesDER:      outerRoots,
		WSSHTTPProxy:                  value.WSSHTTPProxy,
		DescriptorTrustAnchors:        value.DescriptorTrustAnchors,
		PinnedServiceStaticPublicKeys: servicePins,
	}, nil
}

func encodeMany(values [][]byte) []string {
	encoded := make([]string, 0, len(values))
	for _, value := range values {
		encoded = append(encoded, base64.StdEncoding.EncodeToString(value))
	}
	return encoded
}

func decodeMany(values []string) ([][]byte, error) {
	decoded := make([][]byte, 0, len(values))
	for _, value := range values {
		bytes, err := base64.StdEncoding.DecodeString(value)
		if err != nil {
			return nil, err
		}
		decoded = append(decoded, bytes)
	}
	return decoded, nil
}
