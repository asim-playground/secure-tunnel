# Copyright 2026 Asim Ihsan
#
# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
# If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# SPDX-License-Identifier: MPL-2.0

"""Tests for Secure Tunnel Python bindings."""

import json

import pytest

from secure_tunnel import (
    __version__,
    example_service_descriptor_json,
    normalize_service_descriptor_json,
    protocol_id_v1,
    quic_alpn_v1,
    validate_service_descriptor_json,
    wss_subprotocol_v1,
)


def test_version():
    """Test that version is available."""
    assert isinstance(__version__, str)
    assert __version__ != ""


def test_protocol_metadata():
    """Test v1 protocol constants."""
    assert protocol_id_v1() == "secure-tunnel-v1"
    assert quic_alpn_v1() == "secure-tunnel-v1"
    assert wss_subprotocol_v1() == "secure-tunnel-v1"


def test_example_descriptor_validates():
    """Test the bundled example descriptor."""
    descriptor_json = example_service_descriptor_json()
    descriptor = json.loads(descriptor_json)

    assert descriptor["service_id"] == "secure-tunnel-api"
    validate_service_descriptor_json(descriptor_json)


def test_normalize_descriptor_rejects_invalid_protocol_id():
    """Test descriptor validation errors map to ValueError."""
    descriptor = json.loads(example_service_descriptor_json())
    descriptor["protocol_id"] = "wrong"

    with pytest.raises(ValueError, match="protocol_id"):
        normalize_service_descriptor_json(json.dumps(descriptor))


def test_validate_descriptor_rejects_invalid_quic_alpn():
    """Test descriptor validation checks carrier selectors."""
    descriptor = json.loads(example_service_descriptor_json())
    descriptor["carriers"]["quic"]["alpn"] = "wrong"

    with pytest.raises(ValueError, match="QUIC ALPN"):
        validate_service_descriptor_json(json.dumps(descriptor))
