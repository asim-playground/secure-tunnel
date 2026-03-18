#!/usr/bin/env bash

set -euo pipefail

required_targets=(
    "wasm32-unknown-unknown"
    "wasm32-wasip1"
)

if ! command -v rustc >/dev/null 2>&1; then
    echo "rustc not found; run mise install for the pinned Rust toolchain first" >&2
    exit 1
fi

toolchain="${RUSTUP_TOOLCHAIN:-$(rustc -V | awk '{print $2}')}"
cargo_bin_dir="$(dirname "$(command -v cargo)")"

if [[ -x "$cargo_bin_dir/rustup" ]]; then
    rustup_bin="$cargo_bin_dir/rustup"
elif command -v rustup >/dev/null 2>&1; then
    rustup_bin="$(command -v rustup)"
else
    echo "rustup not found; install the pinned Rust toolchain via mise first" >&2
    exit 1
fi

installed_targets="$("$rustup_bin" target list --installed --toolchain "$toolchain")"
missing_targets=()

for target in "${required_targets[@]}"; do
    if ! grep -qx "$target" <<<"$installed_targets"; then
        missing_targets+=("$target")
    fi
done

if (( ${#missing_targets[@]} == 0 )); then
    echo "Rust WASM targets already installed for $toolchain"
    exit 0
fi

"$rustup_bin" target add "${missing_targets[@]}" --toolchain "$toolchain"
