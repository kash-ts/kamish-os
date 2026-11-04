//! Strict Tool Registry and definitions for all 14 device control tools.

use crate::ai::models::{ToolCallResult, ToolDeclaration};
use crate::common::{AppError, AppResult, AppState};
use crate::music::TrackInfo;
use crate::scheduler::models::{CreateAlarmRequest, CreateReminderRequest};
use crate::scheduler::recurrence::{
    parse_recurrence, parse_reminder_trigger_time, parse_time_hh_mm, parse_timezone,
};
use crate::scheduler::repository::{
    create_alarm, create_reminder, delete_alarm, get_alarm_by_id, list_alarms, list_reminders,
};
use crate::system::get_device_info;
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use tracing::info;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Tool trait representing a strictly typed device action
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters_schema(&self) -> Value;
    fn execute<'a>(&'a self, args: Value, state: &'a AppState) -> BoxFuture<'a, AppResult<Value>>;

    fn declaration(&self) -> ToolDeclaration {
        ToolDeclaration {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.parameters_schema(),
        }
    }
}

// -----------------------------------------------------------------------------
// Tool 1: create_alarm
// -----------------------------------------------------------------------------
pub struct CreateAlarmTool;

impl Tool for CreateAlarmTool {
    fn name(&self) -> &'static str {
        "create_alarm"
    }

    fn description(&self) -> &'static str {
        "Create a new alarm with time (HH:MM), optional recurrence pattern, label, and timezone."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "time": {
                    "type": "string",
                    "description": "Alarm time in 24-hour HH:MM format, e.g. '07:30'"
                },
                "recurrence": {
                    "type": "string",
                    "description": "Optional recurrence pattern: 'daily', 'weekdays', 'weekends', or comma-separated days ('mon,wed,fri')"
                },
                "label": {
                    "type": "string",
                    "description": "Optional text description or label for the alarm"
                },
                "timezone": {
                    "type": "string",
                    "description": "Optional IANA timezone name (e.g. 'Europe/Moscow', 'UTC'). Defaults to UTC."
                }
            },
            "required": ["time"]
        })
    }

    fn execute<'a>(&'a self, args: Value, state: &'a AppState) -> BoxFuture<'a, AppResult<Value>> {
        Box::pin(async move {
            let time = args
                .get("time")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Config("Missing required argument 'time'".to_string()))?;

            parse_time_hh_mm(time)?;

            let recurrence = args
                .get("recurrence")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if let Some(ref rec) = recurrence {
                parse_recurrence(Some(rec))?;
            }

            let timezone = args
                .get("timezone")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if let Some(ref tz) = timezone {
                parse_timezone(tz)?;
            }

            let label = args
                .get("label")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let req = CreateAlarmRequest {
                time: time.to_string(),
                recurrence,
                label,
                timezone,
                enabled: Some(true),
            };

            let alarm = create_alarm(&state.db, &req).await?;
            state.scheduler.notify_modified();

            info!("Tool create_alarm created alarm id={}", alarm.id);
            Ok(serde_json::to_value(alarm).unwrap_or(Value::Null))
        })
    }
}

// -----------------------------------------------------------------------------
// Tool 2: delete_alarm
// -----------------------------------------------------------------------------
pub struct DeleteAlarmTool;

impl Tool for DeleteAlarmTool {
    fn name(&self) -> &'static str {
        "delete_alarm"
    }

    fn description(&self) -> &'static str {
        "Delete an alarm by its unique UUID."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The unique UUID of the alarm to delete"
                }
            },
            "required": ["id"]
        })
    }

    fn execute<'a>(&'a self, args: Value, state: &'a AppState) -> BoxFuture<'a, AppResult<Value>> {
        Box::pin(async move {
            let id = args
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Config("Missing required argument 'id'".to_string()))?;

            if id.trim().is_empty() {
                return Err(AppError::Config("Alarm id cannot be empty".to_string()));
            }

            let deleted = delete_alarm(&state.db, id).await?;
            if deleted {
                state.scheduler.notify_modified();
                info!("Tool delete_alarm deleted alarm id={}", id);
                Ok(json!({ "id": id, "deleted": true }))
            } else {
                Err(AppError::NotFound(format!(
                    "Будильник с ID '{id}' не найден"
                )))
            }
        })
    }
}

