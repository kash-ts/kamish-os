#!/usr/bin/env bash
# Разведка BSP-ядра sun60iw2
set -u
cd /tmp/linux-sun60iw2

echo "=== kernel version ==="
head -5 Makefile | grep -E "VERSION|PATCHLEVEL|SUBLEVEL"

echo "=== DTB exists ==="
ls -la arch/arm64/boot/dts/allwinner/ | grep -E "a733" || echo "NO A733 DTS"

echo "=== arm64 defconfigs (sun60/orangepi/a733) ==="
ls arch/arm64/configs/ | grep -iE "sun60|a733|orangepi|4pro" || echo "none found"

echo "=== all arm64 defconfigs (первый десяток) ==="
ls arch/arm64/configs/ | head -15
