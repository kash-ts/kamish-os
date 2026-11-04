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

impl AppError {
    pub fn status_code(&self) -> u16 {
        match self {
            AppError::NotFound(_) => 404,
            AppError::Unauthorized(_) => 401,
            AppError::Forbidden(_) | AppError::PasswordChangeRequired(_) => 403,
            AppError::Validation(_) | AppError::NetworkAuth(_) => 400,
            AppError::Database(_)
            | AppError::Migration(_)
            | AppError::Config(_)
            | AppError::System(_)
            | AppError::Internal(_) => 500,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Unauthorized(_) => "UNAUTHORIZED",
            AppError::Forbidden(_) => "FORBIDDEN",
            AppError::PasswordChangeRequired(_) => "PASSWORD_CHANGE_REQUIRED",
            AppError::Validation(_) => "VALIDATION_ERROR",
            AppError::NetworkAuth(_) => "NETWORK_AUTH_ERROR",
            AppError::Database(_) => "DATABASE_ERROR",
            AppError::Migration(_) => "MIGRATION_ERROR",
            AppError::Config(_) => "CONFIG_ERROR",
            AppError::System(_) => "SYSTEM_ERROR",
            AppError::Internal(_) => "INTERNAL_ERROR",
        }
    }
}
