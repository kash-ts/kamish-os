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

# Create initial formatted ext4 data partition image (64M, auto-expanded by x-systemd.growfs on first boot)
MKE2FS_BIN=""
for candidate in "${HOST_DIR:-}/sbin/mke2fs" "/sbin/mke2fs" "$(command -v mke2fs 2>/dev/null || true)"; do
    if [ -n "${candidate}" ] && [ -x "${candidate}" ]; then
        MKE2FS_BIN="${candidate}"
        break
    fi
done
if [ -z "${MKE2FS_BIN}" ]; then
    echo "ERROR: mke2fs not found; cannot create data.ext4" >&2
    exit 1
fi
rm -f "${BINARIES_DIR}/data.ext4"
"${MKE2FS_BIN}" -q -t ext4 -L data -O ^metadata_csum_seed -F "${BINARIES_DIR}/data.ext4" 64M

# Run Buildroot's default genimage script
exec support/scripts/genimage.sh "$@"
