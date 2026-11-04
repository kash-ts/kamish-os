#!/usr/bin/env python3
"""stt-check.py — проверить локальное распознавание речи (Vosk).

Использование:
    python3 stt-check.py <файл.wav> [путь-к-модели]

Скрипт читает WAV (16 кГц, моно), прогоняет его через Vosk и печатает
распознанный текст. Нужен для того, чтобы убедиться: модель действительно
слышит вашу речь, а не возвращает заготовку.

Если путь к модели не указан, берётся $VOSK_MODEL_PATH, иначе
/opt/kamish-models/stt/vosk-model-small-ru-0.22.
"""
import json
import os
import sys
import wave

DEFAULT_MODEL = "/opt/kamish-models/stt/vosk-model-small-ru-0.22"


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__)
        return 2

    wav_path = sys.argv[1]
    model_path = (
        sys.argv[2]
        if len(sys.argv) > 2
        else os.environ.get("VOSK_MODEL_PATH", DEFAULT_MODEL)
    )

    if not os.path.isfile(wav_path):
        print(f"Файл не найден: {wav_path}", file=sys.stderr)
        return 1
    if not os.path.isdir(model_path):
        print(f"Модель не найдена: {model_path}", file=sys.stderr)
        print("Установите её: ./scripts/install-local-models.sh", file=sys.stderr)
        return 1

    try:
        import vosk
    except ImportError:
        print("Модуль vosk не установлен. Запустите install-local-models.sh", file=sys.stderr)
        return 1

    vosk.SetLogLevel(-1)
    model = vosk.Model(model_path)

    with wave.open(wav_path, "rb") as wf:
        channels = wf.getnchannels()
        rate = wf.getframerate()
        width = wf.getsampwidth()
        if channels != 1 or width != 2:
            print(
                f"Ожидается моно 16-bit WAV, получено: каналов={channels}, "
                f"ширина={width * 8} bit",
                file=sys.stderr,
            )
            return 1
        if rate != 16000:
            print(
                f"Внимание: частота {rate} Гц вместо 16000. "
                "Качество распознавания может снизиться.",
                file=sys.stderr,
            )

        rec = vosk.KaldiRecognizer(model, rate)
        rec.SetWords(False)

        text_parts = []
        while True:
            data = wf.readframes(4000)
            if not data:
                break
            if rec.AcceptWaveform(data):
                res = json.loads(rec.Result())
                if res.get("text"):
                    text_parts.append(res["text"])

        final = json.loads(rec.FinalResult())
        if final.get("text"):
            text_parts.append(final["text"])

    text = " ".join(text_parts).strip()
    print(text if text else "(речь не распознана)")
    return 0 if text else 1


if __name__ == "__main__":
    sys.exit(main())
