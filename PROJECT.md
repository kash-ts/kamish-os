# Project: KAMISH Web Panel Overhaul & Authentication

## Architecture
- **Сервисный слой (Rust)**: Rust 2021, Tokio, Axum 0.8, SQLx SQLite WAL, Argon2id password hashing, native HTTP cookie/header session management.
- **SPA-интерфейс (`crates/kamish-web/frontend/`)**: React 19, TypeScript, Vite, Tailwind CSS, Lucide icons, строгий светлый минималистичный дизайн (темная тема запрещена), нативный WebSocket клиент.
- **Data Flow**:
  - `POST /api/v1/auth/login` -> Argon2id verification -> SQLite `auth_sessions` -> returns session token & Set-Cookie.
  - `Axum Middleware` -> validates token on `/api/v1/*` routes; enforces `must_change_password` (blocks with 403 `PASSWORD_CHANGE_REQUIRED`).
  - `POST /api/v1/auth/change-password` -> verifies old password -> hashes new password -> clears `must_change_password` -> clears prior sessions.
  - `GET /api/v1/setup/status` -> reads `device_state` & runs non-blocking async subsystem probes -> returns status of auth, network, audio, voice, ai.
  - `POST /api/v1/setup/step` -> saves progress in `device_state` -> emits `setup_step_updated` on WebSocket.
  - `POST /api/v1/setup/complete` -> marks `setup:completed = true` in `device_state` -> emits `setup_completed` on WebSocket.
  - `POST /api/v1/setup/simulate-degradation` -> controlled degradation injection for zero-mock testing.
  - SPA-интерфейс: светлая тема (без темной темы), экраны `/login`, forced `/change-password`, пошаговый `/setup` и фиксированный сайдбар (9 разделов).

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | Auth Migration & Repository | SQLite schema for `users` and `auth_sessions`, Argon2id hashing | M1 | R1 |
| 2 | Auth Endpoints | `/login`, `/change-password`, `/logout`, `/session` | M1 | R1 |
| 3 | Axum Auth Middleware | Protects `/api/v1/*`, whitelists status & login, enforces `must_change_password` | M1 | R1 |
| 4 | Setup Wizard Persistence | SQLite `device_state` storage for wizard progress & completion | M2 | R2 |
| 5 | Setup Wizard Endpoints | `/status`, `/step`, `/complete`, `/diagnostics`, `/simulate-degradation` | M2 | R2 |
| 6 | Subsystem Health & Recovery | Live probes (STT/TTS, AI CLI, audio, network, hardware), recovery redirection | M2 | R2 |
| 7 | Light Minimalist UI Design | Строгая светлая тема (темная тема запрещена), чистый дизайн без AI-slop | M3 | R3 |
| 8 | Login & Password Change Screens | Centered login card, blocking forced password change view | M3 | R3 |
| 9 | Setup Wizard Component | Step-by-step verification, non-destructive degradation recovery | M3 | R3 |
| 10 | Dashboard & Fixed Sidebar | 9 navigation sections, real-time status indicator, zero mocks | M3 | R3 |
| 11 | Full E2E & Quality Gate Verification | cargo check, cargo test, cargo fmt check, npm run build | M4 | All |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| M1 | Auth Subsystem (kamish-web) | Cargo `argon2`, `database/schema.sql`, `crates/kamish-web/src/auth/`, `/api/v1/auth/*`, middleware | none | COMPLETED |
| M2 | Setup Wizard (kamish-web) | `SetupRepository`, `/api/v1/setup/*`, diagnostic probes, WS events | none | COMPLETED |
| M3 | SPA-интерфейс (kamish-web) | Светлая минималистичная тема (темная тема запрещена), Login, Change Password, Setup Wizard, 9 разделов | none | COMPLETED |
| M4 | Integration & Quality Gates | End-to-end verification, tests, forensic audit | M1, M2, M3 | IN_PROGRESS |

## Code Layout & Write Boundaries
> Пути приведены к актуальной раскладке Cargo-воркспейса: серверные подсистемы
> живут в `crates/kamish-core/` и `crates/kamish-web/`, а исходники веб-панели —
> в `crates/kamish-web/frontend/`.

