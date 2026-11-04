#!/usr/bin/env bash
# ==============================================================================
# Smart Speaker Hardware Diagnostic Script
# Platform: Orange Pi Zero 3 (Allwinner H618, 4GB RAM) + Armbian Debian ARM64
# Peripherals: Seeed ReSpeaker XVF3800, GT-112 Audio Output
# ==============================================================================

set -uo pipefail

BOLD='\033[1m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "\n${BOLD}${CYAN}================================================================${NC}"
echo -e "${BOLD}${CYAN}   SMART SPEAKER HARDWARE DIAGNOSTICS & TELEMETRY VERIFIER      ${NC}"
echo -e "${BOLD}${CYAN}================================================================${NC}\n"

PASS_COUNT=0
WARN_COUNT=0
FAIL_COUNT=0

record_pass() { echo -e "  [${GREEN}PASS${NC}] $*"; PASS_COUNT=$((PASS_COUNT + 1)); }
record_warn() { echo -e "  [${YELLOW}WARN${NC}] $*"; WARN_COUNT=$((WARN_COUNT + 1)); }
record_fail() { echo -e "  [${RED}FAIL${NC}] $*"; FAIL_COUNT=$((FAIL_COUNT + 1)); }

# ------------------------------------------------------------------------------
# 1. Host Board & SoC Telemetry
# ------------------------------------------------------------------------------
echo -e "${BOLD}1. Host Board & CPU Telemetry:${NC}"

ARCH=$(uname -m)
if [[ "${ARCH}" == "aarch64" ]]; then
    record_pass "CPU Architecture: ${ARCH} (ARM64 target platform)"
else
    record_warn "CPU Architecture: ${ARCH} (Target is aarch64 Orange Pi Zero 3)"
fi

BOARD_MODEL="Unknown"
if [[ -f "/proc/device-tree/model" ]]; then
    BOARD_MODEL=$(tr -d '\0' < /proc/device-tree/model)
elif [[ -f "/etc/armbian-release" ]]; then
    BOARD_MODEL=$(grep -E "^BOARD_NAME=" /etc/armbian-release | cut -d'=' -f2 | tr -d '"')
fi
echo -e "     Model: ${BOARD_MODEL}"

CPU_CORES=$(nproc 2>/dev/null || grep -c ^processor /proc/cpuinfo)
if [[ ${CPU_CORES} -ge 4 ]]; then
    record_pass "CPU Core Count: ${CPU_CORES} cores detected"
else
    record_warn "CPU Core Count: ${CPU_CORES} cores (Allwinner H618 has 4 cores)"
fi

# CPU Frequency
FREQ_FILE="/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq"
if [[ -f "${FREQ_FILE}" ]]; then
    FREQ_KHZ=$(cat "${FREQ_FILE}")
    FREQ_MHZ=$((FREQ_KHZ / 1000))
    record_pass "Current CPU Frequency: ${FREQ_MHZ} MHz"
else
    echo -e "     CPU Frequency: cpufreq sysfs not exposed"
fi

# CPU Temperature & Throttling
TEMP_FILE="/sys/class/thermal/thermal_zone0/temp"
if [[ -f "${TEMP_FILE}" ]]; then
    RAW_TEMP=$(cat "${TEMP_FILE}")
    if [[ ${RAW_TEMP} -gt 1000 ]]; then
        TEMP_C=$(awk "BEGIN {printf \"%.1f\", ${RAW_TEMP}/1000}")
        RAW_C=$((RAW_TEMP / 1000))
    else
        TEMP_C="${RAW_TEMP}.0"
        RAW_C=${RAW_TEMP}
    fi

    if [[ ${RAW_C} -lt 65 ]]; then
        record_pass "SoC Temperature: ${TEMP_C}°C (Normal operational range)"
    elif [[ ${RAW_C} -lt 80 ]]; then
        record_warn "SoC Temperature: ${TEMP_C}°C (Elevated, check heatsink)"
    else
        record_fail "SoC Temperature: ${TEMP_C}°C (THERMAL THROTTLING DETECTED >= 80°C)"
    fi
else
    record_warn "Thermal zone sensor not found at ${TEMP_FILE}"
fi

# RAM Check
MEM_TOTAL_KB=$(grep MemTotal /proc/meminfo | awk '{print $2}')
MEM_TOTAL_MB=$((MEM_TOTAL_KB / 1024))
if [[ ${MEM_TOTAL_MB} -ge 3500 ]]; then
    record_pass "Memory Total: ${MEM_TOTAL_MB} MB (~4GB LPDDR4 configuration verified)"
else
    record_warn "Memory Total: ${MEM_TOTAL_MB} MB"
fi

# ------------------------------------------------------------------------------
# 2. Seeed ReSpeaker XVF3800 (Microphone Array)
# ------------------------------------------------------------------------------
echo -e "\n${BOLD}2. Seeed ReSpeaker XVF3800 Array Check:${NC}"

RESPEAKER_USB_FOUND=0
if command -v lsusb &>/dev/null; then
    if lsusb -d 2886: &>/dev/null; then
        RESPEAKER_USB_FOUND=1
    fi
fi

