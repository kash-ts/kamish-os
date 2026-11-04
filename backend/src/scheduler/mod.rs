//! Scheduler module.
//! Offline alarms and reminders execution, recurrence handling, and SQLite persistence.

pub mod engine;
pub mod models;
pub mod recurrence;
pub mod repository;

pub use engine::{SchedulerEngine, SchedulerEvent, SchedulerHandle};
pub use models::{
    Alarm, CreateAlarmRequest, CreateReminderRequest, Reminder, ReminderQueryFilter,
    SchedulerStatusSummary, ToggleAlarmRequest,
};
