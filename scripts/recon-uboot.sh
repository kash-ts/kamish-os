#!/usr/bin/env bash
# Разведка BSP U-Boot sun60iw2
set -u
cd /tmp/uboot-sun60iw2

echo "=== booti / bootm cmds ==="
ls cmd/ | grep -E "booti|bootm|boot" || true
grep -rn "CONFIG_CMD_BOOTI" cmd/Kconfig 2>/dev/null | head -5

echo "=== distro boot (boot.scr scan) ==="
grep -rln "BOOT_TARGET_DEVICES" include/configs/ | head -5
grep -n "scan_dev_for_boot\|BOOT_TARGET_DEVICES\|boot_scripts" include/configs/sun60iw2.h include/configs/sun8i.h include/configs/sunxi-common.h 2>/dev/null | head -20

echo "=== env ext4 defaults ==="
grep -rn "ENV_EXT4" include/env_default.h include/configs/sun60iw2.h configs/sun60iw2p1_a733_defconfig 2>/dev/null | head -10

echo "=== default env script target ==="
ls include/configs/ | grep -iE "sun60|sunxi" | head -10
