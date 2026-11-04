use crate::error::WebResult;
use crate::state::WebState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use kamish_ipc::CoreRequest;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateReminderPayload {
    pub text: String,
    pub trigger_time: String,
}

pub async fn list_reminders_handler(
    State(state): State<WebState>,
) -> WebResult<Json<serde_json::Value>> {
    let reminders = state.call_core(CoreRequest::ListReminders).await?;
    Ok(Json(reminders))
}

pub async fn create_reminder_handler(
    State(state): State<WebState>,
    Json(payload): Json<CreateReminderPayload>,
) -> WebResult<impl IntoResponse> {
    let reminder = state
        .call_core(CoreRequest::CreateReminder {
            text: payload.text,
            trigger_at: payload.trigger_time,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(reminder)))
}

pub async fn delete_reminder_handler(
    State(state): State<WebState>,
    Path(id): Path<String>,
) -> WebResult<Json<serde_json::Value>> {
    let resp = state.call_core(CoreRequest::DeleteReminder { id }).await?;
    Ok(Json(resp))
}
