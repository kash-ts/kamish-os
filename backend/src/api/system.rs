use crate::audio::AudioRoute;
use crate::common::{AppError, AppResult, AppState};
use crate::system::hardware::{get_hardware_telemetry, HardwareTelemetry};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

/// GET /api/v1/system/hardware
/// Return comprehensive real-time hardware telemetry and peripheral diagnostics.
pub async fn get_hardware_status(State(state): State<AppState>) -> Json<HardwareTelemetry> {
    let routing = state.audio.get_routing_info().await;
    let telemetry = get_hardware_telemetry(routing);
    Json(telemetry)
}

#[derive(Debug, Deserialize)]
pub struct SetVolumeRequest {
    pub volume: u8,
}

#[derive(Debug, Serialize)]
pub struct SetVolumeResponse {
    pub success: bool,
    pub requested_volume: u8,
    pub applied_volume: u8,
    pub safe_ceiling: u8,
}

/// POST /api/v1/system/audio/volume
/// Set volume with safe ceiling enforcement.
pub async fn set_audio_volume(
    State(state): State<AppState>,
    Json(payload): Json<SetVolumeRequest>,
) -> Json<SetVolumeResponse> {
    let applied = state.audio.set_volume(payload.volume);
    Json(SetVolumeResponse {
        success: true,
        requested_volume: payload.volume,
        applied_volume: applied,
        safe_ceiling: state.audio.get_safe_ceiling(),
    })
}

#[derive(Debug, Deserialize)]
pub struct SetRouteRequest {
    pub route: AudioRoute,
}

#[derive(Debug, Serialize)]
pub struct SetRouteResponse {
    pub success: bool,
    pub route: AudioRoute,
    pub description: &'static str,
}

/// POST /api/v1/system/audio/route
/// Switch audio output route.
pub async fn set_audio_route(
    State(state): State<AppState>,
    Json(payload): Json<SetRouteRequest>,
) -> Json<SetRouteResponse> {
    state.audio.set_route(payload.route).await;
    Json(SetRouteResponse {
        success: true,
        route: payload.route,
        description: payload.route.description(),
    })
}

#[derive(Debug, Deserialize)]
pub struct SetCeilingRequest {
    pub ceiling: u8,
}

/// POST /api/v1/system/audio/ceiling
/// Adjust safe volume ceiling (1..=100) to prevent clipping.
pub async fn set_volume_ceiling(
    State(state): State<AppState>,
    Json(payload): Json<SetCeilingRequest>,
) -> AppResult<Json<SetVolumeResponse>> {
    if payload.ceiling == 0 || payload.ceiling > 100 {
        return Err(AppError::Validation(
            "Safe ceiling must be between 1 and 100%".to_string(),
        ));
    }
    state.audio.set_safe_ceiling(payload.ceiling);
    let current = state.audio.get_volume();
    Ok(Json(SetVolumeResponse {
        success: true,
        requested_volume: current,
        applied_volume: current,
        safe_ceiling: state.audio.get_safe_ceiling(),
    }))
}

/// POST /api/v1/system/audio/test
/// Plays test tone on speaker.
pub async fn play_audio_test_handler() -> Json<serde_json::Value> {
    crate::audio::alert::play_alert_tone(crate::audio::alert::AlertType::Alarm).await;
    Json(serde_json::json!({ "status": "ok", "message": "Test tone played" }))
}

// =========================================================================
// OTA Updates & RAUC A/B Slot System Handlers
// =========================================================================

use crate::system::ssh::{
    get_ssh_status as sys_get_ssh_status, toggle_ssh as sys_toggle_ssh, SshStatus,
    SshToggleRequest, SshToggleResponse,
};
use crate::system::update::{
    get_update_status as sys_get_update_status, inspect_bundle, install_bundle, UpdateBundleInfo,
    UpdateInstallResult, UpdateStatus,
};
use axum::extract::Multipart;
use std::path::PathBuf;

/// GET /api/v1/system/update/status
/// Returns active slot (A/B), boot primary, installed version, and updating status.
pub async fn get_update_status_handler() -> Json<UpdateStatus> {
    Json(sys_get_update_status())
}

/// POST /api/v1/system/update/upload
/// Upload a .raucb firmware bundle file, verify signature & compatibility, prepare for installation.
pub async fn upload_update_bundle_handler(
    mut multipart: Multipart,
) -> AppResult<Json<UpdateBundleInfo>> {
    let mut saved_path: Option<PathBuf> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::System(format!("Multipart upload error: {}", e)))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "bundle" || name == "file" || name == "firmware" {
            let file_name = field
                .file_name()
                .unwrap_or("kamish_update.raucb")
                .to_string();
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::System(format!("Failed to read upload stream: {}", e)))?;

            let tmp_dir = std::env::temp_dir();
            let target_file = tmp_dir.join(if file_name.ends_with(".raucb") {
                &file_name
            } else {
                "kamish_update.raucb"
            });

            tokio::fs::write(&target_file, &data).await.map_err(|e| {
                AppError::System(format!("Failed to write temporary bundle: {}", e))
            })?;

            saved_path = Some(target_file);
            break;
        }
    }

    let bundle_path = saved_path.ok_or_else(|| {
        AppError::Validation(
            "No bundle file provided in multipart form (field: bundle)".to_string(),
        )
    })?;

    let bundle_info = inspect_bundle(&bundle_path).map_err(AppError::Validation)?;
    Ok(Json(bundle_info))
}

#[derive(Debug, Deserialize)]
pub struct InstallUpdateRequest {
    pub bundle_path: Option<String>,
}

/// POST /api/v1/system/update/install
/// Trigger installation of uploaded bundle into the inactive slot.
pub async fn install_update_handler(
    State(state): State<AppState>,
    Json(payload): Json<InstallUpdateRequest>,
) -> AppResult<Json<UpdateInstallResult>> {
    let bundle_path = if let Some(path_str) = payload.bundle_path {
        PathBuf::from(path_str)
    } else {
        std::env::temp_dir().join("kamish_update.raucb")
    };

    let result = install_bundle(&bundle_path, &state.events)
        .await
        .map_err(AppError::System)?;

    Ok(Json(result))
}

// =========================================================================
// Dropbear SSH System Handlers (Disabled by default, temporary credentials)
// =========================================================================

/// GET /api/v1/system/ssh/status
/// Check if Dropbear SSH is enabled, active sessions, and security flags.
pub async fn get_ssh_status_handler() -> Json<SshStatus> {
    Json(sys_get_ssh_status())
}

/// POST /api/v1/system/ssh/toggle
/// Enable or disable Dropbear SSH. Generates fresh temporary password on enable.
pub async fn toggle_ssh_handler(
    Json(payload): Json<SshToggleRequest>,
) -> AppResult<Json<SshToggleResponse>> {
    let res = sys_toggle_ssh(payload).map_err(AppError::System)?;
    Ok(Json(res))
}