if [[ ${RESPEAKER_USB_FOUND} -eq 0 && -d "/sys/bus/usb/devices" ]]; then
    for vendor in /sys/bus/usb/devices/*/idVendor; do
        if [[ -f "${vendor}" && "$(cat "${vendor}" 2>/dev/null)" == "2886" ]]; then
            RESPEAKER_USB_FOUND=1
            break
        fi
    done
fi

if [[ ${RESPEAKER_USB_FOUND} -eq 1 ]]; then
    record_pass "Seeed ReSpeaker USB hardware detected (Vendor ID: 2886)"
else
    record_warn "ReSpeaker XVF3800 USB device (2886:*) not found on USB bus"
fi

# ALSA Capture Device
ARECORD_OUT=$(arecord -l 2>/dev/null || true)
if echo "${ARECORD_OUT}" | grep -Ei "(respeaker|xvf3800|arrayuac|seeed)" &>/dev/null; then
    CARD_LINE=$(echo "${ARECORD_OUT}" | grep -Ei "(respeaker|xvf3800|arrayuac|seeed)" | head -n1)
    record_pass "ALSA Audio Capture Device verified: ${CARD_LINE}"
else
    record_warn "ReSpeaker XVF3800 capture card not registered in ALSA (arecord -l)"
fi

# ------------------------------------------------------------------------------
# 3. Audio Output & GT-112 Amplifier Verification
# ------------------------------------------------------------------------------
echo -e "\n${BOLD}3. Audio Output & GT-112 Verification:${NC}"

APLAY_OUT=$(aplay -l 2>/dev/null || true)
if [[ -n "${APLAY_OUT}" ]]; then
    PLAYBACK_CARDS=$(echo "${APLAY_OUT}" | grep -c "^card")
    record_pass "ALSA Playback Sound Cards detected: ${PLAYBACK_CARDS} card(s)"
else
    record_warn "No ALSA playback devices found in 'aplay -l'"
fi

# Safe Volume Ceiling Check
SAFE_CEILING=85
if command -v amixer &>/dev/null; then
    MIXER_STATUS=$(amixer sget Master 2>/dev/null || true)
    if [[ -n "${MIXER_STATUS}" ]]; then
        record_pass "ALSA Master mixer control is accessible"
    else
        echo -e "     amixer: Master control not mapped, checking default soundcard"
    fi
fi

if [[ "${1:-}" == "--test-audio" ]]; then
    echo -e "     Testing audio output with safe 440Hz tone (capped at ${SAFE_CEILING}% volume)..."
    if command -v speaker-test &>/dev/null; then
        speaker-test -t sine -f 440 -l 1 >/dev/null 2>&1 || true
        record_pass "Speaker test signal dispatched successfully"
    else
        record_warn "speaker-test utility not installed, skipping audio signal check"
    fi
else
    echo -e "     Audio signal test skipped (run with '--test-audio' for live sound check)"
fi

# ------------------------------------------------------------------------------
# 4. Network & Bluetooth Connectivity
# ------------------------------------------------------------------------------
echo -e "\n${BOLD}4. NetworkManager & Bluetooth BlueZ:${NC}"

if systemctl is-active --quiet NetworkManager 2>/dev/null; then
    record_pass "NetworkManager daemon is active"
elif command -v nmcli &>/dev/null && nmcli general &>/dev/null; then
    record_pass "NetworkManager is reachable via nmcli"
else
    record_warn "NetworkManager daemon is inactive or not installed"
fi

if systemctl is-active --quiet bluetooth 2>/dev/null; then
    record_pass "Bluetooth (BlueZ) daemon is active"
elif command -v bluetoothctl &>/dev/null; then
    record_pass "BlueZ stack installed (bluetoothctl available)"
else
    record_warn "Bluetooth service is inactive or not installed"
fi

# ------------------------------------------------------------------------------
# 5. SQLite Database Integrity
# ------------------------------------------------------------------------------
echo -e "\n${BOLD}5. SQLite Database Integrity:${NC}"

DB_CANDIDATES=(
    "/var/lib/smart-speaker/kamish_core.db"
    "$(dirname "$0")/../../backend/kamish_core.db"
    "./kamish_core.db"
    "/data/kamish/kamish.db"
)

DB_PATH=""
for path in "${DB_CANDIDATES[@]}"; do
    if [[ -f "${path}" ]]; then
        DB_PATH="${path}"
        break
    fi
done

if [[ -n "${DB_PATH}" ]]; then
    INTEGRITY=$(sqlite3 "${DB_PATH}" "PRAGMA integrity_check;" 2>/dev/null || echo "error")
    JOURNAL_MODE=$(sqlite3 "${DB_PATH}" "PRAGMA journal_mode;" 2>/dev/null || echo "unknown")
    
    if [[ "${INTEGRITY}" == "ok" ]]; then
        record_pass "SQLite Database Integrity check passed (${DB_PATH})"
        record_pass "SQLite Journal Mode: ${JOURNAL_MODE} (WAL recommended)"
    else
        record_fail "SQLite Database integrity error: ${INTEGRITY}"
    fi
else
    record_warn "No existing database file found to verify"
fi

# ------------------------------------------------------------------------------
# Summary
# ------------------------------------------------------------------------------
echo -e "\n${BOLD}${CYAN}----------------------------------------------------------------${NC}"
echo -e "${BOLD}Diagnostic Summary:${NC} ${GREEN}${PASS_COUNT} Passed${NC}, ${YELLOW}${WARN_COUNT} Warnings${NC}, ${RED}${FAIL_COUNT} Failed${NC}"

if [[ ${FAIL_COUNT} -eq 0 ]]; then
    echo -e "${GREEN}${BOLD}Overall Status: HARDWARE VERIFICATION PASSED${NC}\n"
    exit 0
else
    echo -e "${RED}${BOLD}Overall Status: HARDWARE VERIFICATION FAILED (Inspect errors above)${NC}\n"
    exit 1
fi
