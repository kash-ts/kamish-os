#!/usr/bin/env bash
# Fast repack of KAMISH OS disk image (.img) and RAUC OTA bundle (.raucb)
# from a prebuilt Base OS rootfs tarball + fresh application binaries/SPA/overlay.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BOARD_DIR="${REPO_ROOT}/firmware/buildroot-external-kamish/board/kamish/orangepi-4-pro"

BASE_OS_DIR=""
PAYLOAD_DIR=""
OUT_DIR=""
VERSION=""
BUILD_ID=""
MODE="all"
RAUC_CERT=""
RAUC_KEY=""
RAUC_KEYRING=""
SIGNING_MODE="ephemeral-dev"
BASE_RUN_ID=""
STAGING_DIR=""

usage() {
    cat <<EOF
Usage: $0 --base-os-dir <dir> --payload-dir <dir> --out-dir <dir> --version <ver> --build-id <id> [options]

Options:
  --mode <all|raucb_only|img_only>            Output artifacts mode (default: all)
  --rauc-cert <path>                          Signing certificate for RAUC bundle
  --rauc-key <path>                           Signing private key for RAUC bundle
  --rauc-keyring <path>                       Public keyring.pem to embed into /etc/rauc/keyring.pem
  --signing-mode <production-secret|ephemeral-dev>  Recorded in /etc/kamish-release
  --base-run-id <id>                          Base OS workflow run ID (optional)
  --staging-dir <dir>                         Keep unpacked rootfs here for downstream verification
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --base-os-dir) BASE_OS_DIR="$2"; shift 2 ;;
        --payload-dir) PAYLOAD_DIR="$2"; shift 2 ;;
        --out-dir) OUT_DIR="$2"; shift 2 ;;
        --version) VERSION="$2"; shift 2 ;;
        --build-id) BUILD_ID="$2"; shift 2 ;;
        --mode) MODE="$2"; shift 2 ;;
        --rauc-cert) RAUC_CERT="$2"; shift 2 ;;
        --rauc-key) RAUC_KEY="$2"; shift 2 ;;
        --rauc-keyring) RAUC_KEYRING="$2"; shift 2 ;;
        --signing-mode) SIGNING_MODE="$2"; shift 2 ;;
        --base-run-id) BASE_RUN_ID="$2"; shift 2 ;;
        --staging-dir) STAGING_DIR="$2"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "ERROR: Unknown argument: $1" >&2; usage; exit 1 ;;
    esac
done

for req in BASE_OS_DIR PAYLOAD_DIR OUT_DIR VERSION BUILD_ID; do
    if [[ -z "${!req}" ]]; then
        echo "ERROR: Missing required option for ${req}" >&2
        usage
        exit 1
    fi
done

case "${MODE}" in
    all|raucb_only|img_only) ;;
    *) echo "ERROR: Invalid --mode '${MODE}' (expected all|raucb_only|img_only)" >&2; exit 1 ;;
esac

if [[ "${MODE}" != "img_only" ]]; then
    if [[ -z "${RAUC_CERT}" || ! -f "${RAUC_CERT}" ]]; then
        echo "ERROR: RAUC signing certificate not found: ${RAUC_CERT:-<unset>}" >&2
        exit 1
    fi
    if [[ -z "${RAUC_KEY}" || ! -f "${RAUC_KEY}" ]]; then
        echo "ERROR: RAUC signing private key not found: ${RAUC_KEY:-<unset>}" >&2
        exit 1
    fi
fi

# Validate payload inputs
for f in "${PAYLOAD_DIR}/kamish-core" "${PAYLOAD_DIR}/kamish-web" "${PAYLOAD_DIR}/dist/index.html" "${PAYLOAD_DIR}/schema.sql"; do
    if [[ ! -f "${f}" ]]; then
        echo "ERROR: Missing payload file: ${f}" >&2
        exit 1
    fi
done

# Locate Base OS rootfs tarball
BASE_TAR=""
if [[ -f "${BASE_OS_DIR}/rootfs-base.tar.zst" ]]; then
    BASE_TAR="${BASE_OS_DIR}/rootfs-base.tar.zst"
