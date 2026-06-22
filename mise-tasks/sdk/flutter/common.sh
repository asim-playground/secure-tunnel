#!/usr/bin/env bash
# Shared shell helpers for Flutter SDK package tasks.

set -euo pipefail

flutter_sdk_output_root() {
  printf '%s\n' "${MISE_PROJECT_ROOT:?}/target/sdk/flutter"
}

flutter_sdk_package_root() {
  printf '%s\n' "$(flutter_sdk_output_root)/secure_tunnel_flutter"
}

flutter_sdk_remove_under_target() {
  local path="$1"
  case "${path}" in
    "${MISE_PROJECT_ROOT:?}/target/"*)
      rm -rf "${path}"
      ;;
    *)
      echo "refusing to remove path outside target/: ${path}" >&2
      return 1
      ;;
  esac
}

flutter_sdk_codegen_bin() {
  printf '%s\n' "$(mise where cargo:flutter_rust_bridge_codegen)/bin/flutter_rust_bridge_codegen"
}
