# Список задач по изменению кодовой базы (CODE_TASKS.md)

В данном документе фиксируются отложенные задачи по доработке и рефакторингу исходного кода сервисов (`kamish-core`, `kamish-web`), SPA-интерфейса и скриптов развёртывания.

---

## 📋 Очередь задач

### 1. Проверка и валидация функциональности точки доступа (AP)
* **Цель:** Обеспечить безотказную работу режима первичной настройки устройства без экрана.
* **Область:** `crates/kamish-core/src/network/manager.rs`, `crates/kamish-web/src/api/network.rs`, `deploy/scripts/`
* **Шаги реализации:**
  1. Проверить скрипт и вызовы `nmcli` для создания и поднятия точки доступа с SSID `kamish-setup` (WPA2 или open).
  2. Проверить корректность конфигурации встроенного DHCP-сервера (`dnsmasq` или NetworkManager shared mode) для выдачи IP-адресов клиентам в подсети `192.168.4.1/24`.
  3. Проверить сетевую маршрутизацию: доступность веб-интерфейса мастера настройки на `http://192.168.4.1:8080`.
  4. Проверить сценарий автоматического выключения точки доступа (`/api/v1/network/ap/stop`) после успешного сопряжения с домашней Wi-Fi сетью.
  5. Написать интеграционный тест на сценарий включения/отключения AP в `crates/kamish-web/tests/api_system_integration_test.rs`.

---

### 2. Отказ от локального движка рекомендаций музыки
* **Цель:** Исключить избыточный локальный скоринг треков; получать рекомендации исключительно на стороне YouTube Music.
* **Область:** `crates/kamish-core/src/music/recommendation.rs`, `crates/kamish-core/src/music/search.rs`, `deploy/scripts/ytmusic_service.py`
* **Шаги реализации:**
  1. [x] Вырезать из `crates/kamish-core/src/music/recommendation.rs` локальную формулу скоринга (`calculate_score` с весами +35/-25) — выполнено, треки отдаются напрямую как рекомендации YouTube.
  2. [ ] Перевести эндпоинт `GET /api/v1/music/recommendations` на проксирование запросов в YouTube Music API (через `ytmusicapi` / `deploy/scripts/ytmusic_service.py`).
  3. [ ] Обновить модульные тесты в `crates/kamish-core/src/music/repository.rs` и интеграционные тесты `crates/kamish-web/tests/api_music_test.rs`.

---

### 3. Приведение наименования продукта к `kamish` в кодовой базе и инфраструктуре
* **Цель:** Полное искоренение устаревших наименований продукта из кодовой базы и инфраструктуры; единственное каноническое имя — `Kamish OS` / `kamish`.
* **Область:** `crates/kamish-core/`, `crates/kamish-web/src/`, `crates/kamish-web/frontend/`, `deploy/`
* **Шаги реализации:**
  1. [x] **Systemd**: Юнит-файл `deploy/systemd/kamish.service` настроен под процесс `kamish`.
  2. [x] **Исполняемый файл и прошивка**: Установка бинарника `/usr/local/bin/kamish` выполняется внутри Buildroot-пакета (`firmware/buildroot-external-kamish`). Удалён устаревший `deploy/armbian`.
  3. [x] **MPV IPC сокет**: Путь сокета использует `kamish_core_mpv_*.sock` в `crates/kamish-core/src/music/player.rs`.
  4. [x] **SSID точки доступа**: В `crates/kamish-core/src/network/manager.rs` установлен SSID `Kamish-Setup`.
  5. [x] **Имя устройства**: В `crates/kamish-core/src/config/mod.rs` и дефолтных настройках `settings` выставлено `kamish`.
  6. [x] **UI и логи**: Устранены устаревшие строковые наименования продукта в SPA-интерфейсе и логах сервисов — используется единое имя `kamish`.

---

### 4. Интеграция музыкального сервера Lavalink v4 (ADR-010)
* **Цель:** Обеспечить воспроизведение из нескольких сервисов (YouTube Music, Яндекс Музыка, SoundCloud, Spotify) через единый API.
* **Область:** `crates/kamish-core/src/music/`, `firmware/buildroot-external-kamish/`
* **Шаги реализации:**
  1. [ ] Подключить WebSocket/REST клиент к Lavalink v4 в `kamish-core` на Rust.
  2. [ ] Настроить плагин LavaSrc с конфигурацией провайдеров.
  3. [ ] Сохранить легковесный WAV-синтезатор системных тонов и MPV для автономного воспроизведения без Java/сети.

---

### 5. Адаптация под Orange Pi 4 Pro (Allwinner A733) и устранение хардкода платы
* **Цель:** Устранить привязку к старой плате Orange Pi Zero 3 / Allwinner H618.
* **Область:** `crates/kamish-web/frontend/src/pages/`, `crates/kamish-core/tests/`, `deploy/scripts/`
* **Шаги реализации:**
  1. [x] В SPA-интерфейсе (`DashboardPage.tsx`, `DiagnosticsPage.tsx`, `SystemPage.tsx`, `SettingsPreviewPage.tsx`) убрать статический fallback `'Orange Pi Zero 3'` и жестко зашитые строки о H618. Использовать динамические значения из `status?.device.hardware_model`.
  2. [x] В `crates/kamish-core/tests/hardware_telemetry_test.rs` обновить тестовые моки под чистую детекцию релизов.
  3. [x] В `deploy/scripts/hardware_check.sh` обновить целевые проверки процессора и платформы под Buildroot ARM64.
