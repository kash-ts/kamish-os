use crate::common::{AppError, AppResult};
use crate::database::DbPool;
use crate::setup::models::StepStatus;
use chrono::Utc;
use serde_json::Value;

pub struct SetupRepository;

impl SetupRepository {
    pub async fn is_setup_completed(pool: &DbPool) -> AppResult<bool> {
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT value FROM device_state WHERE key = 'setup:completed'",
        )
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row.map(|r| r.0 == "true").unwrap_or(false))
    }

    pub async fn get_completed_at(pool: &DbPool) -> AppResult<Option<String>> {
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT value FROM device_state WHERE key = 'setup:completed_at'",
        )
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row.map(|r| r.0))
    }

    pub async fn set_setup_completed(pool: &DbPool, completed: bool) -> AppResult<String> {
        let now = Utc::now().to_rfc3339();
        let val = if completed { "true" } else { "false" };

        sqlx::query(
            r#"
            INSERT INTO device_state (key, value, updated_at)
            VALUES ('setup:completed', ?, ?)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
            "#,
        )
        .bind(val)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        if completed {
            sqlx::query(
                r#"
                INSERT INTO device_state (key, value, updated_at)
                VALUES ('setup:completed_at', ?, ?)
                ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
                "#,
            )
            .bind(&now)
            .bind(&now)
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        }

        Ok(now)
    }

    pub async fn get_current_step(pool: &DbPool) -> AppResult<String> {
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT value FROM device_state WHERE key = 'setup:current_step'",
        )
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row.map(|r| r.0).unwrap_or_else(|| "network".to_string()))
    }

    pub async fn get_step_status(
        pool: &DbPool,
        step: &str,
    ) -> AppResult<(StepStatus, Option<Value>, Option<String>)> {
        let key = format!("setup:step:{}", step);
        let row = sqlx::query_as::<_, (String, String)>(
            "SELECT value, CAST(updated_at AS TEXT) FROM device_state WHERE key = ?",
        )
        .bind(&key)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        if let Some((json_val, updated_at)) = row {
            if let Ok(parsed) = serde_json::from_str::<Value>(&json_val) {
                let status_str = parsed
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("pending");
                let status = match status_str {
                    "completed" => StepStatus::Completed,
                    "in_progress" => StepStatus::InProgress,
                    "skipped" => StepStatus::Skipped,
                    "degraded" => StepStatus::Degraded,
                    _ => StepStatus::Pending,
                };
                let data = parsed.get("data").cloned();
                return Ok((status, data, Some(updated_at)));
            }
        }

        Ok((StepStatus::Pending, None, None))
    }

    pub async fn save_step(
        pool: &DbPool,
        step: &str,
        status: StepStatus,
        data: Option<Value>,
    ) -> AppResult<String> {
        let key = format!("setup:step:{}", step);
        let now = Utc::now().to_rfc3339();
        let payload = serde_json::json!({
            "status": status.as_str(),
            "data": data,
        });

        sqlx::query(
            r#"
            INSERT INTO device_state (key, value, updated_at)
            VALUES (?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
            "#,
        )
        .bind(&key)
        .bind(payload.to_string())
        .bind(&now)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        // If step is completed, advance current_step to next stage
        let next_step = if status == StepStatus::Completed {
            match step {
                "network" => "audio",
                "audio" => "voice",
                "voice" => "ai",
                _ => step,
            }
        } else {
            step
        };

        sqlx::query(
            r#"
            INSERT INTO device_state (key, value, updated_at)
            VALUES ('setup:current_step', ?, ?)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
            "#,
        )
        .bind(next_step)
        .bind(&now)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

        Ok(now)
    }
}
