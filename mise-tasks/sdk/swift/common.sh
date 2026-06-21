#!/usr/bin/env bash
# Shared shell helpers for Swift SDK package tasks.

set -euo pipefail

: "${MISE_PROJECT_ROOT:?}"

swift_sdk_require_darwin() {
  if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "Swift SDK packaging requires macOS with Xcode" >&2
    exit 1
  fi
}

swift_sdk_output_root() {
  printf '%s\n' "${MISE_PROJECT_ROOT}/target/sdk/swift"
}

swift_sdk_package_root() {
  printf '%s\n' "$(swift_sdk_output_root)/SecureTunnel"
}

swift_sdk_consumer_root() {
  printf '%s\n' "$(swift_sdk_output_root)/SecureTunnelConsumer"
}

swift_sdk_xcframework_path() {
  printf '%s\n' "$(swift_sdk_package_root)/Artifacts/secure_tunnel_sdk_ffiFFI.xcframework"
}

swift_sdk_staticlib_name() {
  printf '%s\n' "libsecure_tunnel_sdk_ffi.a"
}

swift_sdk_generated_swift_dir() {
  printf '%s\n' "${MISE_PROJECT_ROOT}/target/generated-bindings/uniffi/swift"
}

swift_sdk_remove_under_target() {
  local path="$1"
  local root
  local abs
  root="$(cd "${MISE_PROJECT_ROOT}" && pwd -P)"
  abs="$(
    python3 - "${root}" "${path}" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1]).resolve()
path = Path(sys.argv[2]).expanduser()
if not path.is_absolute():
    path = root / path
print(path.resolve(strict=False))
PY
  )"
  case "${abs}" in
    "${root}/target/"*)
      rm -rf "${abs}"
      ;;
    *)
      echo "refusing to remove non-target Swift SDK path: ${abs}" >&2
      exit 1
      ;;
  esac
}

swift_sdk_cargo_build_env() {
  export IPHONEOS_DEPLOYMENT_TARGET="${SECURE_TUNNEL_IOS_DEPLOYMENT_TARGET:-16.0}"
  export MACOSX_DEPLOYMENT_TARGET="${SECURE_TUNNEL_MACOS_DEPLOYMENT_TARGET:-11.0}"
  if [[ -n "${RUSTFLAGS:-}" ]]; then
    export RUSTFLAGS="${RUSTFLAGS} -C debuginfo=0"
  else
    export RUSTFLAGS="-C debuginfo=0"
  fi
}

swift_sdk_find_simulator_udid() {
  xcrun simctl list devices available -j | python3 -c '
import json
import sys

data = json.load(sys.stdin)
devices = []
for runtime, entries in data.get("devices", {}).items():
    if ".iOS-" not in runtime:
        continue
    for device in entries:
        if not device.get("isAvailable", False):
            continue
        if "iPhone" not in device.get("name", ""):
            continue
        devices.append(device)

booted = [device for device in devices if device.get("state") == "Booted"]
selected = (booted or devices)[:1]
if not selected:
    print("no available iOS Simulator iPhone devices found", file=sys.stderr)
    raise SystemExit(1)
print(selected[0]["udid"])
'
}

swift_sdk_xcode_scheme() {
  local package_root="$1"
  (
    cd "${package_root}"
    xcodebuild -list -json
  ) | python3 -c '
import json
import sys

data = json.load(sys.stdin)
schemes = []
for key in ("workspace", "project"):
    schemes.extend(data.get(key, {}).get("schemes", []))

for preferred in ("SecureTunnel-Package", "SecureTunnel"):
    if preferred in schemes:
        print(preferred)
        raise SystemExit(0)
for scheme in schemes:
    if "SecureTunnel" in scheme:
        print(scheme)
        raise SystemExit(0)
if schemes:
    print(schemes[0])
    raise SystemExit(0)
print("no Xcode scheme found for SecureTunnel package", file=sys.stderr)
raise SystemExit(1)
'
}
