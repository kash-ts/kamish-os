#!/usr/bin/env bash
# Размер кэш-тарбола ядра дважды с интервалом 15с — оценка скорости упаковки
F=/home/ipoma/kamish-buildroot/dl/linux/linux-orange-pi-6.6-sun60iw2-br1.tar.gz
if [ -f "$F" ]; then
    S1=$(stat -c %s "$F")
    echo "size: $S1"
    sleep 15
    S2=$(stat -c %s "$F")
    echo "size: $S2"
    echo "rate: $(( (S2 - S1) / 15 / 1024 / 1024 )) MB/s"
else
    echo "tarball not yet created"
fi
