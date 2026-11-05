#!/usr/bin/env python3
"""vosk-transcriber — CLI-обёртка для совместимости с VoskSttProvider.

`VoskSttProvider` (crates/kamish-core/src/speech/stt/vosk.rs) вызывает внешний бинарник так:

    vosk-transcriber -i <файл.wav> -m <путь-к-модели>
    vosk-transcriber -i <файл.wav> -l ru

Пакет `vosk` из pip такого бинарника не ставит, поэтому эта обёртка его
заменяет: читает WAV и печатает распознанный текст в stdout (одна строка),
что и ожидает провайдер.

Установка: положить в /usr/local/bin/vosk-transcriber и сделать исполняемым.
"""
import argparse
import json
import os
import sys
import wave

DEFAULT_MODELS = [
    "/opt/kamish-models/stt/vosk-model-small-ru-0.22",
    "/opt/kamish-models/stt/vosk-model-ru-0.42",
]
LANG_FALLBACK = {
    "ru": "/opt/kamish-models/stt/vosk-model-small-ru-0.22",
    "en": "/opt/kamish-models/stt/vosk-model-small-en-us-0.15",
}


def resolve_model(arg_model: str | None, lang: str) -> str:
    if arg_model:
        return arg_model
    env = os.environ.get("VOSK_MODEL_PATH")
    if env:
        return env
    for candidate in DEFAULT_MODELS:
        if os.path.isdir(candidate):
            return candidate
    fallback = LANG_FALLBACK.get(lang)
    if fallback and os.path.isdir(fallback):
        return fallback
    return DEFAULT_MODELS[0]


def main() -> int:
    parser = argparse.ArgumentParser(description="Vosk transcriber compatibility wrapper")
    parser.add_argument("-i", "--input", required=True, help="Input WAV file")
    parser.add_argument("-m", "--model", default=None, help="Path to Vosk model directory")
    parser.add_argument("-l", "--lang", default="ru", help="Language code")
    args = parser.parse_args()

    if not os.path.isfile(args.input):
        print(f"Input file not found: {args.input}", file=sys.stderr)
        return 1

    model_path = resolve_model(args.model, args.lang)
    if not os.path.isdir(model_path):
        print(f"Vosk model not found: {model_path}", file=sys.stderr)
        print("Run scripts/install-local-models.sh to install it", file=sys.stderr)
        return 1

    try:
        import vosk
    except ImportError:
        print("python package 'vosk' is not installed", file=sys.stderr)
        return 1

    vosk.SetLogLevel(-1)
    model = vosk.Model(model_path)

    with wave.open(args.input, "rb") as wf:
        rate = wf.getframerate()
        rec = vosk.KaldiRecognizer(model, rate)
        parts: list[str] = []
        while True:
            data = wf.readframes(4000)
            if not data:
                break
            if rec.AcceptWaveform(data):
                text = json.loads(rec.Result()).get("text", "")
                if text:
                    parts.append(text)
        text = json.loads(rec.FinalResult()).get("text", "")
        if text:
            parts.append(text)

    print(" ".join(parts).strip())
    return 0


if __name__ == "__main__":
    sys.exit(main())
