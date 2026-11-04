pub mod ai;
pub mod api;
pub mod audio;
pub mod auth;
pub mod bluetooth;
pub mod common;
pub mod config;
pub mod database;
pub mod music;
pub mod network;
pub mod scheduler;
pub mod setup;
pub mod speech;
pub mod system;
pub mod vpn;

use axum::Router;
use common::{AppResult, AppState};
use config::Config;
use tracing::info;

/// Build the Axum application and initialize resources (database, state, background tasks)
pub async fn build_app(config: &Config) -> AppResult<(Router, AppState)> {
    let pool = database::init_pool(&config.database_url).await?;

    // Apply database migrations automatically
    database::run_migrations(&pool).await?;

    let auth_enabled = std::env::var("AUTH_ENABLED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or_else(|_| !config.database_url.contains(":memory:"));

    let state = AppState::with_options_and_auth(
        pool.clone(),
        config.ai_cli_path.clone(),
        config.ai_cli_enabled,
        auth_enabled,
    );

    // Initialize default admin if not present
    state.auth.init_default_admin().await?;

    // Load persisted device state (volume, playback)
    state.media.load_from_db(&pool).await;

    // Spawn background scheduler engine
    let scheduler = state.scheduler.clone();
    tokio::spawn(async move {
        scheduler.run().await;
    });

    // Bridge scheduler events to WebSocket event bus
    let mut sched_rx = state.scheduler.subscribe();
    let events = state.events.clone();
    tokio::spawn(async move {
        while let Ok(event) = sched_rx.recv().await {
            match event {
                crate::scheduler::SchedulerEvent::AlarmTriggered(alarm) => {
                    events.emit(
                        "alarm_triggered",
                        serde_json::json!({
                            "id": alarm.id,
                            "time": alarm.time,
                            "label": alarm.label
                        }),
                    );
                }
                crate::scheduler::SchedulerEvent::ReminderTriggered(reminder) => {
                    events.emit(
                        "reminder_triggered",
                        serde_json::json!({
                            "id": reminder.id,
                            "text": reminder.text
                        }),
                    );
                }
            }
        }
    });

    let router = api::create_router(state.clone());

    // Optionally start the microphone capture loop at boot.
    // Off by default so tests and dev runs never grab a real device unexpectedly.
    let autostart = std::env::var("KAMISH_AUTOSTART_CAPTURE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if autostart {
        let capture = state.capture.clone();
        let boot_state = state.clone();
        tokio::spawn(async move {
            if capture.start(boot_state).await {
                info!("Microphone capture autostarted");
            } else {
                info!("Microphone capture autostart skipped: no usable audio source");
            }
        });
    }

    info!("Application successfully initialized");
    Ok((router, state))
}
