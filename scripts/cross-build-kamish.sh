#!/usr/bin/env bash
# Кросс-сборка kamish-core/kamish-web под aarch64.
# В WSL2 использует rsync в $HOME для скорости, в CI собирает по месту.
set -eu

# Определяем корень проекта
SRC="$(cd "$(dirname "$0")/.." && pwd)"

# В CI используем текущую директорию, в WSL - быструю ФС
if [ "${GITHUB_ACTIONS:-}" = "true" ]; then
    DST="$SRC"
    echo "=== CI mode: building in-place ==="
else
    DST="${KAMISH_BUILD_DIR:-$HOME/kamish-src}"
    echo "=== Dev mode: rsync workspace -> $DST ==="
    rsync -a --delete \
        --exclude .git --exclude target --exclude node_modules \
        --exclude firmware --exclude docs --exclude deploy \
        --exclude .agents --exclude .work --exclude scripts \
        "$SRC/crates" "$SRC/database" "$SRC/Cargo.toml" "$SRC/Cargo.lock" \
        "$DST/" 2>/dev/null || { mkdir -p "$DST"; rsync -a --delete \
        --exclude .git --exclude target --exclude node_modules \
        --exclude firmware --exclude docs --exclude deploy \
        --exclude .agents --exclude .work --exclude scripts \
        "$SRC/crates" "$SRC/database" "$SRC/Cargo.toml" "$SRC/Cargo.lock" "$DST/"; }
fi

source "$HOME/.cargo/env"
export CARGO_BUILD_TARGET=aarch64-unknown-linux-gnu
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc

cd "$DST"
echo "=== cargo build --release (aarch64) ==="
cargo build --release --workspace

echo "=== копирование артефактов в репозиторий ==="
OUT="$SRC/target/aarch64-unknown-linux-gnu/release"
mkdir -p "$OUT"
for bin in kamish-core kamish-web; do
    file="$DST/target/aarch64-unknown-linux-gnu/release/$bin"
    test -f "$file" || { echo "MISSING: $bin" >&2; exit 1; }
    # В CI DST == SRC: пропускаем копирование файла на самого себя
    if [ "$DST" != "$SRC" ]; then
        cp "$file" "$OUT/$bin"
    fi
done
echo "=== артефакты ==="
file "$OUT/kamish-core" "$OUT/kamish-web"
echo "CROSS BUILD OK"
