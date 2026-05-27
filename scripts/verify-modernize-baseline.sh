#!/usr/bin/env bash
set -euo pipefail

repo_root="${1:-.}"
cd "$repo_root"

workflow=".github/workflows/ci.yml"

require_file() {
  local path="$1"
  test -f "$path" || {
    echo "missing required file: $path" >&2
    exit 1
  }
}

require_symlink() {
  local path="$1"
  local target="$2"
  test -L "$path" || {
    echo "$path must be a symlink" >&2
    exit 1
  }
  test "$(readlink "$path")" = "$target" || {
    echo "$path must point at $target" >&2
    exit 1
  }
}

require_contains() {
  local path="$1"
  local pattern="$2"
  grep -Fq -- "$pattern" "$path" || {
    echo "$path is missing: $pattern" >&2
    exit 1
  }
}

require_job_contains() {
  local job="$1"
  local pattern="$2"
  awk -v job="$job" -v pattern="$pattern" '
    $0 ~ "^  " job ":" { in_job = 1; next }
    in_job && $0 ~ "^  [[:alnum:]_-]+:" { in_job = 0 }
    in_job && index($0, pattern) { found = 1 }
    END { exit found ? 0 : 1 }
  ' "$workflow" || {
    echo "$workflow job $job is missing: $pattern" >&2
    exit 1
  }
}

require_file Cargo.toml
require_file Cargo.lock
require_file "$workflow"
require_file dprint.json
require_file AGENTS.md
require_symlink CLAUDE.md AGENTS.md

require_job_contains fmt "cargo fmt --all -- --check"
require_job_contains clippy "cargo clippy"
require_job_contains test "cargo test"
require_job_contains test "actions-rust-lang/setup-rust-toolchain@v1"
require_job_contains dprint "dprint/check@v2.3"
require_job_contains modernize-baseline "scripts/test-modernize-baseline.sh"
require_job_contains modernize-baseline "scripts/verify-modernize-baseline.sh"

require_job_contains clippy "actions/cache@v4"
require_job_contains test "actions/cache@v4"

if grep -R "actions-rs/" .github/workflows >/dev/null; then
  echo "deprecated actions-rs action found" >&2
  exit 1
fi

for command in \
  "cargo fmt --all -- --check" \
  "cargo clippy" \
  "cargo test" \
  "dprint check" \
  "scripts/test-modernize-baseline.sh" \
  "scripts/verify-modernize-baseline.sh"
do
  require_contains AGENTS.md "$command"
done

require_contains AGENTS.md "Conventional Commit"
require_contains AGENTS.md "Allowed types: \`feat\`, \`fix\`, \`docs\`, \`perf\`, \`refactor\`, \`style\`, \`test\`, \`chore\`, \`ci\`, \`revert\`."
require_contains dprint.json "https://plugins.dprint.dev/json-"
require_contains dprint.json "https://plugins.dprint.dev/markdown-"
require_contains dprint.json "https://plugins.dprint.dev/toml-"

if grep -R '^[[:space:]]*log[[:space:]]*=' -- Cargo.toml crates/*/Cargo.toml >/dev/null; then
  echo "direct log dependency found; use tracing instead" >&2
  exit 1
fi
