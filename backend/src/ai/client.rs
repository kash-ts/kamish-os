//! AI Client implementations:
//! - `AiClient` trait
//! - `CliAiClient` (Antigravity CLI wrapper via stdin/stdout)
//! - `MockAiClient` (Intelligent Fallback Engine for offline & tests)
//! - `AiOrchestrator` (unified entrypoint with fallback & session persistence)

use crate::ai::models::{AiResponse, AiSessionMetadata, ToolCallRequest};
use crate::ai::tools::ToolRegistry;
use crate::common::{AppError, AppResult, AppState};
use crate::database::DbPool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{debug, warn};
use uuid::Uuid;

/// AI Client trait for model inference and dispatch
#[allow(async_fn_in_trait)]
pub trait AiClient: Send + Sync {
    async fn ask(
        &self,
        prompt: &str,
        session_id: Option<&str>,
        state: &AppState,
    ) -> AppResult<AiResponse>;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiReadiness {
    pub ready: bool,
    pub mode: String, // "cli" | "mock"
    pub cli_path: String,
    pub cli_version: Option<String>,
    pub error_reason: Option<String>,
}

// -----------------------------------------------------------------------------
// CLI AI Client (Antigravity CLI Wrapper)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CliAiClient {
    pub cli_path: String,
    pub timeout_duration: Duration,
    pub registry: Arc<ToolRegistry>,
}

/// Default system prompt describing the assistant's persona, behavior, and all 14 tools.
pub const DEFAULT_SYSTEM_PROMPT: &str = r#"Ты — голосовой и текстовый ассистент умной колонки «Камыш».
Твоя цель — вежливо, кратко и естественно помогать пользователю управлять колонкой.

ПРАВИЛА ПОВЕДЕНИЯ:
1. Отвечай кратко, чётко, дружелюбно и на чистом русском языке без лишнего формализма и без служебных префиксов. Твои ответы озвучиваются вслух через динамик, поэтому избегай громоздких списков и технических пояснений.
2. Не повторяй вслух имя системы или технические префиксы (запрещено писать: "Ответ Antigravity CLI: получен запрос...").
3. Если пользователь просто здоровается («Привет», «Здравствуйте») или спрашивает «Кто ты?», представься как ассистент умной колонки Камыш и кратко скажи, чем можешь помочь.
4. Для любых действий с колонкой вызывай соответствующий инструмент из списка через Tool Calls.

ДОСТУПНЫЕ ИНСТРУМЕНТЫ (14 инструментов):
- create_alarm(time: string, recurrence?: string, label?: string) — установить будильник (формат времени HH:MM, recurrence: once|daily|weekdays|weekends).
- delete_alarm(id: string) — удалить будильник по ID.
- list_alarms(status?: string) — показать список будильников.
- snooze_alarm(minutes?: number) — отложить текущий будильник на N минут.
- create_reminder(text: string, time?: string, minutes?: number) — создать напоминание.
- list_reminders(status?: string) — показать список активных напоминаний.
- set_volume(volume: number) — установить уровень громкости динамика от 0 до 100% (с защитным лимитом 85%).
- play_music(query?: string) — запустить воспроизведение музыки (поиск по названию или радио).
- pause_music() — приостановить воспроизведение музыки.
- resume_music() — возобновить воспроизведение после паузы.
- stop_music() — полностью остановить музыку.
- search_music(query: string) — найти композиции в каталоге.
- get_current_track() — узнать информацию о текущем треке.
- get_device_status() — проверить статус, температуру и здоровье подсистем умной колонки.
"#;

#[derive(Debug, Serialize, Deserialize)]
struct CliInputPayload {
    session_id: String,
    prompt: String,
    system_prompt: String,
    tools: Vec<crate::ai::models::ToolDeclaration>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CliOutputPayload {
    #[serde(default)]
    response: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ToolCallRequest>>,
    #[serde(default)]
    status: Option<String>,
}

impl CliAiClient {
    pub fn new(cli_path: String, registry: Arc<ToolRegistry>) -> Self {
        Self {
            cli_path,
            timeout_duration: Duration::from_secs(15),
            registry,
        }
    }

