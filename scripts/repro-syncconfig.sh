#!/usr/bin/env bash
# Локальная репродукция ошибки syncconfig внутри контейнера
set -u
cd /mnt/m/Project/Kamish-os/radiant-bardeen/firmware

docker compose run --rm builder bash -c '
set -x
mkdir -p /build/kamish-src/target/aarch64-unknown-linux-gnu/release
cp -r /workspace/Cargo.toml /workspace/Cargo.lock /workspace/database /build/kamish-src/ 2>&1 | head -3
rsync -a --exclude node_modules /workspace/crates /build/kamish-src/ 2>&1 | head -3
make BR2_EXTERNAL=/build/external KAMISH_SITE=/build/kamish-src kamish_orangepi_4_pro_defconfig 2>&1 | tail -5
echo "=== defconfig done, running syncconfig stage ==="
timeout 60 make O=/build/buildroot/output HOSTCC="/usr/bin/gcc" syncconfig 2>&1 | tail -20
echo "=== syncconfig stage finished ==="
'
