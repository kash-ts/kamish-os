#!/usr/bin/env bash
# Установка rustup с retry (сеть рвёт TLS-соединения)
set -u
for i in 1 2 3 4 5 6 7 8; do
    echo "--- rustup attempt $i"
    if curl --proto '=https' --tlsv1.2 -sSf --retry 5 --retry-all-errors \
        https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal; then
        break
    fi
    sleep 5
done
source "$HOME/.cargo/env" || { echo "rustup install FAILED"; exit 1; }
for i in 1 2 3 4 5; do
    if rustup target add aarch64-unknown-linux-gnu; then
        break
    fi
    sleep 5
done
rustc --version
rustup target list --installed
echo "RUST OK"
