use crate::error::WebResult;
use crate::state::WebState;
use axum::{extract::State, Json};
use kamish_ipc::CoreRequest;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SetVolumeRequest {
    pub volume: u8,
}

#[derive(Debug, Deserialize)]
pub struct SetCeilingRequest {
    pub ceiling: u8,
}

#[derive(Debug, Deserialize)]
pub struct SetDeviceRequest {
    /// `"output"` or `"input"`
    pub kind: String,
    pub id: String,
}

pub async fn get_audio_status(State(state): State<WebState>) -> WebResult<Json<serde_json::Value>> {
    let payload = state.call_core(CoreRequest::GetAudioStatus).await?;
    Ok(Json(payload))
}

pub async fn set_volume_handler(
    State(state): State<WebState>,
    Json(payload): Json<SetVolumeRequest>,
) -> WebResult<Json<serde_json::Value>> {
    let resp = state
        .call_core(CoreRequest::SetVolume {
            volume: payload.volume,
        })
        .await?;
    Ok(Json(resp))
}

pub async fn set_ceiling_handler(
    State(state): State<WebState>,
    Json(payload): Json<SetCeilingRequest>,
) -> WebResult<Json<serde_json::Value>> {
    let resp = state
        .call_core(CoreRequest::SetVolumeCeiling {
            ceiling: payload.ceiling,
        })
        .await?;
    Ok(Json(resp))
}

pub async fn list_devices_handler(
    State(state): State<WebState>,
) -> WebResult<Json<serde_json::Value>> {
    let resp = state.call_core(CoreRequest::ListAudioDevices).await?;
    Ok(Json(resp))
}

pub async fn set_device_handler(
    State(state): State<WebState>,
    Json(payload): Json<SetDeviceRequest>,
) -> WebResult<Json<serde_json::Value>> {
    let resp = state
        .call_core(CoreRequest::SetAudioDevice {
            kind: payload.kind,
            id: payload.id,
        })
        .await?;
    Ok(Json(resp))
}

pub async fn test_tone_handler(
    State(state): State<WebState>,
) -> WebResult<Json<serde_json::Value>> {
    let resp = state.call_core(CoreRequest::PlayTestTone).await?;
    Ok(Json(resp))
}