    /// Resolves the CLI binary path checking explicit path, PATH environment variable,
    /// and project/system fallback locations (e.g. scripts/agy-mock, /opt/kamish/bin/agy).
    pub fn resolve_executable(&self) -> PathBuf {
        let path = Path::new(&self.cli_path);
        if path.is_file() {
            return path.to_path_buf();
        }

        // Check PATH
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                let candidate = dir.join(&self.cli_path);
                if candidate.is_file() {
                    return candidate;
                }
            }
        }

        // Check fallback locations when looking for default "agy"
        if self.cli_path == "agy" {
            let candidates = [
                Path::new("scripts/agy-mock"),
                Path::new("../scripts/agy-mock"),
                Path::new("/opt/kamish/bin/agy"),
                Path::new("/usr/local/bin/agy"),
                Path::new("/usr/bin/agy"),
            ];

            for candidate in candidates {
                if candidate.is_file() {
                    return candidate.to_path_buf();
                }
            }
        }

        path.to_path_buf()
    }

    /// Health probe: checks if CLI binary exists and responds to --version
    pub async fn probe(&self) -> Result<String, String> {
        let exe = self.resolve_executable();
        let output = tokio::time::timeout(
            Duration::from_secs(3),
            Command::new(&exe).arg("--version").output(),
        )
        .await
        .map_err(|_| format!("Таймаут проверки CLI '{}' (3 сек)", self.cli_path))?
        .map_err(|e| {
            format!(
                "Бинарник CLI '{}' не найден или не запускается: {}",
                self.cli_path, e
            )
        })?;

        if output.status.success() {
            let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(if ver.is_empty() {
                "Antigravity CLI (версия не указана)".to_string()
            } else {
                ver
            })
        } else {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!(
                "CLI процесс вернул код ошибки {:?}: {}",
                output.status.code(),
                if err.is_empty() {
                    "неизвестная ошибка"
                } else {
                    &err
                }
            ))
        }
    }
}

impl AiClient for CliAiClient {
    async fn ask(
        &self,
        prompt: &str,
        session_id: Option<&str>,
        state: &AppState,
    ) -> AppResult<AiResponse> {
        let sid = session_id
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let payload = CliInputPayload {
            session_id: sid.clone(),
            prompt: prompt.to_string(),
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
            tools: self.registry.list_declarations(),
        };

        let json_input = serde_json::to_string(&payload)
            .map_err(|e| AppError::Config(format!("Failed to serialize CLI input: {e}")))?;

        let exe = self.resolve_executable();
        debug!(
            "Invoking AI CLI binary '{:?}' with session_id='{}'",
            exe, sid
        );

        // First attempt native Antigravity CLI invocation with args (-p ... --output-format json)
        // If native invocation fails or binary is legacy stdin/stdout, fallback gracefully.
        let full_prompt = format!(
            "{}\n\nКоманда пользователя:\n{}",
            DEFAULT_SYSTEM_PROMPT, prompt
        );

        let output = match tokio::time::timeout(
            self.timeout_duration,
            Command::new(&exe)
                .arg("-p")
                .arg(&full_prompt)
                .arg("--output-format")
                .arg("json")
                .output(),
        )
        .await
        {
            Ok(Ok(out)) if out.status.success() => out,
            _ => {
                // Fallback to stdin JSON pipe mode (for custom wrappers / test harness)
                let mut child = Command::new(&exe)
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .map_err(|e| {
                        AppError::System(format!(
                            "Failed to spawn AI CLI process '{}': {e}",
                            self.cli_path
                        ))
                    })?;

                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(json_input.as_bytes()).await;
                    let _ = stdin.flush().await;
                }

                tokio::time::timeout(self.timeout_duration, child.wait_with_output())
                    .await
                    .map_err(|_| {
                        AppError::System(format!(
                            "AI CLI process timed out after {}s",
                            self.timeout_duration.as_secs()
                        ))
                    })?
                    .map_err(|e| {
                        AppError::System(format!("AI CLI process execution failed: {e}"))
                    })?
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::System(format!(
                "AI CLI process exited with code {:?}: {}",
                output.status.code(),
                stderr.trim()
            )));
        }

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout_str.trim();

