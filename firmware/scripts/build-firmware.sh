#!/usr/bin/env bash
# Orchestrates building KAMISH OS image and RAUC bundle using Docker

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIRMWARE_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Building KAMISH OS via Docker Container ==="

cd "${FIRMWARE_DIR}"

mkdir -p output dl keys

# 1. Ensure signing keys exist
"${SCRIPT_DIR}/gen-keys.sh"

# 2. Pre-fetch the Antigravity CLI so it can be bundled into the image.
#    This step is best-effort: it never fails the build, and can be disabled
#    with KAMISH_SKIP_AGY=1 for fully offline builds.
AGY_FETCHER="${FIRMWARE_DIR}/../scripts/fetch-agy.sh"
if [ -x "${AGY_FETCHER}" ]; then
    echo "=== Pre-fetching Antigravity CLI ==="
    KAMISH_AGY_DIR="${KAMISH_AGY_DIR:-${FIRMWARE_DIR}/dl/agy}" \
        "${AGY_FETCHER}" || echo "!! agy prefetch skipped (build continues)"
else
    echo "!! fetch-agy.sh not found, skipping agy prefetch"
fi

# 3. Build via docker compose
if command -v docker >/dev/null 2>&1; then
    docker compose build builder
    docker compose run --rm builder
    echo "=== Build finished! Output images available in firmware/output/images/ ==="
else
    echo "Docker is not available. Please install Docker or run Buildroot natively on an x86_64 Linux host."
    echo "Command: make BR2_EXTERNAL=${FIRMWARE_DIR}/buildroot-external-kamish kamish_orangepi_4_pro_defconfig && make"
fi
