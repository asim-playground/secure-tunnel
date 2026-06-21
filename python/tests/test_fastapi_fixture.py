# Copyright 2026 Asim Ihsan
#
# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
# If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# SPDX-License-Identifier: MPL-2.0

"""Tests for the FastAPI Secure Tunnel fixture shell."""

from __future__ import annotations

import base64
import sys
import time
from pathlib import Path

import pytest
from fastapi.testclient import TestClient

import secure_tunnel as st
from secure_tunnel.fastapi_fixture import (
    FixtureController,
    FixtureSettings,
    ObservabilityFormat,
    ObservabilitySettings,
    RustLogLevel,
    create_app,
)


class _FakeFixture:
    def __init__(self) -> None:
        service_pin = base64.b64encode(bytes([7]) * 32).decode("ascii")
        self.started = False
        self.stopped = False
        self._report = {
            "descriptor_json": st.example_descriptor_json(),
            "outer_root_certificates_der_b64": ["ZmFrZS1yb290"],
            "pinned_service_static_public_keys_b64": [service_pin],
            "now_unix_seconds": 1_742_000_000,
            "smoke_ping_b64": "c21va2UtcGluZw==",
            "smoke_pong_b64": "c21va2UtcG9uZw==",
        }

    def start(self) -> None:
        self.started = True

    def stop(self) -> None:
        self.stopped = True

    def is_running(self) -> bool:
        return self.started and not self.stopped

    def report(self) -> dict[str, object]:
        return dict(self._report)


def test_fastapi_fixture_serves_health_descriptor_and_bootstrap():
    """Test the FastAPI fixture endpoints without launching subprocesses."""
    fixture = _FakeFixture()

    with TestClient(create_app(fixture)) as client:
        health = client.get("/health").json()
        assert health["ok"] is True
        assert health["fixture_running"] is True
        assert health["test_only"] is True
        assert health["observability"]["service_name"] == "secure-tunnel-fastapi-fixture"

        descriptor = client.get("/descriptor").json()
        assert descriptor["test_only"] is True
        assert descriptor["source"] == "secure-tunnel-fastapi-fixture"
        assert descriptor["now_unix_seconds"] == 1_742_000_000
        assert descriptor["descriptor"]["protocol_id"] == st.protocol_id_v1()
        assert (
            descriptor["pinned_service_static_public_keys_b64"]
            == fixture.report()["pinned_service_static_public_keys_b64"]
        )

        binding_fixture = client.get("/binding-fixture").json()
        assert binding_fixture["test_only"] is True
        assert binding_fixture["source"] == "secure-tunnel-fastapi-fixture"
        assert binding_fixture["descriptor_json"] == fixture.report()["descriptor_json"]

        bootstrap = client.get("/bootstrap").json()
        assert bootstrap["test_only"] is True
        assert bootstrap["descriptor_json"] == fixture.report()["descriptor_json"]
        assert bootstrap["observability"]["format"] == "json"

    assert fixture.stopped is True


def test_observability_settings_plumb_rust_environment():
    """Test Python server observability settings map to Rust process env."""
    settings = ObservabilitySettings(
        level=RustLogLevel.DEBUG,
        format=ObservabilityFormat.JSON,
        service_name="secure-tunnel-test",
        otlp_endpoint="http://127.0.0.1:4318",
        resource_attributes={"deployment.environment": "unit", "service.version": "0.1.0"},
    )

    env = settings.rust_env()

    assert env["SECURE_TUNNEL_OBSERVABILITY"] == "1"
    assert env["SECURE_TUNNEL_OBSERVABILITY_FORMAT"] == "json"
    assert env["SECURE_TUNNEL_OBSERVABILITY_LEVEL"] == "debug"
    assert env["OTEL_SERVICE_NAME"] == "secure-tunnel-test"
    assert env["OTEL_EXPORTER_OTLP_ENDPOINT"] == "http://127.0.0.1:4318"
    assert "secure_tunnel_sdk=debug" in env["RUST_LOG"]
    assert "deployment.environment=unit" in env["OTEL_RESOURCE_ATTRIBUTES"]


def test_fixture_controller_cleans_up_invalid_bootstrap():
    """Test invalid Rust bootstrap output does not leave the child running."""
    controller = FixtureController(
        settings=_fast_failure_settings(),
        command=[
            sys.executable,
            "-c",
            "import time; print('not-json', flush=True); time.sleep(30)",
        ],
    )

    with pytest.raises(RuntimeError, match="fixture startup failed"):
        controller.start()

    assert controller.is_running() is False


def test_fixture_controller_cleans_up_bootstrap_timeout():
    """Test a silent Rust child cannot wedge FastAPI startup forever."""
    controller = FixtureController(
        settings=_fast_failure_settings(),
        command=[sys.executable, "-c", "import time; time.sleep(30)"],
    )

    with pytest.raises(RuntimeError, match="timed out waiting"):
        controller.start()

    assert controller.is_running() is False


def test_fixture_controller_does_not_block_on_rust_stderr(tmp_path: Path):
    """Test Rust observability stderr cannot back-pressure fixture startup."""
    marker = tmp_path / "stderr-drained"
    controller = FixtureController(
        settings=_fast_failure_settings(),
        command=[
            sys.executable,
            "-c",
            (
                "import json, pathlib, sys, time; "
                "print(json.dumps({'ok': True}), flush=True); "
                "sys.stderr.write('x' * 2000000); sys.stderr.flush(); "
                f"pathlib.Path({str(marker)!r}).write_text('done'); "
                "time.sleep(30)"
            ),
        ],
    )

    controller.start()
    try:
        deadline = time.monotonic() + 2
        while time.monotonic() < deadline and not marker.exists():
            time.sleep(0.05)
        assert marker.read_text(encoding="utf-8") == "done"
    finally:
        controller.stop()


def _fast_failure_settings() -> FixtureSettings:
    return FixtureSettings(
        startup_timeout_seconds=0.2,
        shutdown_timeout_seconds=0.2,
        kill_timeout_seconds=0.2,
    )
