//! Data models for Alarms and Reminders.

use serde::{Deserialize, Serialize};

/// Alarm entity stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::FromRow)]
pub struct Alarm {
    pub id: String,
    pub time: String, // HH:MM
    pub timezone: String,
    pub recurrence: Option<String>,
    pub label: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Request payload for creating a new alarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAlarmRequest {
    pub time: String, // "HH:MM" format
    pub recurrence: Option<String>,
    pub label: Option<String>,
    #[serde(default = "default_timezone")]
    pub timezone: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: Option<bool>,
}

fn default_timezone() -> Option<String> {
    Some("UTC".to_string())
}

fn default_true() -> Option<bool> {
    Some(true)
}

/// Toggle request (optional payload, if empty toggles current state).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToggleAlarmRequest {
    pub enabled: Option<bool>,
}

/// Reminder entity stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::FromRow)]
pub struct Reminder {
    pub id: String,
    pub trigger_time: String, // ISO-8601
    pub text: String,
    pub timezone: String,
    pub status: String, // pending, completed, cancelled
    pub created_at: String,
}

/// Request payload for creating a new reminder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReminderRequest {
    pub trigger_time: String, // ISO-8601 timestamp
    pub text: String,
    #[serde(default = "default_timezone")]
    pub timezone: Option<String>,
}

/// Query filter for reminders list.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ReminderQueryFilter {
    pub status: Option<String>,
}

/// Summary of scheduler runtime status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStatusSummary {
    pub active_alarms_count: usize,
    pub pending_reminders_count: usize,
    pub running: bool,
}
