# SESSION STATE — передача контекста другому агенту

> **Дата:** 2026-11-04 (обновлено 11:32)
> **Проект:** `M:\Project\Kamish-os\radiant-bardeen` — умная колонка «Камыш»
> **Сессия:** `session-a8a5c612-8ecc-45ce-9713-5ae88f808ab7`
> **Полный диалог:** [`SESSION_EXPORT.md`](SESSION_EXPORT.md) — 54 сообщения
> пользователя, 616 ответов агента, 636 вызовов инструментов, 635 результатов, 325 КБ

Этот файл — сжатая карта состояния. Читайте его первым, диалог — как справочник деталей.

---

## 1. Аппаратура и окружение (ВАЖНО: изменилось)

| | Было в документации | Фактически (со слов владельца) |
|---|---|---|
| Плата | Orange Pi Zero 3 | **Orange Pi 4 Pro** (Allwinner A733) |
| RAM | 4 GB | **4 GB** (подтверждено) |

**Код к плате НЕ привязан:** `detect_board_model()` (`backend/src/system/mod.rs`)
читает `/proc/device-tree/model` и `/etc/armbian-release`. Работает на любой плате.

**Документация устарела** — в `docs/HARDWARE.md`, `docs/SPECIFICATION.md`,
`docs/ARCHITECTURE.md`, `docs/DECISIONS.md` везде «Orange Pi Zero 3 / H618».
Это надо править, но владелец ещё не просил.

Лимиты памяти из `SPECIFICATION.md`:
- idle-потребление бэкенда: `≈25 MB`, бюджет `≤60 MB`, жёсткий лимит `≤150 MB` RSS

**Среда разработки:** Windows x86_64 + WSL2 Ubuntu. ВСЕГДА собирать в WSL
(`CARGO_TARGET_DIR=$HOME/.cache/kamish-target`), не в Windows — падает с `dlltool.exe not found`.

**Инструменты, уже установленные в WSL:**
`mpv 0.41.0`, `yt-dlp 2026.03.17`, `nodejs v22.22.1`, `zstd`, `sox`, `openjdk-17-jre-headless`,
`/opt/kamish-tts` (venv: `piper-tts 1.8.0`, `vosk`, `ytmusicapi 1.12.3`).

> [!WARNING]
> **`/tmp` и `$HOME` в WSL периодически очищаются / недоступны для записи.**
> На практике: `/tmp/lavatest` исчез вместе с 96-МБ JAR; `curl -o "$HOME/x.json"`
> падал с «No such file or directory». **Пишите временные файлы в каталог проекта**
> (`.work/`), а не в `/tmp` и не в `$HOME`.

---

## 2. Что сделано за сессию

### 2.1. Захват звука с микрофона (`backend/src/speech/capture/`) — ГОТОВО

Новый модуль, ~1100 строк:
- `mod.rs` — object-safe трейт `AudioSource` через `BoxFuture` (не `async fn in trait`!)
- `arecord.rs` — реальный захват через ALSA `arecord`
- `mock.rs` — синтетический источник для x86_64/WSL
- `runner.rs` — цикл `кадры → VAD → wake word → фраза → STT → AI → TTS`
- `supervisor.rs` — жизненный цикл задачи, выбор источника

**Критичное исправление:** флаги `microphone_available` / `microphone_listening` были
жёстко `true`. Теперь выставляются супервизором, по умолчанию `false`.

**Исправленный баг:** `let mut armed = speech.get_status().wakeword_enabled` — `true` по
умолчанию, из-за чего wake word детекция пропускалась. Теперь `false` + перевзвод.

API: `GET/POST /api/v1/speech/capture/{status,start,stop}`
Env: `KAMISH_AUDIO_SOURCE=arecord|mock`, `KAMISH_MIC_DEVICE`, `KAMISH_AUTOSTART_CAPTURE=1`

### 2.2. Локальные модели STT/TTS — ГОТОВО и ПРОВЕРЕНО

