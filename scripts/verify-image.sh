#!/usr/bin/env bash
# Verifies KAMISH OS repacked rootfs, ext4 filesystems, SD card image, and RAUC bundle,
# and writes a structured Markdown report (e.g. for GITHUB_STEP_SUMMARY).

set -uo pipefail

OUT_DIR=""
STAGING_DIR=""
MODE="all"
SIGNING_MODE="unknown"
MAX_GLIBC="2.35"
SUMMARY_FILE="${GITHUB_STEP_SUMMARY:-}"

usage() {
    cat <<EOF
Usage: $0 --out-dir <dir> --staging-dir <dir> [options]

Options:
  --mode <all|raucb_only|img_only>  Build mode (default: all)
  --signing-mode <mode>             Signing mode label for report
  --max-glibc <ver>                 Maximum allowed GLIBC symbol version (default: 2.35)
  --summary-file <path>             Markdown summary output path (default: \$GITHUB_STEP_SUMMARY)
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --out-dir) OUT_DIR="$2"; shift 2 ;;
        --staging-dir) STAGING_DIR="$2"; shift 2 ;;
        --mode) MODE="$2"; shift 2 ;;
        --signing-mode) SIGNING_MODE="$2"; shift 2 ;;
        --max-glibc) MAX_GLIBC="$2"; shift 2 ;;
        --summary-file) SUMMARY_FILE="$2"; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "ERROR: Unknown argument: $1" >&2; usage; exit 1 ;;
    esac
done

if [[ -z "${OUT_DIR}" || -z "${STAGING_DIR}" ]]; then
    echo "ERROR: --out-dir and --staging-dir are required" >&2
    usage
    exit 1
fi
OUT_DIR="${OUT_DIR%/}"
STAGING_DIR="${STAGING_DIR%/}"

SUDO=""
if [[ "$(id -u)" -ne 0 ]]; then
    SUDO="sudo"
fi

FAIL_COUNT=0
PASS_COUNT=0
REPORT_ROWS=()

add_result() {
    local check_name="$1"
    local status="$2"
    local details="$3"
    REPORT_ROWS+=("| ${check_name} | ${status} | ${details} |")
    if [[ "${status}" == "FAIL" ]]; then
        echo "[FAIL] ${check_name}: ${details}" >&2
        FAIL_COUNT=$((FAIL_COUNT + 1))
    elif [[ "${status}" == "PASS" ]]; then
        echo "[PASS] ${check_name}: ${details}"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo "[${status}] ${check_name}: ${details}"
    fi
}

echo "=== KAMISH OS Image & RootFS Verification ==="

# ------------------------------------------------------------------------------
# 1. ELF Architecture & GLIBC Compatibility
# ------------------------------------------------------------------------------
GLIBC_INFO="n/a"
TARGET_LIBC="$(${SUDO} find "${STAGING_DIR}/lib" "${STAGING_DIR}/lib64" "${STAGING_DIR}/usr/lib" -name 'libc.so.6' 2>/dev/null | head -n1 || true)"
TARGET_GLIBC_MAX="unknown"
if [[ -n "${TARGET_LIBC}" ]]; then
    TARGET_GLIBC_MAX="$(${SUDO} readelf -V "${TARGET_LIBC}" 2>/dev/null | grep -oE 'GLIBC_[0-9]+(\.[0-9]+)+' | sed 's/GLIBC_//' | sort -V | tail -n1 || echo "unknown")"
fi