        // Try parsing JSON response from CLI
        let parsed: CliOutputPayload = match serde_json::from_str(trimmed) {
            Ok(p) => p,
            Err(_) => {
                // If output was raw text, wrap it into response
                CliOutputPayload {
                    response: Some(trimmed.to_string()),
                    tool_calls: None,
                    status: Some("SUCCESS".to_string()),
                }
            }
        };

        let mut executed_tools = Vec::new();
        let mut failed_errors = Vec::new();

        if let Some(tool_calls) = parsed.tool_calls {
            for call in tool_calls {
                let res = self
                    .registry
                    .execute(&call.name, call.arguments, state)
                    .await;
                if !res.success {
                    if let Some(ref err) = res.error {
                        failed_errors.push(format!("Инструмент '{}': {}", res.tool, err));
                    }
                }
                executed_tools.push(res);
            }
        }

        // Error transparency: if any tool failed, do NOT deceive the user with optimistic success!
        let resp_text = if !failed_errors.is_empty() {
            format!(
                "Не удалось выполнить действие: {}",
                failed_errors.join("; ")
            )
        } else {
            parsed
                .response
                .unwrap_or_else(|| "Команда успешно обработана.".to_string())
        };

        Ok(AiResponse {
            response: resp_text,
            tool_calls: executed_tools,
            session_id: sid,
        })
    }
}

// -----------------------------------------------------------------------------
// Mock AI Client (Intelligent Fallback Engine for Russian & English commands)
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MockAiClient {
    pub registry: Arc<ToolRegistry>,
}

impl MockAiClient {
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self { registry }
    }

    /// Extract 24h time in HH:MM format from text
    fn extract_time(text: &str) -> Option<String> {
        let tokens = text.split_whitespace();
        for token in tokens {
            let clean = token.trim_matches(|c: char| !c.is_numeric() && c != ':');
            if let Some((h_str, m_str)) = clean.split_once(':') {
                if let (Ok(h), Ok(m)) = (h_str.parse::<u32>(), m_str.parse::<u32>()) {
                    if h < 24 && m < 60 {
                        return Some(format!("{:02}:{:02}", h, m));
                    }
                }
            }
        }
        None
    }

    /// Extract recurrence pattern if mentioned
    fn extract_recurrence(text: &str) -> Option<String> {
        let lower = text.to_lowercase();
        if lower.contains("будням") || lower.contains("weekdays") {
            Some("weekdays".to_string())
        } else if lower.contains("выходным") || lower.contains("weekends") {
            Some("weekends".to_string())
        } else if lower.contains("каждый день")
            || lower.contains("ежедневно")
            || lower.contains("daily")
        {
            Some("daily".to_string())
        } else {
            None
        }
    }

    /// Extract label from prompt if specified
    fn extract_label(text: &str) -> Option<String> {
        let lower = text.to_lowercase();
        if let Some(pos) = lower.find("с меткой ") {
            let rem = &text[pos + "с меткой ".len()..];
            let label = rem.trim();
            if !label.is_empty() {
                return Some(label.to_string());
            }
        }
        if let Some(pos) = lower.find("label ") {
            let rem = &text[pos + "label ".len()..];
            let label = rem.trim();
            if !label.is_empty() {
                return Some(label.to_string());
            }
        }
        None
    }

    /// Strip matching case-insensitive prefix
    fn strip_prefix<'a>(text: &'a str, prefixes: &[&str]) -> &'a str {
        let lower = text.to_lowercase();
        for prefix in prefixes {
            if lower.starts_with(prefix) {
                return text[prefix.len()..].trim();
            }
        }
        text.trim()
    }

    /// Extract integer number (e.g. for volume or minutes)
    fn extract_number(text: &str) -> Option<i64> {
        for word in text.split_whitespace() {
            let clean = word.trim_matches(|c: char| !c.is_numeric());
            if let Ok(num) = clean.parse::<i64>() {
                return Some(num);
            }
        }
        None
    }
}