// -----------------------------------------------------------------------------
// Tool 3: list_alarms
// -----------------------------------------------------------------------------
pub struct ListAlarmsTool;

impl Tool for ListAlarmsTool {
    fn name(&self) -> &'static str {
        "list_alarms"
    }

    fn description(&self) -> &'static str {
        "List all configured alarms, optionally filtered to only active/enabled alarms."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "enabled_only": {
                    "type": "boolean",
                    "description": "If true, only return currently enabled alarms"
                }
            }
        })
    }

    fn execute<'a>(&'a self, args: Value, state: &'a AppState) -> BoxFuture<'a, AppResult<Value>> {
        Box::pin(async move {
            let enabled_only = args
                .get("enabled_only")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let all = list_alarms(&state.db).await?;
            let filtered: Vec<_> = if enabled_only {
                all.into_iter().filter(|a| a.enabled).collect()
            } else {
                all
            };

            Ok(serde_json::to_value(filtered).unwrap_or(json!([])))
        })
    }
}

// -----------------------------------------------------------------------------
// Tool 4: snooze_alarm
// -----------------------------------------------------------------------------
pub struct SnoozeAlarmTool;

impl Tool for SnoozeAlarmTool {
    fn name(&self) -> &'static str {
        "snooze_alarm"
    }

    fn description(&self) -> &'static str {
        "Snooze an existing alarm by postponing it by specified minutes (defaults to 10 minutes)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The unique ID of the alarm to snooze"
                },
                "minutes": {
                    "type": "integer",
                    "description": "Number of minutes to snooze (1 to 1440). Defaults to 10."
                }
            },
            "required": ["id"]
        })
    }

    fn execute<'a>(&'a self, args: Value, state: &'a AppState) -> BoxFuture<'a, AppResult<Value>> {
        Box::pin(async move {
            let id = args
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Config("Missing required argument 'id'".to_string()))?;

            let minutes = args.get("minutes").and_then(|v| v.as_i64()).unwrap_or(10);

            if minutes <= 0 || minutes > 1440 {
                return Err(AppError::Config(
                    "Snooze minutes must be between 1 and 1440".to_string(),
                ));
            }

            let alarm = get_alarm_by_id(&state.db, id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("Alarm with id '{id}' not found")))?;

            let snooze_time = Utc::now() + Duration::minutes(minutes);
            let label = alarm.label.as_deref().unwrap_or(&alarm.time);
            let reminder_text = format!("Отложенный будильник: {label}");

            let req = CreateReminderRequest {
                trigger_time: snooze_time.to_rfc3339(),
                text: reminder_text,
                timezone: Some(alarm.timezone.clone()),
            };

            let reminder = create_reminder(&state.db, &req).await?;
            state.scheduler.notify_modified();

            info!(
                "Tool snooze_alarm snoozed alarm id={} for {} mins",
                id, minutes
            );
            Ok(json!({
                "snoozed": true,
                "alarm_id": id,
                "minutes": minutes,
                "snooze_until": snooze_time.to_rfc3339(),
                "reminder_id": reminder.id
            }))
        })
    }
}

// -----------------------------------------------------------------------------
// Tool 5: create_reminder
// -----------------------------------------------------------------------------
pub struct CreateReminderTool;

