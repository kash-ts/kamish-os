# Project: KAMISH Web Panel Overhaul & Authentication

## Architecture
- **Backend**: Rust 2021, Tokio, Axum 0.8, SQLx SQLite WAL, Argon2id password hashing, native HTTP cookie/header session management.
- **Frontend**: React 19, TypeScript, Vite, Tailwind CSS, Lucide icons, strict monochrome minimalist design (D-Link / MTS reference), native WebSocket client.
- **Data Flow**:
  - `POST /api/v1/auth/login` -> Argon2id verification -> SQLite `auth_sessions` -> returns session token & Set-Cookie.
  - `Axum Middleware` -> validates token on `/api/v1/*` routes; enforces `must_change_password` (blocks with 403 `PASSWORD_CHANGE_REQUIRED`).
  - `POST /api/v1/auth/change-password` -> verifies old password -> hashes new password -> clears `must_change_password` -> clears prior sessions.
  - `GET /api/v1/setup/status` -> reads `device_state` & runs non-blocking async subsystem probes -> returns status of auth, network, audio, voice, ai.
  - `POST /api/v1/setup/step` -> saves progress in `device_state` -> emits `setup_step_updated` on WebSocket.
  - `POST /api/v1/setup/complete` -> marks `setup:completed = true` in `device_state` -> emits `setup_completed` on WebSocket.
  - `POST /api/v1/setup/simulate-degradation` -> controlled degradation injection for zero-mock testing.
  - `Frontend`: Single-page app with `/login`, forced `/change-password`, `/setup` (with non-destructive recovery), and Dashboard with fixed D-Link sidebar (9 sections).

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | Auth Migration & Repository | SQLite schema for `users` and `auth_sessions`, Argon2id hashing | M1 | R1 |
| 2 | Auth Endpoints | `/login`, `/change-password`, `/logout`, `/session` | M1 | R1 |
| 3 | Axum Auth Middleware | Protects `/api/v1/*`, whitelists status & login, enforces `must_change_password` | M1 | R1 |
| 4 | Setup Wizard Persistence | SQLite `device_state` storage for wizard progress & completion | M2 | R2 |
| 5 | Setup Wizard Endpoints | `/status`, `/step`, `/complete`, `/diagnostics`, `/simulate-degradation` | M2 | R2 |
| 6 | Subsystem Health & Recovery | Live probes (STT/TTS, AI CLI, audio, network, hardware), recovery redirection | M2 | R2 |
| 7 | Monochrome D-Link UI Design | Light monochrome theme, high contrast borders, zero AI-slop | M3 | R3 |
| 8 | Login & Password Change Screens | Centered login card, blocking forced password change view | M3 | R3 |
| 9 | Setup Wizard Component | Step-by-step verification, non-destructive degradation recovery | M3 | R3 |
| 10 | Dashboard & Fixed Sidebar | 9 navigation sections, real-time status indicator, zero mocks | M3 | R3 |
| 11 | Full E2E & Quality Gate Verification | cargo check, cargo test, cargo fmt check, npm run build | M4 | All |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| M1 | Backend Auth Subsystem | Cargo `argon2`, `0002_auth_schema.sql`, `backend/src/auth/`, `/api/v1/auth/*`, middleware | none | IN_PROGRESS |
| M2 | Backend Setup Wizard | `SetupRepository`, `/api/v1/setup/*`, diagnostic probes, WS events | none | IN_PROGRESS |
| M3 | Frontend Minimalist Overhaul | D-Link layout, Login, Change Password, Setup Wizard, 9 views, zero-mock | none | IN_PROGRESS |
| M4 | Integration & Quality Gates | End-to-end verification, cargo test, npm run build, forensic audit | M1, M2, M3 | PLANNED |

## Code Layout & Write Boundaries
- **Worker 1 (Backend Auth - M1)** exclusively owns:
  - `backend/Cargo.toml`
  - `database/migrations/0002_auth_schema.sql`
  - `backend/src/auth/`
  - `backend/src/api/auth.rs`
  - `backend/tests/api_auth_test.rs`
- **Worker 2 (Backend Setup Wizard - M2)** exclusively owns:
  - `backend/src/setup/`
  - `backend/src/api/setup.rs`
  - `backend/tests/api_setup_test.rs`
- **Worker 3 (Frontend Minimalist Overhaul - M3)** exclusively owns:
  - `frontend/src/` (App.tsx, components/, pages/, api/, styles/)
  - `frontend/index.html`

Note on shared backend files:
`backend/src/api/mod.rs`, `backend/src/common/mod.rs`, and `backend/src/database/mod.rs`:
M1 wires auth routes & AppState. M2 wires setup routes.
To avoid merge collisions, Worker 1 (Backend Lead) can implement both M1 (Auth) and M2 (Setup Wizard API) in clean sequential modules, or Worker 1 focuses on Auth + Setup API while Worker 3 simultaneously focuses on Frontend UI!
Wait, that is an optimal architectural pattern:
- **Backend Specialist Worker**: Implements R1 (Argon2 Auth, sessions, middleware) AND R2 (Setup Wizard, persistence, diagnostic probes). All backend files are cohesively compiled and tested together (`cargo check`, `cargo test`) without race conditions on `backend/src/api/mod.rs`!
- **Frontend Specialist Worker**: Implements R3 (D-Link monochrome design, Login, Change Password, Setup Wizard with recovery, 9 sidebar sections, real API client and WS hooks, kill-ai-slop compliance, `npm run build`). Runs completely in parallel on `frontend/src/`!

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
