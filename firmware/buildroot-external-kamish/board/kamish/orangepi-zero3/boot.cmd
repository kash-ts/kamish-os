# U-Boot script for KAMISH OS RAUC A/B Boot Strategy

echo "=== KAMISH OS Bootloader Initializing ==="

test -n "${BOOT_ORDER}" || setenv BOOT_ORDER "A B"
test -n "${BOOT_A_LEFT}" || setenv BOOT_A_LEFT 3
test -n "${BOOT_B_LEFT}" || setenv BOOT_B_LEFT 3

setenv boot_slot ""

for slot in ${BOOT_ORDER}; do
    if test "${boot_slot}" = ""; then
        if test "${slot}" = "A"; then
            if test ${BOOT_A_LEFT} -gt 0; then
                echo "Attempting to boot Slot A (${BOOT_A_LEFT} attempts left)"
                setexpr BOOT_A_LEFT ${BOOT_A_LEFT} - 1
                setenv boot_slot "A"
                setenv rootpart 2
            fi
        fi
        if test "${slot}" = "B"; then
            if test ${BOOT_B_LEFT} -gt 0; then
                echo "Attempting to boot Slot B (${BOOT_B_LEFT} attempts left)"
                setexpr BOOT_B_LEFT ${BOOT_B_LEFT} - 1
                setenv boot_slot "B"
                setenv rootpart 3
            fi
        fi
    fi
done

if test "${boot_slot}" = ""; then
    echo "ERROR: All boot attempts exhausted on both slots! Resetting counters to fallback..."
    setenv BOOT_A_LEFT 3
    setenv BOOT_B_LEFT 3
    setenv boot_slot "A"
    setenv rootpart 2
fi

saveenv

echo "Booting KAMISH OS from Slot ${boot_slot} (partition ${rootpart})..."

setenv bootargs "console=ttyS0,115200 root=/dev/mmcblk0p${rootpart} ro rootfstype=ext4 rootwait panic=10 quiet"

load mmc 0:${rootpart} ${kernel_addr_r} /boot/Image
load mmc 0:${rootpart} ${fdt_addr_r} /boot/sun50i-h618-orangepi-zero3.dtb

booti ${kernel_addr_r} - ${fdt_addr_r}
