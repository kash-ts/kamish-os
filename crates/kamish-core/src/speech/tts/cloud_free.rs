use crate::common::{AppError, AppResult};
use crate::speech::models::TtsVoice;
use crate::speech::traits::TtsProvider;
use std::time::Duration;
use tracing::debug;

/// Free cloud TTS provider using Google Translate TTS audio endpoint.
///
/// Requires no API keys. Returns high quality natural MP3 speech audio.
pub struct CloudFreeTtsProvider {
    client: reqwest::Client,
}

impl Default for CloudFreeTtsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudFreeTtsProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();

        Self { client }
    }
}

impl TtsProvider for CloudFreeTtsProvider {
    fn name(&self) -> &str {
        "cloud_free"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn available_voices(&self) -> Vec<TtsVoice> {
        vec![
            TtsVoice {
                id: "google_ru".to_string(),
                name: "Google Cloud Free (Russian)".to_string(),
                language: "ru".to_string(),
                provider: "cloud_free".to_string(),
                gender: Some("female".to_string()),
            },
            TtsVoice {
                id: "google_en".to_string(),
                name: "Google Cloud Free (English)".to_string(),
                language: "en".to_string(),
                provider: "cloud_free".to_string(),
                gender: Some("female".to_string()),
            },
        ]
    }

    async fn synthesize(&self, text: &str, voice: Option<&str>, _speed: f32) -> AppResult<Vec<u8>> {
        let clean = text.trim();
        if clean.is_empty() {
            return Err(AppError::Config(
                "Text for synthesis cannot be empty".to_string(),
            ));
        }

        let lang = match voice {
            Some("google_en") | Some("en") => "en",
            _ => "ru",
        };

        // Google Translate TTS query endpoint
        // Handles up to 200 characters per segment
        let query_text = if clean.chars().count() > 180 {
            let truncated: String = clean.chars().take(180).collect();
            truncated
        } else {
            clean.to_string()
        };

        let url = format!(
            "https://translate.google.com/translate_tts?ie=UTF-8&tl={}&client=tw-ob&q={}",
            lang,
            urlencoding_simple(&query_text)
        );

        debug!("CloudFree TTS requesting url: {}", url);
        let resp = self.client.get(&url).send().await.map_err(|e| {
            AppError::System(format!("Failed to connect to Cloud TTS endpoint: {e}"))
        })?;

        if !resp.status().is_success() {
            return Err(AppError::System(format!(
                "Cloud TTS server responded with HTTP status {}",
                resp.status()
            )));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::System(format!("Failed reading Cloud TTS stream: {e}")))?;

        if bytes.is_empty() {
            return Err(AppError::System("Cloud TTS returned 0 bytes".to_string()));
        }

        Ok(bytes.to_vec())
    }
}

/// Simple percent-encoding for query string
fn urlencoding_simple(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            b' ' => encoded.push('+'),
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}