impl Tool for CreateReminderTool {
    fn name(&self) -> &'static str {
        "create_reminder"
    }

    fn description(&self) -> &'static str {
        "Create a reminder with trigger_time (ISO-8601 or relative minutes like '+15' or '15') and text."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "trigger_time": {
                    "type": "string",
                    "description": "Trigger time: either ISO-8601 (e.g. '2026-11-01T15:30:00Z') or relative minutes (e.g. '15', '+30', '5')"
                },
                "text": {
                    "type": "string",
                    "description": "The reminder content / message to speak"
                },
                "timezone": {
                    "type": "string",
                    "description": "Optional IANA timezone name (defaults to UTC)"
                }
            },
            "required": ["trigger_time", "text"]
        })
    }

    fn execute<'a>(&'a self, args: Value, state: &'a AppState) -> BoxFuture<'a, AppResult<Value>> {
        Box::pin(async move {
            let trigger_raw = args
                .get("trigger_time")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    AppError::Config("Missing required argument 'trigger_time'".to_string())
                })?;

            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Config("Missing required argument 'text'".to_string()))?;

            if text.trim().is_empty() {
                return Err(AppError::Config(
                    "Reminder text cannot be empty".to_string(),
                ));
            }

            let timezone = args
                .get("timezone")
                .and_then(|v| v.as_str())
                .unwrap_or("UTC");
            parse_timezone(timezone)?;

            // Resolve trigger_time: handle relative minutes or ISO-8601
            let trimmed = trigger_raw.trim();
            let resolved_iso = if let Ok(mins) = trimmed
                .trim_start_matches('+')
                .trim_end_matches('m')
                .parse::<i64>()
            {
                if mins <= 0 {
                    return Err(AppError::Config(
                        "Relative reminder minutes must be greater than 0".to_string(),
                    ));
                }
                (Utc::now() + Duration::minutes(mins)).to_rfc3339()
            } else {
                let parsed_dt = parse_reminder_trigger_time(trimmed, timezone)?;
                parsed_dt.to_rfc3339()
            };

            let req = CreateReminderRequest {
                trigger_time: resolved_iso,
                text: text.to_string(),
                timezone: Some(timezone.to_string()),
            };

            let reminder = create_reminder(&state.db, &req).await?;
            state.scheduler.notify_modified();

            info!("Tool create_reminder created reminder id={}", reminder.id);
            Ok(serde_json::to_value(reminder).unwrap_or(Value::Null))
        })
    }
}

// -----------------------------------------------------------------------------
// Tool 6: list_reminders
// -----------------------------------------------------------------------------
pub struct ListRemindersTool;

impl Tool for ListRemindersTool {
    fn name(&self) -> &'static str {
        "list_reminders"
    }

    fn description(&self) -> &'static str {
        "List reminders, optionally filtered by status ('pending', 'completed', 'cancelled')."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "description": "Optional status filter: 'pending', 'completed', or 'cancelled'"
                }
            }
        })
    }

    fn execute<'a>(&'a self, args: Value, state: &'a AppState) -> BoxFuture<'a, AppResult<Value>> {
        Box::pin(async move {
            let status = args.get("status").and_then(|v| v.as_str());
            if let Some(s) = status {
                if !["pending", "completed", "cancelled"].contains(&s) {
                    return Err(AppError::Config(format!(
                        "Invalid reminder status '{s}'. Expected 'pending', 'completed', or 'cancelled'."
                    )));
                }
            }

            let list = list_reminders(&state.db, status).await?;
            Ok(serde_json::to_value(list).unwrap_or(json!([])))
        })
    }
}

// -----------------------------------------------------------------------------
// Tool 7: set_volume
// -----------------------------------------------------------------------------
pub struct SetVolumeTool;

impl Tool for SetVolumeTool {
    fn name(&self) -> &'static str {
        "set_volume"
    }

    fn description(&self) -> &'static str {
        "Set the speaker output volume (0-100)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "volume": {
                    "type": "integer",
                    "description": "Target volume percentage from 0 to 100",
                    "minimum": 0,
                    "maximum": 100
                }
            },
            "required": ["volume"]
        })
    }

    fn execute<'a>(&'a self, args: Value, state: &'a AppState) -> BoxFuture<'a, AppResult<Value>> {
        Box::pin(async move {
            let vol_i64 = args.get("volume").and_then(|v| v.as_i64()).ok_or_else(|| {
                AppError::Config("Missing required argument 'volume'".to_string())
            })?;

            if !(0..=100).contains(&vol_i64) {
                return Err(AppError::Config(
                    "Volume must be an integer between 0 and 100".to_string(),
                ));
            }

            let vol = vol_i64 as u8;
            let current = state.media.set_volume(vol, Some(&state.db)).await;
            info!("Tool set_volume set volume to {}%", current);

            Ok(json!({
                "volume": current,
                "status": "ok"
            }))
        })
    }
}

// -----------------------------------------------------------------------------
// Tool 8: play_music
// -----------------------------------------------------------------------------
pub struct PlayMusicTool;