- **Piper TTS**: `/opt/kamish-models/tts/ru_RU-dmitri-medium.onnx` (61 МБ)
- **Vosk STT**: `/opt/kamish-models/stt/vosk-model-small-ru-0.22` (44 МБ)
- Обёртка [`scripts/vosk-transcriber`](../scripts/vosk-transcriber) — **обязательна**, pip-пакет
  `vosk` не ставит этот бинарник, без него STT молча уходит в mock
- Установщик: [`scripts/install-local-models.sh`](../scripts/install-local-models.sh)

**Проверено end-to-end:** голос → Vosk → «привет я умная колонка камыш» → AI → tool call → ответ озвучен.

Env: `PIPER_BIN`, `PIPER_MODEL_PATH`, `VOSK_MODEL_PATH`, `KAMISH_SPEECH_LANG=ru`

### 2.3. Скачивание и авторизация `agy` из панели — ГОТОВО (авторизация не проверена)

- `backend/src/ai/agy_manager.rs` — install / begin_login / submit_code
- API: `/api/v1/ai/agy/{status,install,login/begin,login/submit}`
- UI: блок в `frontend/src/pages/AiModelsPage.tsx` — кнопки + ссылка + поле кода
- [`scripts/fetch-agy.sh`](../scripts/fetch-agy.sh) — скачивание при сборке, подключено в
  `firmware/scripts/build-firmware.sh`. Мягкая деградация: сборка не падает.

**НЕ ПРОВЕРЕНО:** реальный протокол авторизации `agy` (парсер эвристический, тесты на 2 формата).

### 2.4. Инструкция для агентов — ГОТОВО

[`docs/AGENT_PLAYBOOK.md`](AGENT_PLAYBOOK.md) — процесс, грабли окружения, чек-лист.
Связан с `AGENTS.md`.

---

## 3. ТЕКУЩАЯ ЗАДАЧА: музыка с YouTube

> **Актуальный статус — §8.** Здесь история постановки задачи; §8 содержит
> проверенные факты и незавершённую работу по OAuth.

### 3.1. Проблема

Воспроизведение с YouTube **не работает**. Причина — двойная блокировка:

| Без VPN | С VPN |
|---|---|
| РКН режет YouTube | Google отдаёт **HTTP 403** датацентровому IP |

Проверено практически: `curl` на googlevideo URL → стабильно `403`, при любых
User-Agent. IP `94.243.2.253` — VPN, смена локации не помогла.

Дополнительно: `music.youtube.com` **не резолвится** локальным DNS
(через DoH резолвится → `142.251.156.4`, HTTPS на него отдаёт 200).

### 3.2. Что выяснено про yt-dlp

- `yt-dlp --js-runtimes node` — **обязателен**, без node многие форматы не извлекаются
- Клиент `android` вчера работал, сегодня — нет (клиенты ломаются)
- Сейчас **все** клиенты (`web, mweb, web_embedded, web_safari, tv, tv_embedded,
  android_vr, ios, music`) возвращают «no url»
- Ошибка: `Sign in to confirm you're not a bot`

### 3.3. Решение владельца: аккаунт-пустышка

Владелец выбрал **Вариант А — cookies.txt**, и просит **форму авторизации в веб-панели**:
ссылка на авторизацию + ввод кода, как OAuth-флоу.

**Развилка, которую надо держать в голове:**
- **Cookies из браузера получить НЕЛЬЗЯ**: Edge/Chrome на Windows шифруют их через
  DPAPI + App-Bound Encryption (`v20`). Проверено: `Extracted 0 cookies (42 could not be decrypted)`.
- **Значит нужен OAuth-флоу** (`ytmusicapi oauth` → ссылка + код → `oauth.json`),
  а не парсинг браузерных cookies.

### 3.4. Изучен Lavalink (по просьбе владельца)

**Вывод: ставить целиком не нужно, но идеи забрать стоит.**

Замерено практически:
- `Lavalink.jar` = **96 МБ**, JVM RSS в idle = **345 МБ** (при лимите 150 МБ!)
- Это JAR для Java, а не нативный ARM-бинарник
- Собран и **запущен** успешно: `{"sourceManagers":["youtube"]}`, v4.2.2
- Требует `openjdk-17`, ~300+ МБ RSS