impl AiClient for MockAiClient {
    async fn ask(
        &self,
        prompt: &str,
        session_id: Option<&str>,
        state: &AppState,
    ) -> AppResult<AiResponse> {
        let sid = session_id
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let trimmed = prompt.trim();
        if trimmed.is_empty() {
            return Err(AppError::Config("Prompt cannot be empty".to_string()));
        }

        let lower = trimmed.to_lowercase();
        let mut tool_calls = Vec::new();
        let response_text: String;

        // 1. Alarm creation: "поставь будильник на 07:30", "разбуди в 8:00", "set alarm for 07:30"
        if (lower.contains("будильник") || lower.contains("разбуди") || lower.contains("alarm"))
            && (lower.contains("поставь")
                || lower.contains("установи")
                || lower.contains("заведи")
                || lower.contains("на")
                || lower.contains("set")
                || lower.contains("create"))
            && !lower.contains("удали")
            && !lower.contains("delete")
            && !lower.contains("покажи")
            && !lower.contains("список")
            && !lower.contains("отложи")
            && !lower.contains("snooze")
        {
            if let Some(time) = Self::extract_time(&lower) {
                let recurrence = Self::extract_recurrence(&lower);
                let label = Self::extract_label(trimmed);

                let mut args = json!({ "time": time });
                if let Some(ref rec) = recurrence {
                    args["recurrence"] = json!(rec);
                }
                if let Some(ref lbl) = label {
                    args["label"] = json!(lbl);
                }

                let call_res = self.registry.execute("create_alarm", args, state).await;
                let success = call_res.success;
                tool_calls.push(call_res);

                if success {
                    let mut desc = format!("Будильник установлен на {}", time);
                    if let Some(rec) = recurrence {
                        desc.push_str(&format!(" ({})", rec));
                    }
                    if let Some(lbl) = label {
                        desc.push_str(&format!(" с меткой \"{}\"", lbl));
                    }
                    desc.push('.');
                    response_text = desc;
                } else {
                    response_text =
                        "Не удалось установить будильник. Проверьте формат времени.".to_string();
                }
            } else {
                response_text =
                    "Пожалуйста, укажите время для будильника в формате ЧЧ:ММ (например, 07:30)."
                        .to_string();
            }
        }
        // 2. Alarm deletion: "удали будильник <id>", "delete alarm <id>"
        else if (lower.contains("будильник") || lower.contains("alarm"))
            && (lower.contains("удали")
                || lower.contains("delete")
                || lower.contains("сними")
                || lower.contains("remove"))
        {
            // Find uuid or id token
            let id = trimmed
                .split_whitespace()
                .find(|w| {
                    w.len() >= 8
                        && (w.contains('-') || w.chars().all(|c| c.is_ascii_alphanumeric()))
                })
                .map(|w| w.trim_matches(|c: char| c == ',' || c == '.' || c == '"' || c == '\''));

            if let Some(id) = id {
                let call_res = self
                    .registry
                    .execute("delete_alarm", json!({ "id": id }), state)
                    .await;
                let success = call_res.success;
                tool_calls.push(call_res);

                if success {
                    response_text = format!("Будильник с ID {} успешно удален.", id);
                } else {
                    response_text = format!("Будильник с ID {} не найден.", id);
                }
            } else {
                response_text =
                    "Укажите ID будильника для удаления (например, удали будильник 12345)."
                        .to_string();
            }
        }
        // 3. Alarm snooze: "отложи будильник", "snooze alarm"
        else if lower.contains("отложи") || lower.contains("snooze") {
            let minutes = Self::extract_number(&lower).unwrap_or(10);
            // Search for alarm ID or pick first enabled alarm
            let alarms = crate::scheduler::repository::list_alarms(&state.db)
                .await
                .unwrap_or_default();
            let target_alarm = alarms.into_iter().find(|a| a.enabled);

            if let Some(alarm) = target_alarm {
                let call_res = self
                    .registry
                    .execute(
                        "snooze_alarm",
                        json!({ "id": alarm.id, "minutes": minutes }),
                        state,
                    )
                    .await;
                let success = call_res.success;
                let error = call_res.error.clone();
                tool_calls.push(call_res);

                if success {
                    response_text = format!("Будильник отложен на {} минут.", minutes);
                } else {
                    response_text = format!(
                        "Не удалось отложить будильник: {}.",
                        error.unwrap_or_else(|| "внутренняя ошибка".to_string())
                    );
                }
            } else {
                response_text = "Нет активных будильников для откладывания.".to_string();
            }
        }
        // 4. List alarms: "покажи будильники", "список будильников", "list alarms"
        else if (lower.contains("будильник") || lower.contains("alarm"))
            && (lower.contains("покажи")
                || lower.contains("список")
                || lower.contains("какие")
                || lower.contains("list")
                || lower.contains("show")
                || lower.trim() == "будильники")
        {
            let call_res = self.registry.execute("list_alarms", json!({}), state).await;

            let count = call_res.result.as_array().map(|a| a.len()).unwrap_or(0);

            tool_calls.push(call_res);
            if count == 0 {
                response_text = "У вас пока нет настроенных будильников.".to_string();
            } else {
                response_text = format!("У вас настроено {} будильников.", count);
            }
        }
        // 5. Create reminder: "напомни через 15 минут выпить воды", "remind me in 10 minutes to ..."
        else if lower.contains("напомни") || lower.contains("remind") {
            let minutes = Self::extract_number(&lower).unwrap_or(15);
            // Extract text after time or reminder
            let mut text = "Напоминание".to_string();
            if let Some(pos) = lower.find("минут") {
                let rem = &trimmed[pos + "минут".len()..];
                let clean =
                    rem.trim_start_matches(|c: char| c.is_whitespace() || c == ',' || c == '-');
                if !clean.is_empty() {
                    text = clean.to_string();
                }
            } else if let Some(pos) = lower.find("minutes to ") {
                let rem = &trimmed[pos + "minutes to ".len()..];
                if !rem.trim().is_empty() {
                    text = rem.trim().to_string();
                }
            }

            let args = json!({
                "trigger_time": format!("+{minutes}"),
                "text": text,
            });

            let call_res = self.registry.execute("create_reminder", args, state).await;
            let success = call_res.success;
            tool_calls.push(call_res);

            if success {
                response_text =
                    format!("Напоминание создано: \"{}\" через {} минут.", text, minutes);
            } else {
                response_text = "Не удалось создать напоминание.".to_string();
            }
        }
        // 6. List reminders: "покажи напоминания", "list reminders"
        else if (lower.contains("напоминани") || lower.contains("remind"))
            && (lower.contains("покажи")
                || lower.contains("список")
                || lower.contains("какие")
                || lower.contains("list")
                || lower.contains("show"))
        {
            let call_res = self
                .registry
                .execute("list_reminders", json!({ "status": "pending" }), state)
                .await;

            let count = call_res.result.as_array().map(|a| a.len()).unwrap_or(0);

            tool_calls.push(call_res);
            if count == 0 {
                response_text = "У вас нет активных напоминаний.".to_string();
            } else {
                response_text = format!("У вас {} активных напоминаний.", count);
            }
        }
        // 7. Volume: "громкость 70", "поставь громкость 50", "set volume to 80"
        else if lower.contains("громкость")
            || lower.contains("volume")
            || lower.contains("громче")
            || lower.contains("тише")
        {
            let target_vol = if lower.contains("громче") {
                let current = state.media.get_volume();
                (current as i64 + 10).min(100)
            } else if lower.contains("тише") {
                let current = state.media.get_volume();
                (current as i64 - 10).max(0)
            } else {
                Self::extract_number(&lower).unwrap_or(50)
            };

            let call_res = self
                .registry
                .execute("set_volume", json!({ "volume": target_vol }), state)
                .await;
            let success = call_res.success;
            let error = call_res.error.clone();
            tool_calls.push(call_res);

            if success {
                response_text = format!("Громкость установлена на {}%.", target_vol);
            } else {
                response_text = format!(
                    "Не удалось изменить громкость: {}.",
                    error.unwrap_or_else(|| "недопустимое значение".to_string())
                );
            }
        }
        // 8. Music Pause: "пауза", "pause"
        else if lower == "пауза"
            || lower.contains("приостанови")
            || lower == "pause"
            || lower.contains("pause music")
        {
            let call_res = self.registry.execute("pause_music", json!({}), state).await;
            tool_calls.push(call_res);
            response_text = "Воспроизведение приостановлено.".to_string();
        }
        // 9. Music Resume: "продолжи", "возобнови", "resume"
        else if lower.contains("продолжи") || lower.contains("возобнови") || lower == "resume"
        {
            let call_res = self
                .registry
                .execute("resume_music", json!({}), state)
                .await;
            tool_calls.push(call_res);
            response_text = "Воспроизведение возобновлено.".to_string();
        }
        // 10. Music Stop: "стоп", "stop", "останови музыку", "выключи музыку"
        else if lower == "стоп"
            || lower.contains("останови")
            || lower.contains("выключи музыку")
            || lower == "stop"
        {
            let call_res = self.registry.execute("stop_music", json!({}), state).await;
            tool_calls.push(call_res);
            response_text = "Воспроизведение остановлено.".to_string();
        }
        // 11. Music Search: "найди музыку <query>", "search music <query>"
        else if lower.contains("найди музыку")
            || lower.contains("поиск музыки")
            || lower.contains("найди песню")
            || lower.contains("search music")
        {
            let prefixes = [
                "найди музыку",
                "поиск музыки",
                "найди песню",
                "search music",
                "search song",
                "search",
            ];
            let query = Self::strip_prefix(trimmed, &prefixes);

            let call_res = self
                .registry
                .execute("search_music", json!({ "query": query }), state)
                .await;
            tool_calls.push(call_res);
            response_text = format!("Поиск музыки по запросу \"{}\" завершен.", query);
        }
        // 12. Current Track: "что сейчас играет", "what is playing", "текущий трек"
        else if lower.contains("что сейчас играет")
            || lower.contains("текущий трек")
            || lower.contains("what is playing")
        {
            let call_res = self
                .registry
                .execute("get_current_track", json!({}), state)
                .await;
            let status = call_res
                .result
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("stopped")
                .to_string();
            tool_calls.push(call_res);

            if status == "playing" {
                response_text = "Сейчас играет трек.".to_string();
            } else {
                response_text = "Сейчас ничего не играет.".to_string();
            }
        }
        // 13. Play Music: "включи музыку", "поставь музыку", "play music", "включи Queen"
        else if lower.contains("включи") || lower.contains("поставь") || lower.contains("play")
        {
            let prefixes = [
                "включи музыку",
                "поставь музыку",
                "включи песню",
                "поставь песню",
                "включи трек",
                "поставь трек",
                "включи",
                "поставь",
                "play music",
                "play song",
                "play track",
                "play",
            ];
            let query = Self::strip_prefix(trimmed, &prefixes);

            let args = if query.is_empty() {
                json!({})
            } else {
                json!({ "query": query })
            };

            let call_res = self.registry.execute("play_music", args, state).await;
            let success = call_res.success;
            let error = call_res.error.clone();
            tool_calls.push(call_res);

            if success {
                if query.is_empty() {
                    response_text = "Включаю музыку.".to_string();
                } else {
                    response_text = format!("Включаю музыку по запросу \"{}\".", query);
                }
            } else {
                response_text = format!(
                    "Не удалось включить музыку: {}.",
                    error.unwrap_or_else(|| "источник недоступен".to_string())
                );
            }
        }
        // 14. Device Status: "статус устройства", "состояние системы", "device status", "system status", "как дела"
        else if lower.contains("статус")
            || lower.contains("состояние")
            || lower.contains("status")
            || lower.contains("как дела")
        {
            let call_res = self
                .registry
                .execute("get_device_status", json!({}), state)
                .await;
            tool_calls.push(call_res);
            response_text = "Все системы умной колонки работают штатно.".to_string();
        }
        // Conversational fallback
        else if lower.contains("кто ты")
            || lower.contains("ты кто")
            || lower.contains("как тебя зовут")
        {
            response_text = "Я голосовой ассистент умной колонки Камыш. Помогаю ставить будильники и напоминания, включать музыку, регулировать громкость и сообщать о состоянии устройства. Чем могу помочь?".to_string();
        } else if lower.contains("привет")
            || lower.contains("здравствуй")
            || lower == "hello"
            || lower == "hi"
        {
            response_text =
                "Привет! Я голосовой ассистент умной колонки Камыш. Чем могу помочь?".to_string();
        } else {
            response_text = format!(
                "Я готов помочь! Вы можете сказать: \"поставь будильник на 07:30\", \"напомни через 10 минут выпить воды\", \"поставь громкость 60\", \"включи музыку\" или спросить \"статус устройства\"."
            );
        }

        Ok(AiResponse {
            response: response_text,
            tool_calls,
            session_id: sid,
        })
    }
}