impl Tool for PlayMusicTool {
    fn name(&self) -> &'static str {
        "play_music"
    }

    fn description(&self) -> &'static str {
        "Play music by search query or direct stream URI."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Song title, artist, or music genre to search and play"
                },
                "uri": {
                    "type": "string",
                    "description": "Direct audio stream or media URL"
                }
            }
        })
    }

    fn execute<'a>(&'a self, args: Value, state: &'a AppState) -> BoxFuture<'a, AppResult<Value>> {
        Box::pin(async move {
            let query = args.get("query").and_then(|v| v.as_str()).map(|s| s.trim());
            let uri = args.get("uri").and_then(|v| v.as_str()).map(|s| s.trim());

            let track = state
                .media
                .play_by_query_or_uri(query, uri, Some(&state.db))
                .await;

            info!("Tool play_music playing: {:?}", track.title);

            Ok(json!({
                "status": "playing",
                "track": track
            }))
        })
    }
}

// -----------------------------------------------------------------------------
// Tool 9: pause_music
// -----------------------------------------------------------------------------
pub struct PauseMusicTool;

impl Tool for PauseMusicTool {
    fn name(&self) -> &'static str {
        "pause_music"
    }

    fn description(&self) -> &'static str {
        "Pause the currently playing music or audio stream."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn execute<'a>(&'a self, _args: Value, state: &'a AppState) -> BoxFuture<'a, AppResult<Value>> {
        Box::pin(async move {
            state.media.pause(Some(&state.db)).await;
            info!("Tool pause_music paused playback");
            Ok(json!({ "status": "paused" }))
        })
    }
}

// -----------------------------------------------------------------------------
// Tool 10: resume_music
// -----------------------------------------------------------------------------
pub struct ResumeMusicTool;

impl Tool for ResumeMusicTool {
    fn name(&self) -> &'static str {
        "resume_music"
    }

    fn description(&self) -> &'static str {
        "Resume playback of currently paused music or stream."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn execute<'a>(&'a self, _args: Value, state: &'a AppState) -> BoxFuture<'a, AppResult<Value>> {
        Box::pin(async move {
            state.media.resume(Some(&state.db)).await;
            info!("Tool resume_music resumed playback");
            Ok(json!({ "status": "playing" }))
        })
    }
}

// -----------------------------------------------------------------------------
// Tool 11: stop_music
// -----------------------------------------------------------------------------
pub struct StopMusicTool;

impl Tool for StopMusicTool {
    fn name(&self) -> &'static str {
        "stop_music"
    }

    fn description(&self) -> &'static str {
        "Stop music playback completely."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn execute<'a>(&'a self, _args: Value, state: &'a AppState) -> BoxFuture<'a, AppResult<Value>> {
        Box::pin(async move {
            state.media.stop(Some(&state.db)).await;
            info!("Tool stop_music stopped playback");
            Ok(json!({ "status": "stopped" }))
        })
    }
}

// -----------------------------------------------------------------------------
// Tool 12: search_music
// -----------------------------------------------------------------------------
pub struct SearchMusicTool;

impl Tool for SearchMusicTool {
    fn name(&self) -> &'static str {
        "search_music"
    }

    fn description(&self) -> &'static str {
        "Search the music catalog by query with optional limit."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Song title, artist name, or genre keyword to search"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of search results to return (1-50, default 5)",
                    "minimum": 1,
                    "maximum": 50
                }
            },
            "required": ["query"]
        })
    }

    fn execute<'a>(&'a self, args: Value, state: &'a AppState) -> BoxFuture<'a, AppResult<Value>> {
        Box::pin(async move {
            let query = args
                .get("query")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Config("Missing required argument 'query'".to_string()))?;

            if query.trim().is_empty() {
                return Err(AppError::Config("Search query cannot be empty".to_string()));
            }

            let limit = args.get("limit").and_then(|v| v.as_i64()).unwrap_or(5);

            if !(1..=50).contains(&limit) {
                return Err(AppError::Config(
                    "Search limit must be between 1 and 50".to_string(),
                ));
            }

            let search_results = state
                .media
                .search(query, "all", limit as usize, Some(&state.db))
                .await;

            let mut results = Vec::new();
            for t in search_results {
                results.push(TrackInfo {
                    id: Some(t.id),
                    title: t.title,
                    artist: t.artist,
                    duration_secs: t.duration_secs,
                    source: Some(t.source),
                    uri: t.uri,
                });
            }

            Ok(json!({
                "query": query,
                "count": results.len(),
                "tracks": results
            }))
        })
    }
}

