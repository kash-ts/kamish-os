//! SQLite persistence and repository functions for Alarms and Reminders.

use super::models::{Alarm, CreateAlarmRequest, CreateReminderRequest, Reminder};
use crate::common::{AppError, AppResult};
use crate::database::DbPool;
use chrono::Utc;
use uuid::Uuid;

/// Create a new alarm in SQLite.
pub async fn create_alarm(pool: &DbPool, req: &CreateAlarmRequest) -> AppResult<Alarm> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let timezone = req.timezone.clone().unwrap_or_else(|| "UTC".to_string());
    let enabled = req.enabled.unwrap_or(true);

    sqlx::query(
        r#"
        INSERT INTO alarms (id, time, timezone, recurrence, label, enabled, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.time)
    .bind(&timezone)
    .bind(&req.recurrence)
    .bind(&req.label)
    .bind(enabled)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(Alarm {
        id,
        time: req.time.clone(),
        timezone,
        recurrence: req.recurrence.clone(),
        label: req.label.clone(),
        enabled,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// List all alarms from SQLite.
pub async fn list_alarms(pool: &DbPool) -> AppResult<Vec<Alarm>> {
    let alarms = sqlx::query_as::<_, Alarm>(
        r#"
        SELECT id, time, timezone, recurrence, label, enabled,
               CAST(created_at AS TEXT) as created_at,
               CAST(updated_at AS TEXT) as updated_at
        FROM alarms
        ORDER BY time ASC
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(alarms)
}

/// Get alarm by ID.
pub async fn get_alarm_by_id(pool: &DbPool, id: &str) -> AppResult<Option<Alarm>> {
    let alarm = sqlx::query_as::<_, Alarm>(
        r#"
        SELECT id, time, timezone, recurrence, label, enabled,
               CAST(created_at AS TEXT) as created_at,
               CAST(updated_at AS TEXT) as updated_at
        FROM alarms
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(alarm)
}

/// Delete an alarm by ID.
pub async fn delete_alarm(pool: &DbPool, id: &str) -> AppResult<bool> {
    let result = sqlx::query("DELETE FROM alarms WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

    Ok(result.rows_affected() > 0)
}

/// Toggle or set alarm enabled state.
pub async fn toggle_alarm(
    pool: &DbPool,
    id: &str,
    explicit_state: Option<bool>,
) -> AppResult<Alarm> {
    let current = get_alarm_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Alarm with id '{id}' not found")))?;

    let new_state = explicit_state.unwrap_or(!current.enabled);
    let now = Utc::now().to_rfc3339();

    sqlx::query("UPDATE alarms SET enabled = ?, updated_at = ? WHERE id = ?")
        .bind(new_state)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

    get_alarm_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Alarm with id '{id}' not found")))
}

/// Set alarm enabled state directly (e.g. after one-shot alarm fires).
pub async fn set_alarm_enabled(pool: &DbPool, id: &str, enabled: bool) -> AppResult<bool> {
    let now = Utc::now().to_rfc3339();
    let result = sqlx::query("UPDATE alarms SET enabled = ?, updated_at = ? WHERE id = ?")
        .bind(enabled)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

    Ok(result.rows_affected() > 0)
}

/// Count active (enabled) alarms.
pub async fn get_active_alarms_count(pool: &DbPool) -> AppResult<usize> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM alarms WHERE enabled = 1")
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

    Ok(row.0 as usize)
}

/// Create a new reminder in SQLite.
pub async fn create_reminder(pool: &DbPool, req: &CreateReminderRequest) -> AppResult<Reminder> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let timezone = req.timezone.clone().unwrap_or_else(|| "UTC".to_string());
    let status = "pending".to_string();

    sqlx::query(
        r#"
        INSERT INTO reminders (id, trigger_time, text, timezone, status, created_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(&req.trigger_time)
    .bind(&req.text)
    .bind(&timezone)
    .bind(&status)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(Reminder {
        id,
        trigger_time: req.trigger_time.clone(),
        text: req.text.clone(),
        timezone,
        status,
        created_at: now,
    })
}

/// List reminders, optionally filtered by status.
pub async fn list_reminders(
    pool: &DbPool,
    status_filter: Option<&str>,
) -> AppResult<Vec<Reminder>> {
    let reminders = match status_filter {
        Some(status) if !status.trim().is_empty() => {
            sqlx::query_as::<_, Reminder>(
                r#"
                SELECT id, trigger_time, text, timezone, status,
                       CAST(created_at AS TEXT) as created_at
                FROM reminders
                WHERE status = ?
                ORDER BY trigger_time ASC
                "#,
            )
            .bind(status)
            .fetch_all(pool)
            .await
        }
        _ => {
            sqlx::query_as::<_, Reminder>(
                r#"
                SELECT id, trigger_time, text, timezone, status,
                       CAST(created_at AS TEXT) as created_at
                FROM reminders
                ORDER BY trigger_time ASC
                "#,
            )
            .fetch_all(pool)
            .await
        }
    }
    .map_err(AppError::Database)?;

    Ok(reminders)
}

/// Get reminder by ID.
pub async fn get_reminder_by_id(pool: &DbPool, id: &str) -> AppResult<Option<Reminder>> {
    let reminder = sqlx::query_as::<_, Reminder>(
        r#"
        SELECT id, trigger_time, text, timezone, status,
               CAST(created_at AS TEXT) as created_at
        FROM reminders
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(reminder)
}

/// Delete reminder by ID.
pub async fn delete_reminder(pool: &DbPool, id: &str) -> AppResult<bool> {
    let result = sqlx::query("DELETE FROM reminders WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

    Ok(result.rows_affected() > 0)
}

/// Update reminder status (e.g. to "completed" or "cancelled").
pub async fn update_reminder_status(pool: &DbPool, id: &str, status: &str) -> AppResult<bool> {
    let result = sqlx::query("UPDATE reminders SET status = ? WHERE id = ?")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;

    Ok(result.rows_affected() > 0)
}

/// Count pending reminders.
pub async fn get_pending_reminders_count(pool: &DbPool) -> AppResult<usize> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM reminders WHERE status = 'pending'")
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;

    Ok(row.0 as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{init_pool, run_migrations};

    async fn setup_test_db() -> DbPool {
        let pool = init_pool("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_alarm_crud() {
        let pool = setup_test_db().await;

        let req = CreateAlarmRequest {
            time: "08:30".to_string(),
            recurrence: Some("weekdays".to_string()),
            label: Some("Morning Meeting".to_string()),
            timezone: Some("UTC".to_string()),
            enabled: Some(true),
        };

        let alarm = create_alarm(&pool, &req).await.unwrap();
        assert_eq!(alarm.time, "08:30");
        assert!(alarm.enabled);

        let list = list_alarms(&pool).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, alarm.id);

        let toggled = toggle_alarm(&pool, &alarm.id, None).await.unwrap();
        assert!(!toggled.enabled);

        let deleted = delete_alarm(&pool, &alarm.id).await.unwrap();
        assert!(deleted);

        let empty_list = list_alarms(&pool).await.unwrap();
        assert_eq!(empty_list.len(), 0);
    }

    #[tokio::test]
    async fn test_reminder_crud() {
        let pool = setup_test_db().await;

        let req = CreateReminderRequest {
            trigger_time: "2026-11-01T10:00:00Z".to_string(),
            text: "Drink water".to_string(),
            timezone: Some("UTC".to_string()),
        };

        let reminder = create_reminder(&pool, &req).await.unwrap();
        assert_eq!(reminder.text, "Drink water");
        assert_eq!(reminder.status, "pending");

        let pending_list = list_reminders(&pool, Some("pending")).await.unwrap();
        assert_eq!(pending_list.len(), 1);

        let updated = update_reminder_status(&pool, &reminder.id, "completed")
            .await
            .unwrap();
        assert!(updated);

        let pending_after = list_reminders(&pool, Some("pending")).await.unwrap();
        assert_eq!(pending_after.len(), 0);

        let completed_list = list_reminders(&pool, Some("completed")).await.unwrap();
        assert_eq!(completed_list.len(), 1);

        let deleted = delete_reminder(&pool, &reminder.id).await.unwrap();
        assert!(deleted);
    }
}
