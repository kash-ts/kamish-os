use crate::auth::User;
use crate::error::{WebError, WebResult};
use crate::state::WebState;
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
    State(state): State<WebState>,
    Json(payload): Json<LoginRequest>,
) -> WebResult<impl IntoResponse> {
    let (session, user) = state
        .auth
        .authenticate(&payload.username, &payload.password)
        .await?;

    let cookie = format!(
        "kamish_session={}; HttpOnly; Path=/; SameSite=Lax",
        session.token
    );

    let resp = LoginResponse {
        token: session.token,
        user: UserInfo {
            id: user.id,
            username: user.username,
            must_change_password: user.must_change_password,
        },
        expires_at: session.expires_at,
    };

    Ok((StatusCode::OK, [(header::SET_COOKIE, cookie)], Json(resp)))
}

/// POST /api/v1/auth/logout
pub async fn logout_handler(
    State(state): State<WebState>,
    headers: HeaderMap,
) -> WebResult<impl IntoResponse> {
    if let Some(token) = extract_token_from_headers(&headers) {
        state.auth.logout(&token).await?;
    }

    let clear_cookie = "kamish_session=; HttpOnly; Path=/; SameSite=Lax; Max-Age=0";

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, clear_cookie)],
        Json(LogoutResponse {
            status: "success",
            message: "Logged out successfully",
        }),
    ))
}

/// POST /api/v1/auth/change-password
pub async fn change_password_handler(
    State(state): State<WebState>,
    Extension(user): Extension<User>,
    Json(payload): Json<ChangePasswordRequest>,
) -> WebResult<impl IntoResponse> {
    state
        .auth
        .change_password(&user.id, &payload.old_password, &payload.new_password)
        .await?;

    Ok((
        StatusCode::OK,
        Json(ChangePasswordResponse {
            status: "success",
            message: "Password changed successfully",
            must_change_password: false,
        }),
    ))
}

/// GET /api/v1/auth/session
pub async fn session_handler(
    State(state): State<WebState>,
    headers: HeaderMap,
) -> WebResult<impl IntoResponse> {
    if !state.auth.is_auth_enabled() {
        return Ok((
            StatusCode::OK,
            Json(SessionResponse {
                authenticated: true,
                user: UserInfo {
                    id: "system".to_string(),
                    username: "admin".to_string(),
                    must_change_password: false,
                },
                expires_at: "".to_string(),
            }),
        ));
    }

    let token = match extract_token_from_headers(&headers) {
        Some(t) => t,
        None => return Err(WebError::Unauthorized("No session found".to_string())),
    };

    let (session, user) = match state.auth.validate_session(&token).await? {
        Some(val) => val,
        None => {
            return Err(WebError::Unauthorized(
                "Invalid or expired session".to_string(),
            ))
        }
    };

    Ok((
        StatusCode::OK,
        Json(SessionResponse {
            authenticated: true,
            user: UserInfo {
                id: user.id,
                username: user.username,
                must_change_password: user.must_change_password,
            },
            expires_at: session.expires_at,
        }),
    ))
}

fn extract_token_from_headers(headers: &HeaderMap) -> Option<String> {
    if let Some(auth) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            return Some(token.trim().to_string());
        }
    }
    if let Some(cookie) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) {
        for part in cookie.split(';') {
            let part = part.trim();
            if let Some(token) = part.strip_prefix("kamish_session=") {
                return Some(token.to_string());
            }
        }
    }
    None
}
