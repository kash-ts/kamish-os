#!/usr/bin/env bash
# Проверка окружения WSL2 для сборки образа KAMISH OS (Buildroot).
set -u

echo "=== Distro ==="
cat /etc/os-release 2>/dev/null | head -2

echo "=== Kernel/Arch ==="
uname -m

echo "=== Disk space (/home) ==="
df -h "$HOME"

echo "=== Network: buildroot.org (реальное скачивание, не HEAD) ==="
curl -fsSL --retry 2 --max-time 60 -o /tmp/buildroot-test.tar.gz \
    https://buildroot.org/downloads/buildroot-2024.02.1.tar.gz \
    && echo "buildroot download OK ($(du -h /tmp/buildroot-test.tar.gz | cut -f1))" \
    && rm -f /tmp/buildroot-test.tar.gz \
    || echo "buildroot download FAILED"

echo "=== Toolchain essentials ==="
for tool in gcc make rsync file unzip wget bc git python3 perl; do
    if command -v "$tool" >/dev/null 2>&1; then
        echo "$tool: $(command -v "$tool")"
    else
        echo "$tool: MISSING"
    fi
done

echo "=== Rust ==="
if command -v rustup >/dev/null 2>&1; then
    rustup toolchain list 2>/dev/null | head -5
    rustup target list --installed 2>/dev/null | head -10
else
    echo "rustup: MISSING"
fi

echo "=== apt availability ==="
if command -v apt-get >/dev/null 2>&1; then
    echo "apt-get: present"
    sudo -n true 2>/dev/null && echo "sudo: passwordless" || echo "sudo: password required"
else
    echo "apt-get: MISSING"
fi
