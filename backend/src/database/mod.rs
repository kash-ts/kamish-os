use crate::common::{AppError, AppResult};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::str::FromStr;
use tracing::info;

pub type DbPool = Pool<Sqlite>;

/// Initialize the SQLite connection pool and apply PRAGMA settings
pub async fn init_pool(database_url: &str) -> AppResult<DbPool> {
    info!("Initializing SQLite connection pool at: {}", database_url);

    // Parse options and ensure database file is created if missing
    let options = SqliteConnectOptions::from_str(database_url)
        .map_err(|e| AppError::Config(format!("Invalid database URL: {e}")))?
        .create_if_missing(true)
        .busy_timeout(std::time::Duration::from_secs(5))
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(AppError::Database)?;

    info!("SQLite database connection pool established");
    Ok(pool)
}

/// Run embedded database migrations
pub async fn run_migrations(pool: &DbPool) -> AppResult<()> {
    info!("Running database migrations...");
    sqlx::migrate!("../database/migrations")
        .run(pool)
        .await
        .map_err(AppError::Migration)?;
    info!("Database migrations applied successfully");
    Ok(())
}

/// Count applied migrations
pub async fn get_applied_migrations_count(pool: &DbPool) -> i64 {
    let result: Result<(i64,), _> =
        sqlx::query_as("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = 1")
            .fetch_one(pool)
            .await;

    match result {
        Ok((count,)) => count,
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_db_and_migrations() {
        let pool = init_pool("sqlite::memory:")
            .await
            .expect("Failed to init memory db");
        run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        let count = get_applied_migrations_count(&pool).await;
        assert!(count >= 2, "Expected at least 2 migrations applied");

        // Verify base tables exist
        let row: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='settings'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, 1);

        // Verify auth tables exist (0002_auth_schema.sql)
        let users_row: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='users'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(users_row.0, 1);

        let sessions_row: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='auth_sessions'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(sessions_row.0, 1);
    }
}
