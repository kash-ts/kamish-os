# API Спецификация «Камыш» (Kamish)

Данный документ содержит полную спецификацию REST и WebSocket интерфейсов умной колонки «Камыш» (Kamish).

- **Базовый URL API**: `http://localhost:8080/api/v1`
- **Формат данных**: `application/json` (кодировка UTF-8)
- **Аутентификация**: Сессионная аутентификация на базе Argon2id. Все маршруты `/api/v1/*` защищены `auth_middleware` (токен передаётся через Cookie `kamish_session=<token>` или заголовок `Authorization: Bearer <token>`). Публичные маршруты: `GET /api/v1/status` и `POST /api/v1/auth/login`. Если дефолтный пароль не был изменён (`must_change_password: true`), возвращается код `403 Forbidden` с ошибкой `PASSWORD_CHANGE_REQUIRED`.
- **Обработка ошибок**: Все ошибки возвращаются в формате JSON:
  ```json
  {
    "error": "Описание возникшей ошибки",
    "code": "ERROR_CODE"
  }
  ```

---

## Содержание

1. [Общие соглашения и коды ответов](#общие-соглашения-и-коды-ответов)
2. [Аутентификация и сессии (Auth)](#1-аутентификация-и-сессии-auth)
   - [POST /api/v1/auth/login](#post-apiv1authlogin)
   - [POST /api/v1/auth/change-password](#post-apiv1authchange-password)
   - [GET /api/v1/auth/session](#get-apiv1authsession)
   - [POST /api/v1/auth/logout](#post-apiv1authlogout)
3. [Мастер первоначальной настройки (Setup Wizard)](#2-мастер-первоначальной-настройки-setup-wizard)
   - [GET /api/v1/setup/status](#get-apiv1setupstatus)
   - [POST /api/v1/setup/step](#post-apiv1setupstep)
   - [POST /api/v1/setup/complete](#post-apiv1setupcomplete)
   - [GET /api/v1/setup/diagnostics](#get-apiv1setupdiagnostics)
   - [POST /api/v1/setup/simulate-degradation](#post-apiv1setupsimulate-degradation)
4. [Системный статус и диагностика](#3-системный-статус-и-диагностика)
   - [GET /api/v1/status](#get-apiv1status)
5. [Будильники (Alarms)](#4-будильники-alarms)
   - [POST /api/v1/alarms](#post-apiv1alarms)
   - [GET /api/v1/alarms](#get-apiv1alarms)
   - [DELETE /api/v1/alarms/:id](#delete-apiv1alarmsid)
   - [POST /api/v1/alarms/:id/toggle](#post-apiv1alarmsidtoggle)
6. [Напоминания (Reminders)](#5-напоминания-reminders)
   - [POST /api/v1/reminders](#post-apiv1reminders)
   - [GET /api/v1/reminders](#get-apiv1reminders)
   - [DELETE /api/v1/reminders/:id](#delete-apiv1remindersid)
7. [AI-оркестратор и Tool Registry](#6-ai-оркестратор-и-tool-registry)
   - [POST /api/v1/ai/prompt](#post-apiv1aiprompt)
   - [GET /api/v1/ai/tools](#get-apiv1aitools)
   - [GET /api/v1/ai/sessions/:id](#get-apiv1aisessionsid)
8. [Музыкальный плеер (Music Manager)](#7-музыкальный-плеер-music-manager)
   - [GET /api/v1/music/search](#get-apiv1musicsearch)
   - [GET /api/v1/music/current](#get-apiv1musiccurrent)
   - [GET /api/v1/music/history](#get-apiv1musichistory)
   - [GET /api/v1/music/favorites](#get-apiv1musicfavorites)
   - [POST /api/v1/music/play](#post-apiv1musicplay)
   - [POST /api/v1/music/pause](#post-apiv1musicpause)
   - [POST /api/v1/music/resume](#post-apiv1musicresume)
   - [POST /api/v1/music/stop](#post-apiv1musicstop)
   - [POST /api/v1/music/like](#post-apiv1musiclike)
   - [POST /api/v1/music/skip](#post-apiv1musicskip)
   - [GET /api/v1/music/recommendations](#get-apiv1musicrecommendations)
   - [GET /api/v1/music/radio](#get-apiv1musicradio)
   - [GET /api/v1/music/lavalink/status](#get-apiv1musiclavalinkstatus)
   - [POST /api/v1/music/lavalink/config](#post-apiv1musiclavalinkconfig)
   - [GET /api/v1/music/youtube/auth/status](#get-apiv1musicyoutubeauthstatus)
   - [POST /api/v1/music/youtube/auth/begin](#post-apiv1musicyoutubeauthbegin)
   - [POST /api/v1/music/youtube/auth/finish](#post-apiv1musicyoutubeauthfinish)
   - [POST /api/v1/music/youtube/auth/logout](#post-apiv1musicyoutubeauthlogout)
9. [Настройки устройства (Settings)](#8-настройки-устройства-settings)
   - [GET /api/v1/settings](#get-apiv1settings)
   - [PUT /api/v1/settings](#put-apiv1settings)
10. [Bluetooth A2DP Sink](#9-bluetooth-a2dp-sink)
    - [GET /api/v1/bluetooth/status](#get-apiv1bluetoothstatus)
    - [POST /api/v1/bluetooth/scan](#post-apiv1bluetoothscan)
    - [POST /api/v1/bluetooth/pair](#post-apiv1bluetoothpair)
    - [POST /api/v1/bluetooth/connect](#post-apiv1bluetoothconnect)
    - [POST /api/v1/bluetooth/disconnect](#post-apiv1bluetoothdisconnect)
11. [Сетевой менеджер и Wi-Fi](#10-сетевой-менеджер-и-wi-fi)
    - [GET /api/v1/network/status](#get-apiv1networkstatus)
    - [GET /api/v1/network/wifi/scan](#get-apiv1networkwifiscan)
    - [POST /api/v1/network/wifi/connect](#post-apiv1networkwificonnect)
    - [POST /api/v1/network/wifi/disconnect](#post-apiv1networkwifidisconnect)
    - [POST /api/v1/network/ap/start](#post-apiv1networkapstart)
    - [POST /api/v1/network/ap/stop](#post-apiv1networkapstop)
12. [VPN / VLESS туннелирование](#11-vpn--vless-туннелирование)
    - [GET /api/v1/vpn/status](#get-apiv1vpnstatus)
    - [POST /api/v1/vpn/connect](#post-apiv1vpnconnect)
    - [POST /api/v1/vpn/disconnect](#post-apiv1vpndisconnect)
    - [POST /api/v1/vpn/config](#post-apiv1vpnconfig)
13. [WebSocket Real-Time Events](#12-websocket-real-time-events)
    - [WS /api/v1/events](#ws-apiv1events)

---

## 📡 Сводка основных API эндпоинтов

| Метод | Эндпоинт | Описание |
|---|---|---|
| `GET` | `/api/v1/status` | Системный статус, аптайм, телеметрия и здоровье подсистем |
| `POST`| `/api/v1/auth/login` | Аутентификация администратора (Argon2id) |
| `POST`| `/api/v1/auth/change-password` | Смена пароля администратора |
| `GET` | `/api/v1/setup/status` | Статус и прогресс мастера первоначальной настройки |
| `GET` | `/api/v1/system/hardware` | Детальная аппаратная телеметрия SoC и USB |
| `POST`| `/api/v1/system/audio/volume` | Установка громкости с лимитером Safe Ceiling (85%) |
| `POST`| `/api/v1/system/audio/route` | Переключение аудиовыхода (GT-112 / BT / Jack) |
| `GET`/`POST` | `/api/v1/alarms` | Список и создание будильников (разовые, дни недели, зоны) |
| `GET`/`POST` | `/api/v1/reminders` | Список и создание напоминаний |
| `POST`| `/api/v1/ai/prompt` | Запрос к ИИ с выполнением зарегистрированных инструментов |
| `GET` | `/api/v1/ai/tools` | Список всех 14 зарегистрированных схем инструментов |
| `GET`/`POST` | `/api/v1/music/*` | Поиск, радио, текущий трек, лайк, пропуск, рекомендации |
| `GET`/`POST` | `/api/v1/speech/*` | Синтез речи (TTS), распознавание (STT), захват звука |
| `GET`/`POST` | `/api/v1/bluetooth/*` | Статус адаптера, поиск, сопряжение и подключение A2DP |
| `GET`/`POST` | `/api/v1/network/*` | Статус сети, сканирование Wi-Fi, подключение, Setup AP |
| `GET`/`POST` | `/api/v1/vpn/*` | Статус туннеля, статистика трафика, подключение, VLESS URI |
| `GET` | `/api/v1/events` | **WebSocket** шина событий реального времени |

---

## Общие соглашения и коды ответов

| HTTP Код | Название | Значение в контексте API |
|---|---|---|
| `200 OK` | Успешно | Запрос выполнен успешно, тело ответа содержит запрошенные данные или подтверждение действия. |
| `201 Created` | Создано | Ресурс (будильник, напоминание) успешно создан. |
| `400 Bad Request` | Неверный запрос | Ошибка валидации входных параметров (неверный формат времени, недопустимое значение). |
| `401 Unauthorized` | Не авторизован | Отсутствует, поврежден или просрочен токен сессии (`Missing session token`). |
| `403 Forbidden` | Запрещено | Требуется обязательная смена дефолтного пароля администратора (`PASSWORD_CHANGE_REQUIRED`). |
| `404 Not Found` | Не найдено | Запрашиваемый ресурс (ID будильника, сессии) не существует или эндпоинт отсутствует. |
| `500 Internal Error` | Ошибка сервера | Необработанная ошибка базы данных, файловой системы или системного вызова. |

Стандартный формат ошибки:
```json
{
  "error": "Time must be in HH:MM format",
  "code": "CONFIG_ERROR"
}
```

---

## 1. Аутентификация и сессии (Auth)

Подсистема аутентификации использует стойкое хэширование паролей Argon2id и управляет жизненным циклом сессий пользователей в таблицах `users` и `auth_sessions`.
Маршруты защищены `auth_middleware`. Если у пользователя активен флаг `must_change_password`, вызовы защищённых эндпоинтов блокируются ошибкой 403 `PASSWORD_CHANGE_REQUIRED` до момента смены пароля.

### `POST /api/v1/auth/login`

Аутентификация пользователя по логину и паролю. При успехе возвращает токен сессии и выставляет HTTP-Only Cookie `kamish_session`.

#### Заголовки
- `Content-Type: application/json`
- `Accept: application/json`

#### Тело запроса
```json
{
  "username": "admin",
  "password": "admin"
}
```

#### Пример ответа (`200 OK`)
Заголовки ответа включают `Set-Cookie: kamish_session=<token>; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800`.
```json
{
  "token": "4f5e8a71-6b2c-49a3-8d01-e97c413b52f1",
  "user": {
    "id": "1",
    "username": "admin",
    "must_change_password": true
  },
  "expires_at": "2026-11-11T12:00:00Z"
}
```

#### Ошибки
- `401 Unauthorized` (`INVALID_CREDENTIALS`): Неверное имя пользователя или пароль.

---

### `POST /api/v1/auth/change-password`

Смена пароля текущего пользователя. Снимает требование `must_change_password`, отзывает все предыдущие сессии пользователя и обновляет статус этапа `auth` в мастере первоначальной настройки.

#### Заголовки
- `Authorization: Bearer <token>` или Cookie `kamish_session=<token>`
- `Content-Type: application/json`

#### Тело запроса
```json
{
  "old_password": "admin",
  "new_password": "NewSecurePassword123!"
}
```

#### Пример ответа (`200 OK`)
```json
{
  "status": "ok",
  "message": "Password changed successfully",
  "must_change_password": false
}
```

#### Ошибки
- `400 Bad Request` (`PASSWORD_POLICY_VIOLATION`): Пароль не удовлетворяет требованиям сложности.
- `401 Unauthorized` (`UNAUTHORIZED`): Неверный текущий пароль или отсутствует сессия.

---

### `GET /api/v1/auth/session`

Проверка валидности текущей сессии и получение данных профиля пользователя.

#### Заголовки
- `Authorization: Bearer <token>` или Cookie `kamish_session=<token>`

#### Пример ответа (`200 OK`)
```json
{
  "authenticated": true,
  "user": {
    "id": "1",
    "username": "admin",
    "must_change_password": false
  },
  "expires_at": "2026-11-11T12:00:00Z"
}
```

---

### `POST /api/v1/auth/logout`

Завершение сессии. Удаляет запись сессии из базы данных и очищает Cookie `kamish_session`.

#### Заголовки
- `Authorization: Bearer <token>` или Cookie `kamish_session=<token>`

#### Пример ответа (`200 OK`)
```json
{
  "status": "ok",
  "message": "Logged out successfully"
}
```

---

## 2. Мастер первоначальной настройки (Setup Wizard)

Мастер первоначальной настройки координирует пошаговый ввод устройства в эксплуатацию по обязательным этапам: `network` (подключение Wi-Fi), `audio` (калибровка громкости с Safe Ceiling $\le 85\%$), `voice` (проверка микрофона и локального STT/TTS) и `ai` (проверка готовности Antigravity CLI). Прогресс сохраняется в таблице `device_state`.

### `GET /api/v1/setup/status`

Возвращает текущий прогресс настройки, статус завершенности, состояние обязательных этапов и список деградированных компонентов.

#### Заголовки
- `Authorization: Bearer <token>` или Cookie `kamish_session=<token>`

#### Пример ответа (`200 OK`)
```json
{
  "is_completed": false,
  "current_step": "network",
  "completed_at": null,
  "steps": [
    {
      "id": "network",
      "title": "Сетевое подключение",
      "status": "completed",
      "is_mandatory": true,
      "health": {
        "healthy": true,
        "message": "Подключено к Wi-Fi"
      },
      "data": {
        "ssid": "HomeNetwork_5G"
      },
      "updated_at": "2026-11-04T10:15:00Z"
    },
    {
      "id": "audio",
      "title": "Аудиосистема",
      "status": "in_progress",
      "is_mandatory": true,
      "health": {
        "healthy": true,
        "message": "Акустика готова, лимитер 85%"
      },
      "data": {
        "volume": 60,
        "safe_ceiling": 85
      },
      "updated_at": null
    },
    {
      "id": "voice",
      "title": "Голосовой конвейер",
      "status": "pending",
      "is_mandatory": true,
      "health": {
        "healthy": true,
        "message": "Vosk STT и Piper TTS обнаружены"
      },
      "data": null,
      "updated_at": null
    },
    {
      "id": "ai",
      "title": "Интеграция с ИИ",
      "status": "pending",
      "is_mandatory": true,
      "health": {
        "healthy": true,
        "message": "Antigravity CLI готов"
      },
      "data": null,
      "updated_at": null
    }
  ],
  "degraded_components": []
}
```

---

### `POST /api/v1/setup/step`

Сохранение параметров и статуса прохождения этапа мастера настройки. При обновлении шага `audio` автоматически применяется лимитер громкости (Safe Volume Ceiling $\le 85\%$). При успехе эмитит событие WebSocket `setup_step_updated`.

#### Заголовки
- `Authorization: Bearer <token>` или Cookie `kamish_session=<token>`
- `Content-Type: application/json`

#### Тело запроса
```json
{
  "step": "audio",
  "status": "completed",
  "data": {
    "volume": 70,
    "safe_ceiling": 85
  }
}
```

#### Пример ответа (`200 OK`)
```json
{
  "success": true,
  "step": "audio",
  "status": "completed",
  "next_step": "voice",
  "is_completed": false,
  "updated_at": "2026-11-04T10:20:00Z"
}
```

---

### `POST /api/v1/setup/complete`

Финализация мастера настройки. Проверяет завершенность всех обязательных этапов (`network`, `audio`, `voice`, `ai`), фиксирует `is_completed: true` в `device_state` и эмитит событие WebSocket `setup_completed`.

#### Заголовки
- `Authorization: Bearer <token>` или Cookie `kamish_session=<token>`

#### Пример ответа (`200 OK`)
```json
{
  "success": true,
  "is_completed": true,
  "completed_at": "2026-11-04T10:25:00Z",
  "message": "Setup wizard successfully completed. Device is ready for normal operation."
}
```

#### Ошибки
- `400 Bad Request` (`VALIDATION_ERROR`): Не все обязательные этапы завершены (например, `Cannot complete setup: stages [voice, ai] are not completed`).

---

### `GET /api/v1/setup/diagnostics`

Глубокая диагностика всех подсистем устройства для экрана восстановления после сбоев.

#### Заголовки
- `Authorization: Bearer <token>` или Cookie `kamish_session=<token>`

#### Пример ответа (`200 OK`)
```json
{
  "healthy": true,
  "timestamp": "2026-11-04T10:30:00Z",
  "steps": [],
  "degraded_components": [],
  "subsystems": {
    "network": {
      "interface": "wlan0",
      "ip": "192.168.1.120",
      "wifi_connected": true,
      "ethernet_connected": false,
      "ap_active": false
    },
    "audio": {
      "output_device": "GT-112 Line Out",
      "safe_ceiling": 85,
      "volume": 60,
      "mixer_status": "active"
    },
    "voice": {
      "microphone_available": true,
      "active_stt": "vosk",
      "active_tts": "piper"
    },
    "ai": {
      "ready": true,
      "mode": "cli",
      "version": "1.2.0"
    }
  }
}
```

---

### `POST /api/v1/setup/simulate-degradation`

Тестовая эмуляция сбоя подсистемы (для проверки UI и механизмов аварийного восстановления).

#### Заголовки
- `Authorization: Bearer <token>` или Cookie `kamish_session=<token>`
- `Content-Type: application/json`

#### Тело запроса
```json
{
  "component": "voice",
  "degraded": true,
  "error_reason": "Microphone array disconnected"
}
```

#### Пример ответа (`200 OK`)
```json
{
  "status": "ok",
  "component": "voice",
  "degraded": true
}
```

---

## 3. Системный статус и диагностика

### `GET /api/v1/status`

Возвращает комплексную диагностическую информацию об узле, системных ресурсах устройства, подключении к базе данных SQLite, состоянии подсистем и шедулера.

#### Заголовки
- `Accept: application/json`

#### Параметры запроса
Отсутствуют.

#### Пример ответа (`200 OK`)
```json
{
  "status": "ok",
  "version": "0.1.0",
  "app_uptime_secs": 3612,
  "device": {
    "hostname": "kamish",
    "os_name": "Linux",
    "os_version": "Debian GNU/Linux 12 (bookworm)",
    "kernel_version": "6.1.31-sun50iw9",
    "arch": "aarch64",
    "cpu_count": 4,
    "memory_total_bytes": 4153344000,
    "memory_used_bytes": 435128320,
    "system_uptime_secs": 84205
  },
  "database": {
    "connected": true,
    "applied_migrations": 1
  },
  "subsystems": {
    "audio": "ready",
    "bluetooth": "idle",
    "network": "connected",
    "vpn": "disconnected",
    "speech": "ready",
    "ai": "idle",
    "scheduler": "ready",
    "music": "stopped"
  },
  "scheduler": {
    "active_alarms_count": 2,
    "pending_reminders_count": 1,
    "running": true
  }
}
```

#### Возможные ошибки
- `500 Internal Server Error`: `{"error": "Failed to collect system metrics", "code": "SYSTEM_ERROR"}`

#### Пример cURL
```bash
curl -s -X GET http://localhost:8080/api/v1/status \
  -H "Accept: application/json"
```

---

## 2. Будильники (Alarms)

Подсистема будильников работает автономно и offline-first. Воспроизведение звуковых сигналов не зависит от доступа в Интернет.

### `POST /api/v1/alarms`

Создает новый будильник. Поддерживает однократные будильники и правила повторения по дням недели (`daily`, `weekdays`, `weekends` или `mon,wed,fri`).

#### Заголовки
- `Content-Type: application/json`
- `Accept: application/json`

#### Схема тела запроса (JSON Schema)
```json
{
  "type": "object",
  "properties": {
    "time": {
      "type": "string",
      "description": "Время срабатывания в формате 24ч HH:MM (например, '07:30')",
      "pattern": "^([0-1]?[0-9]|2[0-3]):[0-5][0-9]$"
    },
    "recurrence": {
      "type": ["string", "null"],
      "description": "Паттерн повторения: 'daily', 'weekdays', 'weekends', список через запятую ('mon,tue,wed') либо null для однократного",
      "example": "mon,tue,wed,thu,fri"
    },
    "label": {
      "type": ["string", "null"],
      "description": "Пользовательская текстовая метка",
      "example": "Подъем на работу"
    },
    "timezone": {
      "type": ["string", "null"],
      "description": "IANA часовой пояс (по умолчанию 'UTC')",
      "default": "UTC",
      "example": "Europe/Moscow"
    },
    "enabled": {
      "type": ["boolean", "null"],
      "description": "Начальное состояние активности (по умолчанию true)",
      "default": true
    }
  },
  "required": ["time"]
}
```

#### Пример тела запроса
```json
{
  "time": "07:30",
  "recurrence": "weekdays",
  "label": "Подъем на работу",
  "timezone": "Europe/Moscow",
  "enabled": true
}
```

#### Пример ответа (`201 Created`)
```json
{
  "id": "a9018f21-723c-41c1-b0db-6e115fa763b1",
  "time": "07:30",
  "timezone": "Europe/Moscow",
  "recurrence": "weekdays",
  "label": "Подъем на работу",
  "enabled": true,
  "created_at": "2026-11-01 16:30:00",
  "updated_at": "2026-11-01 16:30:00"
}
```

#### Возможные ошибки
- `400 Bad Request`:
  ```json
  {
    "error": "Invalid time format '25:99'. Expected HH:MM (00:00 - 23:59)",
    "code": "CONFIG_ERROR"
  }
  ```
- `400 Bad Request`:
  ```json
  {
    "error": "Unknown recurrence keyword 'every_other_day'",
    "code": "CONFIG_ERROR"
  }
  ```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/alarms \
  -H "Content-Type: application/json" \
  -d '{
    "time": "07:30",
    "recurrence": "weekdays",
    "label": "Подъем на работу",
    "timezone": "Europe/Moscow"
  }'
```

---

### `GET /api/v1/alarms`

Возвращает список всех зарегистрированных будильников.

#### Заголовки
- `Accept: application/json`

#### Параметры запроса
Отсутствуют.

#### Пример ответа (`200 OK`)
```json
[
  {
    "id": "a9018f21-723c-41c1-b0db-6e115fa763b1",
    "time": "07:30",
    "timezone": "Europe/Moscow",
    "recurrence": "weekdays",
    "label": "Подъем на работу",
    "enabled": true,
    "created_at": "2026-11-01 16:30:00",
    "updated_at": "2026-11-01 16:30:00"
  },
  {
    "id": "c138db99-a931-419b-a3d8-5541e2474f19",
    "time": "09:00",
    "timezone": "Europe/Moscow",
    "recurrence": "weekends",
    "label": "Выходные",
    "enabled": false,
    "created_at": "2026-11-01 16:32:00",
    "updated_at": "2026-11-01 16:35:00"
  }
]
```

#### Пример cURL
```bash
curl -s -X GET http://localhost:8080/api/v1/alarms \
  -H "Accept: application/json"
```

---

### `DELETE /api/v1/alarms/:id`

Удаляет будильник по его уникальному идентификатору `id`.

#### Заголовки
- `Accept: application/json`

#### Параметры пути
| Параметр | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` (UUID) | Да | Идентификатор удаляемого будильника |

#### Пример ответа (`200 OK`)
```json
{
  "status": "deleted",
  "id": "a9018f21-723c-41c1-b0db-6e115fa763b1"
}
```

#### Возможные ошибки
- `404 Not Found`:
  ```json
  {
    "error": "Alarm with id 'non-existent-id' not found",
    "code": "NOT_FOUND"
  }
  ```

#### Пример cURL
```bash
curl -s -X DELETE http://localhost:8080/api/v1/alarms/a9018f21-723c-41c1-b0db-6e115fa763b1 \
  -H "Accept: application/json"
```

---

### `POST /api/v1/alarms/:id/toggle`

Переключает состояние активности будильника (включен / выключен). Если тело запроса пустое, состояние инвертируется на противоположное. Если передано поле `enabled`, состояние выставляется явно.

#### Заголовки
- `Content-Type: application/json`
- `Accept: application/json`

#### Параметры пути
| Параметр | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` (UUID) | Да | Идентификатор будильника |

#### Схема тела запроса (опционально)
```json
{
  "type": "object",
  "properties": {
    "enabled": {
      "type": "boolean",
      "description": "Явное задание состояния (true / false). Если отсутствует — инвертирует текущее."
    }
  }
}
```

#### Пример тела запроса
```json
{
  "enabled": false
}
```

#### Пример ответа (`200 OK`)
```json
{
  "id": "a9018f21-723c-41c1-b0db-6e115fa763b1",
  "time": "07:30",
  "timezone": "Europe/Moscow",
  "recurrence": "weekdays",
  "label": "Подъем на работу",
  "enabled": false,
  "created_at": "2026-11-01 16:30:00",
  "updated_at": "2026-11-01 16:40:00"
}
```

#### Возможные ошибки
- `404 Not Found`:
  ```json
  {
    "error": "Alarm with id '00000000-0000-0000-0000-000000000000' not found",
    "code": "NOT_FOUND"
  }
  ```

#### Пример cURL
```bash
# Инвертировать состояние:
curl -s -X POST http://localhost:8080/api/v1/alarms/a9018f21-723c-41c1-b0db-6e115fa763b1/toggle

# Установить конкретное состояние:
curl -s -X POST http://localhost:8080/api/v1/alarms/a9018f21-723c-41c1-b0db-6e115fa763b1/toggle \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'
```

---

## 3. Напоминания (Reminders)

Напоминания привязаны к конкретному моменту времени (ISO-8601) и озвучиваются голосом при наступлении времени срабатывания.

### `POST /api/v1/reminders`

Создает новое напоминание с указанием точного времени и текста.

#### Заголовки
- `Content-Type: application/json`
- `Accept: application/json`

#### Схема тела запроса (JSON Schema)
```json
{
  "type": "object",
  "properties": {
    "trigger_time": {
      "type": "string",
      "description": "Время срабатывания в формате ISO-8601 (например, '2026-11-02T10:00:00Z')",
      "example": "2026-11-02T15:00:00Z"
    },
    "text": {
      "type": "string",
      "description": "Текст напоминания (не может быть пустым)",
      "example": "Полить цветы и выключить духовку"
    },
    "timezone": {
      "type": ["string", "null"],
      "description": "Часовой пояс (по умолчанию 'UTC')",
      "default": "UTC",
      "example": "Europe/Moscow"
    }
  },
  "required": ["trigger_time", "text"]
}
```

#### Пример тела запроса
```json
{
  "trigger_time": "2026-11-02T15:00:00Z",
  "text": "Полить цветы и выключить духовку",
  "timezone": "Europe/Moscow"
}
```

#### Пример ответа (`201 Created`)
```json
{
  "id": "e44d852a-39b1-4d37-88eb-12a865b4c194",
  "trigger_time": "2026-11-02T15:00:00Z",
  "text": "Полить цветы и выключить духовку",
  "timezone": "Europe/Moscow",
  "status": "pending",
  "created_at": "2026-11-01 16:45:00"
}
```

#### Возможные ошибки
- `400 Bad Request`:
  ```json
  {
    "error": "Reminder text cannot be empty",
    "code": "CONFIG_ERROR"
  }
  ```
- `400 Bad Request`:
  ```json
  {
    "error": "Invalid trigger_time timestamp format. Use ISO-8601 (e.g. 2026-11-02T15:00:00Z)",
    "code": "CONFIG_ERROR"
  }
  ```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/reminders \
  -H "Content-Type: application/json" \
  -d '{
    "trigger_time": "2026-11-02T15:00:00Z",
    "text": "Полить цветы",
    "timezone": "Europe/Moscow"
  }'
```

---

### `GET /api/v1/reminders`

Возвращает список напоминаний с возможностью фильтрации по статусу (`pending`, `completed`, `cancelled`).

#### Заголовки
- `Accept: application/json`

#### Параметры строки запроса (Query Parameters)
| Параметр | Тип | Обязательный | По умолчанию | Описание |
|---|---|---|---|---|
| `status` | `string` | Нет | (все) | Фильтр статуса: `pending`, `completed`, `cancelled` |

#### Пример ответа (`200 OK`)
```json
[
  {
    "id": "e44d852a-39b1-4d37-88eb-12a865b4c194",
    "trigger_time": "2026-11-02T15:00:00Z",
    "text": "Полить цветы и выключить духовку",
    "timezone": "Europe/Moscow",
    "status": "pending",
    "created_at": "2026-11-01 16:45:00"
  }
]
```

#### Пример cURL
```bash
# Получить все напоминания
curl -s -X GET http://localhost:8080/api/v1/reminders

# Получить только активные
curl -s -X GET "http://localhost:8080/api/v1/reminders?status=pending"
```

---

### `DELETE /api/v1/reminders/:id`

Удаляет напоминание по его `id`.

#### Заголовки
- `Accept: application/json`

#### Параметры пути
| Параметр | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` (UUID) | Да | Идентификатор напоминания |

#### Пример ответа (`200 OK`)
```json
{
  "status": "deleted",
  "id": "e44d852a-39b1-4d37-88eb-12a865b4c194"
}
```

#### Возможные ошибки
- `404 Not Found`:
  ```json
  {
    "error": "Reminder with id 'e44d852a-0000-0000-0000-000000000000' not found",
    "code": "NOT_FOUND"
  }
  ```

#### Пример cURL
```bash
curl -s -X DELETE http://localhost:8080/api/v1/reminders/e44d852a-39b1-4d37-88eb-12a865b4c194
```

---

## 4. AI-оркестратор и Tool Registry

AI-оркестратор исполняет естественно-языковые команды пользователя через вызовы типизированных инструментов (**Tool Registry**) без прямого доступа языковой модели к оболочке ОС.

### `POST /api/v1/ai/prompt`

Передает текстовую команду в оркестратор (Antigravity CLI с fallback-движком), выполняет сопоставленные системные инструменты и возвращает ответ ассистента с историей выполненных действий.

#### Заголовки
- `Content-Type: application/json`
- `Accept: application/json`

#### Схема тела запроса (JSON Schema)
```json
{
  "type": "object",
  "properties": {
    "prompt": {
      "type": "string",
      "description": "Текстовая команда пользователя на естественном языке",
      "example": "Поставь будильник на 8 утра и включи радио Рекорд"
    },
    "session_id": {
      "type": ["string", "null"],
      "description": "Идентификатор существующей диалоговой сессии (если null, создается новая)",
      "example": "session-550e8400-e29b-41d4-a716-446655440000"
    }
  },
  "required": ["prompt"]
}
```

#### Пример тела запроса
```json
{
  "prompt": "Поставь будильник на 07:30 на будни с названием Работа",
  "session_id": "sess-default-01"
}
```

#### Пример ответа (`200 OK`)
```json
{
  "response": "Будильник установлен на 07:30 (будни) с меткой 'Работа'.",
  "tool_calls": [
    {
      "tool": "create_alarm",
      "arguments": {
        "time": "07:30",
        "recurrence": "weekdays",
        "label": "Работа",
        "timezone": "UTC"
      },
      "result": {
        "id": "f51b63ef-634a-4b08-8eec-45811c750422",
        "time": "07:30",
        "recurrence": "weekdays",
        "label": "Работа",
        "enabled": true
      },
      "success": true
    }
  ],
  "session_id": "sess-default-01"
}
```

#### Возможные ошибки
- `400 Bad Request`:
  ```json
  {
    "error": "Prompt cannot be empty",
    "code": "CONFIG_ERROR"
  }
  ```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/ai/prompt \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "Какая громкость?",
    "session_id": "user-sess-1"
  }'
```

---

### `GET /api/v1/ai/tools`

Возвращает список всех зарегистрированных в системе инструментов (**Tool Registry**) вместе с их JSON Schema описанием, используемым моделью для вызова функций.

#### Заголовки
- `Accept: application/json`

#### Параметры запроса
Отсутствуют.

#### Пример ответа (`200 OK`)
```json
{
  "count": 14,
  "tools": [
    {
      "name": "create_alarm",
      "description": "Create a new alarm with time (HH:MM), optional recurrence pattern, label, and timezone.",
      "parameters": {
        "type": "object",
        "properties": {
          "time": { "type": "string", "description": "Alarm time in 24-hour HH:MM format" },
          "recurrence": { "type": "string", "description": "daily, weekdays, weekends, or comma-separated days" },
          "label": { "type": "string", "description": "Alarm label" },
          "timezone": { "type": "string", "description": "Timezone name" }
        },
        "required": ["time"]
      }
    },
    {
      "name": "delete_alarm",
      "description": "Delete an alarm by its ID or by matching time/label.",
      "parameters": {
        "type": "object",
        "properties": {
          "id": { "type": "string" },
          "time": { "type": "string" },
          "label": { "type": "string" }
        }
      }
    },
    {
      "name": "list_alarms",
      "description": "List all configured alarms with their statuses and schedules.",
      "parameters": { "type": "object", "properties": {} }
    },
    {
      "name": "snooze_alarm",
      "description": "Snooze currently ringing alarm by a given number of minutes.",
      "parameters": {
        "type": "object",
        "properties": {
          "minutes": { "type": "integer", "default": 10 }
        }
      }
    },
    {
      "name": "create_reminder",
      "description": "Create a reminder with trigger time and text message.",
      "parameters": {
        "type": "object",
        "properties": {
          "text": { "type": "string" },
          "trigger_time": { "type": "string" },
          "timezone": { "type": "string" }
        },
        "required": ["text"]
      }
    },
    {
      "name": "list_reminders",
      "description": "List pending or completed reminders.",
      "parameters": {
        "type": "object",
        "properties": {
          "status": { "type": "string", "enum": ["pending", "completed", "cancelled"] }
        }
      }
    },
    {
      "name": "set_volume",
      "description": "Set playback audio volume in percent (0 to 100).",
      "parameters": {
        "type": "object",
        "properties": {
          "volume": { "type": "integer", "minimum": 0, "maximum": 100 }
        },
        "required": ["volume"]
      }
    },
    {
      "name": "play_music",
      "description": "Play a track by title, artist, search query or stream URI.",
      "parameters": {
        "type": "object",
        "properties": {
          "query": { "type": "string" },
          "uri": { "type": "string" }
        }
      }
    },
    {
      "name": "pause_music",
      "description": "Pause currently playing audio.",
      "parameters": { "type": "object", "properties": {} }
    },
    {
      "name": "resume_music",
      "description": "Resume previously paused music playback.",
      "parameters": { "type": "object", "properties": {} }
    },
    {
      "name": "stop_music",
      "description": "Stop audio playback.",
      "parameters": { "type": "object", "properties": {} }
    },
    {
      "name": "search_music",
      "description": "Search tracks in catalog, YouTube Music, or local library.",
      "parameters": {
        "type": "object",
        "properties": {
          "query": { "type": "string" },
          "source": { "type": "string" },
          "limit": { "type": "integer" }
        },
        "required": ["query"]
      }
    },
    {
      "name": "get_current_track",
      "description": "Get current playing track metadata and playback status.",
      "parameters": { "type": "object", "properties": {} }
    },
    {
      "name": "get_device_status",
      "description": "Get device telemetry, RAM, CPU, subsystem states and uptime.",
      "parameters": { "type": "object", "properties": {} }
    }
  ]
}
```

#### Пример cURL
```bash
curl -s -X GET http://localhost:8080/api/v1/ai/tools
```

---

### `GET /api/v1/ai/sessions/:id`

Возвращает историю диалога и лог выполнения системных инструментов для указанного идентификатора сессии.

#### Заголовки
- `Accept: application/json`

#### Параметры пути
| Параметр | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | Да | Идентификатор диалоговой сессии |

#### Пример ответа (`200 OK`)
```json
{
  "session_id": "sess-default-01",
  "messages": [
    {
      "role": "user",
      "content": "Поставь будильник на 07:30 на будни с названием Работа",
      "timestamp": "2026-11-01T16:48:10.123Z",
      "tool_calls": null
    },
    {
      "role": "assistant",
      "content": "Будильник установлен на 07:30 (будни) с меткой 'Работа'.",
      "timestamp": "2026-11-01T16:48:10.540Z",
      "tool_calls": [
        {
          "tool": "create_alarm",
          "arguments": {
            "time": "07:30",
            "recurrence": "weekdays",
            "label": "Работа"
          },
          "result": { "id": "f51b63ef-634a-4b08-8eec-45811c750422" },
          "success": true
        }
      ]
    }
  ],
  "last_active_at": "2026-11-01T16:48:10.540Z"
}
```

#### Возможные ошибки
- `404 Not Found`:
  ```json
  {
    "error": "Session 'unknown-session' not found",
    "code": "NOT_FOUND"
  }
  ```

#### Пример cURL
```bash
curl -s -X GET http://localhost:8080/api/v1/ai/sessions/sess-default-01
```

---

## 5. Музыкальный плеер (Music Manager)

Музыкальный менеджер интегрирует YouTube Music, онлайн-радиостанции, локальную библиотеку и MPV плеер с механизмом рекомендаций на базе лайков и пропусков треков.

### `GET /api/v1/music/search`

Ищет музыкальные композиции по строке запроса в локальной медиатеке, каталоге и внешних сервисах.

#### Заголовки
- `Accept: application/json`

#### Параметры строки запроса (Query Parameters)
| Параметр | Тип | Обязательный | По умолчанию | Описание |
|---|---|---|---|---|
| `q` (или `query`) | `string` | Нет | `""` | Текст поискового запроса (название, исполнитель) |
| `source` | `string` | Нет | `"all"` | Источник поиска: `all`, `catalog`, `youtube`, `radio`, `local` |
| `limit` | `integer` | Нет | `20` | Максимальное количество результатов |

#### Пример ответа (`200 OK`)
```json
{
  "query": "Queen",
  "source": "all",
  "count": 2,
  "results": [
    {
      "id": "yt:dQw4w9WgXcQ",
      "title": "Bohemian Rhapsody",
      "artist": "Queen",
      "duration_secs": 354,
      "source": "youtube",
      "uri": "https://music.youtube.com/watch?v=dQw4w9WgXcQ",
      "thumbnail_url": "https://img.youtube.com/vi/dQw4w9WgXcQ/0.jpg"
    },
    {
      "id": "radio:rock-queen",
      "title": "Queen Classic Radio",
      "artist": "Radio (Classic Rock)",
      "duration_secs": null,
      "source": "radio",
      "uri": "https://stream.example.com/rock.mp3",
      "thumbnail_url": null
    }
  ]
}
```

#### Пример cURL
```bash
curl -s -X GET "http://localhost:8080/api/v1/music/search?q=Queen&limit=5"
```

---

### `GET /api/v1/music/current`

Возвращает текущее состояние воспроизведения, информацию об играющем треке, громкость и таймкод.

#### Заголовки
- `Accept: application/json`

#### Параметры запроса
Отсутствуют.

#### Пример ответа (`200 OK`)
```json
{
  "status": "playing",
  "current_track": {
    "id": "track-c831f24d",
    "title": "Bohemian Rhapsody",
    "artist": "Queen",
    "duration_secs": 354,
    "source": "youtube",
    "uri": "https://music.youtube.com/watch?v=dQw4w9WgXcQ",
    "created_at": "2026-11-01 16:50:00"
  },
  "volume": 65,
  "position_secs": 42,
  "duration_secs": 354
}
```

#### Пример cURL
```bash
curl -s -X GET http://localhost:8080/api/v1/music/current
```

---

### `GET /api/v1/music/history`

Возвращает журнал последних воспроизведенных треков с метками времени и длительностью прослушивания.

#### Заголовки
- `Accept: application/json`

#### Параметры строки запроса (Query Parameters)
| Параметр | Тип | Обязательный | По умолчанию | Описание |
|---|---|---|---|---|
| `limit` | `integer` | Нет | `50` | Количество записей в ответе |

#### Пример ответа (`200 OK`)
```json
{
  "count": 1,
  "history": [
    {
      "id": 14,
      "track_id": "track-c831f24d",
      "track": {
        "id": "track-c831f24d",
        "title": "Bohemian Rhapsody",
        "artist": "Queen",
        "duration_secs": 354,
        "source": "youtube"
      },
      "played_at": "2026-11-01 16:50:00",
      "playback_duration_secs": 354,
      "completed": true
    }
  ]
}
```

#### Пример cURL
```bash
curl -s -X GET "http://localhost:8080/api/v1/music/history?limit=10"
```

---

### `GET /api/v1/music/favorites`

Возвращает список треков, отмеченных пользователем как "Любимые" (лайки).

#### Заголовки
- `Accept: application/json`

#### Параметры строки запроса (Query Parameters)
| Параметр | Тип | Обязательный | По умолчанию | Описание |
|---|---|---|---|---|
| `limit` | `integer` | Нет | `50` | Максимальное количество избранных треков |

#### Пример ответа (`200 OK`)
```json
{
  "count": 1,
  "favorites": [
    {
      "id": "track-c831f24d",
      "title": "Bohemian Rhapsody",
      "artist": "Queen",
      "duration_secs": 354,
      "source": "youtube",
      "created_at": "2026-11-01 16:50:00"
    }
  ]
}
```

#### Пример cURL
```bash
curl -s -X GET http://localhost:8080/api/v1/music/favorites
```

---

### `POST /api/v1/music/play`

Запускает воспроизведение трека по названию, запросу, URL потока или идентификатору.

#### Заголовки
- `Content-Type: application/json`
- `Accept: application/json`

#### Схема тела запроса (JSON Schema)
```json
{
  "type": "object",
  "properties": {
    "track_id": {
      "type": "string",
      "description": "Идентификатор существующего трека в каталоге"
    },
    "query": {
      "type": "string",
      "description": "Поисковый запрос для немедленного воспроизведения первого совпадения"
    },
    "uri": {
      "type": "string",
      "description": "Прямой аудиопоток (HTTP, RTSP, файл)"
    },
    "title": {
      "type": "string",
      "description": "Название композиции"
    },
    "artist": {
      "type": "string",
      "description": "Имя исполнителя"
    }
  }
}
```

#### Пример тела запроса
```json
{
  "title": "Radio Record",
  "artist": "Record Station",
  "uri": "http://air.radiorecord.ru:805/rr_320"
}
```

#### Пример ответа (`200 OK`)
```json
{
  "status": "playing",
  "track": {
    "id": "d173bc5e-8557-41ab-889d-ffbd759b76e1",
    "title": "Radio Record",
    "artist": "Record Station",
    "source": "manual",
    "uri": "http://air.radiorecord.ru:805/rr_320"
  }
}
```

#### Пример cURL
```bash
# Воспроизведение по явному URL
curl -s -X POST http://localhost:8080/api/v1/music/play \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Radio Record",
    "artist": "Record Station",
    "uri": "http://air.radiorecord.ru:805/rr_320"
  }'

# Воспроизведение по поисковому запросу
curl -s -X POST http://localhost:8080/api/v1/music/play \
  -H "Content-Type: application/json" \
  -d '{"query": "Imagine Dragons Bones"}'
```

---

### `POST /api/v1/music/pause`

Ставит текущее воспроизведение на паузу.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "status": "paused"
}
```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/music/pause
```

---

### `POST /api/v1/music/resume`

Возобновляет приостановленное воспроизведение.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "status": "playing"
}
```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/music/resume
```

---

### `POST /api/v1/music/stop`

Останавливает воспроизведение и освобождает звуковой драйвер.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "status": "stopped"
}
```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/music/stop
```

---

### `POST /api/v1/music/like`

Ставит или снимает отметку "нравится" для трека. Положительно влияет на коэффициент рекомендации артиста и жанра.

#### Заголовки
- `Content-Type: application/json`
- `Accept: application/json`

#### Схема тела запроса (JSON Schema)
```json
{
  "type": "object",
  "properties": {
    "track_id": {
      "type": "string",
      "description": "ID трека. Если не указан — оценивается текущий играющий трек."
    },
    "liked": {
      "type": "boolean",
      "description": "Явное задание состояния (true/false). Если null — инвертирует текущее."
    }
  }
}
```

#### Пример тела запроса
```json
{
  "liked": true
}
```

#### Пример ответа (`200 OK`)
```json
{
  "track_id": "track-c831f24d",
  "liked": true
}
```

#### Возможные ошибки
- `400 Bad Request`:
  ```json
  {
    "error": "No track specified and nothing currently playing",
    "code": "CONFIG_ERROR"
  }
  ```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/music/like \
  -H "Content-Type: application/json" \
  -d '{"liked": true}'
```

---

### `POST /api/v1/music/skip`

Пропускает текущий трек. Фиксирует пропуск в таблице `music_skips` со штрафным весом для движка персональных рекомендаций.

#### Заголовки
- `Content-Type: application/json`
- `Accept: application/json`

#### Схема тела запроса
```json
{
  "type": "object",
  "properties": {
    "track_id": {
      "type": "string",
      "description": "ID трека (опционально, по умолчанию берется текущий)"
    },
    "position_secs": {
      "type": "integer",
      "description": "Секунда трека, на которой пользователь совершил пропуск"
    }
  }
}
```

#### Пример ответа (`200 OK`)
```json
{
  "status": "skipped",
  "track_id": "track-c831f24d"
}
```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/music/skip \
  -H "Content-Type: application/json" \
  -d '{"position_secs": 15}'
```

---

### `GET /api/v1/music/recommendations`

Возвращает персональные рекомендации треков, сформированные исключительно на стороне сервиса YouTube Music.

#### Заголовки
- `Accept: application/json`

#### Параметры строки запроса (Query Parameters)
| Параметр | Тип | Обязательный | По умолчанию | Описание |
|---|---|---|---|---|
| `limit` | `integer` | Нет | `10` | Количество рекомендуемых треков |

#### Пример ответа (`200 OK`)
```json
{
  "count": 2,
  "recommendations": [
    {
      "track": {
        "id": "rec-track-01",
        "title": "Don't Stop Me Now",
        "artist": "Queen",
        "duration_secs": 210,
        "source": "catalog"
      },
      "score": 3.75,
      "reason": "Любимый исполнитель: Queen"
    },
    {
      "track": {
        "id": "rec-track-02",
        "title": "Radio Ga Ga",
        "artist": "Queen",
        "duration_secs": 348,
        "source": "catalog"
      },
      "score": 2.5,
      "reason": "Высокий процент дослушивания"
    }
  ]
}
```

#### Пример cURL
```bash
curl -s -X GET "http://localhost:8080/api/v1/music/recommendations?limit=5"
```

---

### `GET /api/v1/music/radio`

Возвращает каталог встроенных интернет-радиостанций с жанрами и URL потоков.

#### Заголовки
- `Accept: application/json`

#### Параметры запроса
Отсутствуют.

#### Пример ответа (`200 OK`)
```json
{
  "count": 3,
  "stations": [
    {
      "id": "record",
      "name": "Radio Record",
      "genre": "Dance / Electronic",
      "stream_uri": "http://air.radiorecord.ru:805/rr_320",
      "homepage_url": "https://www.radiorecord.ru",
      "bitrate": 320,
      "favicon_url": "https://www.radiorecord.ru/favicon.ico"
    },
    {
      "id": "relax-fm",
      "name": "Relax FM",
      "genre": "Chillout / Ambient",
      "stream_uri": "https://relaxfm.hostingradio.ru/relaxfm128.mp3",
      "homepage_url": "https://relax-fm.ru",
      "bitrate": 128,
      "favicon_url": null
    },
    {
      "id": "nashe-radio",
      "name": "Наше Радио",
      "genre": "Russian Rock",
      "stream_uri": "https://nashe1.hostingradio.ru/nashe-128.mp3",
      "homepage_url": "https://www.nashe.ru",
      "bitrate": 128,
      "favicon_url": null
    }
  ]
}
```

#### Пример cURL
```bash
curl -s -X GET http://localhost:8080/api/v1/music/radio
```

---

### `GET /api/v1/music/lavalink/status`

Возвращает статус подключения к аудиосерверу Lavalink v4, активные источники (YouTube, Yandex Music, SoundCloud) и плагины.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "connected": true,
  "host": "127.0.0.1",
  "port": 2333,
  "version": "4.0.8",
  "sources": [
    "youtube",
    "yandexmusic",
    "soundcloud"
  ],
  "plugins": [
    "youtube-plugin",
    "lavasrc-plugin"
  ],
  "yandex_music_configured": true
}
```

#### Пример cURL
```bash
curl -s -X GET http://localhost:8080/api/v1/music/lavalink/status
```

---

### `POST /api/v1/music/lavalink/config`

Обновляет параметры подключения к Lavalink и сохраняет OAuth-токен аккаунта Яндекс Музыки в `device_state`.

#### Заголовки
- `Content-Type: application/json`
- `Accept: application/json`

#### Тело запроса
```json
{
  "host": "127.0.0.1",
  "port": 2333,
  "password": "youshallnotpass",
  "yandex_music_token": "y0_AgAAAA..."
}
```

#### Пример ответа (`200 OK`)
```json
{
  "status": "ok",
  "lavalink": {
    "connected": true,
    "host": "127.0.0.1",
    "port": 2333,
    "version": "4.0.8",
    "sources": [
      "youtube",
      "yandexmusic",
      "soundcloud"
    ],
    "plugins": [
      "youtube-plugin",
      "lavasrc-plugin"
    ],
    "yandex_music_configured": true
  }
}
```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/music/lavalink/config \
  -H "Content-Type: application/json" \
  -d '{"yandex_music_token":"y0_AgAAAA..."}'
```

---

### `GET /api/v1/music/youtube/auth/status`

Возвращает текущий статус авторизации в Google / YouTube, валидность токена и наличие файла `cookies.txt`.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "authorized": true,
  "has_client": true,
  "access_token_valid": true,
  "expires_in": 3540,
  "token_path": "/var/lib/kamish/youtube-oauth.json",
  "cookies_path": "/var/lib/kamish/youtube-cookies.txt",
  "cookies_ready": true,
  "problem": null
}
```

---

### `POST /api/v1/music/youtube/auth/begin`

Запускает процесс OAuth 2.0 Device Flow, возвращая URL подтверждения и разовый код для пользователя.

#### Заголовки
- `Content-Type: application/json`
- `Accept: application/json`

#### Тело запроса
```json
{
  "client_id": "123456789-abc.apps.googleusercontent.com",
  "client_secret": "GOCSPX-..."
}
```

#### Пример ответа (`200 OK`)
```json
{
  "verification_url": "https://www.google.com/device",
  "user_code": "ABCD-EFGH",
  "expires_in": 1800,
  "interval": 5,
  "device_code": "AH-..."
}
```

---

### `POST /api/v1/music/youtube/auth/finish`

Завершает авторизацию после подтверждения пользователем в браузере. Обменивает `device_code` на токены и автоматически генерирует Netscape `cookies.txt` на сервере.

#### Заголовки
- `Content-Type: application/json`
- `Accept: application/json`

#### Тело запроса
```json
{
  "device_code": "AH-..."
}
```

#### Пример ответа (`200 OK`)
```json
{
  "authorized": true,
  "has_client": true,
  "access_token_valid": true,
  "expires_in": 3600,
  "token_path": "/var/lib/kamish/youtube-oauth.json",
  "cookies_path": "/var/lib/kamish/youtube-cookies.txt",
  "cookies_ready": true,
  "problem": null
}
```

---

### `POST /api/v1/music/youtube/auth/logout`

Удаляет сохранённые токены и сгенерированный файл `cookies.txt` с сервера.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "status": "ok",
  "message": "YouTube credentials cleared"
}
```

---

## 6. Менеджер речи (Speech Manager: STT, TTS, Wake Word, VAD)

Модуль распознавания речи (STT), синтеза (TTS), обнаружения активности голоса (VAD) и детекции активационной фразы (Wake Word).

### `GET /api/v1/speech/status`

Возвращает текущее состояние подсистемы речи (активные локальные провайдеры, голос, VAD, Wake Word, состояние микрофона). Распознавание (STT) и синтез (TTS) выполняются строго офлайн.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "microphone_connected": true,
  "active_stt_provider": "vosk",
  "active_tts_provider": "piper",
  "active_voice": "ru_RU-dmitri-medium",
  "vad_enabled": true,
  "wakeword_enabled": true,
  "supported_keywords": ["ассистент", "колонка", "слушай"]
}
```

#### Пример cURL
```bash
curl -s http://localhost:8080/api/v1/speech/status
```

---

### `GET /api/v1/speech/voices`

Возвращает каталог всех доступных локальных голосов синтеза речи (TTS). Облачные провайдеры речи не поддерживаются согласно требованиям приватности.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "count": 3,
  "voices": [
    {
      "id": "ru_RU-dmitri-medium",
      "name": "Дмитрий (Piper Neural)",
      "language": "ru-RU",
      "gender": "Male",
      "provider": "piper"
    },
    {
      "id": "aleksandr",
      "name": "Александр (RHVoice)",
      "language": "ru-RU",
      "gender": "Male",
      "provider": "rhvoice"
    },
    {
      "id": "offline-sine-tone",
      "name": "Резервный автономный голос (Offline Fallback)",
      "language": "ru-RU",
      "gender": "Neutral",
      "provider": "offline_fallback"
    }
  ]
}
```

#### Пример cURL
```bash
curl -s http://localhost:8080/api/v1/speech/voices
```

---

### `POST /api/v1/speech/config`

Динамически переключает активные провайдеры STT/TTS, выбранный голос и тумблеры VAD/Wake Word.

#### Заголовки
- `Content-Type: application/json`

#### Пример тела запроса
```json
{
  "tts_provider": "piper",
  "stt_provider": "vosk",
  "voice": "ru_RU-dmitri-medium",
  "vad_enabled": true,
  "wakeword_enabled": true
}
```

#### Пример ответа (`200 OK`)
```json
{
  "updated": true,
  "status": {
    "active_stt_provider": "vosk",
    "active_tts_provider": "piper",
    "active_voice": "ru_RU-dmitri-medium",
    "vad_enabled": true,
    "wakeword_enabled": true
  }
}
```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/speech/config \
  -H "Content-Type: application/json" \
  -d '{"tts_provider":"piper","voice":"ru_RU-dmitri-medium"}'
```

---

### `POST /api/v1/speech/synthesize` (алиас: `/api/v1/speech/tts`)

Синтезирует переданный текст в аудиопоток WAV через локальный движок или напрямую воспроизводит через динамик умной колонки.

#### Заголовки
- `Content-Type: application/json`

#### Параметры тела запроса
| Поле | Тип | Обязательное | Описание |
|---|---|---|---|
| `text` | string | Да | Текст для озвучивания |
| `voice` | string | Нет | ID голоса |
| `provider` | string | Нет | Локальный провайдер: `piper`, `rhvoice`, `offline_fallback` |
| `speed` | number | Нет | Скорость воспроизведения (0.5 – 2.0, по умолчанию 1.0) |
| `play_on_speaker` | boolean | Нет | Если `true`, звук воспроизводится на физическом динамике колонки |
| `format` | string | Нет | `"binary"` (по умолчанию отдаёт аудио-бинарник) или `"json"` (base64) |

#### Пример cURL (сохранение в файл WAV):
```bash
curl -s -X POST http://localhost:8080/api/v1/speech/synthesize \
  -H "Content-Type: application/json" \
  -d '{"text":"Привет! Я умная колонка.","provider":"offline_fallback"}' \
  --output speech.wav
```

#### Пример cURL (воспроизведение вслух на колонке):
```bash
curl -s -X POST http://localhost:8080/api/v1/speech/synthesize \
  -H "Content-Type: application/json" \
  -d '{"text":"Будильник установлен на семь утра.","play_on_speaker":true}'
```

---

### `POST /api/v1/speech/recognize` (алиас: `/api/v1/speech/stt`)

Распознаёт аудиоданные (PCM/WAV) в текст через локальный STT-провайдер с опциональной сквозной диспетчеризацией в AI Orchestrator.

#### Заголовки
- `Content-Type: application/json`

#### Параметры тела запроса
| Поле | Тип | Обязательное | Описание |
|---|---|---|---|
| `audio_base64` | string | Да | Аудиопоток в кодировке base64 (WAV или сырой PCM 16-bit 16kHz) |
| `language` | string | Нет | Язык (`"ru"`, по умолчанию `"ru"`) |
| `provider` | string | Нет | Локальный STT-провайдер (`"vosk"`, `"sherpa_onnx"`, `"mock"`) |
| `dispatch_ai` | boolean | Нет | Если `true`, распознанная команда сразу отправляется в AI Orchestrator, и возвращается ответ ассистента |

#### Пример ответа (`200 OK`)
```json
{
  "text": "поставь будильник на 07:30",
  "confidence": 0.95,
  "language": "ru",
  "ai_response": {
    "response": "Будильник установлен на 07:30.",
    "session_id": "9a184bf4-7b64-42f5-b6d8-bf5bdf11467a"
  }
}
```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/speech/recognize \
  -H "Content-Type: application/json" \
  -d '{"audio_base64":"UklGRi...", "dispatch_ai": true}'
```

---

### `POST /api/v1/speech/wakeword/simulate`

Программная симуляция фразы активации (Wake Word). Позволяет тестировать полный сквозной цикл голосового помощника без микрофона: *Wake Word -> Голосовая команда -> AI Orchestrator -> Синтез ответа*.

#### Заголовки
- `Content-Type: application/json`

#### Параметры тела запроса
```json
{
  "keyword": "колонка",
  "command": "поставь будильник на 07:30",
  "dispatch_ai": true,
  "speak_response": true
}
```

#### Пример ответа (`200 OK`)
```json
{
  "triggered": true,
  "keyword": "колонка",
  "detected_at": "2026-11-01T07:10:00Z",
  "ai_result": {
    "response": "Будильник установлен на 07:30.",
    "tool_calls": [
      {
        "tool": "create_alarm",
        "arguments": { "time": "07:30" },
        "success": true
      }
    ]
  }
}
```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/speech/wakeword/simulate \
  -H "Content-Type: application/json" \
  -d '{"keyword":"колонка","command":"поставь будильник на 07:30","dispatch_ai":true}' | jq .
```

---

### Микрофонный захват (Capture)

Управление фоновым циклом прослушивания микрофона. Именно эти эндпоинты показывают
**реальное** состояние входа: отчитываются о захвате только если кадры действительно
читаются с устройства.

### `GET /api/v1/speech/capture/status`

Текущее состояние цикла захвата.

```json
{
  "source": "arecord",
  "available": true,
  "listening": true,
  "frames_total": 12950070,
  "wake_words_total": 2,
  "utterances_total": 2,
  "last_transcript": "какая погода сегодня",
  "last_reply": "Сегодня ясно, около 12 градусов."
}
```

Поля `last_error`, `last_transcript`, `last_reply` опускаются, если значения нет.

### `POST /api/v1/speech/capture/start`

Запускает фоновое прослушивание. Возвращает `status: "started"` при успехе либо
`status: "unavailable"` с пояснением, если аудио-устройство/бинарник отсутствует —
эндпоинт никогда не рапортует об успехе без реального источника.

```bash
curl -s -X POST http://localhost:8080/api/v1/speech/capture/start | jq .
```

### `POST /api/v1/speech/capture/stop`

Останавливает прослушивание и освобождает устройство. Идемпотентен: при неработающем
цикле возвращает `status: "idle"`.

```bash
curl -s -X POST http://localhost:8080/api/v1/speech/capture/stop | jq .
```

#### WebSocket-события конвейера
- `speech_capture_started` — цикл запущен (`source`).
- `wake_word_detected` — сработала активационная фраза (`keyword`, `score`).
- `speech_transcript` — распознанный текст (`text`, `confidence`, `provider`).
- `speech_reply` — ответ AI и факт озвучивания (`response`, `spoken`).
- `speech_capture_state` — смена состояния (`waiting_wake_word`, `listening_command`, `stopped`).
- `speech_capture_error` — ошибка источника с текстом причины.

---

### Управление Antigravity CLI (`agy`)

Скачивание и авторизация CLI непосредственно из веб-панели — без доступа к терминалу.

### `GET /api/v1/ai/agy/status`

```json
{
  "installed": true,
  "path": "/opt/kamish/bin/agy",
  "version": "1.2.0",
  "authorized": false,
  "problem": null,
  "download_url": "https://.../agy-linux-arm64.tar.gz",
  "download_enabled": true,
  "architecture": "linux-aarch64"
}
```

### `POST /api/v1/ai/agy/install`

Скачивает релиз под текущую архитектуру и устанавливает его. Адрес задаётся
переменной `KAMISH_AGY_DOWNLOAD_URL`; допускаются только `http(s)`-ссылки,
размер ограничен 200 МБ. Скачивание можно запретить через `KAMISH_ALLOW_AGY_DOWNLOAD=0`.

### `POST /api/v1/ai/agy/login/begin`

Запускает авторизацию и возвращает ссылку и одноразовый код:

```json
{
  "auth_url": "https://antigravity.dev/device",
  "auth_code": "AB12-CD34",
  "instructions": "To authorize, open ... and enter code AB12-CD34"
}
```

### `POST /api/v1/ai/agy/login/submit`

Передаёт введённый код обратно в CLI.

```json
{ "code": "AB12-CD34" }
```

Ответ: `{"authorized": true, "message": "Авторизация завершена успешно"}`.

> [!IMPORTANT]
> Эндпоинты вызывают CLI с фиксированным набором аргументов и 30-секундным таймаутом.
> Токены и переменные окружения CLI в браузер не возвращаются — только ссылка, код
> и текст инструкции.

---

## 7. Настройки устройства (Settings)

Управление системными параметрами и конфигурацией колонки, сохраняемыми в таблице `settings`.

### `GET /api/v1/settings`

Получает текущие системные настройки умной колонки.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "device_name": "kamish",
  "volume_default": 60,
  "timezone": "Europe/Moscow",
  "tts_engine": "piper",
  "tts_voice": "irina",
  "stt_engine": "vosk",
  "stt_model": "vosk-model-small-ru-0.22",
  "ai_provider": "antigravity",
  "alarm_sound": "gentle_bell",
  "night_mode": false,
  "night_mode_start": "23:00",
  "night_mode_end": "07:00"
}
```

#### Пример cURL
```bash
curl -s -X GET http://localhost:8080/api/v1/settings
```

---

### `PUT /api/v1/settings`

Обновляет настройки устройства. Допускается частичное обновление ключей (partial update).

#### Заголовки
- `Content-Type: application/json`
- `Accept: application/json`

#### Пример тела запроса
```json
{
  "volume_default": 75,
  "timezone": "Asia/Novosibirsk",
  "tts_engine": "rhvoice"
}
```

#### Пример ответа (`200 OK`)
```json
{
  "status": "updated",
  "updated_keys": ["volume_default", "timezone", "tts_engine"]
}
```

#### Возможные ошибки
- `400 Bad Request`:
  ```json
  {
    "error": "Invalid timezone 'Invalid/Zone'",
    "code": "CONFIG_ERROR"
  }
  ```

#### Пример cURL
```bash
curl -s -X PUT http://localhost:8080/api/v1/settings \
  -H "Content-Type: application/json" \
  -d '{
    "volume_default": 75,
    "timezone": "Asia/Novosibirsk"
  }'
```

---

## 7. Bluetooth A2DP Sink (Раздел 10)

Управление Bluetooth-контроллером BlueZ для работы колонки в режиме беспроводного аудиоприемника (A2DP Sink) со смартфонов и ПК.

### `GET /api/v1/bluetooth/status`

Возвращает текущее состояние Bluetooth-адаптера, видимость (discoverable), сопряженные и подключенные устройства.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "adapter_enabled": true,
  "discoverable": true,
  "connected_device": {
    "mac": "FC:58:FA:12:34:56",
    "name": "Pixel 8 Pro",
    "connected": true,
    "codec": "SBC",
    "volume": 70
  },
  "paired_devices": [
    {
      "mac": "FC:58:FA:12:34:56",
      "name": "Pixel 8 Pro",
      "trusted": true
    },
    {
      "mac": "E0:D4:E8:98:76:54",
      "name": "MacBook Air",
      "trusted": true
    }
  ]
}
```

#### Пример cURL
```bash
curl -s -X GET http://localhost:8080/api/v1/bluetooth/status
```

---

### `POST /api/v1/bluetooth/scan`

Запускает кратковременное сканирование (discovery) доступных Bluetooth-устройств.

#### Заголовки
- `Content-Type: application/json`
- `Accept: application/json`

#### Схема тела запроса
```json
{
  "type": "object",
  "properties": {
    "duration_secs": {
      "type": "integer",
      "default": 10,
      "description": "Продолжительность сканирования в секундах"
    }
  }
}
```

#### Пример ответа (`200 OK`)
```json
{
  "status": "scanning_complete",
  "devices_found": [
    {
      "mac": "FC:58:FA:12:34:56",
      "name": "Pixel 8 Pro",
      "rssi": -52
    }
  ]
}
```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/bluetooth/scan \
  -H "Content-Type: application/json" \
  -d '{"duration_secs": 5}'
```

---

### `POST /api/v1/bluetooth/pair`

Запускает сопряжение с указанным Bluetooth MAC-адресом.

#### Заголовки
- `Content-Type: application/json`
- `Accept: application/json`

#### Пример тела запроса
```json
{
  "mac": "FC:58:FA:12:34:56"
}
```

#### Пример ответа (`200 OK`)
```json
{
  "status": "paired",
  "mac": "FC:58:FA:12:34:56"
}
```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/bluetooth/pair \
  -H "Content-Type: application/json" \
  -d '{"mac": "FC:58:FA:12:34:56"}'
```

---

### `POST /api/v1/bluetooth/connect`

Устанавливает соединение профиля A2DP Sink с ранее сопряженным устройством.

#### Заголовки
- `Content-Type: application/json`
- `Accept: application/json`

#### Пример тела запроса
```json
{
  "mac": "FC:58:FA:12:34:56"
}
```

#### Пример ответа (`200 OK`)
```json
{
  "status": "connected",
  "mac": "FC:58:FA:12:34:56",
  "profile": "A2DP Sink"
}
```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/bluetooth/connect \
  -H "Content-Type: application/json" \
  -d '{"mac": "FC:58:FA:12:34:56"}'
```

---

### `POST /api/v1/bluetooth/disconnect`

Разрывает текущую Bluetooth аудио-сессию.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "status": "disconnected"
}
```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/bluetooth/disconnect
```

---

## 8. Сетевой менеджер и Wi-Fi (Раздел 11)

Управление сетевыми интерфейсами `wlan0` и `eth0` через NetworkManager D-Bus API, включая аварийную точку доступа (Setup AP).

> [!IMPORTANT]
> Пароли Wi-Fi сетей никогда не возвращаются в ответах API и не пишутся в системные логи.

### `GET /api/v1/network/status`

Возвращает текущую сетевую конфигурацию: статус интерфейсов, текущий IP-адрес, режим подключения и имя активной сети.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "primary_interface": "wlan0",
  "ip_address": "192.168.1.140",
  "subnet_mask": "255.255.255.0",
  "gateway": "192.168.1.1",
  "dns": ["1.1.1.1", "8.8.8.8"],
  "ethernet": {
    "connected": false,
    "speed_mbps": null
  },
  "wifi": {
    "connected": true,
    "ssid": "HomeNetwork_5G",
    "bssid": "34:8A:12:DE:45:90",
    "signal_level_percent": 84,
    "frequency_mhz": 5180
  },
  "ap_mode": {
    "active": false,
    "ssid": "kamish-setup"
  }
}
```

#### Пример cURL
```bash
curl -s -X GET http://localhost:8080/api/v1/network/status
```

---

### `GET /api/v1/network/wifi/scan`

Запускает сканирование эфира и возвращает список доступных точек доступа Wi-Fi.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "count": 3,
  "networks": [
    {
      "ssid": "HomeNetwork_5G",
      "bssid": "34:8A:12:DE:45:90",
      "signal_percent": 88,
      "security": "WPA2-PSK",
      "channel": 36
    },
    {
      "ssid": "Office_Guest",
      "bssid": "78:29:9C:11:22:33",
      "signal_percent": 62,
      "security": "Open",
      "channel": 6
    },
    {
      "ssid": "Neighbor_Wi-Fi",
      "bssid": "BC:F2:AF:99:88:77",
      "signal_percent": 34,
      "security": "WPA3-SAE",
      "channel": 11
    }
  ]
}
```

#### Пример cURL
```bash
curl -s -X GET http://localhost:8080/api/v1/network/wifi/scan
```

---

### `POST /api/v1/network/wifi/connect`

Подключается к указанной Wi-Fi сети.

#### Заголовки
- `Content-Type: application/json`
- `Accept: application/json`

#### Схема тела запроса
```json
{
  "type": "object",
  "properties": {
    "ssid": { "type": "string", "description": "Имя Wi-Fi сети (SSID)" },
    "password": { "type": "string", "description": "Пароль (WPA2/WPA3)" }
  },
  "required": ["ssid"]
}
```

#### Пример тела запроса
```json
{
  "ssid": "HomeNetwork_5G",
  "password": "SecretPassword123"
}
```

#### Пример ответа (`200 OK`)
```json
{
  "status": "connected",
  "ssid": "HomeNetwork_5G",
  "ip_address": "192.168.1.140"
}
```

#### Возможные ошибки
- `400 Bad Request`:
  ```json
  {
    "error": "Authentication failed. Invalid Wi-Fi password or timeout.",
    "code": "NETWORK_AUTH_ERROR"
  }
  ```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/network/wifi/connect \
  -H "Content-Type: application/json" \
  -d '{
    "ssid": "HomeNetwork_5G",
    "password": "SecretPassword123"
  }'
```

---

### `POST /api/v1/network/wifi/disconnect`

Отключается от текущей сети Wi-Fi.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "status": "disconnected"
}
```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/network/wifi/disconnect
```

---

### `POST /api/v1/network/ap/start`

Принудительно поднимает аварийную точку доступа (Setup AP: `kamish-setup`, IP `192.168.4.1`) для начальной настройки устройства или восстановления связи.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "status": "ap_started",
  "ssid": "kamish-setup",
  "gateway_ip": "192.168.4.1"
}
```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/network/ap/start
```

---

### `POST /api/v1/network/ap/stop`

Останавливает режим точки доступа и переключает сетевой интерфейс обратно в режим клиента (Station).

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "status": "ap_stopped"
}
```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/network/ap/stop
```

---

## 9. VPN / VLESS туннелирование (Раздел 12)

Управление проксированием медиа-трафика (YouTube Music API, облачный TTS) через Happ / Xray (протокол VLESS + Reality).

### `GET /api/v1/vpn/status`

Возвращает текущее состояние VPN соединения, активный сервер, пинг и статистику переданного трафика.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "enabled": true,
  "status": "connected",
  "protocol": "VLESS-Reality",
  "server_address": "nl-ams-01.tunnel.net:443",
  "ping_ms": 48,
  "bytes_sent": 15420310,
  "bytes_received": 142850912,
  "connected_since": "2026-11-01T12:00:00Z"
}
```

#### Пример cURL
```bash
curl -s -X GET http://localhost:8080/api/v1/vpn/status
```

---

### `POST /api/v1/vpn/connect`

Устанавливает VPN туннель с использованием сохраненной конфигурации.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "status": "connected",
  "ping_ms": 48
}
```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/vpn/connect
```

---

### `POST /api/v1/vpn/disconnect`

Отключает VPN соединение.

#### Заголовки
- `Accept: application/json`

#### Пример ответа (`200 OK`)
```json
{
  "status": "disconnected"
}
```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/vpn/disconnect
```

---

### `POST /api/v1/vpn/config`

Импортирует новую конфигурацию VPN в виде строки ссылки `vless://...` или JSON профиля.

#### Заголовки
- `Content-Type: application/json`
- `Accept: application/json`

#### Пример тела запроса
```json
{
  "subscription_url": null,
  "vless_uri": "vless://b8313e9a-7a1f-4b08@amsterdam.relay.com:443?security=reality&sni=gateway.microsoft.com&fp=chrome#Netherlands-01"
}
```

#### Пример ответа (`200 OK`)
```json
{
  "status": "configured",
  "server": "amsterdam.relay.com:443",
  "sni": "gateway.microsoft.com"
}
```

#### Возможные ошибки
- `400 Bad Request`:
  ```json
  {
    "error": "Invalid VLESS URI format: missing UUID or port",
    "code": "CONFIG_ERROR"
  }
  ```

#### Пример cURL
```bash
curl -s -X POST http://localhost:8080/api/v1/vpn/config \
  -H "Content-Type: application/json" \
  -d '{
    "vless_uri": "vless://b8313e9a-7a1f-4b08@amsterdam.relay.com:443?security=reality&sni=gateway.microsoft.com&fp=chrome#Netherlands-01"
  }'
```

---

## 10. WebSocket Real-Time Events (Раздел 16)

### `WS /api/v1/events`

Обеспечивает двусторонний полнодуплексный канал связи между SPA-панелью (Vite/TypeScript) и сервером для мгновенной трансляции системных событий без постоянного HTTP-поллинга.

#### URL подключения
`ws://localhost:8080/api/v1/events`

#### Структура передаваемых сообщений
Каждое событие отправляется в виде JSON-пакета следующей структуры:
```json
{
  "event": "event_type",
  "timestamp": "2026-11-01T16:55:00Z",
  "payload": {}
}
```

#### Поддерживаемые типы событий (`event`)

1. **`alarm_triggered`** — сработал будильник:
   ```json
   {
     "event": "alarm_triggered",
     "timestamp": "2026-11-01T17:00:00Z",
     "payload": {
       "id": "a9018f21-723c-41c1-b0db-6e115fa763b1",
       "time": "07:30",
       "label": "Подъем на работу"
     }
   }
   ```

2. **`reminder_triggered`** — сработало напоминание:
   ```json
   {
     "event": "reminder_triggered",
     "timestamp": "2026-11-01T17:00:00Z",
     "payload": {
       "id": "e44d852a-39b1-4d37-88eb-12a865b4c194",
       "text": "Полить цветы"
     }
   }
   ```

3. **`playback_status`** — изменение статуса плеера (playing / paused / stopped):
   ```json
   {
     "event": "playback_status",
     "timestamp": "2026-11-01T17:00:05Z",
     "payload": {
       "status": "playing"
     }
   }
   ```

4. **`track_changed`** — смена текущего трека:
   ```json
   {
     "event": "track_changed",
     "timestamp": "2026-11-01T17:00:05Z",
     "payload": {
       "track": {
         "id": "track-c831f24d",
         "title": "Bohemian Rhapsody",
         "artist": "Queen",
         "duration_secs": 354
       }
     }
   }
   ```

5. **`volume_changed`** — изменение уровня громкости устройства:
   ```json
   {
     "event": "volume_changed",
     "timestamp": "2026-11-01T17:01:10Z",
     "payload": {
       "volume": 75
     }
   }
   ```

6. **`bluetooth_state`** — подключение или отключение смартфона:
   ```json
   {
     "event": "bluetooth_state",
     "timestamp": "2026-11-01T17:02:00Z",
     "payload": {
       "connected": true,
       "device_name": "Pixel 8 Pro"
     }
   }
   ```

7. **`network_state`** — изменение сетевого статуса или IP-адреса:
   ```json
   {
     "event": "network_state",
     "timestamp": "2026-11-01T17:02:15Z",
     "payload": {
       "connected": true,
       "ssid": "HomeNetwork_5G",
       "ip": "192.168.1.140"
     }
   }
   ```

#### Пример подключения в браузере / клиенте (JavaScript)
```javascript
const socket = new WebSocket('ws://' + window.location.host + '/api/v1/events');

socket.onopen = () => {
  console.log('Подключено к WebSocket событиям Kamish');
};

socket.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Получено событие:', data.event, data.payload);
  
  if (data.event === 'alarm_triggered') {
    alert(`Сработал будильник: ${data.payload.label || data.payload.time}`);
  }
};

socket.onclose = () => {
  console.log('Соединение с WebSocket сервером закрыто');
};
```