elif [[ -f "${BASE_OS_DIR}/rootfs.tar.gz" ]]; then
    BASE_TAR="${BASE_OS_DIR}/rootfs.tar.gz"
else
    echo "ERROR: No rootfs-base.tar.zst or rootfs.tar.gz found in ${BASE_OS_DIR}" >&2
    exit 1
fi

if [[ -z "${BASE_RUN_ID}" && -f "${BASE_OS_DIR}/base-os-manifest.json" ]] && command -v python3 >/dev/null 2>&1; then
    BASE_RUN_ID="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("run_id","unknown"))' "${BASE_OS_DIR}/base-os-manifest.json" 2>/dev/null || echo "unknown")"
fi
BASE_RUN_ID="${BASE_RUN_ID:-unknown}"

SUDO=""
if [[ "$(id -u)" -ne 0 ]]; then
    SUDO="sudo"
fi

WORK_TMP="$(mktemp -d /tmp/kamish-repack-XXXXXX)"
CLEANUP_STAGING=0
if [[ -n "${STAGING_DIR}" ]]; then
    mkdir -p "${STAGING_DIR}"
    ${SUDO} find "${STAGING_DIR}" -mindepth 1 -maxdepth 1 -exec rm -rf {} +
    STAGING_ROOTFS="${STAGING_DIR}"
else
    STAGING_ROOTFS="$(mktemp -d /tmp/kamish-staging-XXXXXX)"
    CLEANUP_STAGING=1
fi

cleanup() {
    rm -rf "${WORK_TMP}" || true
    if [[ "${CLEANUP_STAGING}" -eq 1 ]]; then
        ${SUDO} rm -rf "${STAGING_ROOTFS}" || true
    fi
}
trap cleanup EXIT

mkdir -p "${OUT_DIR}"

echo "=== [1/6] Unpacking Base OS rootfs: ${BASE_TAR} ==="
if [[ "${BASE_TAR}" == *.zst ]]; then
    zstd -dc "${BASE_TAR}" | ${SUDO} tar --numeric-owner -xpf - -C "${STAGING_ROOTFS}"
else
    ${SUDO} tar --numeric-owner -xzpf "${BASE_TAR}" -C "${STAGING_ROOTFS}"
fi

echo "=== [2/6] Applying fresh board overlay ==="
${SUDO} cp -a "${BOARD_DIR}/overlay/." "${STAGING_ROOTFS}/"
for sys_dir in \
    "${STAGING_ROOTFS}/etc" \
    "${STAGING_ROOTFS}/etc/rauc" \
    "${STAGING_ROOTFS}/etc/systemd" \
    "${STAGING_ROOTFS}/etc/systemd/system" \
    "${STAGING_ROOTFS}/etc/systemd/network" \
    "${STAGING_ROOTFS}/etc/udev" \
    "${STAGING_ROOTFS}/etc/udev/rules.d"; do
    ${SUDO} mkdir -p "${sys_dir}"
    ${SUDO} chown root:root "${sys_dir}"
    ${SUDO} chmod 0755 "${sys_dir}"
done
if ${SUDO} test -d "${STAGING_ROOTFS}/boot"; then
    ${SUDO} chown root:root "${STAGING_ROOTFS}/boot"
    ${SUDO} chmod 0755 "${STAGING_ROOTFS}/boot"
fi
${SUDO} chown root:root \
    "${STAGING_ROOTFS}/etc/fstab" \
    "${STAGING_ROOTFS}/etc/rauc/system.conf" \
    "${STAGING_ROOTFS}/etc/rauc/keyring.pem" \
    "${STAGING_ROOTFS}/etc/systemd/network/20-wired.network" \
    "${STAGING_ROOTFS}/etc/systemd/system/"kamish*.service \
    "${STAGING_ROOTFS}/etc/udev/rules.d/99-respeaker.rules" \
    "${STAGING_ROOTFS}/usr/sbin/kamish-dropbear-mgr" \
    "${STAGING_ROOTFS}/usr/sbin/kamish-healthcheck"
