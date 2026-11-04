use crate::common::{AppError, AppResult, AppState};
use crate::speech::models::{
    RecognizeRequest, RecognizeResponse, SimulateWakeWordRequest, SimulateWakeWordResponse,
    SpeechStatus, SynthesizeRequest, SynthesizeResponse, TtsVoice, UpdateSpeechConfigRequest,
};
use crate::speech::SpeechManager;
use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SynthesizeQuery {
    pub text: Option<String>,
    pub voice: Option<String>,
    pub provider: Option<String>,
    pub speed: Option<f32>,
    pub play: Option<bool>,
    pub format: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VoicesResponse {
    pub voices: Vec<TtsVoice>,
}

/// GET /api/v1/speech/status
pub async fn get_speech_status_handler(State(state): State<AppState>) -> Json<SpeechStatus> {
    Json(state.speech.get_status())
}

/// GET /api/v1/speech/voices
pub async fn list_voices_handler(State(state): State<AppState>) -> Json<VoicesResponse> {
    Json(VoicesResponse {
        voices: state.speech.list_voices(),
    })
}

/// POST /api/v1/speech/config or PUT /api/v1/speech/config
pub async fn update_speech_config_handler(
    State(state): State<AppState>,
    Json(payload): Json<UpdateSpeechConfigRequest>,
) -> AppResult<Json<SpeechStatus>> {
    state.speech.update_config(payload)?;
    Ok(Json(state.speech.get_status()))
}

/// POST /api/v1/speech/synthesize (and GET /api/v1/speech/synthesize for audio tags)
pub async fn synthesize_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SynthesizeQuery>,
    body: Option<Json<SynthesizeRequest>>,
) -> AppResult<Response> {
    let text = if let Some(Json(ref b)) = body {
        b.text.clone()
    } else if let Some(ref t) = query.text {
        t.clone()
    } else {
        return Err(AppError::Config(
            "Text parameter is required for speech synthesis".to_string(),
        ));
    };

    let voice = body
        .as_ref()
        .and_then(|b| b.voice.as_deref())
        .or(query.voice.as_deref());

    let provider = body
        .as_ref()
        .and_then(|b| b.provider.as_deref())
        .or(query.provider.as_deref());

    let speed = body.as_ref().and_then(|b| b.speed).or(query.speed);

    let play_on_speaker = body
        .as_ref()
        .and_then(|b| b.play_on_speaker)
        .or(query.play)
        .unwrap_or(false);

    let audio = state
        .speech
        .synthesize(&text, voice, provider, speed)
        .await?;

    if play_on_speaker {
        let is_mp3 = audio.format == "mp3";
        SpeechManager::play_audio_bytes(&audio.bytes, is_mp3).await?;

        let resp = SynthesizeResponse {
            status: "playing".to_string(),
            provider: audio.provider,
            voice: audio.voice,
            audio_format: audio.format.to_string(),
            size_bytes: audio.bytes.len(),
            played_on_speaker: true,
            audio_base64: None,
        };

        return Ok((StatusCode::OK, Json(resp)).into_response());
    }

    // Check if client expects JSON response
    let wants_json = query.format.as_deref() == Some("json")
        || headers
            .get(header::ACCEPT)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.contains("application/json"))
            .unwrap_or(false)
            && query.format.as_deref() != Some("audio");

    if wants_json && body.is_some() {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&audio.bytes);
        let resp = SynthesizeResponse {
            status: "ok".to_string(),
            provider: audio.provider,
            voice: audio.voice,
            audio_format: audio.format.to_string(),
            size_bytes: audio.bytes.len(),
            played_on_speaker: false,
            audio_base64: Some(b64),
        };
        return Ok((StatusCode::OK, Json(resp)).into_response());
    }

    // Return binary audio stream
    let content_type = match audio.format {
        "mp3" => "audio/mpeg",
        _ => "audio/wav",
    };

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type),
            (
                header::CONTENT_DISPOSITION,
                &format!("inline; filename=\"speech.{}\"", audio.format),
            ),
        ],
        audio.bytes,
    )
        .into_response())
}

