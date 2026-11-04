use crate::error::WebResult;
use crate::state::WebState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use kamish_ipc::CoreRequest;

pub async fn reboot_handler(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let resp = state.call_core(CoreRequest::TriggerReboot).await?;
    Ok((StatusCode::OK, Json(resp)))
}

pub async fn shutdown_handler(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let resp = state.call_core(CoreRequest::TriggerShutdown).await?;
    Ok((StatusCode::OK, Json(resp)))
}

pub async fn check_updates_handler(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let resp = state.call_core(CoreRequest::CheckUpdates).await?;
    Ok((StatusCode::OK, Json(resp)))
}