${SUDO} chmod 0755 \
    "${STAGING_ROOTFS}/usr/sbin/kamish-dropbear-mgr" \
    "${STAGING_ROOTFS}/usr/sbin/kamish-healthcheck"
${SUDO} chmod 0644 \
    "${STAGING_ROOTFS}/etc/fstab" \
    "${STAGING_ROOTFS}/etc/rauc/system.conf" \
    "${STAGING_ROOTFS}/etc/rauc/keyring.pem" \
    "${STAGING_ROOTFS}/etc/systemd/network/20-wired.network" \
    "${STAGING_ROOTFS}/etc/udev/rules.d/99-respeaker.rules" \
    "${STAGING_ROOTFS}/etc/systemd/system/"kamish*.service

if [[ -n "${RAUC_KEYRING}" ]]; then
    if [[ ! -f "${RAUC_KEYRING}" ]]; then
        echo "ERROR: RAUC keyring not found: ${RAUC_KEYRING}" >&2
        exit 1
    fi
    ${SUDO} mkdir -p "${STAGING_ROOTFS}/etc/rauc"
    ${SUDO} install -m 0644 -o root -g root "${RAUC_KEYRING}" "${STAGING_ROOTFS}/etc/rauc/keyring.pem"
fi

echo "=== [3/6] Ensuring kamish service user and groups ==="
KAMISH_IDS="$(${SUDO} python3 - "${STAGING_ROOTFS}" <<'PYEOF'
import os
import sys

rootfs = sys.argv[1]
etc_dir = os.path.join(rootfs, "etc")
passwd_path = os.path.join(etc_dir, "passwd")
group_path = os.path.join(etc_dir, "group")
shadow_path = os.path.join(etc_dir, "shadow")

def read_lines(path):
    if not os.path.exists(path):
        return []
    with open(path, "r", encoding="utf-8") as f:
        return [line.rstrip("\n") for line in f if line.strip()]

def write_lines(path, lines, mode=0o644):
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        for line in lines:
            f.write(line + "\n")
    os.chmod(path, mode)

groups = read_lines(group_path)
passwds = read_lines(passwd_path)

used_gids = set()
group_map = {}
for idx, line in enumerate(groups):
    parts = line.split(":")
    if len(parts) >= 4:
        name = parts[0]
        try:
            gid = int(parts[2])
            used_gids.add(gid)
        except ValueError:
            pass
        group_map[name] = idx

def next_free_id(used, start=1000):
    candidate = start
    while candidate in used:
        candidate += 1
    used.add(candidate)
    return candidate

if "kamish" not in group_map:
    gid = next_free_id(used_gids, 1000)
    groups.append(f"kamish:x:{gid}:")
    group_map["kamish"] = len(groups) - 1
else:
    gid = int(groups[group_map["kamish"]].split(":")[2])

for sup_group in ("audio", "dialout", "netdev", "bluetooth", "video"):
    if sup_group not in group_map:
        sup_gid = next_free_id(used_gids, 900)
        groups.append(f"{sup_group}:x:{sup_gid}:kamish")
        group_map[sup_group] = len(groups) - 1
    else:
        idx = group_map[sup_group]
        parts = groups[idx].split(":")
        while len(parts) < 4:
            parts.append("")
        members = [m for m in parts[3].split(",") if m]
        if "kamish" not in members:
            members.append("kamish")
        parts[3] = ",".join(members)
        groups[idx] = ":".join(parts)

used_uids = set()
kamish_uid = None
kamish_passwd_idx = None
for idx, line in enumerate(passwds):
    parts = line.split(":")
    if len(parts) >= 4:
        try:
            uid_val = int(parts[2])
            used_uids.add(uid_val)
        except ValueError:
            pass
        if parts[0] == "kamish":
            kamish_uid = int(parts[2])
            kamish_passwd_idx = idx

if kamish_uid is None:
    kamish_uid = next_free_id(used_uids, 1000)
    passwds.append(f"kamish:x:{kamish_uid}:{gid}:Kamish OS Service User:/home/kamish:/bin/sh")
else:
    parts = passwds[kamish_passwd_idx].split(":")
    parts[3] = str(gid)
    passwds[kamish_passwd_idx] = ":".join(parts)

