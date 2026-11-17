#!/usr/bin/env bash
# Клонирование BSP-источников с retry (сеть рвёт длинные соединения)
set -u

clone_with_retry() {
    local url="$1" branch="$2" dir="$3"
    local attempt
    for attempt in 1 2 3 4 5 6 7 8; do
        rm -rf "$dir"
        echo "--- attempt $attempt: $dir"
        if git clone --depth 1 -b "$branch" "$url" "$dir" 2>&1 | tail -2; then
            if [ -f "$dir/README" ] || [ -f "$dir/Makefile" ] || [ -f "$dir/Kbuild" ]; then
                echo "CLONE OK: $dir"
                return 0
            fi
        fi
        echo "RETRY in 5s..."
        sleep 5
    done
    echo "CLONE FAILED: $dir"
    return 1
}

git config --global http.postBuffer 524288000
git config --global http.lowSpeedLimit 0
git config --global http.lowSpeedTime 999999

clone_with_retry https://github.com/orangepi-xunlong/linux-orangepi.git orange-pi-6.6-sun60iw2 /tmp/linux-sun60iw2
clone_with_retry https://github.com/orangepi-xunlong/u-boot-orangepi.git v2018.05-sun60iw2 /tmp/uboot-sun60iw2
echo "ALL CLONES DONE"
