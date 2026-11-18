# Orange Pi 4 Pro (Allwinner A733 / sun60iw2) RAUC A/B Boot Script
echo "=== KAMISH OS Bootloader Initializing ==="

# 1. Define non-overlapping DRAM load addresses
setenv kernel_addr_r  "0x41000000"
setenv fdt_addr_r     "0x4a000000"
setenv ramdisk_addr_r "0x4b000000"

# 2. Disable DTB and initrd relocation in 32-bit U-Boot
setenv fdt_high "off"
setenv initrd_high "off"

# 3. RAUC A/B Slot Selection
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

# 4. Kernel boot arguments
setenv bootargs "console=ttyS0,115200 root=/dev/mmcblk0p${rootpart} ro rootfstype=ext4 rootwait panic=10 quiet earlycon"

# 5. Load kernel Image
load mmc 0:${rootpart} ${kernel_addr_r} /boot/Image

# 6. Load DTB
if load mmc 0:${rootpart} ${fdt_addr_r} /boot/sun60i-a733-orangepi-4-pro.dtb; then
    echo "Loaded DTB from /boot/sun60i-a733-orangepi-4-pro.dtb"
elif load mmc 0:${rootpart} ${fdt_addr_r} /boot/allwinner/sun60i-a733-orangepi-4-pro.dtb; then
    echo "Loaded DTB from /boot/allwinner/sun60i-a733-orangepi-4-pro.dtb"
elif load mmc 0:${rootpart} ${fdt_addr_r} /boot/dtb/allwinner/sun60i-a733-orangepi-4-pro.dtb; then
    echo "Loaded DTB from /boot/dtb/allwinner/sun60i-a733-orangepi-4-pro.dtb"
fi

# 7. Apply DTB fixup for vendor U-Boot zero-RAM bug
fdt addr ${fdt_addr_r}
fdt resize 0x10000
fdt set /memory reg <0x0 0x40000000 0x3 0x00000000>

# 8. Boot kernel
booti ${kernel_addr_r} - ${fdt_addr_r}
