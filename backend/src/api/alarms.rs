//! Alarms API HTTP handlers.

use crate::common::{AppError, AppResult, AppState};
use crate::scheduler::models::{Alarm, CreateAlarmRequest, ToggleAlarmRequest};
use crate::scheduler::recurrence::{parse_recurrence, parse_time_hh_mm, parse_timezone};
use crate::scheduler::repository::{create_alarm, delete_alarm, list_alarms, toggle_alarm};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

/// POST /api/v1/alarms
pub async fn create_alarm_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateAlarmRequest>,
) -> AppResult<impl IntoResponse> {
    // Validate time format
    parse_time_hh_mm(&payload.time)?;

    // Validate recurrence pattern if specified
    parse_recurrence(payload.recurrence.as_deref())?;

    // Validate timezone if specified
    if let Some(ref tz) = payload.timezone {
        parse_timezone(tz)?;
    }

    let alarm = create_alarm(&state.db, &payload).await?;

    // Notify background scheduler
    state.scheduler.notify_modified();

    Ok((StatusCode::CREATED, Json(alarm)))
}

/// GET /api/v1/alarms
pub async fn list_alarms_handler(State(state): State<AppState>) -> AppResult<Json<Vec<Alarm>>> {
    let alarms = list_alarms(&state.db).await?;
    Ok(Json(alarms))
}

/// DELETE /api/v1/alarms/:id
pub async fn delete_alarm_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let deleted = delete_alarm(&state.db, &id).await?;
    if !deleted {
        return Err(AppError::NotFound(format!(
            "Alarm with id '{id}' not found"
        )));
    }

    state.scheduler.notify_modified();
    Ok(Json(json!({ "status": "deleted", "id": id })))
}

/// POST /api/v1/alarms/:id/toggle
pub async fn toggle_alarm_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
    payload: Option<Json<ToggleAlarmRequest>>,
) -> AppResult<Json<Alarm>> {
    let explicit_state = payload.and_then(|p| p.0.enabled);
    let updated = toggle_alarm(&state.db, &id, explicit_state).await?;

    state.scheduler.notify_modified();
    Ok(Json(updated))
}