ELF_DETAILS=""
ELF_OK=1
for bin_name in kamish-core kamish-web; do
    bin_path="${STAGING_DIR}/opt/kamish/bin/${bin_name}"
    if [[ ! -f "${bin_path}" ]]; then
        ELF_OK=0
        ELF_DETAILS="${ELF_DETAILS}missing ${bin_name}; "
        continue
    fi
    file_out="$(file -b "${bin_path}")"
    if ! echo "${file_out}" | grep -q "ARM aarch64"; then
        ELF_OK=0
        ELF_DETAILS="${ELF_DETAILS}${bin_name} wrong arch (${file_out}); "
        continue
    fi
    max_req="$(readelf -V "${bin_path}" 2>/dev/null | grep -oE 'GLIBC_[0-9]+(\.[0-9]+)+' | sed 's/GLIBC_//' | sort -V | tail -n1 || true)"
    max_req="${max_req:-static}"
    GLIBC_INFO="${bin_name}: ${max_req}, ${GLIBC_INFO}"
    if [[ "${max_req}" != "static" ]]; then
        cmp_ok="$(python3 - "${max_req}" "${MAX_GLIBC}" "${TARGET_GLIBC_MAX}" <<'PYEOF'
import sys
def parse_ver(v):
    return tuple(int(x) for x in v.split("."))
req = parse_ver(sys.argv[1])
cap = parse_ver(sys.argv[2])
ok = req <= cap
if sys.argv[3] not in ("", "unknown"):
    ok = ok and (req <= parse_ver(sys.argv[3]))
print("1" if ok else "0")
PYEOF
)"
        if [[ "${cmp_ok}" != "1" ]]; then
            ELF_OK=0
            ELF_DETAILS="${ELF_DETAILS}${bin_name} requires GLIBC_${max_req} (limit ${MAX_GLIBC}, target libc ${TARGET_GLIBC_MAX}); "
        else
            ELF_DETAILS="${ELF_DETAILS}${bin_name}=aarch64/GLIBC_${max_req}; "
        fi
    else
        ELF_DETAILS="${ELF_DETAILS}${bin_name}=aarch64/static; "
    fi
done
ELF_DETAILS="${ELF_DETAILS}target_libc=${TARGET_GLIBC_MAX}"
if [[ "${ELF_OK}" -eq 1 ]]; then
    add_result "ELF & GLIBC Compatibility" "PASS" "${ELF_DETAILS}"
else
    add_result "ELF & GLIBC Compatibility" "FAIL" "${ELF_DETAILS}"
fi

# ------------------------------------------------------------------------------
# 2. Native ARM64 Dynamic Linker Smoke Check
# ------------------------------------------------------------------------------
HOST_ARCH="$(uname -m)"
if [[ "${HOST_ARCH}" == "aarch64" ]]; then
    LD_SO_PATH="$(${SUDO} find "${STAGING_DIR}/lib" "${STAGING_DIR}/lib64" "${STAGING_DIR}/usr/lib" -name 'ld-linux-aarch64.so.1' 2>/dev/null | head -n1 || true)"
    if [[ -z "${LD_SO_PATH}" ]]; then
        add_result "Native aarch64 Dynamic Linker" "FAIL" "ld-linux-aarch64.so.1 not found in rootfs"
    else
        LD_SO_REL="/${LD_SO_PATH#"${STAGING_DIR}/"}"
        LDD_OK=1
        LDD_MSG=""
        for bin_name in kamish-core kamish-web; do
            ld_out="$(${SUDO} chroot "${STAGING_DIR}" "${LD_SO_REL}" --list "/opt/kamish/bin/${bin_name}" 2>&1 || true)"
            if [[ -z "${ld_out}" ]] || echo "${ld_out}" | grep -qiE "not found|error|cannot|no such"; then
                LDD_OK=0
                LDD_MSG="${LDD_MSG}${bin_name} failed (${ld_out//$'\n'/; }); "
            else
                lib_cnt="$(echo "${ld_out}" | wc -l | tr -d ' ')"
                LDD_MSG="${LDD_MSG}${bin_name} (${lib_cnt} libs resolved); "
            fi
        done
        if [[ "${LDD_OK}" -eq 1 ]]; then
            add_result "Native aarch64 Dynamic Linker" "PASS" "${LDD_MSG}"
        else
            add_result "Native aarch64 Dynamic Linker" "FAIL" "${LDD_MSG}"
        fi
    fi
