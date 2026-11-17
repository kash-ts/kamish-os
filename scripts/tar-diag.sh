#!/usr/bin/env bash
# Диагностика: на чём блокируется tar внутри контейнера
docker exec firmware-builder-run-246482f172c2 bash -c '
for P in /proc/[0-9]*; do
    if grep -qs "^tar$" "$P/comm"; then P="$P"; break; fi
done
echo "tar pid: $P"
echo "wchan: $(cat $P/wchan 2>/dev/null)"
grep -E "rchar|wchar|read_bytes|write_bytes" "$P/io" 2>/dev/null
ls -l "$P/fd" 2>/dev/null | head -10
'
