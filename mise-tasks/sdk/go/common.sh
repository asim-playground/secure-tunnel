#!/usr/bin/env bash
# Shared shell helpers for native Go SDK package tasks.

set -euo pipefail

# shellcheck disable=SC1091
source "${MISE_PROJECT_ROOT:?}/scripts/tasks/go-env.sh"

go_sdk_package_root() {
	printf '%s\n' "${MISE_PROJECT_ROOT:?}/crates/go/go"
}

go_sdk_header_path() {
	printf '%s\n' "${MISE_PROJECT_ROOT:?}/crates/go/binding.h"
}