// -----------------------------------------------------------------------------
// Tool 13: get_current_track
// -----------------------------------------------------------------------------
pub struct GetCurrentTrackTool;

impl Tool for GetCurrentTrackTool {
    fn name(&self) -> &'static str {
        "get_current_track"
    }

    fn description(&self) -> &'static str {
        "Get information about the currently playing track and playback status."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn execute<'a>(&'a self, _args: Value, state: &'a AppState) -> BoxFuture<'a, AppResult<Value>> {
        Box::pin(async move {
            let status = state.media.get_status().await;
            let current = state.media.get_current_track().await;
            let volume = state.media.get_volume();

            Ok(json!({
                "status": status.to_string(),
                "current_track": current,
                "volume": volume
            }))
        })
    }
}

// -----------------------------------------------------------------------------
// Tool 14: get_device_status
// -----------------------------------------------------------------------------
pub struct GetDeviceStatusTool;

impl Tool for GetDeviceStatusTool {
    fn name(&self) -> &'static str {
        "get_device_status"
    }

    fn description(&self) -> &'static str {
        "Get device system status, resource usage, scheduler counts, volume, and playback state."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn execute<'a>(&'a self, _args: Value, state: &'a AppState) -> BoxFuture<'a, AppResult<Value>> {
        Box::pin(async move {
            let info = get_device_info();
            let sched = state.scheduler.get_status_summary().await;
            let volume = state.media.get_volume();
            let status = state.media.get_status().await;
            let app_uptime_secs = state.start_time.elapsed().as_secs();

            Ok(json!({
                "device": info,
                "scheduler": sched,
                "media": {
                    "volume": volume,
                    "playback_status": status.to_string(),
                },
                "app_uptime_secs": app_uptime_secs,
                "status": "ok"
            }))
        })
    }
}