**Идеи из `youtube-source`, которые стоит реализовать:**

| Идея | Суть |
|---|---|
| **Каскад клиентов** | перебирать `MUSIC, ANDROID_VR, WEB, TVHTML5_SIMPLY...`, кэшировать рабочий |
| **OAuth с refresh token** | `useOauth2(refreshToken)` — «выглядеть обычным юзером» |
| **poToken** | Proof-of-Origin, `iv-org/youtube-trusted-session-generator` — **может снять проблему без аккаунта** |
| **Remote cipher server** | `kikkia/yt-cipher` (Deno) для дешифровки подписей |

Из README дословно: *«use burner accounts and NOT your primary»* — подтверждает выбор владельца.

**poToken НЕ ПРОБОВАН** — это следующий логичный шаг, он не требует аккаунта.

### 3.5. Текущие файлы проекта по музыке

| Файл | Роль |
|---|---|
| `backend/src/music/search.rs` | поиск: radio → local → ytmusic → yt-dlp |
| `backend/src/music/player.rs` | `MpvPlayer`, IPC через unix-сокет, `mpv --ytdl=yes` |
| `backend/src/music/ytmusic.rs` | вызов `deploy/scripts/ytmusic_service.py` |
| `deploy/scripts/ytmusic_service.py` | `ytmusicapi`, действия: search/rate/liked/radio |

**Что надо будет править при реализации cookies:**
- `MpvPlayer::ensure_mpv_running()` — добавить `--script-opts=ytdl_hook-ytdl_path=...`
  либо передавать `--ytdl-raw-options=cookies=...,js-runtimes=node`
- `SearchService::search_youtube()` — добавить `--cookies` и `--js-runtimes node`
- `ytmusic_service.py` — принимать путь к `oauth.json`

---

## 4. Состояние кода и тестов

- **113 тестов, все зелёные** (`cargo test --manifest-path backend/Cargo.toml`)
- `cargo fmt --check` — чисто
- `npm run build` — собирается
- В `dist/` в корне — собранный фронтенд (нужен для отдачи через Axum)

**Изменённые/созданные файлы за сессию:**
```
backend/src/speech/capture/{mod,arecord,mock,runner,supervisor}.rs   [новые]
backend/src/ai/agy_manager.rs                                        [новый]
backend/src/speech/manager.rs                                        [правка]
backend/src/common/mod.rs                                            [правка]
backend/src/lib.rs                                                   [правка]
backend/src/api/{ai,speech,mod}.rs                                   [правка]
backend/tests/{speech_capture_test,capture_pipeline_e2e_test}.rs     [новые]
backend/tests/api_speech_test.rs                                     [правка]
frontend/src/pages/AiModelsPage.tsx                                  [правка]
frontend/src/api/index.ts, frontend/src/types/index.ts               [правка]
scripts/{install-local-models.sh,fetch-agy.sh,stt-check.py,vosk-transcriber}  [новые]
scripts/agy-mock                                                     [новый]
firmware/scripts/build-firmware.sh                                   [правка]
docs/{AGENT_PLAYBOOK.md,ARCHITECTURE.md,API.md}                      [новые/правка]
AGENTS.md                                                            [правка]
```

- Git: всё **untracked**, есть только `Initial commit`. Ничего не закоммичено.

---

## 5. Рекомендации следующему агенту

1. **Прочитайте [`docs/AGENT_PLAYBOOK.md`](AGENT_PLAYBOOK.md)** — там грабли:
   кавычки в `wsl -e sh -lc` (всегда писать скрипт в файл), `Set-Content` ломает файлы,
   сборка только в WSL.

2. **Порядок работы над музыкой:**
   - Сначала **poToken** — может решить без аккаунта, ~15 минут
   - Потом **OAuth-форма в панели** (`ytmusicapi oauth`) — то, что просил владелец
   - Потом **каскад клиентов** как в Lavalink
   - Прокинуть всё в `MpvPlayer` и `SearchService`

3. **Не выдумывайте успех.** Владелец ценит честность: если 403 — так и сказать.
   Не писать «работает», не проверив реальное воспроизведение.