// -----------------------------------------------------------------------------
// Unified AI Orchestrator with CLI + Fallback & Session Persistence
// -----------------------------------------------------------------------------

pub struct AiOrchestrator {
    pub registry: Arc<ToolRegistry>,
    cli_client: Arc<tokio::sync::RwLock<CliAiClient>>,
    mock_client: MockAiClient,
    cli_enabled: Arc<tokio::sync::RwLock<bool>>,
    db: DbPool,
}

impl AiOrchestrator {
    pub fn new(db: DbPool, cli_path: String, cli_enabled: bool) -> Self {
        let registry = Arc::new(ToolRegistry::new());
        let cli_client = Arc::new(tokio::sync::RwLock::new(CliAiClient::new(
            cli_path,
            registry.clone(),
        )));
        let mock_client = MockAiClient::new(registry.clone());

        Self {
            registry,
            cli_client,
            mock_client,
            cli_enabled: Arc::new(tokio::sync::RwLock::new(cli_enabled)),
            db,
        }
    }

    /// Check AI readiness: verifies CLI binary if in CLI mode, or returns ready for Mock mode
    pub async fn get_readiness(&self) -> AiReadiness {
        let enabled = *self.cli_enabled.read().await;
        let cli_guard = self.cli_client.read().await;
        let cli_path = cli_guard.cli_path.clone();

        if !enabled {
            return AiReadiness {
                ready: true,
                mode: "mock".to_string(),
                cli_path,
                cli_version: None,
                error_reason: None,
            };
        }

        match cli_guard.probe().await {
            Ok(ver) => AiReadiness {
                ready: true,
                mode: "cli".to_string(),
                cli_path,
                cli_version: Some(ver),
                error_reason: None,
            },
            Err(e) => AiReadiness {
                ready: false,
                mode: "cli".to_string(),
                cli_path,
                cli_version: None,
                error_reason: Some(e),
            },
        }
    }

