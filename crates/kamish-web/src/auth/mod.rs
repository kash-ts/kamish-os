pub mod middleware;
pub mod password;
pub mod repository;

pub use middleware::auth_middleware;
pub use password::{hash_password, verify_password};
pub use repository::{AuthRepository, AuthSession, User};

use crate::error::{WebError as AppError, WebResult as AppResult};
use chrono::Duration;
use sqlx::SqlitePool as DbPool;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct AuthService {
    pool: DbPool,
    auth_enabled: AtomicBool,
}

impl AuthService {
    pub fn new(pool: DbPool, auth_enabled: bool) -> Self {
        Self {
            pool,
            auth_enabled: AtomicBool::new(auth_enabled),
        }
    }

    pub fn is_auth_enabled(&self) -> bool {
        self.auth_enabled.load(Ordering::SeqCst)
    }

    pub fn set_auth_enabled(&self, enabled: bool) {
        self.auth_enabled.store(enabled, Ordering::SeqCst);
    }

    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> AppResult<(AuthSession, User)> {
        let user = AuthRepository::find_user_by_username(&self.pool, username)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Invalid username or password".to_string()))?;

        if !verify_password(password, &user.password_hash) {
            return Err(AppError::Unauthorized(
                "Invalid username or password".to_string(),
            ));
        }

        // Sessions expire after 7 days
        let session =
            AuthRepository::create_session(&self.pool, &user.id, Duration::days(7)).await?;
        Ok((session, user))
    }

    pub async fn validate_session(&self, token: &str) -> AppResult<Option<(AuthSession, User)>> {
        let session = match AuthRepository::find_session_by_token(&self.pool, token).await? {
            Some(s) => s,
            None => return Ok(None),
        };

        let user = match AuthRepository::find_user_by_id(&self.pool, &session.user_id).await? {
            Some(u) => u,
            None => return Ok(None),
        };

        Ok(Some((session, user)))
    }

    pub async fn change_password(
        &self,
        user_id: &str,
        old_password: &str,
        new_password: &str,
    ) -> AppResult<()> {
        if new_password.trim().len() < 4 {
            return Err(AppError::Validation(
                "New password must be at least 4 characters long".to_string(),
            ));
        }

        let user = AuthRepository::find_user_by_id(&self.pool, user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        if !verify_password(old_password, &user.password_hash) {
            return Err(AppError::Unauthorized(
                "Invalid current password".to_string(),
            ));
        }

        let (new_hash, new_salt) = hash_password(new_password)?;
        AuthRepository::update_password(&self.pool, user_id, &new_hash, &new_salt).await?;

        Ok(())
    }

    pub async fn logout(&self, token: &str) -> AppResult<()> {
        AuthRepository::delete_session(&self.pool, token).await
    }

    pub async fn init_default_admin(&self) -> AppResult<()> {
        AuthRepository::init_default_admin(&self.pool).await
    }
}
