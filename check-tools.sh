#!/usr/bin/env bash
for t in python3 python git curl wget tar xz gcc cmake podman buildah docker make rsync cpio unzip bc; do
  if command -v "$t" >/dev/null 2>&1; then
    echo "OK   $t: $(command -v "$t")"
  else
    echo "MISS $t"
  fi
done
echo "--- groups ---"
id -nG
echo "--- wsl.conf ---"
cat /etc/wsl.conf 2>/dev/null || echo "(none)"