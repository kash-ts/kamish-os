//! Reminders API HTTP handlers.

use crate::common::{AppError, AppResult, AppState};
use crate::scheduler::models::{CreateReminderRequest, Reminder, ReminderQueryFilter};
use crate::scheduler::recurrence::{parse_reminder_trigger_time, parse_timezone};
use crate::scheduler::repository::{create_reminder, delete_reminder, list_reminders};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

/// POST /api/v1/reminders
pub async fn create_reminder_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateReminderRequest>,
) -> AppResult<impl IntoResponse> {
    if payload.text.trim().is_empty() {
        return Err(AppError::Config(
            "Reminder text cannot be empty".to_string(),
        ));
    }

    let tz_str = payload.timezone.as_deref().unwrap_or("UTC");
    parse_timezone(tz_str)?;

    // Validate ISO-8601 trigger time
    parse_reminder_trigger_time(&payload.trigger_time, tz_str)?;

    let reminder = create_reminder(&state.db, &payload).await?;

    // Notify background scheduler
    state.scheduler.notify_modified();

    Ok((StatusCode::CREATED, Json(reminder)))
}

/// GET /api/v1/reminders
pub async fn list_reminders_handler(
    State(state): State<AppState>,
    Query(filter): Query<ReminderQueryFilter>,
) -> AppResult<Json<Vec<Reminder>>> {
    let reminders = list_reminders(&state.db, filter.status.as_deref()).await?;
    Ok(Json(reminders))
}

/// DELETE /api/v1/reminders/:id
pub async fn delete_reminder_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let deleted = delete_reminder(&state.db, &id).await?;
    if !deleted {
        return Err(AppError::NotFound(format!(
            "Reminder with id '{id}' not found"
        )));
    }

    state.scheduler.notify_modified();
    Ok(Json(json!({ "status": "deleted", "id": id })))
}
