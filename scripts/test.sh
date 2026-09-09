#!/usr/bin/env bash
# Complete local verification gate for the Rust runtime.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

echo "== offline packaging fixtures =="
bash -n scripts/install.sh scripts/uninstall.sh scripts/packaging-manifest.sh tests/packaging.sh
sh -n bin/cmux-herdr-fetch
bash tests/packaging.sh
python3 tests/packaging_release.py

echo "== cargo fmt =="
cargo fmt --all --check

echo "== cargo clippy =="
cargo clippy --all-targets --all-features -- -D warnings

echo "== cargo test =="
cargo test --locked

echo "OK: all cmux-herdr Rust checks passed"
