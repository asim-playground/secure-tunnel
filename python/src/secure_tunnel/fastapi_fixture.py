# Copyright 2026 Asim Ihsan
#
# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
# If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# SPDX-License-Identifier: MPL-2.0

"""FastAPI fixture shell for Rust-backed Secure Tunnel server smoke tests."""

from __future__ import annotations

import json
import os
import selectors
import signal
import subprocess
import tempfile
import time
from collections.abc import AsyncIterator
from contextlib import asynccontextmanager
from dataclasses import dataclass, field
from enum import StrEnum
from pathlib import Path
from typing import Any, Protocol, TextIO, cast

from fastapi import FastAPI, HTTPException

JsonMap = dict[str, Any]


class FixtureRuntime(StrEnum):
    """Rust-backed server runtime selected by the Python shell."""

    BINDING_FIXTURE = "binding-fixture"


class ObservabilityFormat(StrEnum):
    """Structured Rust tracing output format."""

    COMPACT = "compact"
    JSON = "json"


class RustLogLevel(StrEnum):
    """Coarse Rust tracing level controlled by the Python server."""

    OFF = "off"
    ERROR = "error"
    WARN = "warn"
    INFO = "info"
    DEBUG = "debug"
    TRACE = "trace"


@dataclass(frozen=True)
class ObservabilitySettings:
    """Observability configuration plumbed from Python into Rust subprocesses."""

    level: RustLogLevel = RustLogLevel.INFO
    format: ObservabilityFormat = ObservabilityFormat.JSON
    service_name: str = "secure-tunnel-fastapi-fixture"
    rust_log: str | None = None
    otlp_endpoint: str | None = None
    resource_attributes: dict[str, str] = field(
        default_factory=lambda: {
            "deployment.environment": "test",
            "service.namespace": "secure-tunnel",
        },
    )

    @classmethod
    def from_env(cls) -> ObservabilitySettings:
        """Load observability settings from environment variables."""
        return cls(
            level=_enum_from_env("SECURE_TUNNEL_OBSERVABILITY_LEVEL", RustLogLevel.INFO),
            format=_enum_from_env(
                "SECURE_TUNNEL_OBSERVABILITY_FORMAT",
                ObservabilityFormat.JSON,
            ),
            service_name=os.environ.get(
                "SECURE_TUNNEL_OBSERVABILITY_SERVICE_NAME",
                "secure-tunnel-fastapi-fixture",
            ),
            rust_log=os.environ.get("SECURE_TUNNEL_RUST_LOG") or os.environ.get("RUST_LOG"),
            otlp_endpoint=os.environ.get("OTEL_EXPORTER_OTLP_ENDPOINT"),
        )

    def rust_env(self) -> dict[str, str]:
        """Return environment variables consumed by the Rust CLI process."""
        if self.level is RustLogLevel.OFF:
            return {"SECURE_TUNNEL_OBSERVABILITY": "0"}

        env = {
            "SECURE_TUNNEL_OBSERVABILITY": "1",
            "SECURE_TUNNEL_OBSERVABILITY_FORMAT": self.format.value,
            "SECURE_TUNNEL_OBSERVABILITY_LEVEL": self.level.value,
            "SECURE_TUNNEL_OBSERVABILITY_SERVICE_NAME": self.service_name,
            "RUST_LOG": self.rust_log or _default_rust_log(self.level),
            "OTEL_SERVICE_NAME": self.service_name,
        }
        if self.otlp_endpoint:
            env["OTEL_EXPORTER_OTLP_ENDPOINT"] = self.otlp_endpoint
        if self.resource_attributes:
            env["OTEL_RESOURCE_ATTRIBUTES"] = ",".join(
                f"{key}={value}" for key, value in sorted(self.resource_attributes.items())
            )
        return env

    def public_metadata(self) -> JsonMap:
        """Return non-sensitive observability settings for readiness output."""
        return {
            "level": self.level.value,
            "format": self.format.value,
            "service_name": self.service_name,
            "otlp_endpoint_configured": self.otlp_endpoint is not None,
        }


