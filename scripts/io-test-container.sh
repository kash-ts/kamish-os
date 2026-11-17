#!/usr/bin/env bash
# Тест I/O внутри контейнера: чтение дерева git и запись dd
docker exec firmware-builder-run-246482f172c2 bash -c '
echo "=== чтение дерева git (первый проход, холодный кэш):"
time tar cf /dev/null -C /build/buildroot/dl/linux/git . 2>/dev/null
echo "=== второй проход (тёплый):"
time tar cf /dev/null -C /build/buildroot/dl/linux/git . 2>/dev/null
echo "=== запись 200МБ на volume:"
dd if=/dev/zero of=/build/buildroot/output/build/ddtest bs=1M count=200 2>&1 | tail -1
rm -f /build/buildroot/output/build/ddtest
'
