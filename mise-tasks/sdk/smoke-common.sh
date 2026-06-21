#!/usr/bin/env bash
# Shared shell helpers for UniFFI binding smoke tasks.

set -euo pipefail

secure_tunnel_library_name() {
  case "$(uname -s)" in
    Darwin)
      printf '%s\n' "libsecure_tunnel_sdk_ffi.dylib"
      ;;
    Linux)
      printf '%s\n' "libsecure_tunnel_sdk_ffi.so"
      ;;
    CYGWIN* | MINGW* | MSYS*)
      printf '%s\n' "secure_tunnel_sdk_ffi.dll"
      ;;
    *)
      echo "unsupported platform: $(uname -s)" >&2
      return 1
      ;;
  esac
}

start_secure_tunnel_fixture() {
  local fixture_json="$1"
  cargo build -p secure-tunnel-cli >/dev/null
  target/debug/secure-tunnel-cli binding-fixture --format json >"${fixture_json}" &
  SECURE_TUNNEL_FIXTURE_PID=$!
  export SECURE_TUNNEL_FIXTURE_PID
  for _ in $(seq 1 100); do
    if [[ -s "${fixture_json}" ]]; then
      return 0
    fi
    if ! kill -0 "${SECURE_TUNNEL_FIXTURE_PID}" 2>/dev/null; then
      wait "${SECURE_TUNNEL_FIXTURE_PID}"
      return 1
    fi
    sleep 0.1
  done
  echo "timed out waiting for binding fixture" >&2
  return 1
}

stop_secure_tunnel_fixture() {
  if [[ -n "${SECURE_TUNNEL_FIXTURE_PID:-}" ]]; then
    kill "${SECURE_TUNNEL_FIXTURE_PID}" 2>/dev/null || true
    wait "${SECURE_TUNNEL_FIXTURE_PID}" 2>/dev/null || true
  fi
}
