use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Network authentication error: {0}")]
    NetworkAuth(String),

    #[error("System error: {0}")]
    System(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Password change required: {0}")]
    PasswordChangeRequired(String),

    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg.clone()),
            AppError::PasswordChangeRequired(msg) => (
                StatusCode::FORBIDDEN,
                "PASSWORD_CHANGE_REQUIRED",
                msg.clone(),
            ),
            AppError::Database(err) => {
                tracing::error!("Database query failed: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "A database error occurred".to_string(),
                )
            }
            AppError::Migration(err) => {
                tracing::error!("Migration failed: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "MIGRATION_ERROR",
                    "Database migration failed".to_string(),
                )
            }
            AppError::Config(msg) => (StatusCode::BAD_REQUEST, "CONFIG_ERROR", msg.clone()),
            AppError::Validation(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg.clone()),
            AppError::NetworkAuth(msg) => {
                (StatusCode::BAD_REQUEST, "NETWORK_AUTH_ERROR", msg.clone())
            }
            AppError::System(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "SYSTEM_ERROR",
                msg.clone(),
            ),
            AppError::Internal(err) => {
                tracing::error!("Internal error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal error occurred".to_string(),
                )
            }
        };

        (
            status,
            Json(ErrorResponse {
                error: message,
                code: code.to_string(),
            }),
        )
            .into_response()
    }
}
