#!/usr/bin/env bash
# ==============================================================================
# Antigravity CLI Installer / Updater for Smart Speaker (Linux ARM64 / x86_64)
# ==============================================================================
set -euo pipefail

REPO="google-antigravity/antigravity-cli"
INSTALL_DIR="/usr/local/bin"
TARGET_NAME="agy"

ARCH="$(uname -m)"
case "${ARCH}" in
    x86_64)
        ASSET_PATTERN="agy_cli_linux_x64.tar.gz"
        ;;
    aarch64|arm64)
        ASSET_PATTERN="agy_cli_linux_arm64.tar.gz"
        ;;
    *)
        echo "[ERROR] Unsupported architecture: ${ARCH}" >&2
        exit 1
        ;;
esac

echo "[INFO] Detecting latest Antigravity CLI release for ${ARCH}..."
API_URL="https://api.github.com/repos/${REPO}/releases/latest"
RELEASE_JSON="$(curl -sSL -H "User-Agent: smart-speaker-installer" "${API_URL}")"

DOWNLOAD_URL="$(echo "${RELEASE_JSON}" | grep "browser_download_url" | grep "${ASSET_PATTERN}" | cut -d '"' -f 4 | head -n 1)"
TAG_NAME="$(echo "${RELEASE_JSON}" | grep "tag_name" | cut -d '"' -f 4 | head -n 1)"

if [[ -z "${DOWNLOAD_URL}" ]]; then
    echo "[ERROR] Could not find download URL for asset ${ASSET_PATTERN} in release ${TAG_NAME}" >&2
    exit 1
fi

echo "[INFO] Downloading ${TAG_NAME} (${ASSET_PATTERN})..."
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

curl -sSL -o "${TMP_DIR}/archive.tar.gz" "${DOWNLOAD_URL}"

echo "[INFO] Extracting binary..."
tar -xzf "${TMP_DIR}/archive.tar.gz" -C "${TMP_DIR}"

BIN_PATH="$(find "${TMP_DIR}" -type f -name "agy*" | head -n 1)"
if [[ -z "${BIN_PATH}" || ! -f "${BIN_PATH}" ]]; then
    echo "[ERROR] Failed to locate agy binary inside archive" >&2
    exit 1
fi

chmod +x "${BIN_PATH}"

echo "[INFO] Installing to ${INSTALL_DIR}/${TARGET_NAME}..."
mkdir -p "${INSTALL_DIR}"
cp "${BIN_PATH}" "${INSTALL_DIR}/${TARGET_NAME}"

echo "[OK] Antigravity CLI successfully installed!"
"${INSTALL_DIR}/${TARGET_NAME}" --version || true
