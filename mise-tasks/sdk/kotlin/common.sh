#!/usr/bin/env bash
# Shared shell helpers for Kotlin SDK package tasks.

set -euo pipefail

kotlin_sdk_output_root() {
  printf '%s\n' "${MISE_PROJECT_ROOT}/target/sdk/kotlin"
}

kotlin_sdk_package_root() {
  printf '%s\n' "$(kotlin_sdk_output_root)/SecureTunnelKotlin"
}

kotlin_sdk_consumer_root() {
  printf '%s\n' "$(kotlin_sdk_output_root)/consumer"
}

kotlin_sdk_maven_repo() {
  printf '%s\n' "$(kotlin_sdk_output_root)/maven"
}

kotlin_sdk_library_name() {
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

kotlin_sdk_jna_resource_dir() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "${arch}" in
    arm64 | aarch64)
      arch="aarch64"
      ;;
    x86_64 | amd64)
      arch="x86-64"
      ;;
    *)
      echo "unsupported architecture: ${arch}" >&2
      return 1
      ;;
  esac
  case "${os}" in
    Darwin)
      printf 'darwin-%s\n' "${arch}"
      ;;
    Linux)
      printf 'linux-%s\n' "${arch}"
      ;;
    CYGWIN* | MINGW* | MSYS*)
      printf 'win32-%s\n' "${arch}"
      ;;
    *)
      echo "unsupported platform: ${os}" >&2
      return 1
      ;;
  esac
}

kotlin_sdk_remove_under_target() {
  local path="$1"
  case "${path}" in
    "${MISE_PROJECT_ROOT}/target/"*)
      rm -rf "${path}"
      ;;
    *)
      echo "refusing to remove path outside target: ${path}" >&2
      return 1
      ;;
  esac
}
