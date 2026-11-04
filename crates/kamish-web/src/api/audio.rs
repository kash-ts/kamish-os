use crate::error::WebResult;
use crate::state::WebState;
use axum::{extract::State, Json};
use kamish_ipc::CoreRequest;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SetVolumeRequest {
    pub volume: u8,
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
