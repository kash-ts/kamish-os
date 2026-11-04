use crate::error::WebResult;
use crate::state::WebState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use kamish_ipc::CoreRequest;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SpeechConfigPayload {
    pub wakeword_enabled: Option<bool>,
    pub vad_threshold: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct SynthesizePayload {
    pub text: String,
}

pub async fn get_speech_status_handler(
    State(state): State<WebState>,
) -> WebResult<impl IntoResponse> {
    let status = state.call_core(CoreRequest::GetSpeechStatus).await?;
    Ok((StatusCode::OK, Json(status)))
}

pub async fn update_speech_config_handler(
    State(state): State<WebState>,
    Json(payload): Json<SpeechConfigPayload>,
) -> WebResult<impl IntoResponse> {
    if let Some(enabled) = payload.wakeword_enabled {
        let _ = state
            .call_core(CoreRequest::SetWakeWordDetection { enabled })
            .await?;
    }
    if let Some(threshold) = payload.vad_threshold {
        let _ = state
            .call_core(CoreRequest::SetVadThreshold { threshold })
            .await?;
    }
    let status = state.call_core(CoreRequest::GetSpeechStatus).await?;
    Ok((StatusCode::OK, Json(status)))
}

pub async fn synthesize_handler(
    State(state): State<WebState>,
    Json(payload): Json<SynthesizePayload>,
) -> WebResult<impl IntoResponse> {
    let result = state
        .call_core(CoreRequest::SynthesizeSpeech { text: payload.text })
        .await?;
    Ok((StatusCode::OK, Json(result)))
}

pub async fn start_capture_handler(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let res = state.call_core(CoreRequest::StartCapture).await?;
    Ok((StatusCode::OK, Json(res)))
}

pub async fn stop_capture_handler(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let res = state.call_core(CoreRequest::StopCapture).await?;
    Ok((StatusCode::OK, Json(res)))
}
