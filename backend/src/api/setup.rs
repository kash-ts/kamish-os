use crate::common::{AppError, AppResult, AppState};
use crate::setup::{
    CompleteSetupResponse, SaveStepRequest, SaveStepResponse, SetupRepository, SetupStatusResponse,
    SimulateDegradationRequest, StepStatus,
};
use axum::{extract::State, Json};
use chrono::Utc;
use serde_json::json;

const MANDATORY_STEPS: [&str; 4] = ["network", "audio", "voice", "ai"];

fn get_next_step(current: &str) -> &'static str {
    match current {
        "network" => "audio",
        "audio" => "voice",
        "voice" => "ai",
        "ai" => "completed",
        _ => "completed",
    }
}

/// GET /api/v1/setup/status
pub async fn get_setup_status_handler(
    State(state): State<AppState>,
) -> AppResult<Json<SetupStatusResponse>> {
    let is_completed = SetupRepository::is_setup_completed(&state.db).await?;
    let completed_at = SetupRepository::get_completed_at(&state.db).await?;
    let current_step = SetupRepository::get_current_step(&state.db).await?;

    let (steps, degraded_components) = state.diagnostics.probe_subsystems(&state).await;

    Ok(Json(SetupStatusResponse {
        is_completed,
        current_step,
        completed_at,
        steps,
        degraded_components,
    }))
}

/// POST /api/v1/setup/step
pub async fn save_setup_step_handler(
    State(state): State<AppState>,
    Json(mut payload): Json<SaveStepRequest>,
) -> AppResult<Json<SaveStepResponse>> {
    let step = payload.step.trim().to_lowercase();
    if !MANDATORY_STEPS.contains(&step.as_str()) {
        return Err(AppError::Validation(format!(
            "Invalid step '{}'. Must be one of: {}",
            step,
            MANDATORY_STEPS.join(", ")
        )));
    }

    let status = payload.status.unwrap_or(StepStatus::Completed);

    // If step is "audio", enforce Safe Volume Ceiling (<= 85%)
    if step == "audio" {
        if let Some(ref mut data) = payload.data {
            if let Some(vol) = data.get("volume").and_then(|v| v.as_u64()) {
                if vol > 85 {
                    if let Some(obj) = data.as_object_mut() {
                        obj.insert("volume".to_string(), json!(85));
                    }
                }
            }
        }
    }

    let updated_at =
        SetupRepository::save_step(&state.db, &step, status, payload.data.clone()).await?;

    let next_step = get_next_step(&step).to_string();

    state.events.emit(
        "setup_step_updated",
        json!({
            "step": step,
            "status": status.as_str(),
            "next_step": next_step,
            "is_completed": false,
            "updated_at": updated_at,
        }),
    );

    Ok(Json(SaveStepResponse {
        success: true,
        step,
        status,
        next_step,
        is_completed: false,
        updated_at,
    }))
}

/// POST /api/v1/setup/complete
pub async fn complete_setup_handler(
    State(state): State<AppState>,
) -> AppResult<Json<CompleteSetupResponse>> {
    // Check if all mandatory stages are completed
    let mut incomplete_stages = Vec::new();
    for mandatory_step in MANDATORY_STEPS {
        let (status, _, _) = SetupRepository::get_step_status(&state.db, mandatory_step).await?;
        if status != StepStatus::Completed && status != StepStatus::Skipped {
            incomplete_stages.push(mandatory_step);
        }
    }

    if !incomplete_stages.is_empty() {
        return Err(AppError::Validation(format!(
            "Cannot complete setup: stages [{}] are not completed",
            incomplete_stages.join(", ")
        )));
    }

    let completed_at = SetupRepository::set_setup_completed(&state.db, true).await?;

    state.events.emit(
        "setup_completed",
        json!({
            "is_completed": true,
            "completed_at": completed_at,
        }),
    );

    Ok(Json(CompleteSetupResponse {
        success: true,
        is_completed: true,
        completed_at,
        message: "Setup wizard successfully completed. Device is ready for normal operation."
            .to_string(),
    }))
}

/// GET /api/v1/setup/diagnostics
pub async fn get_setup_diagnostics_handler(
    State(state): State<AppState>,
) -> AppResult<Json<serde_json::Value>> {
    let (steps, degraded_components) = state.diagnostics.probe_subsystems(&state).await;
    let is_all_healthy = degraded_components.is_empty();

    let routing = state.audio.get_routing_info().await;
    let net = state.network.get_status().await;
    let speech = state.speech.get_status();
    let ai = state.ai.get_readiness().await;

    Ok(Json(json!({
        "healthy": is_all_healthy,
        "timestamp": Utc::now().to_rfc3339(),
        "steps": steps,
        "degraded_components": degraded_components,
        "subsystems": {
            "network": {
                "interface": net.primary_interface,
                "ip": net.ip_address,
                "wifi_connected": net.wifi.as_ref().map(|w| w.connected).unwrap_or(false),
                "ethernet_connected": net.ethernet.connected,
                "ap_active": net.ap_mode.active,
            },
            "audio": {
                "output_device": routing.output_device,
                "safe_ceiling": routing.safe_volume_ceiling,
                "volume": routing.volume,
                "mixer_status": routing.mixer_status,
            },
            "voice": {
                "microphone_available": speech.microphone_available,
                "active_stt": speech.active_stt_provider,
                "active_tts": speech.active_tts_provider,
            },
            "ai": {
                "ready": ai.ready,
                "mode": ai.mode,
                "version": ai.cli_version,
            }
        }
    })))
}

/// POST /api/v1/setup/simulate-degradation
pub async fn simulate_degradation_handler(
    State(state): State<AppState>,
    Json(payload): Json<SimulateDegradationRequest>,
) -> AppResult<Json<serde_json::Value>> {
    state
        .diagnostics
        .simulate_degradation(
            &payload.component,
            payload.degraded,
            payload.error_reason.as_deref(),
        )
        .await?;

    let recovery_step = payload.component.clone();
    let recovery_url = format!("/setup?step={}&recovery=true", payload.component);

    state.events.emit(
        "component_health_changed",
        json!({
            "component": payload.component,
            "healthy": !payload.degraded,
            "status": if payload.degraded { "degraded" } else { "healthy" },
            "recovery_step": recovery_step,
            "recovery_url": recovery_url,
            "reason": payload.error_reason,
            "can_fallback": true,
        }),
    );

    Ok(Json(json!({
        "success": true,
        "component": payload.component,
        "currently_degraded": payload.degraded,
        "error_reason": payload.error_reason,
    })))
}
