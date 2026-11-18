#!/usr/bin/env bash
set -e

BOARD_DIR="$(dirname "$0")"
BINARIES_DIR="${BINARIES_DIR:-$1}"

# Ensure Allwinner sun60iw2 bootloader blobs are copied to images dir for genimage
cp "${BOARD_DIR}/bootloader/boot0_sdcard.fex" "${BINARIES_DIR}/boot0_sdcard.fex"
cp "${BOARD_DIR}/bootloader/boot_package.fex" "${BINARIES_DIR}/boot_package.fex"

# Run Buildroot's default genimage script
exec support/scripts/genimage.sh "$@"
