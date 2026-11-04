use crate::common::{AppError, AppResult};
use crate::speech::models::SttResult;
use crate::speech::traits::SttProvider;
use std::time::Duration;
use tracing::debug;

/// Cloud speech recognition provider (Whisper API / HTTP STT compatible).
pub struct CloudFreeSttProvider {
    endpoint_url: String,
    client: reqwest::Client,
}

impl Default for CloudFreeSttProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudFreeSttProvider {
    pub fn new() -> Self {
        let endpoint_url = std::env::var("CLOUD_STT_URL")
            .unwrap_or_else(|_| "http://localhost:8000/v1/audio/transcriptions".to_string());

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            endpoint_url,
            client,
        }
    }

    pub fn with_url(endpoint_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            endpoint_url,
            client,
        }
    }
}

impl SttProvider for CloudFreeSttProvider {
    fn name(&self) -> &str {
        "cloud_free"
    }

    fn is_available(&self) -> bool {
        // Can be configured or reachable
        !self.endpoint_url.is_empty()
    }

    async fn recognize(&self, audio_data: &[u8], language: &str) -> AppResult<SttResult> {
        if audio_data.is_empty() {
            return Err(AppError::Config(
                "Audio data for recognition cannot be empty".to_string(),
            ));
        }

        debug!(
            "Cloud STT sending {} bytes to {}",
            audio_data.len(),
            self.endpoint_url
        );

        let part = reqwest::multipart::Part::bytes(audio_data.to_vec())
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| AppError::System(format!("Multipart error: {e}")))?;

        let form = reqwest::multipart::Form::new()
            .text("language", language.to_string())
            .part("file", part);

        let resp = self
            .client
            .post(&self.endpoint_url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| AppError::System(format!("Cloud STT HTTP connection failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(AppError::System(format!(
                "Cloud STT endpoint returned HTTP error {}",
                resp.status()
            )));
        }

        let json_body: serde_json::Value = resp.json().await.map_err(|e| {
            AppError::System(format!("Failed parsing Cloud STT response JSON: {e}"))
        })?;

        let text = json_body["text"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .to_string();

        Ok(SttResult {
            text,
            confidence: 0.95,
            language: language.to_string(),
            provider: "cloud_free".to_string(),
            duration_secs: Some(audio_data.len() as f32 / 32000.0),
        })
    }
}
