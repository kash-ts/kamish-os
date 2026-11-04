use crate::auth::User;
use crate::common::{AppError, AppResult, AppState};
use axum::{
    extract::{Extension, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub must_change_password: bool,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
    pub expires_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct ChangePasswordResponse {
    pub status: &'static str,
    pub message: &'static str,
    pub must_change_password: bool,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub authenticated: bool,
    pub user: UserInfo,
    pub expires_at: String,
}

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub status: &'static str,
    pub message: &'static str,
}

/// POST /api/v1/auth/login
pub async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<impl IntoResponse> {
    let (session, user) = state
        .auth
        .authenticate(&payload.username, &payload.password)
        .await?;

    let cookie_val = format!(
        "kamish_session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800",
        session.token
    );
    let cookie_legacy = format!(
        "session_token={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800",
        session.token
    );

    let mut headers = HeaderMap::new();
    headers.append(
        header::SET_COOKIE,
        cookie_val
            .parse()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?,
    );
    headers.append(
        header::SET_COOKIE,
        cookie_legacy
            .parse()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("{e}")))?,
    );

    let res_body = LoginResponse {
        token: session.token,
        user: UserInfo {
            id: user.id,
            username: user.username,
            must_change_password: user.must_change_password,
        },
        expires_at: session.expires_at,
    };

    Ok((StatusCode::OK, headers, Json(res_body)))
}

/// POST /api/v1/auth/change-password
pub async fn change_password_handler(
    State(state): State<AppState>,
    user: Option<Extension<User>>,
    Json(payload): Json<ChangePasswordRequest>,
) -> AppResult<Json<ChangePasswordResponse>> {
    let current_user = user
        .map(|Extension(u)| u)
        .ok_or_else(|| AppError::Unauthorized("Authentication required".to_string()))?;

    state
        .auth
        .change_password(
            &current_user.id,
            &payload.old_password,
            &payload.new_password,
        )
        .await?;

    // Also update setup status for auth step in device_state
    let _ = crate::setup::SetupRepository::save_step(
        &state.db,
        "auth",
        crate::setup::StepStatus::Completed,
        Some(serde_json::json!({
            "username": current_user.username,
            "password_changed": true,
        })),
    )
    .await;

    state.events.emit(
        "setup_step_updated",
        serde_json::json!({
            "step": "auth",
            "status": "completed",
            "next_step": "network",
            "is_completed": false,
        }),
    );

    Ok(Json(ChangePasswordResponse {
        status: "ok",
        message: "Password changed successfully",
        must_change_password: false,
    }))
}

/// POST /api/v1/auth/logout
pub async fn logout_handler(
    State(state): State<AppState>,
    session: Option<Extension<crate::auth::AuthSession>>,
) -> AppResult<impl IntoResponse> {
    if let Some(Extension(sess)) = session {
        let _ = state.auth.logout(&sess.token).await;
    }

    let mut headers = HeaderMap::new();
    headers.append(
        header::SET_COOKIE,
        "kamish_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0"
            .parse()
            .unwrap(),
    );
    headers.append(
        header::SET_COOKIE,
        "session_token=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0"
            .parse()
            .unwrap(),
    );

    Ok((
        StatusCode::OK,
        headers,
        Json(LogoutResponse {
            status: "ok",
            message: "Logged out successfully",
        }),
    ))
}

/// GET /api/v1/auth/session
pub async fn session_handler(
    State(state): State<AppState>,
    user: Option<Extension<User>>,
    session: Option<Extension<crate::auth::AuthSession>>,
) -> AppResult<Json<SessionResponse>> {
    let current_user = match user {
        Some(Extension(u)) => u,
        None => {
            // If auth is disabled (e.g. tests/dev), provide a mock admin user
            if !state.auth.is_auth_enabled() {
                return Ok(Json(SessionResponse {
                    authenticated: true,
                    user: UserInfo {
                        id: "admin_dev".to_string(),
                        username: "admin".to_string(),
                        must_change_password: false,
                    },
                    expires_at: chrono::Utc::now().to_rfc3339(),
                }));
            }
            return Err(AppError::Unauthorized("Not authenticated".to_string()));
        }
    };

    // Re-check latest user state from DB
    let refreshed_user =
        crate::auth::repository::AuthRepository::find_user_by_id(&state.db, &current_user.id)
            .await?
            .unwrap_or(current_user);

    let expires = session
        .map(|Extension(s)| s.expires_at)
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    Ok(Json(SessionResponse {
        authenticated: true,
        user: UserInfo {
            id: refreshed_user.id,
            username: refreshed_user.username,
            must_change_password: refreshed_user.must_change_password,
        },
        expires_at: expires,
    }))
}
