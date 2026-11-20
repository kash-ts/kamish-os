# Orange Pi 4 Pro (Allwinner A733 / sun60iw2) Universal RAUC A/B Boot Script (USB & microSD)
test -n "${devtype}" || setenv devtype "mmc"
test -n "${devnum}" || setenv devnum "0"
echo "=== KAMISH OS Bootloader Initializing (${devtype} ${devnum}) ==="

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

if test "${devtype}" = "mmc"; then
    saveenv
fi

echo "Booting KAMISH OS from Slot ${boot_slot} (${devtype} ${devnum}:${rootpart})..."

# 4. Resolve root block device (PARTUUID works uniformly on USB and microSD)
setenv root_dev ""
if part uuid ${devtype} ${devnum}:${rootpart} root_partuuid; then
    setenv root_dev "PARTUUID=${root_partuuid}"
elif test "${devtype}" = "usb"; then
    setenv root_dev "/dev/sda${rootpart}"
else
    setenv root_dev "/dev/mmcblk${devnum}p${rootpart}"
fi

# 5. Kernel boot arguments (UART + HDMI console, BSP earlyprintk & clock/DMA flags)
setenv bootargs "console=ttyS0,115200 console=tty1 earlyprintk=sunxi-uart,0x02500000 initcall_debug=0 consoleblank=0 clk_ignore_unused swiotlb=65536 root=${root_dev} ro rootfstype=ext4 rootwait rootdelay=3 panic=10 loglevel=7"

# 6. Load kernel Image from active boot media
if load ${devtype} ${devnum}:${rootpart} ${kernel_addr_r} /boot/Image; then
    echo "Loaded /boot/Image from ${devtype} ${devnum}:${rootpart}"
else
    echo "ERROR: Failed to load /boot/Image from ${devtype} ${devnum}:${rootpart}"
    exit
fi

# 7. Load DTB from active boot media
if load ${devtype} ${devnum}:${rootpart} ${fdt_addr_r} /boot/sun60i-a733-orangepi-4-pro.dtb; then
    echo "Loaded DTB from /boot/sun60i-a733-orangepi-4-pro.dtb"
elif load ${devtype} ${devnum}:${rootpart} ${fdt_addr_r} /boot/allwinner/sun60i-a733-orangepi-4-pro.dtb; then
    echo "Loaded DTB from /boot/allwinner/sun60i-a733-orangepi-4-pro.dtb"
elif load ${devtype} ${devnum}:${rootpart} ${fdt_addr_r} /boot/dtb/allwinner/sun60i-a733-orangepi-4-pro.dtb; then
    echo "Loaded DTB from /boot/dtb/allwinner/sun60i-a733-orangepi-4-pro.dtb"
else
    echo "ERROR: Failed to load sun60i-a733-orangepi-4-pro.dtb from ${devtype} ${devnum}:${rootpart}"
    exit
fi

# 8. Apply DTB fixup for vendor U-Boot zero-RAM bug (4 GiB RAM @ 0x40000000)
fdt addr ${fdt_addr_r}
fdt resize 0x10000
if fdt set /memory@40000000 reg <0x0 0x40000000 0x1 0x00000000>; then
    echo "Updated /memory@40000000 reg to 4 GiB"
elif fdt set /memory reg <0x0 0x40000000 0x1 0x00000000>; then
    echo "Updated /memory reg to 4 GiB"
fi

# 9. Boot kernel
echo "Booting kernel (${root_dev})..."
booti ${kernel_addr_r} - ${fdt_addr_r}