write_lines(group_path, groups, 0o644)
write_lines(passwd_path, passwds, 0o644)

if os.path.exists(shadow_path):
    shadows = read_lines(shadow_path)
    if not any(line.split(":")[0] == "kamish" for line in shadows):
        shadows.append("kamish:!:19000:0:99999:7:::")
        write_lines(shadow_path, shadows, 0o640)

print(f"{kamish_uid}:{gid}")
PYEOF
)"
KAMISH_UID="${KAMISH_IDS%%:*}"
KAMISH_GID="${KAMISH_IDS##*:}"
echo "Resolved user kamish -> ${KAMISH_UID}:${KAMISH_GID}"

echo "=== [4/6] Injecting application binaries, SPA dist, schema, and systemd links ==="
${SUDO} mkdir -p \
    "${STAGING_ROOTFS}/opt/kamish/bin" \
    "${STAGING_ROOTFS}/opt/kamish/dist" \
    "${STAGING_ROOTFS}/opt/kamish/database" \
    "${STAGING_ROOTFS}/data" \
    "${STAGING_ROOTFS}/var/lib/kamish" \
    "${STAGING_ROOTFS}/var/log/kamish" \
    "${STAGING_ROOTFS}/etc/kamish" \
    "${STAGING_ROOTFS}/home/kamish"

${SUDO} install -D -m 0755 -o root -g root "${PAYLOAD_DIR}/kamish-core" "${STAGING_ROOTFS}/opt/kamish/bin/kamish-core"
${SUDO} install -D -m 0755 -o root -g root "${PAYLOAD_DIR}/kamish-web" "${STAGING_ROOTFS}/opt/kamish/bin/kamish-web"

${SUDO} find "${STAGING_ROOTFS}/opt/kamish/dist" -mindepth 1 -maxdepth 1 -exec rm -rf {} +
${SUDO} cp -a "${PAYLOAD_DIR}/dist/." "${STAGING_ROOTFS}/opt/kamish/dist/"
${SUDO} chown -R root:root "${STAGING_ROOTFS}/opt/kamish/dist"
${SUDO} chmod -R a+rX "${STAGING_ROOTFS}/opt/kamish/dist"

${SUDO} install -D -m 0644 -o root -g root "${PAYLOAD_DIR}/schema.sql" "${STAGING_ROOTFS}/opt/kamish/database/schema.sql"

${SUDO} chown "${KAMISH_UID}:${KAMISH_GID}" \
    "${STAGING_ROOTFS}/var/lib/kamish" \
    "${STAGING_ROOTFS}/var/log/kamish" \
    "${STAGING_ROOTFS}/home/kamish"

${SUDO} mkdir -p \
    "${STAGING_ROOTFS}/etc/systemd/system/sysinit.target.wants" \
    "${STAGING_ROOTFS}/etc/systemd/system/multi-user.target.wants"

${SUDO} ln -sf /etc/systemd/system/kamish-mounts.service \
    "${STAGING_ROOTFS}/etc/systemd/system/sysinit.target.wants/kamish-mounts.service"

for unit in kamish-core.service kamish-web.service kamish.service kamish-healthcheck.service; do
    ${SUDO} ln -sf "/etc/systemd/system/${unit}" \
        "${STAGING_ROOTFS}/etc/systemd/system/multi-user.target.wants/${unit}"
done

GIT_SHA="$(git -C "${REPO_ROOT}" rev-parse --short HEAD 2>/dev/null || echo "${GITHUB_SHA:-unknown}")"
BUILD_DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

${SUDO} tee "${STAGING_ROOTFS}/etc/kamish-release" >/dev/null <<EOF
VERSION=${VERSION}
BUILD_ID=${BUILD_ID}
GIT_SHA=${GIT_SHA}
BUILD_DATE=${BUILD_DATE}
BASE_OS_RUN_ID=${BASE_RUN_ID}
RAUC_SIGNING_MODE=${SIGNING_MODE}
EOF
${SUDO} chmod 0644 "${STAGING_ROOTFS}/etc/kamish-release"

