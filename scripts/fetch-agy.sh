#!/usr/bin/env bash
# ==============================================================================
# fetch-agy.sh — скачать Antigravity CLI (agy) при сборке ядра
#
# Зачем: бинарник agy проприетарный и не лежит в репозитории, но без него
# колонка отвечает заготовкой (agy-mock) вместо живого ИИ. Скрипт вызывается
# из сборки, чтобы не качать CLI вручную каждый раз.
#
# По умолчанию берётся последний релиз из GitHub. Поведение консервативное:
#   * KAMISH_SKIP_AGY=1  — полностью пропустить шаг (офлайн-сборка);
#   * KAMISH_AGY_FORCE=1 — перекачать, даже если уже установлен;
#   * при любой сетевой ошибке сборка НЕ падает: шаг пропускается с предупреждением.
#
# Переменные:
#   KAMISH_AGY_DIR           каталог установки (по умолчанию /opt/kamish/bin)
#   KAMISH_AGY_DOWNLOAD_URL  прямой адрес архива (переопределяет GitHub)
#   KAMISH_AGY_VERSION       конкретный тег вместо latest
# ==============================================================================
set -uo pipefail

AGY_DIR="${KAMISH_AGY_DIR:-/opt/kamish/bin}"
AGY_BIN="${AGY_DIR}/agy"
REPO="google-antigravity/antigravity-cli"

log()  { printf '%s\n' "==> $*"; }
warn() { printf '%s\n' "!!  $*" >&2; }

if [ "${KAMISH_SKIP_AGY:-0}" = "1" ]; then
    log "Пропускаю установку agy (KAMISH_SKIP_AGY=1)"
    exit 0
fi

if [ "${KAMISH_AGY_FORCE:-0}" != "1" ] && [ -x "${AGY_BIN}" ]; then
    log "agy уже установлен: ${AGY_BIN}"
    "${AGY_BIN}" --version 2>/dev/null || true
    exit 0
fi

# --- Определяем архитектуру ---------------------------------------------------
MACHINE="$(uname -m)"
case "${MACHINE}" in
    x86_64)        ASSET_PATTERN="agy_cli_linux_x64.tar.gz" ;;
    aarch64|arm64) ASSET_PATTERN="agy_cli_linux_arm64.tar.gz" ;;
    *) warn "Неподдерживаемая архитектура: ${MACHINE}"; exit 0 ;;
esac

# Buildroot-образы Kamish Linux собраны с musl; если доступен musl-вариант
# и сборка идёт для musl-таргета, предпочитаем его (glibc-бинарь там не стартует).
if [ "${KAMISH_AGY_LIBC:-}" = "musl" ]; then
    case "${ASSET_PATTERN}" in
        *.tar.gz) ASSET_PATTERN="${ASSET_PATTERN%.tar.gz}_musl.tar.gz" ;;
    esac
fi

# --- Определяем адрес загрузки -------------------------------------------------
DOWNLOAD_URL="${KAMISH_AGY_DOWNLOAD_URL:-}"
TAG_NAME=""

if [ -z "${DOWNLOAD_URL}" ]; then
    log "Определяю релиз ${REPO} для ${MACHINE}..."
    if [ -n "${KAMISH_AGY_VERSION:-}" ]; then
        API_URL="https://api.github.com/repos/${REPO}/releases/tags/${KAMISH_AGY_VERSION}"
    else
        API_URL="https://api.github.com/repos/${REPO}/releases/latest"
    fi

    RELEASE_JSON="$(curl -sSL -m 30 -H "User-Agent: kamish-builder" "${API_URL}" 2>/dev/null)"
    if [ -z "${RELEASE_JSON}" ]; then
        warn "Не удалось получить сведения о релизе (нет сети?). Пропускаю agy."
        exit 0
    fi

    DOWNLOAD_URL="$(printf '%s' "${RELEASE_JSON}" \
        | grep -o '"browser_download_url"[[:space:]]*:[[:space:]]*"[^"]*"' \
        | grep "${ASSET_PATTERN}" \
        | sed 's/.*"\(http[^"]*\)".*/\1/' | head -n 1)"
    TAG_NAME="$(printf '%s' "${RELEASE_JSON}" \
        | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' \
        | sed 's/.*"\([^"]*\)"$/\1/' | head -n 1)"

    if [ -z "${DOWNLOAD_URL}" ]; then
        warn "В релизе ${TAG_NAME:-unknown} нет файла ${ASSET_PATTERN}. Пропускаю agy."
        exit 0
    fi
fi

log "Скачиваю agy ${TAG_NAME} (${ASSET_PATTERN})"

mkdir -p "${AGY_DIR}" 2>/dev/null || {
    warn "Нет прав на создание ${AGY_DIR}. Пропускаю (задайте KAMISH_AGY_DIR)."
    exit 0
}

TMP_DIR="$(mktemp -d)" || { warn "mktemp недоступен"; exit 0; }
trap 'rm -rf "${TMP_DIR}"' EXIT

ARCHIVE="${TMP_DIR}/agy.archive"
if ! curl -sSL -m 300 --retry 3 --retry-delay 2 -o "${ARCHIVE}" "${DOWNLOAD_URL}"; then
    warn "Скачивание не удалось. Продолжаю сборку без agy."
    exit 0
fi

# --- Распаковка ---------------------------------------------------------------
case "${DOWNLOAD_URL}" in
    *.tar.gz|*.tgz)
        tar -xzf "${ARCHIVE}" -C "${TMP_DIR}" 2>/dev/null || {
            warn "Не удалось распаковать архив"; exit 0; }
        ;;
    *.zip)
        if command -v unzip >/dev/null 2>&1; then
            unzip -q -o "${ARCHIVE}" -d "${TMP_DIR}" 2>/dev/null || true
        else
            warn "unzip недоступен, использую файл как есть"
        fi
        ;;
esac

BIN_PATH="$(find "${TMP_DIR}" -type f -name 'agy' 2>/dev/null | head -n 1)"
if [ -z "${BIN_PATH}" ]; then
    warn "Не удалось найти бинарник agy в загрузке. Пропускаю."
    exit 0
fi

chmod 755 "${BIN_PATH}"
cp -f "${BIN_PATH}" "${AGY_BIN}"
chmod 755 "${AGY_BIN}"

log "agy установлен: ${AGY_BIN}"
"${AGY_BIN}" --version 2>/dev/null || warn "agy не ответил на --version"
