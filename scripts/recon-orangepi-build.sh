#!/usr/bin/env bash
# Разведка orangepi-build: как собирается u-boot и boot0 для sun60iw2 (A733)
set -u
cd /tmp/orangepi-build

echo "=== board configs ==="
ls lib/config/boards/ 2>/dev/null | grep -i -E "4pro|a733|zero3w|zero4" || true

echo "=== grep sun60iw2 / a733 in build scripts ==="
grep -rn -i -l "sun60iw2\|a733" lib external userpatches 2>/dev/null | head -30

echo "=== u-boot build script hints (BSP) ==="
ls lib/ 2>/dev/null
