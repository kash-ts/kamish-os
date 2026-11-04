//! YouTube authorization API handlers.
//!
//! The web panel drives OAuth 2.0 Device Flow through these endpoints:
//! `begin` returns a URL + code for the user, `finish` completes the exchange.

use crate::common::{AppError, AppResult, AppState};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct BeginAuthRequest {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Deserialize)]
pub struct FinishAuthRequest {
    pub device_code: String,
}

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub status: &'static str,
    pub message: &'static str,
}

/// GET /api/v1/music/youtube/auth/status
pub async fn yt_auth_status_handler(
    State(state): State<AppState>,
) -> Json<crate::music::YouTubeAuthStatus> {
    Json(state.youtube_auth.status().await)
}

/// POST /api/v1/music/youtube/auth/begin
///
/// Starts the device flow and returns the verification URL plus the user code.
pub async fn yt_auth_begin_handler(
    State(state): State<AppState>,
    Json(payload): Json<BeginAuthRequest>,
) -> AppResult<Json<crate::music::DeviceCodeChallenge>> {
    let challenge = state
        .youtube_auth
        .begin(&payload.client_id, &payload.client_secret)
        .await?;

    Ok(Json(challenge))
}

/// POST /api/v1/music/youtube/auth/finish
///
/// Completes authorization once the user has entered the code at Google.
pub async fn yt_auth_finish_handler(
    State(state): State<AppState>,
    Json(payload): Json<FinishAuthRequest>,
) -> AppResult<Json<crate::music::YouTubeAuthStatus>> {
    let status = state.youtube_auth.finish(&payload.device_code).await?;

    state.events.emit(
        "youtube_authorized",
        serde_json::json!({
            "authorized": status.authorized,
            "cookies_ready": status.cookies_ready,
        }),
    );

    Ok(Json(status))
}

/// POST /api/v1/music/youtube/auth/refresh
pub async fn yt_auth_refresh_handler(
    State(state): State<AppState>,
) -> AppResult<Json<crate::music::YouTubeAuthStatus>> {
    let status = state.youtube_auth.refresh().await?;
    Ok(Json(status))
}

/// POST /api/v1/music/youtube/auth/logout
pub async fn yt_auth_logout_handler(
    State(state): State<AppState>,
) -> AppResult<Json<LogoutResponse>> {
    state.youtube_auth.logout().await?;

    state
        .events
        .emit("youtube_logged_out", serde_json::json!({ "authorized": false }));

    Ok(Json(LogoutResponse {
        status: "ok",
        message: "Учётные данные YouTube удалены",
    }))
}

/// Reject an obviously malformed client id early, with a helpful message.
pub fn validate_client_id(client_id: &str) -> AppResult<()> {
    let id = client_id.trim();
    if id.is_empty() {
        return Err(AppError::Validation("Client ID обязателен".to_string()));
    }
    if !id.ends_with(".apps.googleusercontent.com") {
        return Err(AppError::Validation(
            "Client ID должен заканчиваться на .apps.googleusercontent.com".to_string(),
        ));
    }
    Ok(())
}