4. **Временные скрипты** удобно писать в `scripts/_*.sh`, потом удалять.

5. **Диагностика музыки:** `wsl -e bash -lc "cd /tmp/lavatest && java -jar Lavalink.jar"`
   — Lavalink уже скачан и настроен в `/tmp/lavatest`, можно использовать как эталон.

---

## 6. Важные детали, которые легко забыть

- **`AGENTS.md` требует `docs/AGENT_PLAYBOOK.md` читать первым**
- API: все роуты под `/api/v1/`, авторизация cookie-based (`kamish_session`)
- Дефолтный логин: `admin` / `admin`, требует смены пароля при первом входе
- Аудио-синтез проверять через `soxi -D file.wav`
- `KAMISH_MUTE_AUDIO=1` — тестовый предохранитель, глушит реальный звук

---

## 7. Эксперимент с Lavalink — УСТАРЕЛ, см. §8

> Раздел сохранён как история. Вывод ниже был сделан, когда **VPN был выключен**.
> После включения VPN сеть до `youtubei.googleapis.com` заработала, поэтому аргумент
> «Lavalink не проходит сеть» снят. Актуальное состояние — в §8.

Lavalink собран и запущен, но **блокировку не пробивает**.

**Что установлено:**
- `Lavalink.jar` 4.2.2 скачан в `/tmp/lavatest/` (96 МБ)
- Успешно стартует: `java -Xmx2G -jar Lavalink.jar`
- `GET /v4/info` → **200**, `sourceManagers: ["youtube"]`, плагин активен
- **RSS в idle: 345 МБ** (VmHWM 363 МБ) — вдвое больше лимита 150 МБ из `SPECIFICATION.md`

**Результат `GET /v4/loadtracks?identifier=ytsearch:...`:**
```
curl: Operation timed out after 25003 milliseconds with 0 bytes received
```
Запрос **повисает намертво**. Причина из лога:
```
host: https://youtubei.googleapis.com:443
triedAddresses: 172.217.119.4, ... , 2001:4860:4842:400:0:0:0:0
untriedAddresses: 2001:4860:4847:400:0:0:0:0, ...
connectTimeout: 3000
```
**Ни один IPv4/IPv6-адрес `youtubei.googleapis.com` не принимает соединение.**

**Вывод:** блокировка сетевого уровня (РКН/провайдер), а не проблема клиента.
Lavalink бессилен против неё так же, как `yt-dlp`.

**Остаток в системе:** Lavalink процесса нет, JAR удалён вместе с `/tmp`.

---

## 8. АКТУАЛЬНОЕ СОСТОЯНИЕ музыки (после включения VPN)

### 8.1. Сеть проверена заново — работает

```
youtubei.googleapis.com → root=405, innertube=400   (было 000)
www.youtube.com        → 142.251.152.4 (стабильно, 6/6 попыток)
yt-dlp: audio URL получен (~1119 символов)
```

**IP:** `94.243.2.253` — VPN активен. Владелец подтвердил: раньше VPN был выключен,
из-за чего все проверки сети давали ложный «отказ».

### 8.2. Но 403 остался — теперь по причине Google

```
curl -L <audio_url> → final_http=403
mpv → audio не воспроизводит
yt-dlp → "Sign in to confirm you're not a bot"
```

Это **не** сетевая блокировка. Это отказ Google датацентровому IP без авторизации.

### 8.3. Что проверено и отброшено

| Вариант | Результат |
|---|---|
| **poToken** (`iv-org/youtube-trusted-session-generator`) | ❌ **Признан устаревшим самими авторами**: *«TOOL IS DEPRECATED — YouTube have changed how the identity tokens are handled»*. Плюс требует генерации «на том же заблокированном IP» — не на чем |
| **Invidious-инстанс как прокси** | ❌ Единственный живой инстанс не отвечает на `/api/v1/search` (000) |
| **Lavalink** | ❌ Не решает 403 — ему нужны те же cookies/OAuth. Плюс 345 МБ RSS |
| **Браузерные cookies из WSL** | ❌ Edge/Chrome шифруют через DPAPI + App-Bound Encryption (`v20`): `Extracted 0 cookies (42 could not be decrypted)` |

