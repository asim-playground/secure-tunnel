#!/usr/bin/env bash
# mise sources this file with bash to collect exported environment variables.

case "$(uname -s)" in
  Linux)
    if [[ -r /etc/os-release ]]; then
      # shellcheck disable=SC1091
      source /etc/os-release
      if [[ "${ID:-}" == "ubuntu" && "${VERSION_ID:-}" == "24.04" ]]; then
        export MISE_SWIFT_PLATFORM="ubuntu24.04"
      fi
    fi
    ;;
esac
