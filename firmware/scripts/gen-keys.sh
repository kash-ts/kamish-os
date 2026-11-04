#!/usr/bin/env bash
# Generates development X.509 PKI keys and certificates for RAUC bundle signing

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KEYS_DIR="${SCRIPT_DIR}/../keys"
mkdir -p "${KEYS_DIR}"
KEYS_DIR="$(cd "${KEYS_DIR}" && pwd)"

CA_KEY="${KEYS_DIR}/ca.key.pem"
CA_CERT="${KEYS_DIR}/ca.cert.pem"
SIGN_KEY="${KEYS_DIR}/development-1.key.pem"
SIGN_CERT="${KEYS_DIR}/development-1.cert.pem"
KEYRING="${KEYS_DIR}/keyring.pem"

echo "=== Generating KAMISH OS RAUC PKI Certificates in ${KEYS_DIR} ==="

if [[ -f "${SIGN_KEY}" && -f "${SIGN_CERT}" && -f "${KEYRING}" ]]; then
    echo "Keys and certificates already exist. Keeping existing keys."
    exit 0
fi

# 1. Generate Development CA
openssl req -x509 -newkey rsa:4096 -nodes -keyout "${CA_KEY}" -out "${CA_CERT}" \
    -subj "/O=Kamish Smart Speaker/CN=Kamish Development Root CA" -days 3650

# 2. Generate Development Signing Key & CSR
openssl req -newkey rsa:2048 -nodes -keyout "${SIGN_KEY}" -out "${KEYS_DIR}/development-1.csr" \
    -subj "/O=Kamish Smart Speaker/CN=Kamish Release Signer"

# 3. Sign development certificate with CA
openssl x509 -req -in "${KEYS_DIR}/development-1.csr" -CA "${CA_CERT}" -CAkey "${CA_KEY}" \
    -CAcreateserial -out "${SIGN_CERT}" -days 1825

# 4. Create keyring with trusted CA to embed into RootFS
cp "${CA_CERT}" "${KEYRING}"

# Also copy keyring into Buildroot overlay
OVERLAY_RAUC_DIR="${SCRIPT_DIR}/../buildroot-external-kamish/board/kamish/orangepi-zero3/overlay/etc/rauc"
mkdir -p "${OVERLAY_RAUC_DIR}"
cp "${KEYRING}" "${OVERLAY_RAUC_DIR}/keyring.pem"

echo "=== RAUC Keys generated successfully! ==="
echo "Private signing key: ${SIGN_KEY}"
echo "Signing cert:        ${SIGN_CERT}"
echo "Public keyring:      ${KEYRING} (copied to overlay/etc/rauc/keyring.pem)"
