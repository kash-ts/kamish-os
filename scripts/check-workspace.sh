#!/bin/bash
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR" || exit 1
export CARGO_TARGET_DIR="${HOME}/.cache/kamish-target"
unset LD_LIBRARY_PATH
mkdir -p "$CARGO_TARGET_DIR"
echo "CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
cargo check --workspace --all-targets 2>&1 | tail -45