// -----------------------------------------------------------------------------
// Tool Registry
// -----------------------------------------------------------------------------
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tools_count", &self.tools.len())
            .finish()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            tools: HashMap::new(),
        };

        // Register all 14 tools
        reg.register(Box::new(CreateAlarmTool));
        reg.register(Box::new(DeleteAlarmTool));
        reg.register(Box::new(ListAlarmsTool));
        reg.register(Box::new(SnoozeAlarmTool));
        reg.register(Box::new(CreateReminderTool));
        reg.register(Box::new(ListRemindersTool));
        reg.register(Box::new(SetVolumeTool));
        reg.register(Box::new(PlayMusicTool));
        reg.register(Box::new(PauseMusicTool));
        reg.register(Box::new(ResumeMusicTool));
        reg.register(Box::new(StopMusicTool));
        reg.register(Box::new(SearchMusicTool));
        reg.register(Box::new(GetCurrentTrackTool));
        reg.register(Box::new(GetDeviceStatusTool));

        reg
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|b| b.as_ref())
    }

    pub fn list_declarations(&self) -> Vec<ToolDeclaration> {
        let mut list: Vec<_> = self.tools.values().map(|t| t.declaration()).collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    pub async fn execute(&self, name: &str, args: Value, state: &AppState) -> ToolCallResult {
        match self.get(name) {
            Some(tool) => match tool.execute(args.clone(), state).await {
                Ok(val) => ToolCallResult {
                    tool: name.to_string(),
                    arguments: args,
                    result: val,
                    success: true,
                    error: None,
                },
                Err(err) => ToolCallResult {
                    tool: name.to_string(),
                    arguments: args,
                    result: Value::Null,
                    success: false,
                    error: Some(err.to_string()),
                },
            },
            None => ToolCallResult {
                tool: name.to_string(),
                arguments: args,
                result: Value::Null,
                success: false,
                error: Some(format!("Tool '{name}' is not registered")),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{init_pool, run_migrations};

    async fn create_test_state() -> AppState {
        let pool = init_pool("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        AppState::new(pool)
    }

    #[tokio::test]
    async fn test_registry_contains_all_14_tools() {
        let reg = ToolRegistry::new();
        let decls = reg.list_declarations();
        assert_eq!(decls.len(), 14);

        let names: Vec<_> = decls.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"create_alarm"));
        assert!(names.contains(&"delete_alarm"));
        assert!(names.contains(&"list_alarms"));
        assert!(names.contains(&"snooze_alarm"));
        assert!(names.contains(&"create_reminder"));
        assert!(names.contains(&"list_reminders"));
        assert!(names.contains(&"set_volume"));
        assert!(names.contains(&"play_music"));
        assert!(names.contains(&"pause_music"));
        assert!(names.contains(&"resume_music"));
        assert!(names.contains(&"stop_music"));
        assert!(names.contains(&"search_music"));
        assert!(names.contains(&"get_current_track"));
        assert!(names.contains(&"get_device_status"));
    }

    #[tokio::test]
    async fn test_tools_1_to_4_alarms_lifecycle() {
        let state = create_test_state().await;
        let reg = ToolRegistry::new();

        // 1. Create alarm
        let res = reg
            .execute(
                "create_alarm",
                json!({
                    "time": "07:30",
                    "recurrence": "weekdays",
                    "label": "Morning Wakeup"
                }),
                &state,
            )
            .await;
        assert!(res.success);
        let alarm_id = res.result["id"].as_str().unwrap().to_string();
        assert_eq!(res.result["time"], "07:30");
        assert_eq!(res.result["recurrence"], "weekdays");
        assert_eq!(res.result["label"], "Morning Wakeup");

        // Validation error: invalid time
        let err_res = reg
            .execute("create_alarm", json!({ "time": "25:99" }), &state)
            .await;
        assert!(!err_res.success);

        // Validation error: invalid recurrence
        let err_res2 = reg
            .execute(
                "create_alarm",
                json!({ "time": "08:00", "recurrence": "funday" }),
                &state,
            )
            .await;
        assert!(!err_res2.success);

        // 2. List alarms
        let list_res = reg.execute("list_alarms", json!({}), &state).await;
        assert!(list_res.success);
        let alarms = list_res.result.as_array().unwrap();
        assert_eq!(alarms.len(), 1);

        // 3. Snooze alarm
        let snooze_res = reg
            .execute(
                "snooze_alarm",
                json!({ "id": alarm_id, "minutes": 10 }),
                &state,
            )
            .await;
        assert!(snooze_res.success);
        assert_eq!(snooze_res.result["snoozed"], true);
        assert_eq!(snooze_res.result["minutes"], 10);

        // Snooze invalid minutes
        let err_snooze = reg
            .execute(
                "snooze_alarm",
                json!({ "id": alarm_id, "minutes": 0 }),
                &state,
            )
            .await;
        assert!(!err_snooze.success);

        // Snooze non-existing alarm
        let err_snooze2 = reg
            .execute(
                "snooze_alarm",
                json!({ "id": "non-existent-id", "minutes": 5 }),
                &state,
            )
            .await;
        assert!(!err_snooze2.success);

        // 4. Delete alarm
        let del_res = reg
            .execute("delete_alarm", json!({ "id": alarm_id }), &state)
            .await;
        assert!(del_res.success);
        assert_eq!(del_res.result["deleted"], true);

        // Delete empty id error
        let del_err = reg
            .execute("delete_alarm", json!({ "id": "" }), &state)
            .await;
        assert!(!del_err.success);
    }

    #[tokio::test]
    async fn test_tools_5_and_6_reminders() {
        let state = create_test_state().await;
        let reg = ToolRegistry::new();

        // 5. Create reminder with relative minutes
        let res = reg
            .execute(
                "create_reminder",
                json!({
                    "trigger_time": "+15",
                    "text": "Drink water"
                }),
                &state,
            )
            .await;
        assert!(res.success);
        assert_eq!(res.result["text"], "Drink water");
        assert_eq!(res.result["status"], "pending");

        // Create reminder with ISO-8601
        let res2 = reg
            .execute(
                "create_reminder",
                json!({
                    "trigger_time": "2026-11-01T20:00:00Z",
                    "text": "Evening stretch"
                }),
                &state,
            )
            .await;
        assert!(res2.success);

        // Reminder validation error: empty text
        let err_res = reg
            .execute(
                "create_reminder",
                json!({ "trigger_time": "+10", "text": "   " }),
                &state,
            )
            .await;
        assert!(!err_res.success);

        // 6. List reminders
        let list_res = reg
            .execute("list_reminders", json!({ "status": "pending" }), &state)
            .await;
        assert!(list_res.success);
        assert_eq!(list_res.result.as_array().unwrap().len(), 2);

        // Invalid status filter
        let err_status = reg
            .execute("list_reminders", json!({ "status": "unknown" }), &state)
            .await;
        assert!(!err_status.success);
    }

    #[tokio::test]
    async fn test_tool_7_set_volume() {
        let state = create_test_state().await;
        let reg = ToolRegistry::new();

        // Set volume to 75
        let res = reg
            .execute("set_volume", json!({ "volume": 75 }), &state)
            .await;
        assert!(res.success);
        assert_eq!(res.result["volume"], 75);
        assert_eq!(state.media.get_volume(), 75);

        // Min and max limits
        let res_min = reg
            .execute("set_volume", json!({ "volume": 0 }), &state)
            .await;
        assert!(res_min.success);
        assert_eq!(state.media.get_volume(), 0);

        let res_max = reg
            .execute("set_volume", json!({ "volume": 100 }), &state)
            .await;
        assert!(res_max.success);
        assert_eq!(state.media.get_volume(), 100);

        // Out of bounds error
        let err_res = reg
            .execute("set_volume", json!({ "volume": 101 }), &state)
            .await;
        assert!(!err_res.success);

        let err_res2 = reg
            .execute("set_volume", json!({ "volume": -1 }), &state)
            .await;
        assert!(!err_res2.success);
    }

    #[tokio::test]
    async fn test_tools_8_to_13_music_playback() {
        let state = create_test_state().await;
        let reg = ToolRegistry::new();

        // 8. Play music
        let play_res = reg
            .execute(
                "play_music",
                json!({ "query": "Bohemian Rhapsody" }),
                &state,
            )
            .await;
        assert!(play_res.success);
        assert_eq!(play_res.result["status"], "playing");
        assert_eq!(play_res.result["track"]["title"], "Bohemian Rhapsody");

        // 13. Get current track
        let cur_res = reg.execute("get_current_track", json!({}), &state).await;
        assert!(cur_res.success);
        assert_eq!(cur_res.result["status"], "playing");
        assert_eq!(
            cur_res.result["current_track"]["title"],
            "Bohemian Rhapsody"
        );

        // 9. Pause music
        let pause_res = reg.execute("pause_music", json!({}), &state).await;
        assert!(pause_res.success);
        assert_eq!(pause_res.result["status"], "paused");

        // 10. Resume music
        let resume_res = reg.execute("resume_music", json!({}), &state).await;
        assert!(resume_res.success);
        assert_eq!(resume_res.result["status"], "playing");

        // 11. Stop music
        let stop_res = reg.execute("stop_music", json!({}), &state).await;
        assert!(stop_res.success);
        assert_eq!(stop_res.result["status"], "stopped");

        // 12. Search music
        let search_res = reg
            .execute(
                "search_music",
                json!({ "query": "Queen", "limit": 3 }),
                &state,
            )
            .await;
        assert!(search_res.success);
        assert!(search_res.result["tracks"].as_array().unwrap().len() >= 1);

        // Search validation error: empty query
        let search_err = reg
            .execute("search_music", json!({ "query": "" }), &state)
            .await;
        assert!(!search_err.success);
    }

    #[tokio::test]
    async fn test_tool_14_get_device_status() {
        let state = create_test_state().await;
        let reg = ToolRegistry::new();

        let status_res = reg.execute("get_device_status", json!({}), &state).await;
        assert!(status_res.success);
        assert_eq!(status_res.result["status"], "ok");
        assert!(status_res.result["device"]["cpu_count"].as_u64().unwrap() > 0);
        assert!(status_res.result["scheduler"]["running"]
            .as_bool()
            .is_some());
        assert_eq!(status_res.result["media"]["volume"], 50);
    }
}
