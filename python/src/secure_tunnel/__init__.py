# Copyright 2026 Asim Ihsan
#
# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
# If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# SPDX-License-Identifier: MPL-2.0

"""Python SDK wrapper for the Secure Tunnel UniFFI facade."""

from __future__ import annotations

from importlib import import_module
from importlib.metadata import PackageNotFoundError, version
from typing import Any

try:
    __version__ = version("secure-tunnel")
except PackageNotFoundError:
    __version__ = "0.0.0"


_native: Any = import_module("secure_tunnel._native.secure_tunnel_sdk_ffi")

AccountAuthMode = _native.AccountAuthMode
AccountAuthReport = _native.AccountAuthReport
AccountAuthRequest = _native.AccountAuthRequest
AccountFreshness = _native.AccountFreshness
CacheDisposition = _native.CacheDisposition
CandidateSource = _native.CandidateSource
Carrier = _native.Carrier
ClientConfig = _native.ClientConfig
CloseClassification = _native.CloseClassification
CloseReport = _native.CloseReport
ConnectOptions = _native.ConnectOptions
ConnectReport = _native.ConnectReport
DescriptorTrustAnchor = _native.DescriptorTrustAnchor
DeviceAuthChallenge = _native.DeviceAuthChallenge
DeviceAuthReport = _native.DeviceAuthReport
DeviceState = _native.DeviceState
FallbackReason = _native.FallbackReason
SecureChannelArtifacts = _native.SecureChannelArtifacts
SecureTunnelClient = _native.SecureTunnelClient
SecureTunnelConnection = _native.SecureTunnelConnection
SecureTunnelError = _native.SecureTunnelError
SessionState = _native.SessionState
TransportAttemptOutcome = _native.TransportAttemptOutcome
TransportAttemptReport = _native.TransportAttemptReport
TransportCacheSnapshot = _native.TransportCacheSnapshot


def protocol_id_v1() -> str:
    """Return the stable v1 protocol identifier."""
    return _native.protocol_id_v1()


def quic_alpn_v1() -> str:
    """Return the v1 QUIC ALPN value."""
    return protocol_id_v1()


def wss_subprotocol_v1() -> str:
    """Return the v1 WSS subprotocol value."""
    return protocol_id_v1()


def default_client_config() -> Any:
    """Return a default generated-binding client configuration."""
    return _native.default_client_config()


def example_descriptor_json() -> str:
    """Return a sample service descriptor as JSON."""
    return _native.example_descriptor_json()


def normalize_descriptor_json(descriptor_json: str) -> str:
    """Validate and re-encode a service descriptor JSON document."""
    return _native.normalize_descriptor_json(descriptor_json)


def example_service_descriptor_json() -> str:
    """Return a sample service descriptor as JSON."""
    return example_descriptor_json()


def validate_service_descriptor_json(descriptor_json: str) -> None:
    """Validate a service descriptor JSON document."""
    try:
        normalize_descriptor_json(descriptor_json)
    except SecureTunnelError as error:
        raise ValueError(error.message()) from error


def normalize_service_descriptor_json(descriptor_json: str) -> str:
    """Validate and re-encode a service descriptor JSON document."""
    try:
        return normalize_descriptor_json(descriptor_json)
    except SecureTunnelError as error:
        raise ValueError(error.message()) from error


__all__ = [
    "__version__",
    "AccountAuthMode",
    "AccountAuthReport",
    "AccountAuthRequest",
    "AccountFreshness",
    "CacheDisposition",
    "CandidateSource",
    "Carrier",
    "ClientConfig",
    "CloseClassification",
    "CloseReport",
    "ConnectOptions",
    "ConnectReport",
    "DescriptorTrustAnchor",
    "DeviceAuthChallenge",
    "DeviceAuthReport",
    "DeviceState",
    "FallbackReason",
    "SecureChannelArtifacts",
    "SecureTunnelClient",
    "SecureTunnelConnection",
    "SecureTunnelError",
    "SessionState",
    "TransportAttemptOutcome",
    "TransportAttemptReport",
    "TransportCacheSnapshot",
    "default_client_config",
    "example_descriptor_json",
    "example_service_descriptor_json",
    "normalize_descriptor_json",
    "normalize_service_descriptor_json",
    "protocol_id_v1",
    "quic_alpn_v1",
    "validate_service_descriptor_json",
    "wss_subprotocol_v1",
]
