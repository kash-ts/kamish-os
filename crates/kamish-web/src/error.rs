use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

pub type WebResult<T> = Result<T, WebError>;

#[derive(Error, Debug)]
pub enum WebError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Password change required: {0}")]
    PasswordChangeRequired(String),

    #[error("Core service unavailable: {0}")]
    CoreUnavailable(String),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            WebError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone()),
            WebError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg.clone()),
            WebError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg.clone()),
            WebError::PasswordChangeRequired(msg) => (
                StatusCode::FORBIDDEN,
                "PASSWORD_CHANGE_REQUIRED",
                msg.clone(),
            ),
            WebError::Validation(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg.clone()),
            WebError::CoreUnavailable(msg) => {
                (StatusCode::SERVICE_UNAVAILABLE, "CORE_OFFLINE", msg.clone())
            }
            WebError::Database(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                err.to_string(),
            ),
            WebError::Config(msg) => (StatusCode::BAD_REQUEST, "CONFIG_ERROR", msg.clone()),
            WebError::Internal(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                err.to_string(),
            ),
        };

        let body = Json(ErrorResponse {
            error: message,
            code: code.to_string(),
        });

        (status, body).into_response()
    }
}
