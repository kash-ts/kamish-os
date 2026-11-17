#!/usr/bin/env bash
# Локальная репродукция ошибки external-дерева Buildroot
set -u
cd "$HOME"
rm -rf buildroot-test
mkdir -p buildroot-test
tar -xzf /mnt/m/Project/Kamish-os/radiant-bardeen/firmware/buildroot-2024.02.1.tar.gz \
    -C buildroot-test --strip-components=1
cd buildroot-test

echo "=== как buildroot разбирает external.desc ==="
grep -rn "EXTERNAL_DESC\|external.desc" Makefile support/scripts/br2-external | head -10

echo "=== попытка defconfig ==="
make BR2_EXTERNAL=/mnt/m/Project/Kamish-os/radiant-bardeen/firmware/buildroot-external-kamish kamish_orangepi_4_pro_defconfig 2>&1 | tail -5
