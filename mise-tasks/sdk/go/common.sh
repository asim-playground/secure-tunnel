#!/usr/bin/env bash
# Shared shell helpers for native Go SDK package tasks.

set -euo pipefail

# shellcheck disable=SC1091
source "${MISE_PROJECT_ROOT:?}/scripts/tasks/go-env.sh"

go_sdk_package_root() {
	printf '%s\n' "${MISE_PROJECT_ROOT:?}/crates/go"
}

go_sdk_header_path() {
	printf '%s\n' "${MISE_PROJECT_ROOT:?}/crates/go/binding.h"
}

go_sdk_version() {
	sed -n 's/^const Version = "\([^"]*\)"/\1/p' \
		"${MISE_PROJECT_ROOT:?}/crates/go/types.go" | head -n 1
}

go_sdk_module_path() {
	sed -n 's/^module \([^[:space:]]*\)/\1/p' \
		"${MISE_PROJECT_ROOT:?}/crates/go/go.mod" | head -n 1
}

go_sdk_release_archive_path() {
	printf '%s\n' \
		"${MISE_PROJECT_ROOT:?}/target/sdk-release/artifacts/secure_tunnel_go-$(go_sdk_version).tar"
}

go_sdk_native_platform() {
	local os arch
	case "$(uname -s)" in
		Linux)
			os="linux"
			;;
		Darwin)
			os="darwin"
			;;
		CYGWIN* | MINGW* | MSYS*)
			os="windows"
			;;
		*)
			echo "unsupported Go native SDK platform: $(uname -s)" >&2
			return 1
			;;
	esac
	case "$(uname -m)" in
		arm64 | aarch64)
			arch="arm64"
			;;
		x86_64 | amd64)
			arch="amd64"
			;;
		*)
			echo "unsupported Go native SDK architecture: $(uname -m)" >&2
			return 1
			;;
	esac
	printf '%s-%s\n' "${os}" "${arch}"
}
