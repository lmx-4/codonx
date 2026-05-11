#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "[codonx check] cargo fmt"
cargo fmt --all -- --check

echo "[codonx check] cargo check"
cargo check --locked

echo "[codonx check] cargo clippy"
cargo clippy --locked --all-targets -- -D warnings

echo "[codonx check] cargo test"
cargo test --locked

echo "[codonx check] release build"
cargo build --release --locked

echo "[codonx check] done"
