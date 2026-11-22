use crate::error::WebError as AppError;
use crate::state::WebState;
use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};

/// Middleware protecting `/api/v1/*` routes.
/// - Whitelists `/api/v1/status` and `/api/v1/auth/login`.
/// - Enforces `must_change_password` (blocks access with 403 PASSWORD_CHANGE_REQUIRED for non-auth endpoints).
/// - Bypasses when `auth_enabled == false` or `X-Test-Bypass: true`.
pub async fn auth_middleware(
    State(state): State<WebState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 1. Check if auth is globally disabled or test bypass requested
    if !state.auth.is_auth_enabled() {
        return Ok(next.run(req).await);
    }

    if req
        .headers()
        .get("X-Test-Bypass")
        .and_then(|v| v.to_str().ok())
        == Some("true")
    {
        return Ok(next.run(req).await);
    }

    let path = req.uri().path();

    // 2. Whitelisted endpoints that never require authentication.
    // Note: Axum `.nest("/api/v1", ...)` strips the prefix before middleware runs,
    // so `path` is `/auth/login` or `/status` (keep prefixed paths as fallback).
    if matches!(
        path,
        "/status"
            | "/api/v1/status"
            | "/v1/status"
            | "/auth/login"
            | "/api/v1/auth/login"
            | "/v1/auth/login"
    ) {
        return Ok(next.run(req).await);
    }

    // 3. Extract session token from Authorization header, custom header, cookie, or query
    let token = match extract_token(&req) {
        Some(t) => t,
        None => {
            return Err(AppError::Unauthorized(
                "Authentication required. Missing session token".to_string(),
            ));
        }
    };

    // 4. Validate session in database
    let (session, user) = match state.auth.validate_session(&token).await? {
        Some(pair) => pair,
        None => {
            return Err(AppError::Unauthorized(
                "Invalid or expired session token".to_string(),
            ));
        }
    };

    // 5. Enforce password change requirement
    if user.must_change_password {
        let is_auth_endpoint = matches!(
            path,
            "/auth/change-password"
                | "/api/v1/auth/change-password"
                | "/v1/auth/change-password"
                | "/auth/session"
                | "/api/v1/auth/session"
                | "/v1/auth/session"
                | "/auth/logout"
                | "/api/v1/auth/logout"
                | "/v1/auth/logout"
                | "/setup/status"
                | "/api/v1/setup/status"
                | "/v1/setup/status"
                | "/setup/step"
                | "/api/v1/setup/step"
                | "/v1/setup/step"
                | "/setup/complete"
                | "/api/v1/setup/complete"
                | "/v1/setup/complete"
                | "/network/status"
                | "/api/v1/network/status"
                | "/v1/network/status"
                | "/network/wifi/scan"
                | "/api/v1/network/wifi/scan"
                | "/v1/network/wifi/scan"
                | "/network/scan"
                | "/api/v1/network/scan"
                | "/v1/network/scan"
                | "/network/wifi/connect"
                | "/api/v1/network/wifi/connect"
                | "/v1/network/wifi/connect"
                | "/network/connect"
                | "/api/v1/network/connect"
                | "/v1/network/connect"
        );

        if !is_auth_endpoint {
            return Err(AppError::PasswordChangeRequired(
                "Default administrator password must be changed before using the system"
                    .to_string(),
            ));
        }
    }

    // 6. Attach authenticated user and session to request extensions
    req.extensions_mut().insert(user);
    req.extensions_mut().insert(session);

    Ok(next.run(req).await)
}

fn extract_token(req: &Request) -> Option<String> {
    // 1. Authorization: Bearer <token>
    if let Some(auth_header) = req.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                let token = token.trim();
                if !token.is_empty() {
                    return Some(token.to_string());
                }
            }
        }
    }

    // 2. X-Session-Token: <token>
    if let Some(token_header) = req.headers().get("X-Session-Token") {
        if let Ok(token_str) = token_header.to_str() {
            let token = token_str.trim();
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }
    }

    // 3. Cookie: kamish_session=<token>; session_token=<token>
    if let Some(cookie_header) = req.headers().get(header::COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if let Some(val) = cookie.strip_prefix("kamish_session=") {
                    let val = val.trim();
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
                if let Some(val) = cookie.strip_prefix("session_token=") {
                    let val = val.trim();
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
            }
        }
    }

    // 4. Query: ?token=<token>
    if let Some(query) = req.uri().query() {
        for pair in query.split('&') {
            if let Some(token) = pair.strip_prefix("token=") {
                let token = token.trim();
                if !token.is_empty() {
                    return Some(token.to_string());
                }
            }
        }
    }

    None
}
