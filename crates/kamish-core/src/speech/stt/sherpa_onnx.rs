use crate::common::{AppError, AppResult};
use crate::speech::models::SttResult;
use crate::speech::traits::SttProvider;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::debug;

/// Sherpa-onnx offline STT provider (next-gen Kaldi).
///
/// Embedded-friendly ONNX neural speech recognition.
pub struct SherpaOnnxSttProvider {
    bin_path: Option<PathBuf>,
    tokens_path: Option<PathBuf>,
    encoder_path: Option<PathBuf>,
    decoder_path: Option<PathBuf>,
}

impl Default for SherpaOnnxSttProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SherpaOnnxSttProvider {
    pub fn new() -> Self {
        let bin = std::env::var("SHERPA_ONNX_BIN")
            .map(PathBuf::from)
            .ok()
            .or_else(Self::find_binary);

        let tokens = std::env::var("SHERPA_TOKENS_PATH").map(PathBuf::from).ok();
        let encoder = std::env::var("SHERPA_ENCODER_PATH").map(PathBuf::from).ok();
        let decoder = std::env::var("SHERPA_DECODER_PATH").map(PathBuf::from).ok();

        Self {
            bin_path: bin,
            tokens_path: tokens,
            encoder_path: encoder,
            decoder_path: decoder,
        }
    }

    fn find_binary() -> Option<PathBuf> {
        let candidates = [
            "sherpa-onnx-offline",
            "sherpa-onnx",
            "/usr/bin/sherpa-onnx-offline",
            "/usr/local/bin/sherpa-onnx-offline",
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

impl SttProvider for SherpaOnnxSttProvider {
    fn name(&self) -> &str {
        "sherpa_onnx"
    }

    fn is_available(&self) -> bool {
        self.bin_path.is_some()
    }

    async fn recognize(&self, audio_data: &[u8], language: &str) -> AppResult<SttResult> {
        let bin = self.bin_path.as_ref().ok_or_else(|| {
            AppError::Config(
                "Sherpa-onnx binary ('sherpa-onnx-offline') is not installed on host".to_string(),
            )
        })?;

        let temp_dir = std::env::temp_dir();
        let input_path = temp_dir.join(format!("sherpa_in_{}.wav", uuid::Uuid::new_v4()));

        tokio::fs::write(&input_path, audio_data)
            .await
            .map_err(|e| AppError::System(format!("Failed writing temp audio for Sherpa: {e}")))?;

        let mut cmd = Command::new(bin);
        if let Some(ref t) = self.tokens_path {
            cmd.arg(format!("--tokens={}", t.display()));
        }
        if let Some(ref e) = self.encoder_path {
            cmd.arg(format!("--encoder={}", e.display()));
        }
        if let Some(ref d) = self.decoder_path {
            cmd.arg(format!("--decoder={}", d.display()));
        }
        cmd.arg(&input_path);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        debug!("Running Sherpa-onnx command: {:?}", cmd);
        let output = cmd.output().await.map_err(|e| {
            let _ = std::fs::remove_file(&input_path);
            AppError::System(format!("Failed executing Sherpa process: {e}"))
        })?;

        let _ = tokio::fs::remove_file(&input_path).await;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::System(format!("Sherpa process error: {err}")));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let text = stdout.trim().to_string();

        Ok(SttResult {
            text,
            confidence: 0.92,
            language: language.to_string(),
            provider: "sherpa_onnx".to_string(),
            duration_secs: Some(audio_data.len() as f32 / 32000.0),
        })
    }
}
