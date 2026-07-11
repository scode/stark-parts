#!/usr/bin/env bash

set -euo pipefail

# Toolchain setup belongs inside the cached task so a cache hit does not install build tools it will never run.
if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 --fail --silent --show-error https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain none
fi

if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck disable=SC1091 -- rustup owns this stable per-user entrypoint.
  source "$HOME/.cargo/env"
fi

rustup toolchain install
if [[ "$(trunk --version 2>/dev/null || true)" != "trunk 0.21.14" ]]; then
  cargo install trunk --version 0.21.14 --locked
fi

NO_COLOR=true trunk build --release
