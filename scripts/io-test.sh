#!/usr/bin/env bash
# Тест скорости записи на ext4 WSL и чтения из дерева git
dd if=/dev/zero of=/home/ipoma/ddtest.bin bs=1M count=200 oflag=direct 2>&1 | tail -1
rm -f /home/ipoma/ddtest.bin
echo "=== чтение из дерева ядра:"
D=/home/ipoma/kamish-buildroot/dl/linux/git
du -s "$D" 2>/dev/null
time tar cf /dev/null -C "$D" . 2>&1 | tail -1 || true
