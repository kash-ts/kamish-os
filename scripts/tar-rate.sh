#!/usr/bin/env bash
# Скорость записи промежуточного тарбола ядра
T=/home/ipoma/kamish-buildroot/output/build/.linux-orange-pi-6.6-sun60iw2-br1.tar.gz.EtZmuU/tmp.ioDOfhABZm.tar
S1=$(stat -c %s "$T" 2>/dev/null || echo 0)
echo "t0 size: $S1"
sleep 20
S2=$(stat -c %s "$T" 2>/dev/null || echo 0)
echo "t1 size: $S2"
echo "rate: $(( (S2 - S1) / 20 / 1024 )) KB/s"
echo "est remaining (GB): $(( (1450000000 - S2) / 1073741824 ))"
