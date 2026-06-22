#!/usr/bin/env python3
# Copyright 2026 Asim Ihsan
#
# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
# If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# SPDX-License-Identifier: MPL-2.0

"""End-to-end smoke client for generated UniFFI Python bindings."""

from __future__ import annotations

import base64
import json
import sys
from pathlib import Path

import secure_tunnel_sdk_ffi as st


def _decode_many(values: list[str]) -> list[bytes]:
    return [base64.b64decode(value) for value in values]


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: client.py <binding-fixture.json>", file=sys.stderr)
        return 2

    fixture = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    defaults = st.default_client_config()
    config = st.ClientConfig(
        quic_reprobe_delay_seconds=300,
        connect_timeout_ms=defaults.connect_timeout_ms,
        quic_connect_timeout_ms=defaults.quic_connect_timeout_ms,
        wss_connect_timeout_ms=defaults.wss_connect_timeout_ms,
        secure_ready_timeout_ms=defaults.secure_ready_timeout_ms,
        record_read_timeout_ms=defaults.record_read_timeout_ms,
        record_write_timeout_ms=defaults.record_write_timeout_ms,
        outer_root_certificates_der=_decode_many(
            fixture["outer_root_certificates_der_b64"],
        ),
        wss_http_proxy=None,
        descriptor_trust_anchors=defaults.descriptor_trust_anchors,
        pinned_service_static_public_keys=_decode_many(
            fixture["pinned_service_static_public_keys_b64"],
        ),
    )
    client = st.SecureTunnelClient(config)
    connection = client.connect(
        st.ConnectOptions(
            descriptor_json=fixture["descriptor_json"],
            now_unix_seconds=fixture["now_unix_seconds"],
            transport_cache=None,
        ),
    )
    report = connection.report()
    if report.selected_carrier != st.Carrier.QUIC:
        raise AssertionError(f"expected QUIC, got {report.selected_carrier!r}")
    artifacts = connection.security_artifacts()
    if artifacts.service_static_public_key not in config.pinned_service_static_public_keys:
        raise AssertionError("unexpected service static public key")

    auth = connection.authenticate_account(
        st.AccountAuthRequest(
            account_id="python-smoke",
            credential_payload=b"credential",
            mode=st.AccountAuthMode.FRESH,
        ),
    )
    if auth.account_id != "python-smoke":
        raise AssertionError(f"unexpected account id: {auth.account_id}")

    payload = base64.b64decode(fixture["smoke_ping_b64"])
    expected = base64.b64decode(fixture["smoke_pong_b64"])
    response = connection.request(payload)
    if response != expected:
        raise AssertionError("unexpected smoke response")

    close = connection.close(1000, True)
    print(
        json.dumps(
            {
                "language": "python",
                "protocol": st.protocol_id_v1(),
                "carrier": report.selected_carrier.name,
                "close": close.classification.name,
            },
            sort_keys=True,
        ),
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
