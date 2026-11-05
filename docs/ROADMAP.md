# Дорожная карта разработки (ROADMAP.md)

В документе отражены этапы реализации автономной умной колонки **«Камыш» (Kamish)**, текущий статус компонентов, перечень выполненных задач и критерии готовности.

---

## 📊 Сводный статус этапов

| Этап | Описание | Статус | Тестовое покрытие |
|---|---|---|---|
| **Этап 1** | Базовый сервисный слой (Rust, Axum, Tokio, SQLx SQLite, REST API, Tracing) | ✅ **Завершён** | `api_status_test.rs`, `database/tests` |
| **Этап 2** | Scheduler & Алерты (Будильники, напоминания, временные зоны, генератор тонов) | ✅ **Завершён** | `api_alarms_test.rs`, `api_reminders_test.rs`, `scheduler_engine_test.rs` |
| **Этап 3** | SPA-интерфейс (React 19, TypeScript, Vite, Tailwind CSS, kill-ai-slop) | ✅ **Завершён** | `frontend_serving_test.rs`, UI билд `crates/kamish-web/frontend/dist` |
| **Этап 4** | AI Orchestrator & Tool Registry (Antigravity CLI, 14 типизированных инструментов) | ✅ **Завершён** | `api_ai_test.rs`, `ai/tools/tests` |
| **Этап 5** | Music Subsystem (MPV IPC, Radio, yt-dlp, рекомендации YouTube Music) | ✅ **Завершён** | `api_music_test.rs`, `music/repository/tests` |
| **Этап 6** | Speech Pipeline (VAD RMS/ZCR, Wakeword spotter, TTS генератор, STT диспатчер) | ✅ **Завершён** | `api_speech_test.rs`, `speech/vad/tests`, `speech/wakeword/tests` |
| **Этап 7** | System Integration (Bluetooth A2DP, Wi-Fi NetworkManager, Setup AP, VLESS Reality) | ✅ **Завершён** | `api_system_integration_test.rs` |
| **Этап 8** | Hardware & Telemetry (Kamish Linux, телеметрия процессора, XVF3800, GT-112 Safe Ceiling 85%) | ✅ **Завершён** | `hardware_telemetry_test.rs` |
| **Этап 9** | Auth & Setup Wizard (Argon2id, сессии, Setup Wizard, Light Minimalist UI) | ✅ **Завершён** | `api_auth_test.rs`, `api_setup_test.rs` |

---

## 🔍 Детализация этапов

### ✅ Этап 1: Базовый сервисный слой
- [x] Инициализация Rust-воркспейса (`crates/kamish-core`, `crates/kamish-web`, `crates/kamish-ipc`).
- [x] Асинхронный HTTP сервер на базе Axum 0.8 и Tokio.
- [x] Интеграция базы данных SQLite через SQLx c режимом WAL.
- [x] Единая встраиваемая схема базы данных (`database/schema.sql`, отслеживание через `PRAGMA user_version`).
- [x] Централизованное логирование и трассировка (`tracing`, `tracing-subscriber`).
- [x] Эндпоинты `/api/v1/status`, `/api/v1/system/info`.

### ✅ Этап 2: Scheduler и звуковые оповещения
- [x] Таблицы `alarms` и `reminders` с поддержкой повторяемости и временных зон.
- [x] Движок планировщика (`SchedulerEngine`) с периодическим опросом и отслеживанием пробуждения.
- [x] Встроенный автономный генератор WAV-тонов тревоги без внешних библиотек.
- [x] Поддержка действий `snooze` (отложить) и `dismiss` (отключить).
- [x] Автоматическое глушение звука в тестовом окружении (`IS_TEST_RUNNER`).

### ✅ Этап 3: SPA-интерфейс `kamish-web`
- [x] Стек: React 19, TypeScript, Vite, Tailwind CSS.
- [x] Полноценная адаптивная панель: статус, будильники, плеер, чат с ИИ, сети Wi-Fi, системная телеметрия.
- [x] Очистка интерфейса по правилам `kill-ai-slop` (минималистичный дизайн без фиолетовых градиентов и визуального мусора).
- [x] Встраивание раздачи статики через Axum (`crates/kamish-web/frontend/dist`) с fallback на `index.html`.

### ✅ Этап 4: AI Orchestrator & Tool Registry
- [x] Интеграция с Antigravity CLI через подпроцесс с передачей структурированного JSON.
- [x] Резервный офлайн-клиент `MockAiClient` для работы без интернета и ключей.
- [x] 14 строго типизированных инструментов для полного управления колонкой.
- [x] История сессий диалогов с сохранением контекста в SQLite (`ai_sessions`, `ai_messages`).
- [x] Прозрачный возврат ошибок выполнения инструментов в языковую модель.

