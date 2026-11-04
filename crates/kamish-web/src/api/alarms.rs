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
pub struct CreateAlarmPayload {
    pub time: String,
    pub label: Option<String>,
    pub sound: Option<String>,
    pub recurrence: Option<String>,
    pub days_of_week: Option<Vec<u8>>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ToggleAlarmPayload {
    pub enabled: Option<bool>,
}

pub async fn list_alarms_handler(
    State(state): State<WebState>,
) -> WebResult<Json<serde_json::Value>> {
    let alarms = state.call_core(CoreRequest::ListAlarms).await?;
    Ok(Json(alarms))
}

pub async fn create_alarm_handler(
    State(state): State<WebState>,
    Json(payload): Json<CreateAlarmPayload>,
) -> WebResult<impl IntoResponse> {
    let days = payload.days_of_week.unwrap_or_default();
    let sound = payload.sound.unwrap_or_else(|| "default".to_string());
    let enabled = payload.enabled.unwrap_or(true);

    let alarm = state
        .call_core(CoreRequest::CreateAlarm {
            time: payload.time,
            days,
            sound,
            enabled,
            label: payload.label,
        })
        .await?;

    Ok((StatusCode::CREATED, Json(alarm)))
}

pub async fn delete_alarm_handler(
    State(state): State<WebState>,
    Path(id): Path<String>,
) -> WebResult<Json<serde_json::Value>> {
    let resp = state.call_core(CoreRequest::DeleteAlarm { id }).await?;
    Ok(Json(resp))
}

pub async fn toggle_alarm_handler(
    State(state): State<WebState>,
    Path(id): Path<String>,
    body: Option<Json<ToggleAlarmPayload>>,
) -> WebResult<Json<serde_json::Value>> {
    let enabled = body.and_then(|b| b.enabled);
    let resp = state
        .call_core(CoreRequest::UpdateAlarm {
            id,
            time: None,
            days: None,
            sound: None,
            enabled,
            label: None,
        })
        .await?;
    Ok(Json(resp))
}