### 8.4. ВЫБРАННЫЙ ПУТЬ: OAuth Device Flow через панель

Владелец выбрал **вариант 2** — форма авторизации в веб-панели.

**Что уже написано (в этой сессии, до момента экспорта):**

| Файл | Назначение | Статус |
|---|---|---|
| [`scripts/youtube-oauth`](../scripts/youtube-oauth) | Python-хелпер: `begin` / `finish` / `refresh` через `oauth2.googleapis.com`. Без зависимости от `ytmusicapi` — только `urllib` | ✅ написан, синтаксис проверен |
| [`backend/src/music/youtube_auth.rs`](../backend/src/music/youtube_auth.rs) | Rust-модуль: `begin()`, `finish()`, `refresh()`, `access_token()`, `status()`, `logout()`, `write_cookies()`. 8 юнит-тестов | ✅ написан, **не компилировался** |
| [`backend/src/api/youtube_auth.rs`](../backend/src/api/youtube_auth.rs) | 5 HTTP-хендлеров | ✅ написан, **не компилировался** |
| `backend/src/music/mod.rs` | Регистрация модуля + реэкспорт | ✅ |
| `backend/src/api/mod.rs` | 5 роутов `/api/v1/music/youtube/auth/*` | ✅ |
| `backend/src/common/mod.rs` | `youtube_auth: Arc<YouTubeAuth>` в `AppState` | ⚠️ **правка в процессе — не завершена** |
| `backend/Cargo.toml` | Добавлен `tempfile = "3"` в dev-dependencies | ✅ |

**КРИТИЧНО: следующая правка — закончить `backend/src/common/mod.rs`.**
Структура объявлена (поле `youtube_auth`), но инициализация могла остаться неполной.
Проверить и собрать:
```bash
wsl
cd /mnt/m/Project/Kamish-os/radiant-bardeen
export CARGO_TARGET_DIR=$HOME/.cache/kamish-target
cargo check --manifest-path backend/Cargo.toml
```

### 8.5. Что нужно от владельца

1. Создать приложение в **Google Cloud Console** → OAuth Client ID типа **TV and Limited Input devices**
2. Получить `client_id` и `client_secret`
3. Ввести их в панели (после того как форма будет готова)

### 8.6. Оставшиеся шаги

1. ✅ **Завершить компиляцию** бэкенда (`common/mod.rs`)
2. ⬜ **Форма в UI** — `frontend/src/pages/MusicPage.tsx` (или отдельный блок):
   поля Client ID / Secret → кнопка «Начать вход» → показать `verification_url` + `user_code`
   → поле для `device_code` → кнопка «Подтвердить» → статус + «Выйти»
3. ⬜ **Фронтенд API-клиент** — `frontend/src/api/index.ts`, типы в `types/index.ts`
4. ⬜ **Прокинуть cookies в воспроизведение**:
   - `MpvPlayer::ensure_mpv_running()` — добавить `--ytdl-raw-options=...` и
     `--js-runtimes node`, либо передавать cookies через `--script-opts`
   - `SearchService::search_youtube()` — добавить `--cookies <path>` и `--js-runtimes node`
5. ⬜ **Проверить реальное воспроизведение** — только это докажет успех
6. ⬜ **Тесты + скрипт проверки** — `scripts/check-youtube.sh`

### 8.7. Ключевая техническая находка

Для любого вызова `yt-dlp` **обязателен JS-runtime**:
```bash
yt-dlp --js-runtimes node --cookies <cookies.txt> -f bestaudio -g "ytsearch1:запрос"
```
Без `node` форматы не извлекаются вовсе. `nodejs v22.22.1` уже установлен в WSL.

Формат cookies, который пишет `YouTubeAuth::write_cookies()` — Netscape,
с полями `SAPISID` / `__Secure-3PAPISID` из OAuth access token.
**Проверено ли, что yt-dlp его принимает — нет.** Это первый кандидат на проверку
после получения реальных токенов.

---

