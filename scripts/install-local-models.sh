#!/bin/sh
# ---------------------------------------------------------------------------
# install-local-models.sh — установка локальных моделей STT и TTS для «Камыша»
#
# Ставит:
#   * Piper TTS  (нейросетевой синтез русской речи, ONNX)
#   * Vosk STT   (оффлайн-распознавание русской речи)
#
# Всё складывается в KAMISH_MODELS_DIR (по умолчанию /opt/kamish-models),
# Python-пакеты — в venv KAMISH_VENV_DIR (по умолчанию /opt/kamish-tts).
#
# Скрипт идемпотентен: уже скачанное не перекачивается.
# Ничего не запускает от имени сервиса и не требует systemd.
# ---------------------------------------------------------------------------
set -eu

MODELS_DIR="${KAMISH_MODELS_DIR:-/opt/kamish-models}"
VENV_DIR="${KAMISH_VENV_DIR:-/opt/kamish-tts}"
PIPER_VOICE="${KAMISH_PIPER_VOICE:-ru_RU-dmitri-medium}"
VOSK_MODEL="${KAMISH_VOSK_MODEL:-vosk-model-small-ru-0.22}"
VOSK_URL="${KAMISH_VOSK_URL:-https://alphacephei.com/vosk/models/${VOSK_MODEL}.zip}"
PIPER_VOICES_BASE="https://huggingface.co/rhasspy/piper-voices/resolve/main"

log() { printf '%s\n' "==> $*"; }
warn() { printf '%s\n' "!!  $*" >&2; }

# --- pip-обёртка: pip может отсутствовать, тогда используем venv python ---
pip_install() {
    if [ -x "$VENV_DIR/bin/pip" ]; then
        "$VENV_DIR/bin/pip" install -q "$@"
    else
        python3 -m pip install -q --break-system-packages "$@"
    fi
}

need_cmd() {
    command -v "$1" >/dev/null 2>&1 || {
        warn "Не найдена команда '$1'. Установите: apt-get install -y $2"
        exit 1
    }
}

need_cmd curl "curl"
need_cmd unzip "unzip"
need_cmd python3 "python3 python3-venv"

mkdir -p "$MODELS_DIR/tts" "$MODELS_DIR/stt"

# ---------------------------------------------------------------------------
# 1. Python-окружение
# ---------------------------------------------------------------------------
log "Готовлю Python-окружение: $VENV_DIR"
if [ ! -x "$VENV_DIR/bin/python" ]; then
    python3 -m venv "$VENV_DIR" || {
        warn "Не удалось создать venv. Установите: apt-get install -y python3-venv"
        exit 1
    }
fi
"$VENV_DIR/bin/pip" install -q --upgrade pip 2>/dev/null || true

# ---------------------------------------------------------------------------
# 2. Piper TTS
# ---------------------------------------------------------------------------
log "Устанавливаю piper-tts"
pip_install piper-tts

PIPER_BIN="$VENV_DIR/bin/piper"
if [ ! -x "$PIPER_BIN" ]; then
    warn "Бинарник piper не появился в $VENV_DIR/bin"
    exit 1
fi

# Разбираем имя голоса: ru_RU-dmitri-medium -> ru/ru_RU/dmitri/medium
VOICE_LANG=$(printf '%s' "$PIPER_VOICE" | cut -d_ -f1)
VOICE_REGION=$(printf '%s' "$PIPER_VOICE" | cut -d- -f1)
VOICE_NAME=$(printf '%s' "$PIPER_VOICE" | cut -d- -f2)
VOICE_QUALITY=$(printf '%s' "$PIPER_VOICE" | cut -d- -f3)

ONNX="$MODELS_DIR/tts/${PIPER_VOICE}.onnx"
JSON="$MODELS_DIR/tts/${PIPER_VOICE}.onnx.json"

if [ -s "$ONNX" ] && [ -s "$JSON" ]; then
    log "Голос уже на месте: $ONNX"
else
    log "Скачиваю голос $PIPER_VOICE"
    BASE="$PIPER_VOICES_BASE/${VOICE_LANG}/${VOICE_REGION}/${VOICE_NAME}/${VOICE_QUALITY}"
    curl -fL --retry 3 --retry-delay 2 -o "$ONNX" "${BASE}/${PIPER_VOICE}.onnx"
    curl -fL --retry 3 --retry-delay 2 -o "$JSON" "${BASE}/${PIPER_VOICE}.onnx.json"
fi

# ---------------------------------------------------------------------------
# 3. Vosk STT
# ---------------------------------------------------------------------------
log "Устанавливаю vosk"
pip_install vosk

VOSK_DIR="$MODELS_DIR/stt/$VOSK_MODEL"
if [ -d "$VOSK_DIR" ] && [ -n "$(ls -A "$VOSK_DIR" 2>/dev/null)" ]; then
    log "Модель уже на месте: $VOSK_DIR"
else
    log "Скачиваю модель $VOSK_MODEL (~42 МБ)"
    TMPZIP="$MODELS_DIR/stt/.vosk.zip"
    rm -f "$TMPZIP"
    # Сервер отдаёт файл медленно — качаем с докачкой и без жёсткого общего таймаута.
    curl -fL --retry 5 --retry-delay 3 --retry-all-errors -C - -o "$TMPZIP" "$VOSK_URL" \
        || { warn "Не удалось скачать модель Vosk с $VOSK_URL"; exit 1; }
    unzip -q -o "$TMPZIP" -d "$MODELS_DIR/stt"
    rm -f "$TMPZIP"
fi

# ---------------------------------------------------------------------------
# 4. Итог
# ---------------------------------------------------------------------------
echo
log "Готово. Проверка синтеза:"
echo "    echo 'Привет, я колонка Камыш' | \\"
echo "      $PIPER_BIN -m $ONNX -f /tmp/tts-check.wav"
echo "    soxi /tmp/tts-check.wav"
echo
log "Проверка распознавания:"
echo "    $VENV_DIR/bin/python $(dirname "$0")/../scripts/stt-check.py /tmp/tts-check.wav"
echo
log "Пропишите в окружении сервиса:"
echo "    PIPER_BIN=$PIPER_BIN"
echo "    PIPER_MODEL_PATH=$ONNX"
echo "    VOSK_MODEL_PATH=$VOSK_DIR"
echo "    KAMISH_SPEECH_LANG=ru"
