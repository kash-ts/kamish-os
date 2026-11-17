#!/usr/bin/env bash
# Установка кросс-инструментов для сборки Kamish под aarch64 в WSL2
set -eu

echo "=== [root] apt: cross toolchain + build deps ==="
apt-get update -qq
DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    gcc-aarch64-linux-gnu \
    build-essential \
    pkg-config \
    libssl-dev \
    curl \
    file

echo "=== [user ipoma] rustup ==="
su - ipoma -c '
set -e
curl --proto =https --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
source "$HOME/.cargo/env"
rustup target add aarch64-unknown-linux-gnu
rustc --version
rustup target list --installed
'
echo "TOOLCHAIN OK"