    /// Update AI mode ("cli" | "mock") and CLI path dynamically
    pub async fn update_config(&self, mode: &str, new_cli_path: Option<String>) -> AiReadiness {
        let mut enabled_guard = self.cli_enabled.write().await;
        if mode.eq_ignore_ascii_case("cli") {
            *enabled_guard = true;
        } else if mode.eq_ignore_ascii_case("mock") {
            *enabled_guard = false;
        }

        if let Some(path) = new_cli_path {
            if !path.trim().is_empty() {
                let mut cli_guard = self.cli_client.write().await;
                cli_guard.cli_path = path.trim().to_string();
            }
        }

        drop(enabled_guard);
        self.get_readiness().await
    }

    /// Load or create session metadata from SQLite `ai_sessions` table
    async fn load_session(&self, session_id: &str) -> AppResult<AiSessionMetadata> {
        let row = sqlx::query_as::<_, (Option<String>,)>(
            "SELECT metadata_json FROM ai_sessions WHERE id = ?",
        )
        .bind(session_id)
        .fetch_optional(&self.db)
        .await
        .map_err(AppError::Database)?;

        match row {
            Some((Some(json_str),)) => {
                let meta = serde_json::from_str::<AiSessionMetadata>(&json_str).unwrap_or_default();
                Ok(meta)
            }
            _ => Ok(AiSessionMetadata::default()),
        }
    }

