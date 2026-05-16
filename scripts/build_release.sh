#!/usr/bin/env bash
set -euo pipefail

# codonx Linux release binary builder.
# Linux-first. Builds the Rust MVP into a directly runnable binary.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="$ROOT_DIR/dist"
BIN_NAME="codonx"

cd "$ROOT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
  echo "[codonx build] error: cargo not found. Please install Rust toolchain first." >&2
  exit 1
fi

echo "[codonx build] project root: $ROOT_DIR"
echo "[codonx build] building release binary..."

cargo build --release --locked

mkdir -p "$DIST_DIR"
cp "$ROOT_DIR/target/release/$BIN_NAME" "$DIST_DIR/$BIN_NAME"
chmod +x "$DIST_DIR/$BIN_NAME"

echo "[codonx build] binary created: $DIST_DIR/$BIN_NAME"
echo "[codonx build] version:"
"$DIST_DIR/$BIN_NAME" --version || true