/// POST /api/v1/speech/recognize
pub async fn recognize_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<RecognizeRequest>,
    body_bytes: Bytes,
) -> AppResult<Json<RecognizeResponse>> {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let (audio_data, language, provider, dispatch_ai) = if content_type.contains("application/json")
    {
        let payload: RecognizeRequest = serde_json::from_slice(&body_bytes)
            .map_err(|e| AppError::Config(format!("Invalid JSON payload: {e}")))?;

        let bytes = if let Some(ref b64) = payload.audio_base64 {
            base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|e| AppError::Config(format!("Invalid base64 audio data: {e}")))?
        } else {
            return Err(AppError::Config(
                "Either 'audio_base64' in JSON or raw audio binary body must be provided"
                    .to_string(),
            ));
        };

        (
            bytes,
            payload.language.or(query.language),
            payload.provider.or(query.provider),
            payload.dispatch_ai.or(query.dispatch_ai).unwrap_or(false),
        )
    } else {
        // Raw audio bytes
        if body_bytes.is_empty() {
            return Err(AppError::Config(
                "Request body contains no audio data".to_string(),
            ));
        }

        (
            body_bytes.to_vec(),
            query.language,
            query.provider,
            query.dispatch_ai.unwrap_or(false),
        )
    };

    let result = state
        .speech
        .recognize_and_dispatch(
            &audio_data,
            language.as_deref(),
            provider.as_deref(),
            dispatch_ai,
            &state,
        )
        .await?;

    Ok(Json(result))
}

/// POST /api/v1/speech/wakeword/simulate
pub async fn simulate_wakeword_handler(
    State(state): State<AppState>,
    Json(payload): Json<SimulateWakeWordRequest>,
) -> AppResult<Json<SimulateWakeWordResponse>> {
    let dispatch = payload.dispatch_ai.unwrap_or(true);
    let resp = state
        .speech
        .simulate_wakeword(
            payload.keyword.as_deref(),
            payload.command.as_deref(),
            dispatch,
            &state,
        )
        .await?;

    Ok(Json(resp))
}

/// GET /api/v1/speech/capture/status
///
/// Reports the real state of the microphone capture loop: which source is in use,
/// whether frames are actually being read, and the last transcript / reply.
pub async fn capture_status_handler(
    State(state): State<AppState>,
) -> Json<crate::speech::capture::runner::CaptureStatus> {
    Json(state.capture.status())
}

/// POST /api/v1/speech/capture/start
pub async fn capture_start_handler(
    State(state): State<AppState>,
) -> AppResult<Json<CaptureActionResponse>> {
    let started = state.capture.start(state.clone()).await;
    let status = state.capture.status();

    if !started {
        return Ok(Json(CaptureActionResponse {
            status: "unavailable".to_string(),
            message: "Audio source is not available on this host, or capture is already running"
                .to_string(),
            capture: status,
        }));
    }

    Ok(Json(CaptureActionResponse {
        status: "started".to_string(),
        message: format!("Listening via '{}'", status.source),
        capture: status,
    }))
}

/// POST /api/v1/speech/capture/stop
pub async fn capture_stop_handler(
    State(state): State<AppState>,
) -> AppResult<Json<CaptureActionResponse>> {
    let stopped = state.capture.stop().await;
    let status = state.capture.status();

    Ok(Json(CaptureActionResponse {
        status: if stopped { "stopped" } else { "idle" }.to_string(),
        message: if stopped {
            "Audio capture stopped".to_string()
        } else {
            "Audio capture was not running".to_string()
        },
        capture: status,
    }))
}

/// Response returned by capture start/stop control endpoints.
#[derive(Debug, Serialize)]
pub struct CaptureActionResponse {
    pub status: String,
    pub message: String,
    pub capture: crate::speech::capture::runner::CaptureStatus,
}