    /// Persist updated session metadata to SQLite `ai_sessions` table
    async fn save_session(&self, session_id: &str, meta: &AiSessionMetadata) -> AppResult<()> {
        let json_str = serde_json::to_string(meta)
            .map_err(|e| AppError::Config(format!("Failed to serialize session metadata: {e}")))?;

        sqlx::query(
            r#"
            INSERT INTO ai_sessions (id, created_at, metadata_json)
            VALUES (?, CURRENT_TIMESTAMP, ?)
            ON CONFLICT(id) DO UPDATE SET metadata_json = excluded.metadata_json
            "#,
        )
        .bind(session_id)
        .bind(&json_str)
        .execute(&self.db)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    /// Execute user prompt, dispatching tools and managing session persistence
    pub async fn ask(
        &self,
        prompt: &str,
        session_id: Option<&str>,
        state: &AppState,
    ) -> AppResult<AiResponse> {
        let trimmed = prompt.trim();
        if trimmed.is_empty() {
            return Err(AppError::Config("Prompt cannot be empty".to_string()));
        }

        let sid = session_id
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let readiness = self.get_readiness().await;

        // If in CLI mode and readiness check failed, REFUSE to answer and require setup
        if readiness.mode == "cli" && !readiness.ready {
            let reason = readiness
                .error_reason
                .unwrap_or_else(|| "бинарник CLI не найден или не исполняем".to_string());
            warn!(
                "Speaker refusing command because AI CLI setup is incomplete: {}",
                reason
            );
            return Err(AppError::System(format!(
                "Настройка колонки не завершена: {}. Проверьте статус в веб-панели управления.",
                reason
            )));
        }

        // Dispatch based on active mode
        let ai_response = if readiness.mode == "cli" {
            let cli_guard = self.cli_client.read().await;
            cli_guard.ask(trimmed, Some(&sid), state).await?
        } else {
            self.mock_client.ask(trimmed, Some(&sid), state).await?
        };

        // Persist session history
        if let Ok(mut meta) = self.load_session(&sid).await {
            meta.add_user_message(trimmed);
            meta.add_assistant_message(&ai_response.response, ai_response.tool_calls.clone());
            if let Err(e) = self.save_session(&sid, &meta).await {
                warn!("Failed to persist AI session '{}': {}", sid, e);
            }
        }

        Ok(ai_response)
    }
}
