# Security Policy and Threat Model: Kamish OS

This document defines the security architecture, threat model, vulnerability reporting protocol, and strict behavioral guardrails for **Kamish OS** («Камыш») — an offline-first voice device running on ARM64 hardware (Orange Pi 4 Pro / Allwinner A733) with Seeed ReSpeaker XVF3800 microphone array and GT-112 acoustic system under Kamish Linux (Buildroot ARM64).

---

## 1. Scope & Security Philosophy

Kamish OS is designed under an **Offline-First, Zero-Trust Voice** paradigm:
1. **Local Privacy by Default**: Voice capturing, wake word spotting, speech-to-text (STT), and text-to-speech (TTS) execute 100% locally on the device. User voice streams and ambient audio NEVER leave the hardware.
2. **AI Sandboxing & Zero-Privilege Execution**: The Large Language Model (LLM) operates in a restricted sandbox through the Antigravity CLI (`agy`) or `MockAiClient`. The AI model is treated as untrusted user input and has **no direct access** to the shell, filesystem, system services, or private keys. All device modifications pass through a strictly typed, schema-validated **Tool Registry (14 tools)**.
3. **Decoupled Process Architecture (ADR-011)**:
   - **`kamish-core`**: High-privilege hardware daemon (ALSA audio, GPIO, I2C, BlueZ Bluetooth, NetworkManager, SQLite `core.db`). Listens strictly on a local UNIX domain socket (`/run/kamish/core.sock`, permission `0660`, user `kamish:kamish`). It has NO open network ports.
   - **`kamish-web`**: Low-privilege HTTP/WebSocket gateway (Axum `:8080`, Argon2id user sessions, `web.db`). Proxies authorized client requests to `kamish-core` over length-delimited JSON IPC (`kamish-ipc`). It has NO direct access to hardware devices or host secrets.
4. **Physical & Acoustic Protection**: Hard safety ceilings enforced in kernel/audio daemon layers to prevent transducer burnout, amplifier clipping, and battery thermal runaway.

---

## 2. Threat Model & Trust Boundaries

```
                      ┌───────────────────────────────────────────────┐
                      │             External / Cloud AI               │
                      │         (Antigravity CLI / agy LLM)           │
                      └──────────────────────▲────────────────────────┘
                                             │ JSON IPC (stdio)
                                             ▼
┌──────────────────┐    HTTP / WS     ┌──────────────┐   UNIX Socket   ┌──────────────┐   ALSA / I2C   ┌─────────────────┐
│ Web Browser / UI │ ───────────────> │  kamish-web  │ ──────────────> │ kamish-core  │ ─────────────> │ Hardware / Audio │
│  (React 19 SPA)  │ <─────────────── │ (Axum :8080) │ <────────────── │ (Daemon 24/7)│                │ (GT-112/XVF3800)│
└──────────────────┘  Cookie/Bearer   └──────────────┘    Length-JSON  └──────────────┘                └─────────────────┘
                                             │                                │
                                             ▼                                ▼
                                       SQLite web.db                    SQLite core.db
                                     (auth_sessions)                 (alarms, state, radio)
```

### Trust Boundary Definitions

| Boundary | Components Involved | Protection Mechanisms | Threat Mitigations |
|---|---|---|---|
| **Boundary 1: Public Network $\to$ Web API** | Client Browser $\leftrightarrow$ `kamish-web` | Argon2id auth, `auth_middleware`, mandatory default password rotation, rate limiting, Serde schema validation | Prevents unauthorized device takeover, brute-force attacks, CSRF, session hijacking |
| **Boundary 2: Web API $\to$ Core Daemon** | `kamish-web` $\leftrightarrow$ `kamish-core` | Local UNIX domain socket (`/run/kamish/core.sock`), length-delimited typed JSON protocol (`kamish-ipc`), systemd process sandboxing | Isolates web exploits from compromising hardware or accessing physical sensors |
| **Boundary 3: AI Model $\to$ System Core** | LLM (`agy`) $\leftrightarrow$ AI Orchestrator | Strictly typed **Tool Registry (14 tools)**, Serde parameter validation, no shell access, error-preserving return codes | Prevents prompt injection from executing arbitrary commands, reading host files, or exfiltrating data |
| **Boundary 4: Audio Capture $\to$ Speech Engines** | XVF3800 $\leftrightarrow$ VAD/Vosk/Piper | Local-only PCM buffer, stateful disarmed runner, zero cloud telemetry, local model directory (`/opt/kamish-models`) | Eliminates cloud wiretapping, rogue recording, and unauthorized audio exfiltration |
| **Boundary 5: Audio Output $\to$ Physical Speaker** | Audio Engine $\leftrightarrow$ GT-112 Amplifier | Programmatic **Safe Volume Ceiling = 85%**, thermal throttling monitoring ($\ge 80^\circ\text{C}$) | Prevents amplifier saturation/clipping, speaker coil burnout, and acoustic damage |

