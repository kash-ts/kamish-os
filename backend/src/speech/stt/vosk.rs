use crate::common::{AppError, AppResult};
use crate::speech::models::SttResult;
use crate::speech::traits::SttProvider;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::debug;

/// Vosk offline STT provider.
///
/// Uses Vosk speech recognition CLI or subprocess on host system.
pub struct VoskSttProvider {
    bin_path: Option<PathBuf>,
    model_path: Option<PathBuf>,
}

impl Default for VoskSttProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl VoskSttProvider {
    pub fn new() -> Self {
        let bin = std::env::var("VOSK_BIN")
            .map(PathBuf::from)
            .ok()
            .or_else(Self::find_binary);

        let model = std::env::var("VOSK_MODEL_PATH").map(PathBuf::from).ok();

        Self {
            bin_path: bin,
            model_path: model,
        }
    }

    fn find_binary() -> Option<PathBuf> {
        let candidates = [
            "vosk-transcriber",
            "/usr/bin/vosk-transcriber",
            "/usr/local/bin/vosk-transcriber",
        ];

        for c in candidates {
            let path = Path::new(c);
            if path.is_file() {
                return Some(path.to_path_buf());
            }
            if let Ok(path_var) = std::env::var("PATH") {
                for dir in std::env::split_paths(&path_var) {
                    let full = dir.join(c);
                    if full.is_file() {
                        return Some(full);
                    }
                }
            }
        }

        None
    }
}

impl SttProvider for VoskSttProvider {
    fn name(&self) -> &str {
        "vosk"
    }

    fn is_available(&self) -> bool {
        self.bin_path.is_some()
    }

    async fn recognize(&self, audio_data: &[u8], language: &str) -> AppResult<SttResult> {
        let bin = self.bin_path.as_ref().ok_or_else(|| {
            AppError::Config(
                "Vosk transcriber binary ('vosk-transcriber') is not available on host".to_string(),
            )
        })?;

        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join(format!("vosk_in_{}.wav", uuid::Uuid::new_v4()));

        tokio::fs::write(&input_path, audio_data)
            .await
            .map_err(|e| AppError::System(format!("Failed writing temp audio for Vosk: {e}")))?;

        let mut cmd = Command::new(bin);
        cmd.arg("-i").arg(&input_path);

        if let Some(ref model) = self.model_path {
            cmd.arg("-m").arg(model);
        } else {
            cmd.arg("-l").arg(language);
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        debug!("Running Vosk command: {:?}", cmd);

        let output = cmd.output().await.map_err(|e| {
            let _ = std::fs::remove_file(&input_path);
            AppError::System(format!("Failed executing Vosk process: {e}"))
        })?;

        let _ = tokio::fs::remove_file(&input_path).await;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::System(format!("Vosk process error: {err}")));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Vosk transcriber prints recognized text or JSON
        let text = stdout.trim().to_string();

        Ok(SttResult {
            text,
            confidence: 0.90,
            language: language.to_string(),
            provider: "vosk".to_string(),
            duration_secs: Some(audio_data.len() as f32 / 32000.0),
        })
    }
}
