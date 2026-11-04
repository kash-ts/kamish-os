use crate::error::{WebError, WebResult};
use crate::state::WebState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use kamish_ipc::CoreRequest;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PromptPayload {
    pub prompt: String,
    pub session_id: Option<String>,
}

pub async fn prompt_handler(
    State(state): State<WebState>,
    Json(payload): Json<PromptPayload>,
) -> WebResult<impl IntoResponse> {
    if payload.prompt.trim().is_empty() {
        return Err(WebError::Validation("Prompt cannot be empty".to_string()));
    }

    let resp = state
        .call_core(CoreRequest::ExecutePrompt {
            prompt: payload.prompt,
            session_id: payload.session_id,
        })
        .await?;

    Ok((StatusCode::OK, Json(resp)))
}

pub async fn get_ai_status(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let status = state.call_core(CoreRequest::GetAgyStatus).await?;
    Ok((StatusCode::OK, Json(status)))
}
