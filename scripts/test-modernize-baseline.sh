#!/usr/bin/env bash
set -euo pipefail

fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT

copy_fixture() {
  rm -rf "$fixture"/*
  mkdir -p "$fixture/.github/workflows" "$fixture/crates/example" "$fixture/scripts"
  cp Cargo.toml Cargo.lock dprint.json AGENTS.md "$fixture/"
  ln -s AGENTS.md "$fixture/CLAUDE.md"
  cp .github/workflows/ci.yml "$fixture/.github/workflows/ci.yml"
  cp scripts/verify-modernize-baseline.sh "$fixture/scripts/verify-modernize-baseline.sh"
  cp scripts/test-modernize-baseline.sh "$fixture/scripts/test-modernize-baseline.sh"
  cat >"$fixture/crates/example/Cargo.toml" <<'EOF'
[package]
name = "example"
version = "0.0.0"
edition = "2024"
EOF
}

expect_failure() {
  local label="$1"
  shift

  copy_fixture
  "$@"

  if scripts/verify-modernize-baseline.sh "$fixture" >/dev/null 2>&1; then
    echo "expected modernize baseline failure: $label" >&2
    exit 1
  fi
}

copy_fixture
scripts/verify-modernize-baseline.sh "$fixture"

expect_failure "missing CLAUDE symlink" rm "$fixture/CLAUDE.md"
expect_failure "test job missing cargo test" perl -0pi -e 's/cargo test/cargo check/' "$fixture/.github/workflows/ci.yml"
expect_failure "test job missing Rust toolchain setup" perl -0pi -e 's#\n      - uses: actions-rust-lang/setup-rust-toolchain\@v1\n      - run: cargo test#\n      - run: cargo test#' "$fixture/.github/workflows/ci.yml"
expect_failure "dprint missing markdown plugin" perl -0pi -e 's#\n    "https://plugins\.dprint\.dev/markdown-[^"]+",##' "$fixture/dprint.json"