else
    add_result "Native aarch64 Dynamic Linker" "SKIP" "Host arch is ${HOST_ARCH} (runs natively on aarch64)"
fi

# ------------------------------------------------------------------------------
# 3. RootFS Structure, DTB, User, Fstab, RAUC & Systemd Units
# ------------------------------------------------------------------------------
ROOTFS_MISSING=()
${SUDO} test -s "${STAGING_DIR}/boot/Image" || ROOTFS_MISSING+=("/boot/Image")

DTB_FOUND=""
for dtb_rel in \
    "boot/sun60i-a733-orangepi-4-pro.dtb" \
    "boot/allwinner/sun60i-a733-orangepi-4-pro.dtb" \
    "boot/dtb/allwinner/sun60i-a733-orangepi-4-pro.dtb"; do
    if ${SUDO} test -s "${STAGING_DIR}/${dtb_rel}"; then
        DTB_FOUND="/${dtb_rel}"
        break
    fi
done
if [[ -z "${DTB_FOUND}" ]]; then
    ROOTFS_MISSING+=("sun60i-a733-orangepi-4-pro.dtb")
fi

${SUDO} test -x "${STAGING_DIR}/opt/kamish/bin/kamish-core" || ROOTFS_MISSING+=("/opt/kamish/bin/kamish-core")
${SUDO} test -x "${STAGING_DIR}/opt/kamish/bin/kamish-web" || ROOTFS_MISSING+=("/opt/kamish/bin/kamish-web")
${SUDO} test -s "${STAGING_DIR}/opt/kamish/dist/index.html" || ROOTFS_MISSING+=("/opt/kamish/dist/index.html")
${SUDO} test -s "${STAGING_DIR}/opt/kamish/database/schema.sql" || ROOTFS_MISSING+=("/opt/kamish/database/schema.sql")
${SUDO} test -s "${STAGING_DIR}/etc/kamish-release" || ROOTFS_MISSING+=("/etc/kamish-release")
${SUDO} test -d "${STAGING_DIR}/data" || ROOTFS_MISSING+=("/data mountpoint dir")
${SUDO} test -s "${STAGING_DIR}/etc/rauc/system.conf" || ROOTFS_MISSING+=("/etc/rauc/system.conf")
${SUDO} test -s "${STAGING_DIR}/etc/rauc/keyring.pem" || ROOTFS_MISSING+=("/etc/rauc/keyring.pem")
${SUDO} test -s "${STAGING_DIR}/etc/systemd/network/20-wired.network" || ROOTFS_MISSING+=("/etc/systemd/network/20-wired.network")
${SUDO} test -s "${STAGING_DIR}/lib/firmware/aic8800d80/fmacfw_8800d80_u02.bin" || ROOTFS_MISSING+=("/lib/firmware/aic8800d80 (Wi-Fi firmware)")

CRLF_BAD="$(${SUDO} python3 - "${STAGING_DIR}" <<'PYEOF'
import glob
import os
import sys

root = sys.argv[1]
patterns = [
    os.path.join(root, "usr/sbin/kamish-dropbear-mgr"),
    os.path.join(root, "usr/sbin/kamish-healthcheck"),
    os.path.join(root, "etc/systemd/system/kamish*.service"),
    os.path.join(root, "etc/fstab"),
    os.path.join(root, "etc/rauc/system.conf"),
    os.path.join(root, "etc/systemd/network/20-wired.network"),
]
bad = []
for pat in patterns:
    for path in sorted(glob.glob(pat)):
        if os.path.isfile(path) and not os.path.islink(path):
            with open(path, "rb") as f:
                if b"\r" in f.read():
                    bad.append("/" + os.path.relpath(path, root))
print(" ".join(bad))
PYEOF
)"
if [[ -n "${CRLF_BAD}" ]]; then
    ROOTFS_MISSING+=("CRLF line endings in: ${CRLF_BAD}")
