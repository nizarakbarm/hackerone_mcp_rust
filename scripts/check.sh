#!/usr/bin/env bash
# One check command: format, lint, test. CI runs exactly this.
set -euo pipefail
cd "$(dirname "$0")/.."

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
