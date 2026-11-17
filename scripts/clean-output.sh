#!/usr/bin/env bash
# Очистка output (повреждён гонкой двух сборок), сохранение dl-кэша
set -u
CACHE=/home/ipoma/kamish-buildroot

echo "=== до очистки:"
du -sh "$CACHE/dl" 2>/dev/null
du -sh "$CACHE/output" 2>/dev/null

rm -rf "$CACHE/output"
mkdir -p "$CACHE/output"

echo "=== после очистки:"
ls -la "$CACHE"
