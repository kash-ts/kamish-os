#!/bin/bash
cd /mnt/m/Project/Kamish-os/radiant-bardeen || exit 1
export CARGO_TARGET_DIR="/home/$(whoami)/.cache/kamish-target"
unset LD_LIBRARY_PATH
mkdir -p "$CARGO_TARGET_DIR"
echo "CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
cargo check --workspace 2>&1 | tail -45
