use crate::error::WebResult;
use crate::state::WebState;
use axum::{extract::State, Json};
use kamish_ipc::CoreRequest;

pub async fn get_status(State(state): State<WebState>) -> WebResult<Json<serde_json::Value>> {
    let payload = state.call_core(CoreRequest::GetStatus).await?;
    Ok(Json(payload))
}

pub async fn get_hardware(State(state): State<WebState>) -> WebResult<Json<serde_json::Value>> {
    let payload = state.call_core(CoreRequest::GetHardwareTelemetry).await?;
    Ok(Json(payload))
}