@dataclass(frozen=True)
class FixtureSettings:
    """Typed settings for the Python FastAPI fixture shell."""

    runtime: FixtureRuntime = FixtureRuntime.BINDING_FIXTURE
    binding_fixture_bin: Path | None = None
    working_directory: Path | None = None
    startup_timeout_seconds: float = 10.0
    shutdown_timeout_seconds: float = 5.0
    kill_timeout_seconds: float = 2.0
    source: str = "secure-tunnel-fastapi-fixture"
    test_only: bool = True
    observability: ObservabilitySettings = field(default_factory=ObservabilitySettings)

    @classmethod
    def from_env(cls) -> FixtureSettings:
        """Load fixture settings from environment variables."""
        configured_bin = os.environ.get("SECURE_TUNNEL_BINDING_FIXTURE_BIN")
        configured_workdir = os.environ.get("SECURE_TUNNEL_FIXTURE_WORKDIR")
        return cls(
            binding_fixture_bin=Path(configured_bin) if configured_bin else None,
            working_directory=Path(configured_workdir) if configured_workdir else None,
            startup_timeout_seconds=_float_from_env(
                "SECURE_TUNNEL_FIXTURE_STARTUP_TIMEOUT_SECONDS",
                10.0,
            ),
            shutdown_timeout_seconds=_float_from_env(
                "SECURE_TUNNEL_FIXTURE_SHUTDOWN_TIMEOUT_SECONDS",
                5.0,
            ),
            kill_timeout_seconds=_float_from_env(
                "SECURE_TUNNEL_FIXTURE_KILL_TIMEOUT_SECONDS",
                2.0,
            ),
            observability=ObservabilitySettings.from_env(),
        )

    def command(self) -> list[str]:
        """Return the Rust fixture command for this settings object."""
        if self.runtime is not FixtureRuntime.BINDING_FIXTURE:
            raise RuntimeError(f"unsupported fixture runtime: {self.runtime}")
        binary = self.binding_fixture_bin or _repo_root() / "target/debug/secure-tunnel-cli"
        return [str(binary), "binding-fixture", "--format", "json"]


class FixtureLifecycle(Protocol):
    """Lifecycle contract used by the FastAPI fixture shell."""

    def start(self) -> None: ...

    def stop(self) -> None: ...

    def is_running(self) -> bool: ...

    def report(self) -> JsonMap: ...


class FixtureController:
    """Starts and stops the Rust binding-fixture subprocess."""

    def __init__(
        self,
        settings: FixtureSettings | None = None,
        command: list[str] | None = None,
    ) -> None:
        self._settings = settings or FixtureSettings.from_env()
        self._command = command or self._settings.command()
        self._process: subprocess.Popen[str] | None = None
        self._stderr_file: TextIO | None = None
        self._report: JsonMap | None = None

    @property
    def settings(self) -> FixtureSettings:
        """Return the immutable fixture settings."""
        return self._settings

    def start(self) -> None:
        """Start the fixture subprocess and capture its bootstrap report."""
        if self.is_running():
            return

        env = os.environ.copy()
        env.update(self._settings.observability.rust_env())
        stderr_file = tempfile.TemporaryFile(mode="w+", encoding="utf-8")
        process = subprocess.Popen(
            self._command,
            cwd=self._settings.working_directory or _repo_root(),
            env=env,
            stdout=subprocess.PIPE,
            stderr=stderr_file,
            text=True,
        )
        if process.stdout is None:
            _stop_process(process, self._settings, graceful=False, stderr_file=stderr_file)
            stderr_file.close()
            raise RuntimeError("fixture stdout was not captured")

        try:
            line = _readline_with_timeout(process, self._settings.startup_timeout_seconds)
            if not line:
                raise RuntimeError("fixture did not emit bootstrap JSON")
            loaded = json.loads(line)
            if not isinstance(loaded, dict):
                raise RuntimeError("fixture bootstrap JSON must be an object")
        except Exception as error:
            _stdout, stderr = _stop_process(
                process,
                self._settings,
                graceful=False,
                stderr_file=stderr_file,
            )
            stderr_file.close()
            suffix = f": {stderr.strip()}" if stderr.strip() else ""
            raise RuntimeError(f"fixture startup failed: {error}{suffix}") from error

        self._process = process
        self._stderr_file = stderr_file
        self._report = cast(JsonMap, loaded)

    def stop(self) -> None:
        """Stop the fixture subprocess."""
        process = self._process
        stderr_file = self._stderr_file
        self._process = None
        self._stderr_file = None
        self._report = None
        if process is None:
            return

        _stop_process(process, self._settings, graceful=True, stderr_file=stderr_file)
        if stderr_file is not None:
            stderr_file.close()

    def is_running(self) -> bool:
        """Return whether the fixture subprocess is active."""
        return self._process is not None and self._process.poll() is None

    def report(self) -> JsonMap:
        """Return the captured binding fixture report."""
        if self._report is None:
            raise RuntimeError("fixture has not started")
        return dict(self._report)


