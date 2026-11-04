//! AI Assistant HTTP API handlers.

use crate::ai::models::{AiPromptRequest, AiSessionMetadata};
use crate::common::{AppError, AppResult, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

/// POST /api/v1/ai/prompt
/// Executes a natural language command using Antigravity CLI or Intelligent Fallback Engine
pub async fn prompt_handler(
    State(state): State<AppState>,
    Json(payload): Json<AiPromptRequest>,
) -> AppResult<impl IntoResponse> {
    let trimmed = payload.prompt.trim();
    if trimmed.is_empty() {
        return Err(AppError::Config("Prompt cannot be empty".to_string()));
    }

    let response = state
        .ai
        .ask(trimmed, payload.session_id.as_deref(), &state)
        .await?;

    Ok((StatusCode::OK, Json(response)))
}

/// GET /api/v1/ai/tools
/// Returns JSON schemas for all 14 registered device control tools
pub async fn list_tools_handler(State(state): State<AppState>) -> impl IntoResponse {
    let declarations = state.ai.registry.list_declarations();
    let count = declarations.len();

    (
        StatusCode::OK,
        Json(json!({
            "count": count,
            "system_prompt": crate::ai::client::DEFAULT_SYSTEM_PROMPT,
            "tools": declarations
        })),
    )
}

/// GET /api/v1/ai/sessions/:id
/// Returns conversation history and tool execution log for the given session ID
pub async fn get_session_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<impl IntoResponse> {
    let row = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT id, metadata_json FROM ai_sessions WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::Database)?;

    match row {
        Some((session_id, Some(meta_str))) => {
            let meta: AiSessionMetadata = serde_json::from_str(&meta_str).unwrap_or_default();
            Ok(Json(json!({
                "session_id": session_id,
                "messages": meta.messages,
                "last_active_at": meta.last_active_at
            })))
        }
        Some((session_id, None)) => Ok(Json(json!({
            "session_id": session_id,
            "messages": [],
            "last_active_at": ""
        }))),
        None => Err(AppError::NotFound(format!("Session '{id}' not found"))),
    }
}

// -----------------------------------------------------------------------------
// AI Provider Status, Configuration & CLI Health Probe
// -----------------------------------------------------------------------------

use crate::ai::client::AiReadiness;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UpdateAiConfigRequest {
    pub mode: String, // "cli" | "mock"
    pub cli_path: Option<String>,
}

/// GET /api/v1/ai/status
/// Check AI subsystem readiness, active mode, and CLI verification details
pub async fn get_ai_status_handler(State(state): State<AppState>) -> Json<AiReadiness> {
    Json(state.ai.get_readiness().await)
}

/// POST /api/v1/ai/config
/// Switch between Antigravity CLI and built-in Mock engine, and update CLI path
pub async fn update_ai_config_handler(
    State(state): State<AppState>,
    Json(payload): Json<UpdateAiConfigRequest>,
) -> Json<AiReadiness> {
    let updated = state
        .ai
        .update_config(&payload.mode, payload.cli_path)
        .await;
    Json(updated)
}

/// POST /api/v1/ai/probe
/// Run immediate health probe against configured Antigravity CLI binary
pub async fn probe_ai_handler(State(state): State<AppState>) -> Json<AiReadiness> {
    Json(state.ai.get_readiness().await)
}

// -----------------------------------------------------------------------------
// Antigravity CLI lifecycle (install + authorization) driven from the web panel
// -----------------------------------------------------------------------------

/// GET /api/v1/ai/agy/status
///
/// Reports whether the CLI is installed, its version, and whether it is authorized.
pub async fn agy_status_handler(State(state): State<AppState>) -> Json<crate::ai::AgyStatus> {
    Json(state.agy.status().await)
}

/// POST /api/v1/ai/agy/install
///
/// Downloads the CLI release matching this host's architecture and installs it.
pub async fn agy_install_handler(
    State(state): State<AppState>,
) -> AppResult<Json<crate::ai::AgyStatus>> {
    let status = state.agy.install().await?;
    state.events.emit(
        "agy_installed",
        json!({ "path": status.path, "version": status.version }),
    );
    Ok(Json(status))
}

/// POST /api/v1/ai/agy/login/begin
///
/// Starts the authorization flow and returns the URL plus one-time code for the user.
pub async fn agy_login_begin_handler(
    State(state): State<AppState>,
) -> AppResult<Json<crate::ai::AgyLoginChallenge>> {
    let challenge = state.agy.begin_login().await?;
    Ok(Json(challenge))
}

/// POST /api/v1/ai/agy/login/submit
///
/// Passes the code entered in the web panel back to the CLI.
pub async fn agy_login_submit_handler(
    State(state): State<AppState>,
    Json(payload): Json<crate::ai::SubmitCodeRequest>,
) -> AppResult<Json<crate::ai::AgyLoginResult>> {
    let result = state.agy.submit_code(&payload.code).await?;

    if result.authorized {
        state
            .events
            .emit("agy_authorized", json!({ "authorized": true }));
    }

    Ok(Json(result))
}
