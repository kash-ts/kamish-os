#!/usr/bin/env bash
# Вытащить имена defconfig из сохранённого JSON-листинга u-boot configs
set -eu
f="$1"
python3 - "$f" <<'EOF'
import json, sys
data = json.load(open(sys.argv[1]))
names = sorted(e["name"] for e in data)
for n in names:
    print(n)
EOF