def create_app(
    controller: FixtureLifecycle | None = None,
    settings: FixtureSettings | None = None,
) -> FastAPI:
    """Create the FastAPI app around a Rust-backed fixture lifecycle."""
    app_settings = settings or FixtureSettings.from_env()
    fixture = controller or FixtureController(app_settings)
    if isinstance(fixture, FixtureController):
        app_settings = fixture.settings

    @asynccontextmanager
    async def lifespan(_app: FastAPI) -> AsyncIterator[None]:
        fixture.start()
        try:
            yield
        finally:
            fixture.stop()

    app = FastAPI(title="Secure Tunnel Fixture", lifespan=lifespan)

    @app.get("/health")
    def health() -> JsonMap:
        return {
            "ok": True,
            "fixture_running": fixture.is_running(),
            "test_only": app_settings.test_only,
            "observability": app_settings.observability.public_metadata(),
        }

    @app.get("/binding-fixture")
    def binding_fixture() -> JsonMap:
        report = fixture.report()
        return {
            "source": app_settings.source,
            "test_only": app_settings.test_only,
            **report,
        }

    @app.get("/bootstrap")
    def bootstrap() -> JsonMap:
        report = fixture.report()
        return {
            "source": app_settings.source,
            "test_only": app_settings.test_only,
            "observability": app_settings.observability.public_metadata(),
            **report,
        }

    @app.get("/descriptor")
    def descriptor() -> JsonMap:
        report = fixture.report()
        descriptor_json = report.get("descriptor_json")
        if not isinstance(descriptor_json, str):
            raise HTTPException(status_code=500, detail="missing descriptor_json")
        descriptor_value = json.loads(descriptor_json)
        if not isinstance(descriptor_value, dict):
            raise HTTPException(status_code=500, detail="descriptor_json must decode to an object")
        return {
            "source": app_settings.source,
            "test_only": app_settings.test_only,
            "now_unix_seconds": report["now_unix_seconds"],
            "descriptor": descriptor_value,
            "pinned_service_static_public_keys_b64": report[
                "pinned_service_static_public_keys_b64"
            ],
        }

    return app


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[3]


def _default_rust_log(level: RustLogLevel) -> str:
    targets = [
        "secure_tunnel_cli",
        "secure_tunnel_core",
        "secure_tunnel_harness",
        "secure_tunnel_sdk",
        "secure_tunnel_transport",
    ]
    return ",".join(f"{target}={level.value}" for target in targets)


def _enum_from_env[T: StrEnum](name: str, default: T) -> T:
    value = os.environ.get(name)
    if value is None:
        return default
    enum_type = type(default)
    try:
        return enum_type(value.lower())
    except ValueError as error:
        raise RuntimeError(f"invalid {name}: {value}") from error


def _float_from_env(name: str, default: float) -> float:
    value = os.environ.get(name)
    if value is None:
        return default
    try:
        parsed = float(value)
    except ValueError as error:
        raise RuntimeError(f"invalid {name}: {value}") from error
    if parsed <= 0:
        raise RuntimeError(f"{name} must be positive")
    return parsed


def _readline_with_timeout(process: subprocess.Popen[str], timeout_seconds: float) -> str:
    if process.stdout is None:
        raise RuntimeError("fixture stdout was not captured")

    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    deadline = time.monotonic() + timeout_seconds
    try:
        while True:
            if process.poll() is not None:
                return process.stdout.readline()
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("timed out waiting for fixture bootstrap JSON")
            if selector.select(min(remaining, 0.1)):
                return process.stdout.readline()
    finally:
        selector.close()


def _stop_process(
    process: subprocess.Popen[str],
    settings: FixtureSettings,
    *,
    graceful: bool,
    stderr_file: TextIO | None = None,
) -> tuple[str, str]:
    if process.poll() is None:
        if graceful:
            process.send_signal(signal.SIGINT)
        else:
            process.terminate()
    try:
        stdout, stderr = process.communicate(timeout=settings.shutdown_timeout_seconds)
    except subprocess.TimeoutExpired:
        process.kill()
        try:
            stdout, stderr = process.communicate(timeout=settings.kill_timeout_seconds)
        except subprocess.TimeoutExpired:
            return "", "fixture process did not exit after kill"
    return stdout or "", _stderr_text(stderr_file, stderr)


def _stderr_text(stderr_file: TextIO | None, fallback: str | None) -> str:
    if stderr_file is None:
        return fallback or ""
    stderr_file.flush()
    stderr_file.seek(0)
    return stderr_file.read()


app = create_app()
