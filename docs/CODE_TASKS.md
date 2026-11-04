# Список задач по изменению кодовой базы (CODE_TASKS.md)

В данном документе фиксируются отложенные задачи по доработке и рефакторингу исходного кода бэкенда, фронтенда и скриптов развёртывания.

---

## 📋 Очередь задач

### 1. Проверка и валидация функциональности точки доступа (AP)
* **Цель:** Обеспечить безотказную работу режима первичной настройки устройства без экрана.
* **Область:** `backend/src/network/manager.rs`, `backend/src/api/network.rs`, `deploy/scripts/`
* **Шаги реализации:**
  1. Проверить скрипт и вызовы `nmcli` для создания и поднятия точки доступа с SSID `kamish-setup` (WPA2 или open).
  2. Проверить корректность конфигурации встроенного DHCP-сервера (`dnsmasq` или NetworkManager shared mode) для выдачи IP-адресов клиентам в подсети `192.168.4.1/24`.
  3. Проверить сетевую маршрутизацию: доступность веб-интерфейса мастера настройки на `http://192.168.4.1:8080`.
  4. Проверить сценарий автоматического выключения точки доступа (`/api/v1/network/ap/stop`) после успешного сопряжения с домашней Wi-Fi сетью.
  5. Написать интеграционный тест на сценарий включения/отключения AP в `backend/tests/api_system_integration_test.rs`.

---

### 2. Отказ от локального движка рекомендаций музыки
* **Цель:** Исключить избыточный локальный скоринг треков; получать рекомендации исключительно на стороне YouTube Music.
* **Область:** `backend/src/music/recommendation.rs`, `backend/src/music/search.rs`, `deploy/scripts/ytmusic_service.py`
* **Шаги реализации:**
  1. [x] Вырезать из `backend/src/music/recommendation.rs` локальную формулу скоринга (`calculate_score` с весами +35/-25) — выполнено, треки отдаются напрямую как рекомендации YouTube.
  2. [ ] Перевести эндпоинт `GET /api/v1/music/recommendations` на проксирование запросов в YouTube Music API (через `ytmusicapi` / `deploy/scripts/ytmusic_service.py`).
  3. [ ] Обновить модульные тесты в `backend/src/music/repository.rs` и интеграционные тесты `backend/tests/api_music_test.rs`.

---

### 3. Переименование `smart-speaker` $\to$ `kamish` в кодовой базе и инфраструктуре
* **Цель:** Полное искоренение устаревшего наименования `SmartSpeaker` / `smart-speaker`.
* **Область:** `backend/`, `frontend/`, `deploy/`
* **Шаги реализации:**
  1. [x] **Systemd**: Юнит-файл `deploy/systemd/kamish.service` настроен под процесс `kamish`.
  2. [x] **Исполняемый файл и прошивка**: Установка бинарника `/usr/local/bin/kamish` выполняется внутри Buildroot-пакета (`firmware/buildroot-external-kamish`). Удалён устаревший `deploy/armbian`.
  3. [x] **MPV IPC сокет**: Путь сокета использует `kamish_core_mpv_*.sock` в `backend/src/music/player.rs`.
  4. [x] **SSID точки доступа**: В `backend/src/network/manager.rs` установлен SSID `Kamish-Setup`.
  5. [x] **Имя устройства**: В `backend/src/config/mod.rs` и дефолтных настройках `settings` выставлено `kamish`.
  6. [x] **UI и логи**: Заменены строковые упоминания `Smart Speaker` во фронтенде и логах бэкенда на `kamish`.

---

### 4. Интеграция музыкального сервера Lavalink v4 (ADR-010)
* **Цель:** Обеспечить воспроизведение из нескольких сервисов (YouTube Music, Яндекс Музыка, SoundCloud, Spotify) через единый API.
* **Область:** `backend/src/music/`, `firmware/buildroot-external-kamish/`
* **Шаги реализации:**
  1. [ ] Подключить WebSocket/REST клиент к Lavalink v4 в бэкенд на Rust.
  2. [ ] Настроить плагин LavaSrc с конфигурацией провайдеров.
  3. [ ] Сохранить легковесный WAV-синтезатор системных тонов и MPV для автономного воспроизведения без Java/сети.

---

### 5. Адаптация под Orange Pi 4 Pro (Allwinner A733) и устранение хардкода платы
* **Цель:** Устранить привязку к старой плате Orange Pi Zero 3 / Allwinner H618.
* **Область:** `frontend/src/pages/`, `backend/tests/`, `deploy/scripts/`
* **Шаги реализации:**
  1. [x] Во фронтенде (`DashboardPage.tsx`, `DiagnosticsPage.tsx`, `SystemPage.tsx`, `SettingsPreviewPage.tsx`) убрать статический fallback `'Orange Pi Zero 3'` и жестко зашитые строки о H618. Использовать динамические значения из `status?.device.hardware_model`.
  2. [x] В `backend/tests/hardware_telemetry_test.rs` обновить тестовые моки под чистую детекцию релизов.
  3. [x] В `deploy/scripts/hardware_check.sh` обновить целевые проверки процессора и платформы под Buildroot ARM64.
