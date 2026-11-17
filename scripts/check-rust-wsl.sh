#!/usr/bin/env bash
# Проверка Rust-окружения WSL для кросс-сборки aarch64
set -u
echo "=== rustup/cargo ==="
if command -v rustup >/dev/null 2>&1; then
    rustup --version
    rustup toolchain list
    rustup target list --installed
else
    echo "rustup MISSING"
fi
echo "=== cross gcc ==="
if command -v aarch64-linux-gnu-gcc >/dev/null 2>&1; then
    aarch64-linux-gnu-gcc --version | head -1
else
    echo "aarch64-linux-gnu-gcc MISSING"
fi
echo "=== disk ==="
df -h "$HOME" | tail -1