---

## 3. Mandatory Invariants (Rules for AI Agents and Developers)

> [!CRITICAL]
> **ATTENTION AI AGENTS & CONTRIBUTORS**: The rules below are non-negotiable architectural invariants.
> Any automated tool, AI coding assistant, or developer PR that violates these constraints MUST BE REJECTED.

### 🔴 NEVER (Categorically Forbidden Actions)

1. **NO Cloud STT or TTS Services**:
   - **Never** add cloud-based Speech-to-Text (e.g., OpenAI Whisper Cloud API, Google Cloud Speech) or Cloud TTS services. All transcription and voice synthesis MUST remain 100% on-device (Vosk, Piper, RHVoice, internal WAV synthesizer).
2. **NO Direct Shell or Arbitrary Execution for AI**:
   - **Never** grant the LLM process, Tool Registry, or AI Orchestrator direct access to `/bin/sh`, `/bin/bash`, `std::process::Command` with arbitrary strings, or filesystem inspection tools.
   - All actions must be discrete, schema-typed tools registered in `ai::tools::registry`.
3. **NO Secrets Leakage in Logs or API Responses**:
   - **Never** log, serialize, or transmit Wi-Fi passwords (WPA-PSK), VLESS / Reality private keys, session tokens, or password hashes into `tracing` logs, stdout/stderr, or REST API payloads.
   - Always mask sensitive credentials (e.g., `has_password: true`, `psk: "********"`).
4. **NO Bypassing Authentication or Default Password Enforcement**:
   - **Never** disable `auth_middleware` or remove the `must_change_password` check.
   - The default account (`admin`) MUST remain restricted with HTTP 403 `PASSWORD_CHANGE_REQUIRED` until the master password is changed via `/api/v1/auth/change-password`.
5. **NO Raising Volume Beyond Safe Ceiling (85%)**:
   - **Never** alter or bypass `SAFE_VOLUME_CEILING = 85` in `audio/output.rs` or `audio/manager.rs`.
   - Hardware amplifier GT-112 suffers severe square-wave clipping and coil damage if driven above 85% on 5V 3A supply rail.
6. **NO Raw SQL Concatenation**:
   - **Never** construct SQL queries using string formatting or interpolation (`format!("SELECT ... WHERE x = '{}'", input)`). Always use SQLx parameterized queries (`sqlx::query!`, `query_as!`, or `.bind()`).
7. **NO Degrading or Skipping Security Tests**:
   - **Never** disable (`#[ignore]`), comment out, or weaken assertion logic in security and authentication test suites (`tests/api_auth_test.rs`, `tests/hardware_telemetry_test.rs`).

---

### 🟢 ALWAYS (Mandatory Requirements)

1. **Enforce Principle of Least Privilege**:
   - Run `kamish-core` and `kamish-web` as non-root service user `kamish` under systemd sandbox profiles (`ProtectSystem=strict`, `ProtectHome=true`, `PrivateTmp=true`, `NoNewPrivileges=true`).
2. **Enforce Offline-First Resilience**:
   - Ensure alarms, reminders, local audio playback, system diagnostics, and volume controls remain 100% functional even when internet access or VPN is completely unavailable.
3. **Validate All Serialized Input**:
   - Every HTTP and IPC payload must be strictly deserialized into Rust structs with Serde, enforcing strict bounds on strings, arrays, and numeric values.
4. **Transparent Tool Error Reporting**:
   - If a tool fails (`ToolCallResult.success = false`), return the concrete error message to the AI agent. Never synthesize a false success response.
5. **Mask Audio in Automated Testing**:
   - Keep `KAMISH_MUTE_AUDIO=1` active during all CI/test executions to protect developer environments from accidental audio playback.

---

## 4. Subsystem Security Specifications

### 4.1 Authentication & Session Management (`crates/kamish-web/src/auth/`)
* **Hashing Algorithm**: **Argon2id** (`argon2` crate) configured with RFC 9106 recommended parameters:
  - Memory cost: 64 MB (or 19 MB embedded profile)
  - Time iterations: 3 passes
  - Parallelism: 1 lane
  - Cryptographically secure random 16-byte salt per hash.
