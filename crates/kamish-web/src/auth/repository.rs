use crate::error::{WebError as AppError, WebResult as AppResult};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool as DbPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub salt: Option<String>,
    pub must_change_password: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuthSession {
    pub token: String,
    pub user_id: String,
    pub created_at: String,
    pub expires_at: String,
}

pub struct AuthRepository;

impl AuthRepository {
    pub async fn find_user_by_username(pool: &DbPool, username: &str) -> AppResult<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, password_hash, salt, must_change_password, 
                   CAST(created_at AS TEXT) as created_at, 
                   CAST(updated_at AS TEXT) as updated_at
            FROM users 
            WHERE username = ?
            "#,
        )
        .bind(username)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(user)
    }

    pub async fn find_user_by_id(pool: &DbPool, user_id: &str) -> AppResult<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, password_hash, salt, must_change_password, 
                   CAST(created_at AS TEXT) as created_at, 
                   CAST(updated_at AS TEXT) as updated_at
            FROM users 
            WHERE id = ?
            "#,
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(user)
    }

    pub async fn update_password(
        pool: &DbPool,
        user_id: &str,
        new_hash: &str,
        new_salt: &str,
    ) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE users 
            SET password_hash = ?, salt = ?, must_change_password = 0, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(new_hash)
        .bind(new_salt)
        .bind(now)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(())
    }

    pub async fn create_session(
        pool: &DbPool,
        user_id: &str,
        duration: Duration,
    ) -> AppResult<AuthSession> {
        let token = format!("sess_{}", Uuid::new_v4().simple());
        let now = Utc::now();
        let expires_at = now + duration;
        let created_at_str = now.to_rfc3339();
        let expires_at_str = expires_at.to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO auth_sessions (token, user_id, created_at, expires_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&token)
        .bind(user_id)
        .bind(&created_at_str)
        .bind(&expires_at_str)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(AuthSession {
            token,
            user_id: user_id.to_string(),
            created_at: created_at_str,
            expires_at: expires_at_str,
        })
    }

    pub async fn find_session_by_token(
        pool: &DbPool,
        token: &str,
    ) -> AppResult<Option<AuthSession>> {
        let now = Utc::now().to_rfc3339();
        let session = sqlx::query_as::<_, AuthSession>(
            r#"
            SELECT token, user_id, 
                   CAST(created_at AS TEXT) as created_at, 
                   CAST(expires_at AS TEXT) as expires_at
            FROM auth_sessions 
            WHERE token = ? AND expires_at > ?
            "#,
        )
        .bind(token)
        .bind(now)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(session)
    }

    pub async fn delete_session(pool: &DbPool, token: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM auth_sessions WHERE token = ?")
            .bind(token)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;

        Ok(())
    }

    pub async fn delete_user_sessions_except(
        pool: &DbPool,
        user_id: &str,
        except_token: Option<&str>,
    ) -> AppResult<()> {
        if let Some(token) = except_token {
            sqlx::query("DELETE FROM auth_sessions WHERE user_id = ? AND token != ?")
                .bind(user_id)
                .bind(token)
                .execute(pool)
                .await
                .map_err(AppError::Database)?;
        } else {
            sqlx::query("DELETE FROM auth_sessions WHERE user_id = ?")
                .bind(user_id)
                .execute(pool)
                .await
                .map_err(AppError::Database)?;
        }
        Ok(())
    }

    pub async fn init_default_admin(pool: &DbPool) -> AppResult<()> {
        let admin = Self::find_user_by_username(pool, "admin").await?;
        if let Some(user) = admin {
            // If user still has must_change_password and hash is invalid, update to valid Argon2id hash
            if user.must_change_password
                && !super::password::verify_password("admin", &user.password_hash)
            {
                let (hash, salt) = super::password::hash_password("admin")?;
                sqlx::query("UPDATE users SET password_hash = ?, salt = ? WHERE id = ?")
                    .bind(hash)
                    .bind(salt)
                    .bind(&user.id)
                    .execute(pool)
                    .await
                    .map_err(AppError::Database)?;
            }
        } else {
            let (hash, salt) = super::password::hash_password("admin")?;
            let now = Utc::now().to_rfc3339();
            sqlx::query(
                r#"
                INSERT INTO users (id, username, password_hash, salt, must_change_password, created_at, updated_at)
                VALUES ('admin_default', 'admin', ?, ?, 1, ?, ?)
                "#,
            )
            .bind(hash)
            .bind(salt)
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        }
        Ok(())
    }
}
