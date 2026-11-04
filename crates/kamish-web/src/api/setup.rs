use crate::error::WebResult;
use crate::state::WebState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use kamish_ipc::CoreRequest;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SaveStepPayload {
    pub step: serde_json::Value,
    pub data: Option<serde_json::Value>,
}

pub async fn get_setup_status_handler(
    State(state): State<WebState>,
) -> WebResult<impl IntoResponse> {
    let status = state.call_core(CoreRequest::GetSetupStatus).await?;
    Ok((StatusCode::OK, Json(status)))
}

pub async fn save_setup_step_handler(
    State(state): State<WebState>,
    Json(payload): Json<SaveStepPayload>,
) -> WebResult<impl IntoResponse> {
    let step_num = if let Some(n) = payload.step.as_u64() {
        n as u8
    } else if let Some(s) = payload.step.as_str() {
        match s {
            "network" => 1,
            "audio" => 2,
            "voice" => 3,
            "ai" => 4,
            _ => 5,
        }
    } else {
        1
    };

    let res = state
        .call_core(CoreRequest::CompleteSetupStep {
            step: step_num,
            data: payload.data.unwrap_or(serde_json::Value::Null),
        })
        .await?;
    Ok((StatusCode::OK, Json(res)))
}

pub async fn run_diagnostics_handler(
    State(state): State<WebState>,
) -> WebResult<impl IntoResponse> {
    let res = state.call_core(CoreRequest::RunDiagnostics).await?;
    Ok((StatusCode::OK, Json(res)))
}
