use crate::ai::client::AiReadiness;
use crate::common::AppState;
use crate::database::get_applied_migrations_count;
use crate::scheduler::models::SchedulerStatusSummary;
use crate::system::hardware::{get_hardware_telemetry, HardwareTelemetry};
use crate::system::{get_device_info, DeviceInfo};
use axum::{extract::State, Json};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct SystemReadiness {
    pub is_ready: bool,
    pub setup_complete: bool,
    pub issues: Vec<String>,
    pub ai: AiReadiness,
}

#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub app_uptime_secs: u64,
    pub device: DeviceInfo,
    pub database: DatabaseStatus,
    pub subsystems: SubsystemStatus,
    pub scheduler: SchedulerStatusSummary,
    pub hardware: HardwareTelemetry,
    pub readiness: SystemReadiness,
}

#[derive(Debug, Serialize)]
pub struct DatabaseStatus {
    pub connected: bool,
    pub applied_migrations: i64,
}

#[derive(Debug, Serialize)]
pub struct SubsystemStatus {
    pub audio: &'static str,
    pub bluetooth: &'static str,
    pub network: &'static str,
    pub vpn: &'static str,
    pub speech: &'static str,
    pub ai: &'static str,
    pub scheduler: &'static str,
    pub music: &'static str,
}

pub async fn get_status(State(state): State<AppState>) -> Json<StatusResponse> {
    let app_uptime_secs = state.start_time.elapsed().as_secs();
    let applied_migrations = get_applied_migrations_count(&state.db).await;
    let device = get_device_info();

    // Check DB connection
    let db_connected = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();

    // Check AI subsystem readiness
    let ai_readiness = state.ai.get_readiness().await;

    let mut issues = Vec::new();
    if !db_connected {
        issues.push("База данных SQLite недоступна".to_string());
    }

    if !ai_readiness.ready {
        let reason = ai_readiness
            .error_reason
            .clone()
            .unwrap_or_else(|| "бинарник CLI не найден или не исполняем".to_string());
        issues.push(format!(
            "ИИ-провайдер Antigravity CLI не настроен: {}",
            reason
        ));
    }

    let is_ready = issues.is_empty();
    let setup_complete = issues.is_empty();

    let readiness = SystemReadiness {
        is_ready,
        setup_complete,
        issues,
        ai: ai_readiness,
    };

    // Get live scheduler stats
    let scheduler_summary = state.scheduler.get_status_summary().await;

    // Get hardware telemetry
    let routing = state.audio.get_routing_info().await;
    let hardware = get_hardware_telemetry(routing);

    let ai_subsystem_status = if readiness.ai.ready {
        if readiness.ai.mode == "cli" {
            "cli_ready"
        } else {
            "mock_ready"
        }
    } else {
        "not_configured"
    };

    let net_status = state.network.get_status().await;
    let is_network_connected = net_status.ethernet.connected
        || net_status
            .wifi
            .as_ref()
            .map(|w| w.connected)
            .unwrap_or(false)
        || net_status.ip_address.is_some();
    let network_subsystem = if is_network_connected {
        "connected"
    } else if net_status.ap_mode.active {
        "ap_mode"
    } else {
        "disconnected"
    };

    let bt_status = state.bluetooth.get_status().await;
    let bluetooth_subsystem = if !bt_status.adapter_enabled {
        "disabled"
    } else if bt_status.connected_device.is_some() {
        "connected"
    } else {
        "idle"
    };

    let vpn_status = state.vpn.get_status().await;
    let vpn_subsystem = if vpn_status.enabled && vpn_status.status == "connected" {
        "connected"
    } else {
        "disconnected"
    };

    let speech_status = state.speech.get_status();
    let speech_subsystem = if speech_status.microphone_available {
        "ready"
    } else {
        "no_mic"
    };

    let audio_subsystem = if state.audio.is_user_muted() {
        "muted"
    } else {
        "ready"
    };

    let music_status = state.media.get_status().await;
    let music_subsystem = match music_status {
        crate::music::PlaybackStatus::Playing => "playing",
        crate::music::PlaybackStatus::Paused => "paused",
        crate::music::PlaybackStatus::Stopped => "stopped",
    };

    Json(StatusResponse {
        status: "ok",
        version: state.version,
        app_uptime_secs,
        device,
        database: DatabaseStatus {
            connected: db_connected,
            applied_migrations,
        },
        subsystems: SubsystemStatus {
            audio: audio_subsystem,
            bluetooth: bluetooth_subsystem,
            network: network_subsystem,
            vpn: vpn_subsystem,
            speech: speech_subsystem,
            ai: ai_subsystem_status,
            scheduler: "ready",
            music: music_subsystem,
        },
        scheduler: scheduler_summary,
        hardware,
        readiness,
    })
}