* **Session Storage**: Ephemeral session tokens (UUID v4) stored in SQLite (`auth_sessions`) with:
  - Time-to-Live (TTL): 7 days with sliding refresh.
  - Revocation on logout (`POST /api/v1/auth/logout`) and password change.
* **Cookie Security**:
  - `Set-Cookie: kamish_session=<token>; HttpOnly; SameSite=Lax; Path=/; Max-Age=604800`
* **First-Boot Blocking Policy**:
  - When `users.must_change_password = true`, `auth_middleware` intercepts every route except `/api/v1/auth/change-password`, `/api/v1/auth/session`, `/api/v1/auth/logout`, and public `/api/v1/status`.
  - Any attempt to access business endpoints returns **HTTP 403 Forbidden** with payload `{"code": "PASSWORD_CHANGE_REQUIRED"}`.

### 4.2 AI Tool Sandboxing & Prompt Injection Defense (`crates/kamish-core/src/ai/`)
* **Strict Tool Registry**: The LLM interacts with the physical speaker solely through 14 predefined tool definitions:
  - `create_alarm`, `delete_alarm`, `list_alarms`, `snooze_alarm`
  - `create_reminder`, `list_reminders`
  - `set_volume` (clamped to $[0, 85]$)
  - `play_music`, `pause_music`, `resume_music`, `stop_music`, `search_music`, `get_current_track`
  - `get_device_status`
* **Zero Host Access**: The CLI process runs with restricted environmental variables. Stdin/stdout communication uses strict line-buffered JSON messages.
* **Prompt Injection Resilience**: Even if malicious text is fed into the AI conversation (e.g., via spoken voice or audio metadata), the AI has no capability to execute shell commands, delete databases, or modify system files.

### 4.3 Network & Setup Mode Security (`crates/kamish-core/src/network/`)
* **Setup Access Point (`kamish-setup`)**:
  - Subnet: `192.168.4.1/24`.
  - Purpose: Initial headless onboarding.
  - Automatic Teardown: As soon as the device successfully connects to a target Wi-Fi network, the AP interface is stopped immediately.
* **VLESS / Reality Tunneling (`crates/kamish-core/src/vpn/`)**:
  - Outbound music streaming traffic uses Happ/VLESS Reality tunnel to circumvent regional audio stream blocks.
  - Private keys and short IDs are stored with restrictive file permissions (`0600`) and never exposed in `GET /api/v1/vpn/status`.

### 4.4 Hardware & Acoustic Protection (`crates/kamish-core/src/audio/`, `system/`)
* **Volume Limiter**:
  - Software ceiling: `85%`.
  - Calls to `/api/v1/system/audio/volume` with values $>85$ are automatically clamped to 85.
* **Thermal Supervision**:
  - Sysfs thermal monitor: `/sys/class/thermal/thermal_zone0/temp`.
  - Warning threshold: $\ge 70^\circ\text{C}$ (emits event `hardware_thermal_warning`).
  - Throttling threshold: $\ge 80^\circ\text{C}$ (emits `hardware_throttling_alert`, reduces CPU frequency / limits audio power).

---

## 5. Security Verification & Auditing Checklist

Before merging code or deploying firmware, verify the following gates:

```bash
# 1. Type safety and dependency vulnerability audit
cargo check --workspace
cargo audit # optional advisory audit

# 2. Complete test suite verification (100% green required)
wsl bash -c "cd /mnt/m/Project/Kamish-os/radiant-bardeen && export CARGO_TARGET_DIR=\$HOME/.cache/kamish-target && cargo test --workspace"

# 3. Secret leakage check (must return empty)
git grep -E "password|secret|token|private_key|keyring" -- crates/ \
  | grep -v "test" | grep -v "schema.sql" | grep -v "docs/" | grep -E "\"(ghp_|ey|BEGIN PRIVATE KEY)"

# 4. SPA static bundle build integrity
cd crates/kamish-web/frontend && npm run build
```

---

## 6. Reporting a Vulnerability

We take the security of Kamish OS and its users seriously. If you discover a security vulnerability, please follow coordinated disclosure:

1. **Do NOT open a public GitHub issue** for undisclosed security vulnerabilities.
2. Send a detailed report to the security contact:
   - **Email**: `security@kamish-os.org` (or directly via repository security advisories)
3. Please include:
   - Description of the vulnerability and attack vector.
   - Proof of concept (PoC) script or reproduction steps.
   - Potential impact on physical hardware, network, or user privacy.
4. **SLA**: We acknowledge receipt of reports within **48 hours** and provide a patch timeline within **7 business days**.
