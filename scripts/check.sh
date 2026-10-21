#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."

# Local checks only. Never dispatch hosted builds or publish releases here.
cargo +stable fmt --all -- --check
cargo +stable test --locked --workspace --all-targets
cargo +stable clippy --locked --workspace --all-targets -- -D warnings
python3 -m unittest discover -s scripts -p 'test_*.py' -v
cargo +stable build --locked
release_version="$(cargo +stable xtask version)"
cargo +stable xtask release --version "$release_version" --dry-run
