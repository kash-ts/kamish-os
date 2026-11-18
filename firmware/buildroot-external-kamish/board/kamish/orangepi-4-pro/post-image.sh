#!/usr/bin/env bash
set -e

BOARD_DIR="$(dirname "$0")"
BINARIES_DIR="${BINARIES_DIR:-$1}"

# Ensure Allwinner sun60iw2 bootloader blobs are copied to images dir for genimage
cp "${BOARD_DIR}/bootloader/boot0_sdcard.fex" "${BINARIES_DIR}/boot0_sdcard.fex"
cp "${BOARD_DIR}/bootloader/boot_package.fex" "${BINARIES_DIR}/boot_package.fex"

# Ensure boot.scr exists in BINARIES_DIR
if [ ! -f "${BINARIES_DIR}/boot.scr" ] && [ -x "${HOST_DIR}/bin/mkimage" ]; then
    echo "Generating boot.scr from ${BOARD_DIR}/boot.cmd..."
    "${HOST_DIR}/bin/mkimage" -C none -A arm -T script -d "${BOARD_DIR}/boot.cmd" "${BINARIES_DIR}/boot.scr"
fi

# Run Buildroot's default genimage script
exec support/scripts/genimage.sh "$@"
