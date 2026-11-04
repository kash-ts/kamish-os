use crate::common::{AppError, AppResult};
use crate::speech::models::SttResult;
use crate::speech::traits::SttProvider;
use std::sync::RwLock;

/// Offline & testing STT provider.
///
/// Guaranteed to work in unit and integration test environments without internet or external binaries.
pub struct MockSttProvider {
    custom_transcription: RwLock<Option<String>>,
}

impl Default for MockSttProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockSttProvider {
    pub fn new() -> Self {
        Self {
            custom_transcription: RwLock::new(None),
        }
    }

    /// Set an explicit transcription string for tests
    pub fn set_mock_text(&self, text: Option<String>) {
        if let Ok(mut lock) = self.custom_transcription.write() {
            *lock = text;
        }
    }
}

impl SttProvider for MockSttProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn recognize(&self, audio_data: &[u8], language: &str) -> AppResult<SttResult> {
        if audio_data.is_empty() {
            return Err(AppError::Config(
                "Audio data for recognition cannot be empty".to_string(),
            ));
        }

        // Check if a custom mock transcription was explicitly configured
        if let Ok(lock) = self.custom_transcription.read() {
            if let Some(ref text) = *lock {
                return Ok(SttResult {
                    text: text.clone(),
                    confidence: 0.98,
                    language: language.to_string(),
                    provider: "mock".to_string(),
                    duration_secs: Some(audio_data.len() as f32 / 32000.0),
                });
            }
        }

        // Check if raw audio payload has a embedded test marker, e.g. "PROMPT:..."
        if let Ok(utf8_str) = std::str::from_utf8(audio_data) {
            let trimmed = utf8_str.trim();
            if let Some(stripped) = trimmed.strip_prefix("CMD:") {
                return Ok(SttResult {
                    text: stripped.trim().to_string(),
                    confidence: 0.99,
                    language: language.to_string(),
                    provider: "mock".to_string(),
                    duration_secs: Some(1.0),
                });
            }
            if let Some(stripped) = trimmed.strip_prefix("PROMPT:") {
                return Ok(SttResult {
                    text: stripped.trim().to_string(),
                    confidence: 0.99,
                    language: language.to_string(),
                    provider: "mock".to_string(),
                    duration_secs: Some(1.0),
                });
            }
        }

        // Default recognizable phrase for offline testing
        let default_text = match language {
            "en" => "What is the system status?",
            _ => "Какая погода сегодня и какой статус системы?",
        };

        Ok(SttResult {
            text: default_text.to_string(),
            confidence: 0.95,
            language: language.to_string(),
            provider: "mock".to_string(),
            duration_secs: Some(audio_data.len() as f32 / 32000.0),
        })
    }
}
