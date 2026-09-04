#!/usr/bin/env bash
# Complete local verification gate for the Rust runtime.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

echo "== cargo fmt =="
cargo fmt --all --check

echo "== cargo clippy =="
cargo clippy --all-targets --all-features -- -D warnings

echo "== cargo test =="
cargo test --locked

echo "OK: all cmux-herdr Rust checks passed"
