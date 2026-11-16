#!/usr/bin/env bash
# Скачивание тарбола Buildroot с retry (сеть рвёт соединения)
set -u
DEST="${1:-"$(cd "$(dirname "$0")/.." && pwd)/firmware/buildroot-2024.02.1.tar.gz"}"
URL=https://buildroot.org/downloads/buildroot-2024.02.1.tar.gz

for i in 1 2 3 4 5 6 7 8; do
    echo "--- attempt $i"
    if curl -fL --retry 3 --retry-all-errors --max-time 300 -o "$DEST" "$URL"; then
        echo "DOWNLOAD OK"
        break
    fi
    sleep 5
done

if [ ! -s "$DEST" ]; then
    echo "DOWNLOAD FAILED"
    exit 1
fi
ls -lh "$DEST"