- **Worker 1 (Auth - M1)** exclusively owns:
  - `crates/kamish-web/Cargo.toml`
  - `database/schema.sql`
  - `crates/kamish-web/src/auth/`
  - `crates/kamish-web/src/api/auth.rs`
- **Worker 2 (Setup Wizard - M2)** exclusively owns:
  - `crates/kamish-core/src/setup/`
  - `crates/kamish-web/src/api/setup.rs`
  - `crates/kamish-core/tests/`
- **Worker 3 (SPA-интерфейс, минималистичный оверхаул — M3)** exclusively owns:
  - `crates/kamish-web/frontend/src/` (App.tsx, components/, pages/, api/, styles/)
  - `crates/kamish-web/frontend/index.html`

Note on shared crate files (общие точки сборки `kamish-web` / `kamish-core`):
`crates/kamish-web/src/api/mod.rs`, `crates/kamish-core/src/common/mod.rs`, and
`crates/kamish-core/src/database/mod.rs` (+ `crates/kamish-web/src/database.rs`):
M1 wires auth routes & AppState. M2 wires setup routes.
To avoid merge collisions, Worker 1 (Service Layer Lead) can implement both M1 (Auth) and M2 (Setup Wizard API) in clean sequential modules, or Worker 1 focuses on Auth + Setup API while Worker 3 simultaneously focuses on the SPA-интерфейс.
Rationale for this split:
- **Service Layer Specialist Worker**: Implements R1 (Argon2 Auth, sessions, middleware) AND R2 (Setup Wizard, persistence, diagnostic probes). All server-side files are cohesively compiled and tested together (`cargo check`, `cargo test`) without race conditions on `crates/kamish-web/src/api/mod.rs`.
- **SPA Specialist Worker**: Implements R3 (D-Link monochrome design, Login, Change Password, Setup Wizard with recovery, 9 sidebar sections, real API client and WS hooks, kill-ai-slop compliance, `npm run build`). Runs completely in parallel on `crates/kamish-web/frontend/src/`.

## Interface Contracts
### Auth API Contract
- `POST /api/v1/auth/login`:
  - Request: `{"username": "admin", "password": "..."}`
  - Response (200): `{"token": "uuid", "user": {"id": 1, "username": "admin", "must_change_password": true}}` + Cookie `kamish_session=uuid; HttpOnly; Path=/`
  - Response (401): `{"error": "INVALID_CREDENTIALS"}`
- `POST /api/v1/auth/change-password`:
  - Headers: `Authorization: Bearer <token>` or Cookie
  - Request: `{"old_password": "...", "new_password": "..."}`
  - Response (200): `{"status": "ok", "message": "Password changed successfully"}`
- `GET /api/v1/auth/session`:
  - Response (200): `{"authenticated": true, "user": {"id": 1, "username": "admin", "must_change_password": false}}`
- `POST /api/v1/auth/logout`:
  - Response (200): `{"status": "ok"}`

### Setup Wizard API Contract
- `GET /api/v1/setup/status`:
  - Response (200):
    ```json
    {
      "is_completed": false,
      "current_step": "auth",
      "steps": {
        "auth": {"status": "pending", "must_change_password": true},
        "network": {"status": "ok", "connected": true, "ssid": "..."},
        "audio": {"status": "ok", "ceiling": 85, "volume": 50},
        "voice": {"status": "ok", "stt_active": true, "tts_active": true},
        "ai": {"status": "ok", "cli_ready": true}
      },
      "degraded_components": []
    }
    ```
- `POST /api/v1/setup/step`:
  - Request: `{"step_id": "network", "data": {...}}`
  - Response (200): `{"status": "ok", "next_step": "audio"}`
- `POST /api/v1/setup/complete`:
  - Response (200): `{"status": "ok", "is_completed": true}`
- `POST /api/v1/setup/simulate-degradation`:
  - Request: `{"component": "ai", "degraded": true, "reason": "CLI timeout"}`
  - Response (200): `{"status": "ok", "recovery_step": "ai"}`