echo "=== [5/6] Building ext4 filesystems (rootfs.ext4 & data.ext4) ==="
EXT4_DISABLE_OPTS="^metadata_csum"
if grep -q "metadata_csum_seed" /etc/mke2fs.conf 2>/dev/null; then
    EXT4_DISABLE_OPTS="${EXT4_DISABLE_OPTS},^metadata_csum_seed"
fi
if grep -q "orphan_file" /etc/mke2fs.conf 2>/dev/null; then
    EXT4_DISABLE_OPTS="${EXT4_DISABLE_OPTS},^orphan_file"
fi
echo "Using mke2fs ext4 options: -O ${EXT4_DISABLE_OPTS}"

rm -f "${OUT_DIR}/rootfs.ext4" "${OUT_DIR}/data.ext4"
${SUDO} mke2fs -q -t ext4 -d "${STAGING_ROOTFS}" -L system_a -m 1 -O "${EXT4_DISABLE_OPTS}" -F "${OUT_DIR}/rootfs.ext4" 1024M
${SUDO} chown "$(id -u):$(id -g)" "${OUT_DIR}/rootfs.ext4"

mke2fs -q -t ext4 -L data -O "${EXT4_DISABLE_OPTS}" -F "${OUT_DIR}/data.ext4" 64M

echo "=== [6/6] Packaging outputs (mode=${MODE}) ==="
if [[ "${MODE}" != "raucb_only" ]]; then
    GEN_IN="${WORK_TMP}/genimage-in"
    GEN_TMP="${WORK_TMP}/genimage-tmp"
    GEN_ROOT="${WORK_TMP}/genimage-root"
    mkdir -p "${GEN_IN}" "${GEN_TMP}" "${GEN_ROOT}"

    mkimage -C none -A arm -T script -d "${BOARD_DIR}/boot.cmd" "${GEN_IN}/boot.scr"

    for blob in boot0_sdcard.fex boot_package.fex; do
        if [[ -f "${BASE_OS_DIR}/${blob}" ]]; then
            cp "${BASE_OS_DIR}/${blob}" "${GEN_IN}/${blob}"
        elif [[ -f "${BOARD_DIR}/bootloader/${blob}" ]]; then
            cp "${BOARD_DIR}/bootloader/${blob}" "${GEN_IN}/${blob}"
        else
            echo "ERROR: Missing bootloader blob ${blob}" >&2
            exit 1
        fi
    done

    ln "${OUT_DIR}/rootfs.ext4" "${GEN_IN}/rootfs.ext4" 2>/dev/null || cp "${OUT_DIR}/rootfs.ext4" "${GEN_IN}/rootfs.ext4"
    ln "${OUT_DIR}/data.ext4" "${GEN_IN}/data.ext4" 2>/dev/null || cp "${OUT_DIR}/data.ext4" "${GEN_IN}/data.ext4"

    rm -f "${OUT_DIR}/kamish-sdcard.img"
    genimage \
        --rootpath "${GEN_ROOT}" \
        --tmppath "${GEN_TMP}" \
        --inputpath "${GEN_IN}" \
        --outputpath "${OUT_DIR}" \
        --config "${BOARD_DIR}/genimage.cfg"

    test -s "${OUT_DIR}/kamish-sdcard.img"
    echo "Created disk image: ${OUT_DIR}/kamish-sdcard.img"
fi

if [[ "${MODE}" != "img_only" ]]; then
    RAUC_BUNDLE_PATH="${OUT_DIR}/kamish-os-v${VERSION}-${BUILD_ID}.raucb"
    RAUC_CERT="${RAUC_CERT}" \
    RAUC_KEY="${RAUC_KEY}" \
    RAUC_OUTPUT="${RAUC_BUNDLE_PATH}" \
        bash "${REPO_ROOT}/firmware/scripts/build-bundle.sh" "${VERSION}" "${BUILD_ID}" "${OUT_DIR}/rootfs.ext4"
    test -s "${RAUC_BUNDLE_PATH}"
fi

echo "=== Repack completed successfully ==="
ls -lh "${OUT_DIR}"