### ✅ Этап 5: Музыкальная подсистема
- [x] Headless MPV проигрыватель с управлением через UNIX IPC сокет (`/tmp/mpv-kamish.sock`).
- [x] Автоматический фоллбэк на виртуальный Mock-драйвер в среде тестирования и разработки.
- [x] Поддержка поиска треков, интернет-радиостанций и локальных треков.
- [x] Делегирование рекомендаций музыки на сторону YouTube Music (без локального скоринга).
- [x] Сохранение истории воспроизведения в SQLite.

### ✅ Этап 6: Речевой пайплайн (Speech Pipeline)
- [x] Собственная реализация VAD (Voice Activity Detection) на чистом Rust (RMS + Zero Crossing Rate) с временем удержания (*hangover*).
- [x] Споттер ключевых слов («ассистент», «колонка», «слушай») с симулятором для тестов.
- [x] Модульная архитектура провайдеров STT (Vosk, Sherpa-onnx, Cloud) и TTS (Piper, RHVoice, Cloud).
- [x] Автономный синтезатор системных оповещений в WAV-поток.
- [x] Эндпоинты `/api/v1/speech/*` с поддержкой бинарного вывода WAV и JSON метаданных.

### ✅ Этап 7: Системная интеграция
- [x] Модуль Bluetooth A2DP Sink на базе BlueZ / BlueALSA: поиск, сопряжение, статус.
- [x] Управление сетью через NetworkManager (`nmcli`): сканирование Wi-Fi, безопасное подключение.
- [x] Режим точки доступа первичной настройки `kamish-setup` (`192.168.4.1`).
- [x] Клиент туннелирования Happ / VLESS Reality для обхода блокировок.
- [x] Централизованная WebSocket шина событий (`/api/v1/events`).
- [x] Маскирование всех паролей, ключей и секретов.

### ✅ Этап 8: Аппаратная адаптация и телеметрия
- [x] Динамическое определение модели платы и параметров системы.
- [x] Чтение температуры и частоты CPU через `/sys/class/thermal/` и `/sys/devices/system/cpu/`.
- [x] Мониторинг троттлинга процессора ($T \ge 80^\circ\text{C}$).
- [x] Аппаратный ограничитель громкости **Safe Volume Ceiling = 85%** для защиты колонок GT-112.
- [x] Сборка прошивки Kamish Linux через Buildroot (`firmware/buildroot-external-kamish`), unit-файлы `systemd`.
- [x] Диагностический скрипт аппаратных средств (`deploy/scripts/hardware_check.sh`).

### ✅ Этап 9: Безопасность, Auth и Мастер первоначальной настройки
- [x] Хэширование паролей Argon2id и управление сессиями (`auth_sessions`, куки `kamish_session`, TTL 7 суток).
- [x] Защитный middleware `auth_middleware` с обязательным требованием смены дефолтного пароля `admin` (код 403 `PASSWORD_CHANGE_REQUIRED`).
- [x] Пошаговый мастер настройки (Setup Wizard) с 5 этапами (`auth`, `network`, `audio`, `voice`, `ai`) и сохранением в `device_state`.
- [x] Светлый минималистичный веб-интерфейс (дизайн очищен по `kill-ai-slop`, тёмная тема строго запрещена, 9 основных разделов).

---

## 🚀 Ближайшие планы и улучшения (Бэклог)

1. **Проверка функциональности точки доступа (kamish-setup AP)**:
   - Проверка поднятия AP через `NetworkManager` (`nmcli`).
   - Проверка раздачи IP-адресов по DHCP клиентам в подсети `192.168.4.1/24`.
   - Проверка доступности веб-интерфейса мастера настройки на `http://192.168.4.1:8080`.
   - Проверка сценария автоматического выключения точки доступа после успешного подключения к домашнему Wi-Fi.
2. **Переход на Lavalink v4 с плагином LavaSrc (ADR-010)**:
   - Интеграция клиента Lavalink в `kamish-core` на Rust.
   - Подключение сервисов YouTube Music, Яндекс Музыка, SoundCloud, Spotify через единый протокол.
   - Маршрутизация запросов рекомендаций напрямую через Lavalink / YouTube Music.
3. **CI/CD & Кросс-компиляция**:
   - Настройка сборки бинарника под `aarch64-unknown-linux-gnu` через `cross` в GitHub Actions.
4. **Модели распознавания на устройстве**:
   - Подключение оптимизированной легковесной модели Sherpa-ONNX (int8) под NPU процессора.
5. **Firmware OTA**:
   - Безопасное обновление прошивки с проверкой цифровой подписи (RAUC) и двойным слотом отката (A/B partitioning).
