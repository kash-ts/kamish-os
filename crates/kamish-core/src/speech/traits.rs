use crate::common::AppResult;
use crate::speech::models::{SttResult, TtsVoice, WakeWordEvent};

/// Modular provider interface for Speech-To-Text (STT).
#[allow(async_fn_in_trait)]
pub trait SttProvider: Send + Sync {
    /// Unique provider identifier (e.g. "vosk", "sherpa_onnx", "cloud_free", "mock")
    fn name(&self) -> &str;

    /// Checks if this provider is ready and available on the host system
    fn is_available(&self) -> bool;

    /// Transcribe raw audio data (PCM or WAV bytes) to text
    async fn recognize(&self, audio_data: &[u8], language: &str) -> AppResult<SttResult>;
}

/// Modular provider interface for Text-To-Speech (TTS).
#[allow(async_fn_in_trait)]
pub trait TtsProvider: Send + Sync {
    /// Unique provider identifier (e.g. "piper", "rhvoice", "cloud_free", "offline_fallback")
    fn name(&self) -> &str;

    /// Checks if this provider is ready and available on the host system
    fn is_available(&self) -> bool;

    /// List available voices for this provider
    fn available_voices(&self) -> Vec<TtsVoice>;

    /// Synthesizes text into audio bytes (WAV or MP3)
    async fn synthesize(&self, text: &str, voice: Option<&str>, speed: f32) -> AppResult<Vec<u8>>;
}

/// Interface for Wake Word detection algorithms and models.
pub trait WakeWordDetector: Send + Sync {
    /// Detector algorithm identifier
    fn name(&self) -> &str;

    /// Process a single PCM audio frame (16-bit mono) and spot wake words
    fn process_frame(&mut self, pcm_frame: &[i16]) -> Option<WakeWordEvent>;

    /// Adjust trigger sensitivity [0.0 - 1.0]
    fn set_sensitivity(&mut self, sensitivity: f32);

    /// Reset internal detection states/buffers
    fn reset(&mut self);
}

/// Interface for Voice Activity Detection (VAD).
pub trait VadDetector: Send + Sync {
    /// Analyzes a PCM frame (16-bit mono) and returns true if speech activity is detected
    fn is_speech(&mut self, pcm_frame: &[i16]) -> bool;

    /// Reset internal state
    fn reset(&mut self);
}
