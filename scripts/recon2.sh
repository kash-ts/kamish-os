#!/usr/bin/env bash
set -u
inc="/tmp/orangepi-build/external/config/sources/families/include/sunxi64_common.inc"
grep -n -E "KERNELSOURCE|UBOOTSOURCE|MAINLINE_UBOOT_SOURCE|ATFSOURCE" "$inc" | head -20
