#!/usr/bin/env python3
# Copyright 2026 Asim Ihsan
#
# This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
# If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
#
# SPDX-License-Identifier: MPL-2.0

"""End-to-end smoke from the Rust client CLI to the Python FastAPI fixture."""

from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any
from urllib.error import URLError
from urllib.request import urlopen

REPO_ROOT = Path(__file__).resolve().parents[2]
PYTHON_ROOT = REPO_ROOT / "python"


def main() -> int:
    cli_bin = Path(
        os.environ.get("SECURE_TUNNEL_CLI_BIN", REPO_ROOT / "target/debug/secure-tunnel-cli"),
    )
    if not cli_bin.exists():
        print(f"missing secure-tunnel-cli binary: {cli_bin}", file=sys.stderr)
        return 2

    port = _free_port()
    env = os.environ.copy()
    env["SECURE_TUNNEL_BINDING_FIXTURE_BIN"] = str(cli_bin)
    server = subprocess.Popen(
        [
            sys.executable,
            "-m",
            "uvicorn",
            "secure_tunnel.fastapi_fixture:app",
            "--host",
            "127.0.0.1",
            "--port",
            str(port),
            "--log-level",
            "warning",
        ],
        cwd=PYTHON_ROOT,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    failed = True
    try:
        base_url = f"http://127.0.0.1:{port}"
        _wait_for_health(server, f"{base_url}/health")
        fixture = _get_json(f"{base_url}/binding-fixture")
        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as fixture_file:
            json.dump(fixture, fixture_file)
            fixture_path = Path(fixture_file.name)
        try:
            client = subprocess.run(
                [str(cli_bin), "binding-fixture-client", str(fixture_path), "--format", "json"],
                cwd=REPO_ROOT,
                check=False,
                capture_output=True,
                text=True,
            )
        finally:
            fixture_path.unlink(missing_ok=True)
        if client.returncode != 0:
            print(client.stdout, file=sys.stderr)
            print(client.stderr, file=sys.stderr)
            return client.returncode

        report = json.loads(client.stdout)
        if report["selected_carrier"] != "quic":
            raise AssertionError(f"expected quic, got {report['selected_carrier']!r}")
        if report["close_classification"] != "graceful":
            raise AssertionError(f"expected graceful close, got {report['close_classification']!r}")
        if not report["ok"] or not report["secure_ready"] or not report["application_exchange"]:
            raise AssertionError(f"unexpected Rust client report: {report!r}")

        print(
            json.dumps(
                {
                    "close": report["close_classification"],
                    "language": "rust-client",
                    "server": "python-fastapi-fixture",
                    "carrier": report["selected_carrier"],
                },
                sort_keys=True,
            ),
        )
        failed = False
        return 0
    finally:
        stdout, stderr = _stop_server(server)
        if failed:
            logs = "\n".join(part for part in (stdout.strip(), stderr.strip()) if part)
            if logs:
                print(logs, file=sys.stderr)


def _free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def _wait_for_health(process: subprocess.Popen[str], url: str) -> None:
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"FastAPI fixture exited early with {process.returncode}")
        try:
            health = _get_json(url)
        except URLError, TimeoutError, ConnectionError:
            time.sleep(0.1)
            continue
        if health.get("ok") is True and health.get("fixture_running") is True:
            return
        time.sleep(0.1)
    raise TimeoutError("FastAPI fixture did not become healthy")


def _get_json(url: str) -> dict[str, Any]:
    with urlopen(url, timeout=1) as response:
        loaded = json.loads(response.read().decode("utf-8"))
    if not isinstance(loaded, dict):
        raise RuntimeError(f"expected JSON object from {url}")
    return loaded


def _stop_server(process: subprocess.Popen[str]) -> tuple[str, str]:
    if process.poll() is None:
        process.terminate()
        try:
            return process.communicate(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
    return process.communicate(timeout=5)


if __name__ == "__main__":
    raise SystemExit(main())