fi

if ! ${SUDO} grep -qE '^(LABEL=data|/dev/mmcblk0p4)[[:space:]]+/data[[:space:]]+ext4[[:space:]].*x-systemd\.growfs' "${STAGING_DIR}/etc/fstab" 2>/dev/null; then
    ROOTFS_MISSING+=("/etc/fstab (/data + x-systemd.growfs)")
fi
if ! ${SUDO} grep -q '^kamish:' "${STAGING_DIR}/etc/passwd" 2>/dev/null; then
    ROOTFS_MISSING+=("user kamish in /etc/passwd")
fi
if ! ${SUDO} grep -q '^kamish:' "${STAGING_DIR}/etc/group" 2>/dev/null; then
    ROOTFS_MISSING+=("group kamish in /etc/group")
fi

SYSTEMD_LINKS=(
    "etc/systemd/system/sysinit.target.wants/kamish-mounts.service"
    "etc/systemd/system/multi-user.target.wants/kamish-core.service"
    "etc/systemd/system/multi-user.target.wants/kamish-web.service"
    "etc/systemd/system/multi-user.target.wants/kamish.service"
    "etc/systemd/system/multi-user.target.wants/kamish-healthcheck.service"
)
for link_rel in "${SYSTEMD_LINKS[@]}"; do
    link_abs="${STAGING_DIR}/${link_rel}"
    if ! ${SUDO} test -L "${link_abs}"; then
        ROOTFS_MISSING+=("symlink /${link_rel}")
    else
        target_rel="$(${SUDO} readlink "${link_abs}")"
        if [[ "${target_rel}" == /* ]]; then
            target_abs="${STAGING_DIR}${target_rel}"
        else
            target_abs="$(dirname "${link_abs}")/${target_rel}"
        fi
        if ! ${SUDO} test -f "${target_abs}"; then
            ROOTFS_MISSING+=("unit target for /${link_rel} -> ${target_rel}")
        fi
    fi
done

if [[ "${#ROOTFS_MISSING[@]}" -eq 0 ]]; then
    add_result "RootFS Layout & Services" "PASS" "Image + DTB (${DTB_FOUND}) + kamish user + fstab + RAUC + 5 systemd units"
else
    add_result "RootFS Layout & Services" "FAIL" "Missing/invalid: ${ROOTFS_MISSING[*]}"
fi

# ------------------------------------------------------------------------------
# 3b. Kernel BSP Config (IKCFG) Verification for Allwinner A733 (sun60iw2)
# ------------------------------------------------------------------------------
IKCFG_RES="$(${SUDO} python3 - "${STAGING_DIR}/boot/Image" <<'PYEOF'
import gzip
import sys

img_path = sys.argv[1]
required_y = [
    "CONFIG_AW_BSP",
    "CONFIG_ARCH_SUN60IW2",
    "CONFIG_AW_SUN60IW2_CCU",
    "CONFIG_AW_PINCTRL_SUN60IW2",
    "CONFIG_AW_UART_NG",
    "CONFIG_AW_EARLY_PRINTK",
    "CONFIG_AW_MMC",
    "CONFIG_USB_STORAGE",
    "CONFIG_USB_SUNXI_HCI",
    "CONFIG_USB_DWC3",
    "CONFIG_USB_XHCI_HCD",
    "CONFIG_AW_USB_POWER_CTRL",
    "CONFIG_AW_DRM",
    "CONFIG_AW_FB_CONSOLE",
    "CONFIG_FRAMEBUFFER_CONSOLE",
    "CONFIG_STMMAC_ETH",
    "CONFIG_DWMAC_SUNXI",
    "CONFIG_SQUASHFS",
    "CONFIG_DM_VERITY",
]
required_ym = [
    "CONFIG_AW_STMMAC_ETH",
    "CONFIG_AW_DWMAC_SUNXI",
    "CONFIG_MOTORCOMM_PHY",
    "CONFIG_AIC8800_WLAN_SUPPORT",
]
try:
    with open(img_path, "rb") as f:
        data = f.read()
    st = data.find(b"IKCFG_ST")
    if st == -1:
        print("FAIL:IKCFG_ST marker missing in /boot/Image")
        sys.exit(0)
    ed = data.find(b"IKCFG_ED", st)
    if ed == -1:
        print("FAIL:IKCFG_ED marker missing in /boot/Image")
        sys.exit(0)
    cfg_lines = set(gzip.decompress(data[st + 8 : ed]).decode("utf-8", errors="replace").splitlines())
    missing = [opt for opt in required_y if f"{opt}=y" not in cfg_lines]
    missing.extend(
        opt for opt in required_ym if f"{opt}=y" not in cfg_lines and f"{opt}=m" not in cfg_lines
    )
    if missing:
        print("FAIL:Missing options: " + ", ".join(missing))
    else:
        total = len(required_y) + len(required_ym)
        print(f"PASS:All {total} sun60iw2 BSP + Ethernet/WiFi + USB + DRM + RAUC options verified in IKCFG")
except Exception as exc:
    print(f"FAIL:Exception reading IKCFG: {exc}")
PYEOF
)"
if [[ "${IKCFG_RES}" == PASS:* ]]; then
    add_result "Kernel BSP Config (IKCFG)" "PASS" "${IKCFG_RES#PASS:}"
else
    add_result "Kernel BSP Config (IKCFG)" "FAIL" "${IKCFG_RES#FAIL:}"
fi

# ------------------------------------------------------------------------------
# 4. ext4 Filesystems Integrity & U-Boot Compatibility
# ------------------------------------------------------------------------------
FS_OK=1
FS_MSG=""
for fs_name in rootfs.ext4 data.ext4; do
    fs_path="${OUT_DIR}/${fs_name}"
    if [[ ! -s "${fs_path}" ]]; then
        FS_OK=0
        FS_MSG="${FS_MSG}${fs_name} missing; "
        continue
    fi
    if e2fsck -fn "${fs_path}" >/dev/null 2>&1; then
        FS_MSG="${FS_MSG}${fs_name} fsck=OK; "
    else
        FS_OK=0
        FS_MSG="${FS_MSG}${fs_name} fsck=FAILED; "
    fi
    if [[ "${fs_name}" == "rootfs.ext4" ]] && command -v dumpe2fs >/dev/null 2>&1; then
        feat_line="$(dumpe2fs -h "${fs_path}" 2>/dev/null | grep '^Filesystem features:' || true)"
        if echo "${feat_line}" | grep -qE '\b(orphan_file|metadata_csum_seed)\b'; then
            FS_OK=0
            FS_MSG="${FS_MSG}incompatible U-Boot ext4 feature in rootfs.ext4 (${feat_line}); "
        fi
    fi
done
if [[ "${FS_OK}" -eq 1 ]]; then
    add_result "ext4 Filesystems (e2fsck)" "PASS" "${FS_MSG}"
else
    add_result "ext4 Filesystems (e2fsck)" "FAIL" "${FS_MSG}"
fi

# ------------------------------------------------------------------------------
# 5. SD Card Disk Image Verification (MBR, Allwinner Boot0, sfdisk, FAT boot.scr)
# ------------------------------------------------------------------------------
IMG_FILE="${OUT_DIR}/kamish-sdcard.img"
PART_TABLE_TEXT=""
if [[ "${MODE}" != "raucb_only" ]]; then
    if [[ ! -s "${IMG_FILE}" ]]; then
        add_result "SD Image Boot & Partitions" "FAIL" "kamish-sdcard.img not found"
    else
        IMG_ISSUES=()
        RAW_CHECK="$(python3 - "${IMG_FILE}" <<'PYEOF'
import sys
path = sys.argv[1]
with open(path, "rb") as f:
    f.seek(440)
    disk_sig = f.read(4)
    f.seek(510)
    mbr_sig = f.read(2)
    f.seek(8192 + 4)
    egon_magic = f.read(8)
    f.seek(16400 * 1024)
    boot_pkg = f.read(64)
print(f"disk_sig={disk_sig.hex()}")
print(f"mbr={mbr_sig.hex()}")
print(f"egon={egon_magic.decode('ascii', errors='replace')}")
print(f"bootpkg_nonzero={1 if any(b != 0 for b in boot_pkg) else 0}")
PYEOF
)"
        DISK_SIG_HEX="$(echo "${RAW_CHECK}" | awk -F= '/^disk_sig=/{print $2}')"
        MBR_HEX="$(echo "${RAW_CHECK}" | awk -F= '/^mbr=/{print $2}')"
        EGON_STR="$(echo "${RAW_CHECK}" | awk -F= '/^egon=/{print $2}')"
        BOOTPKG_NZ="$(echo "${RAW_CHECK}" | awk -F= '/^bootpkg_nonzero=/{print $2}')"

        [[ -n "${DISK_SIG_HEX}" && "${DISK_SIG_HEX}" != "00000000" ]] || IMG_ISSUES+=("zero MBR disk-signature")
        [[ "${MBR_HEX}" == "55aa" ]] || IMG_ISSUES+=("MBR sig=${MBR_HEX}")
        [[ "${EGON_STR}" == "eGON.BT0" ]] || IMG_ISSUES+=("boot0 magic=${EGON_STR}")
        [[ "${BOOTPKG_NZ}" == "1" ]] || IMG_ISSUES+=("empty boot_package.fex at 16400K")

        PART_TABLE_TEXT="$(sfdisk -d "${IMG_FILE}" 2>/dev/null || true)"
        PART_COUNT="$(echo "${PART_TABLE_TEXT}" | grep -cE '([1-4])[[:space:]]*:[[:space:]]*start=' || true)"
        PART1_LINE="$(echo "${PART_TABLE_TEXT}" | grep -E '1[[:space:]]*:[[:space:]]*start=' | head -n1 || true)"
        echo "${PART1_LINE}" | grep -qiE 'type=(0*c|W95 FAT32)' || IMG_ISSUES+=("part1 type=c missing")
        echo "${PART1_LINE}" | grep -q 'bootable' || IMG_ISSUES+=("part1 bootable flag missing")
        echo "${PART_TABLE_TEXT}" | grep -qE '2[[:space:]]*:[[:space:]]*start=.*type=83' || IMG_ISSUES+=("part2 type=83 missing")
        echo "${PART_TABLE_TEXT}" | grep -qE '3[[:space:]]*:[[:space:]]*start=.*type=83' || IMG_ISSUES+=("part3 type=83 missing")
        echo "${PART_TABLE_TEXT}" | grep -qE '4[[:space:]]*:[[:space:]]*start=.*type=83' || IMG_ISSUES+=("part4 type=83 missing")

        MDIR_OUT="$(MTOOLS_SKIP_CHECK=1 mdir -i "${IMG_FILE}@@33554432" :: 2>/dev/null || true)"
        if ! echo "${MDIR_OUT}" | grep -qiE 'boot([[:space:]]+|\.)scr'; then
            IMG_ISSUES+=("boot.scr missing in FAT boot partition")
        fi

        if [[ "${#IMG_ISSUES[@]}" -eq 0 ]]; then
            IMG_SIZE_MB="$(( $(stat -c%s "${IMG_FILE}") / 1024 / 1024 ))"
            add_result "SD Image Boot & Partitions" "PASS" "MBR=55aa, boot0=eGON.BT0, 4 partitions (${PART_COUNT}), boot.scr in FAT, size=${IMG_SIZE_MB}MiB"
        else
            add_result "SD Image Boot & Partitions" "FAIL" "${IMG_ISSUES[*]}"
        fi
    fi
else
    add_result "SD Image Boot & Partitions" "SKIP" "Skipped in mode=${MODE}"
fi

# ------------------------------------------------------------------------------
# 6. RAUC OTA Bundle Signature & Manifest Verification
# ------------------------------------------------------------------------------
RAUC_INFO_TEXT=""
if [[ "${MODE}" != "img_only" ]]; then
    RAUCB_FILE="$(find "${OUT_DIR}" -maxdepth 1 -name '*.raucb' | head -n1 || true)"
    if [[ -z "${RAUCB_FILE}" || ! -s "${RAUCB_FILE}" ]]; then
        add_result "RAUC OTA Bundle" "FAIL" "No .raucb file found in ${OUT_DIR}"
    else
        RAUC_INFO_TEXT="$(rauc info --keyring="${STAGING_DIR}/etc/rauc/keyring.pem" "${RAUCB_FILE}" 2>&1 || true)"
        if echo "${RAUC_INFO_TEXT}" | grep -q "Compatible:.*kamish-orangepi-4-pro" && echo "${RAUC_INFO_TEXT}" | grep -q "rootfs\.ext4"; then
            BUNDLE_SIZE_MB="$(( $(stat -c%s "${RAUCB_FILE}") / 1024 / 1024 ))"
            add_result "RAUC OTA Bundle" "PASS" "$(basename "${RAUCB_FILE}") (${BUNDLE_SIZE_MB}MiB), sig=OK, compatible=kamish-orangepi-4-pro"
        else
            add_result "RAUC OTA Bundle" "FAIL" "rauc info validation failed: ${RAUC_INFO_TEXT//$'\n'/; }"
        fi
    fi
else
    add_result "RAUC OTA Bundle" "SKIP" "Skipped in mode=${MODE}"
fi

# ------------------------------------------------------------------------------
# 7. Write Markdown Summary Report
# ------------------------------------------------------------------------------
if [[ -n "${SUMMARY_FILE}" ]]; then
    {
        echo "## Kamish OS Image Verification Report"
        echo ""
        if [[ "${SIGNING_MODE}" == "ephemeral-dev" ]]; then
            echo "> [!WARNING]"
            echo "> RAUC bundle was signed with an **ephemeral development key** generated for this run. Configure \`RAUC_SIGN_KEY_PEM\`, \`RAUC_SIGN_CERT_PEM\`, and \`RAUC_KEYRING_PEM\` in GitHub Repository Secrets for persistent production OTA trust."
            echo ""
        fi
        echo "**Summary:** ${PASS_COUNT} passed, ${FAIL_COUNT} failed (mode: \`${MODE}\`, signing: \`${SIGNING_MODE}\`)"
        echo ""
        echo "| Check | Status | Details |"
        echo "|---|---|---|"
        for row in "${REPORT_ROWS[@]}"; do
            echo "${row}"
        done
        echo ""
        echo "### Artifacts in Output Directory"
        echo '```text'
        ls -lh "${OUT_DIR}" || true
        echo '```'
        if [[ -n "${PART_TABLE_TEXT}" ]]; then
            echo ""
            echo "### Partition Table (\`sfdisk -d\`)"
            echo '```text'
            echo "${PART_TABLE_TEXT}"
            echo '```'
        fi
        if [[ -n "${RAUC_INFO_TEXT}" ]]; then
            echo ""
            echo "### RAUC Bundle Info (\`rauc info\`)"
            echo '```text'
            echo "${RAUC_INFO_TEXT}"
            echo '```'
        fi
    } >> "${SUMMARY_FILE}"
fi

if [[ "${FAIL_COUNT}" -gt 0 ]]; then
    echo "=== Verification FAILED (${FAIL_COUNT} failed check(s)) ===" >&2
    exit 1
fi

echo "=== Verification PASSED (${PASS_COUNT} check(s)) ==="
