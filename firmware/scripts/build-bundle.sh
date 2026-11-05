#!/usr/bin/env bash
# Packs and signs a KAMISH OS update bundle (.raucb)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KEYS_DIR="${SCRIPT_DIR}/../keys"
BOARD_DIR="${SCRIPT_DIR}/../buildroot-external-kamish/board/kamish/orangepi-4-pro"
VERSION="${1:-1.0.0}"
BUILD_ID="${2:-$(date +%Y%m%d%H%M)}"
INPUT_ROOTFS="${3:-}"

if [[ -z "${INPUT_ROOTFS}" || ! -f "${INPUT_ROOTFS}" ]]; then
    echo "Usage: $0 <version> <build_id> <path-to-rootfs.ext4>"
    echo "Example: $0 1.0.1 20261101 /path/to/output/images/rootfs.ext4"
    exit 1
fi

BUNDLE_DIR=$(mktemp -d /tmp/kamish-bundle-XXXXXX)
trap 'rm -rf "${BUNDLE_DIR}"' EXIT

echo "=== Preparing KAMISH OS Update Bundle (v${VERSION}, build ${BUILD_ID}) ==="

# 1. Copy rootfs image
cp "${INPUT_ROOTFS}" "${BUNDLE_DIR}/rootfs.ext4"

# 2. Render manifest
sed -e "s/@VERSION@/${VERSION}/g" \
    -e "s/@BUILD_ID@/${BUILD_ID}/g" \
    "${BOARD_DIR}/rauc/manifest.raucm.template" > "${BUNDLE_DIR}/manifest.raucm"

OUTPUT_BUNDLE="${SCRIPT_DIR}/../kamish-os-v${VERSION}-${BUILD_ID}.raucb"

# 3. Sign and pack bundle with RAUC
if command -v rauc >/dev/null 2>&1; then
    rauc bundle \
        --cert="${KEYS_DIR}/development-1.cert.pem" \
        --key="${KEYS_DIR}/development-1.key.pem" \
        "${BUNDLE_DIR}" \
        "${OUTPUT_BUNDLE}"
    echo "=== RAUC Bundle signed and created: ${OUTPUT_BUNDLE} ==="
else
    echo "Notice: 'rauc' command not found on host. Packing tar bundle container for buildroot..."
    tar -czf "${OUTPUT_BUNDLE}.tar.gz" -C "${BUNDLE_DIR}" manifest.raucm rootfs.ext4
    echo "Tarball created: ${OUTPUT_BUNDLE}.tar.gz (sign with rauc bundle inside container)"
fi
