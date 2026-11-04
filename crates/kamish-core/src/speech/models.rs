use serde::{Deserialize, Serialize};

/// Result returned from a Speech-To-Text provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttResult {
    pub text: String,
    pub confidence: f32,
    pub language: String,
    pub provider: String,
    pub duration_secs: Option<f32>,
}

/// Event triggered when a wake word is spotted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WakeWordEvent {
    pub keyword: String,
    pub score: f32,
    pub timestamp_ms: u64,
}

/// Description of an available Text-To-Speech voice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsVoice {
    pub id: String,
    pub name: String,
    pub language: String,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
}

/// Overall status of the speech subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechStatus {
    pub microphone_available: bool,
    pub microphone_listening: bool,
    pub active_stt_provider: String,
    pub active_tts_provider: String,
    pub active_voice: String,
    pub vad_enabled: bool,
    pub wakeword_enabled: bool,
    pub available_stt_providers: Vec<String>,
    pub available_tts_providers: Vec<String>,
    pub last_wake_word: Option<WakeWordEvent>,
}

/// Request payload for TTS synthesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizeRequest {
    pub text: String,
    pub voice: Option<String>,
    pub provider: Option<String>,
    pub speed: Option<f32>,
    pub play_on_speaker: Option<bool>,
}

/// Response returned when synthesizing or playing speech.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizeResponse {
    pub status: String,
    pub provider: String,
    pub voice: String,
    pub audio_format: String,
    pub size_bytes: usize,
    pub played_on_speaker: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_base64: Option<String>,
}

/// Request payload for speech recognition (STT) via JSON.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecognizeRequest {
    pub audio_base64: Option<String>,
    pub language: Option<String>,
    pub provider: Option<String>,
    pub dispatch_ai: Option<bool>,
}

/// Response returned after speech recognition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecognizeResponse {
    pub stt: SttResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_response: Option<crate::ai::models::AiResponse>,
    pub spoken_response: bool,
}

/// Request payload to simulate wake word trigger.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SimulateWakeWordRequest {
    pub keyword: Option<String>,
    pub command: Option<String>,
    pub dispatch_ai: Option<bool>,
}

/// Response after simulating wake word.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateWakeWordResponse {
    pub event: WakeWordEvent,
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_response: Option<crate::ai::models::AiResponse>,
    pub spoken_response: bool,
}

/// Request to update speech configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateSpeechConfigRequest {
    pub active_stt_provider: Option<String>,
    pub active_tts_provider: Option<String>,
    pub active_voice: Option<String>,
    pub vad_enabled: Option<bool>,
    pub wakeword_enabled: Option<bool>,
}
