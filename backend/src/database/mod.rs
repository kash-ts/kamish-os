use crate::common::{AppError, AppResult};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::str::FromStr;
use tracing::info;

pub type DbPool = Pool<Sqlite>;

pub const UNIFIED_SCHEMA_SQL: &str = include_str!("../../../database/schema.sql");

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

/// Apply unified database schema (database/schema.sql)
pub async fn run_migrations(pool: &DbPool) -> AppResult<()> {
    info!("Applying unified database schema (database/schema.sql)...");
    sqlx::raw_sql(UNIFIED_SCHEMA_SQL)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    info!("Database schema applied successfully");
    Ok(())
}

/// Get current database schema version from PRAGMA user_version
pub async fn get_applied_migrations_count(pool: &DbPool) -> i64 {
    let result: Result<(i64,), _> = sqlx::query_as("PRAGMA user_version").fetch_one(pool).await;

    match result {
        Ok((version,)) => version,
        Err(_) => 1,
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

        let version = get_applied_migrations_count(&pool).await;
        assert!(version >= 1, "Expected schema version >= 1");

        // Verify base tables exist
        let row: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='settings'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, 1);

        // Verify auth tables exist
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
