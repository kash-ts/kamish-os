use crate::common::{AppError, AppResult};
use crate::speech::models::TtsVoice;
use crate::speech::traits::TtsProvider;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{debug, warn};

/// Piper neural TTS provider.
///
/// Uses Piper fast local neural TTS (ONNX) if installed on Orange Pi Zero 3 / Linux.
pub struct PiperTtsProvider {
    piper_bin: PathBuf,
    model_path: Option<PathBuf>,
}

impl Default for PiperTtsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl PiperTtsProvider {
    pub fn new() -> Self {
        let bin_path = std::env::var("PIPER_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("piper"));

        let model_path = std::env::var("PIPER_MODEL_PATH").map(PathBuf::from).ok();

        Self {
            piper_bin: bin_path,
            model_path,
        }
    }

    pub fn with_paths(bin: PathBuf, model: Option<PathBuf>) -> Self {
        Self {
            piper_bin: bin,
            model_path: model,
        }
    }

    fn find_piper_executable(&self) -> Option<PathBuf> {
        if self.piper_bin.is_absolute() && self.piper_bin.exists() {
            return Some(self.piper_bin.clone());
        }

        // Check PATH
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                let candidate = dir.join(&self.piper_bin);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }

        let common_paths = [
            Path::new("/usr/bin/piper"),
            Path::new("/usr/local/bin/piper"),
            Path::new("/opt/piper/piper"),
        ];

        for path in common_paths {
            if path.exists() {
                return Some(path.to_path_buf());
            }
        }

        None
    }
}

impl TtsProvider for PiperTtsProvider {
    fn name(&self) -> &str {
        "piper"
    }

    fn is_available(&self) -> bool {
        self.find_piper_executable().is_some()
    }

    fn available_voices(&self) -> Vec<TtsVoice> {
        vec![
            TtsVoice {
                id: "piper_ru_natia".to_string(),
                name: "Piper Natia (Russian Neural)".to_string(),
                language: "ru".to_string(),
                provider: "piper".to_string(),
                gender: Some("female".to_string()),
            },
            TtsVoice {
                id: "piper_ru_dmitri".to_string(),
                name: "Piper Dmitri (Russian Neural)".to_string(),
                language: "ru".to_string(),
                provider: "piper".to_string(),
                gender: Some("male".to_string()),
            },
            TtsVoice {
                id: "piper_en_lessac".to_string(),
                name: "Piper Lessac (English Neural)".to_string(),
                language: "en".to_string(),
                provider: "piper".to_string(),
                gender: Some("female".to_string()),
            },
        ]
    }

    async fn synthesize(&self, text: &str, voice: Option<&str>, speed: f32) -> AppResult<Vec<u8>> {
        let exe = self.find_piper_executable().ok_or_else(|| {
            AppError::Config(
                "Piper neural TTS binary ('piper') is not installed or not in PATH".to_string(),
            )
        })?;

        let mut cmd = Command::new(&exe);
        cmd.arg("--output_file").arg("-");

        // Length scale is inversely proportional to speed
        let length_scale = 1.0 / speed.clamp(0.5, 2.0);
        cmd.arg("--length_scale")
            .arg(format!("{:.2}", length_scale));

        if let Some(ref model) = self.model_path {
            cmd.arg("--model").arg(model);
        } else if let Some(v) = voice {
            if v.contains(".onnx") {
                cmd.arg("--model").arg(v);
            }
        }

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!("Spawning Piper command: {:?}", cmd);
        let mut child = cmd
            .spawn()
            .map_err(|e| AppError::System(format!("Failed to spawn Piper process: {e}")))?;

        if let Some(mut stdin) = child.stdin.take() {
            let text_bytes = text.as_bytes().to_vec();
            tokio::spawn(async move {
                if let Err(e) = stdin.write_all(&text_bytes).await {
                    warn!("Failed writing to Piper stdin: {}", e);
                }
            });
        }

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| AppError::System(format!("Piper execution failed: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::System(format!(
                "Piper exited with code {:?}: {}",
                output.status.code(),
                stderr
            )));
        }

        if output.stdout.is_empty() {
            return Err(AppError::System(
                "Piper produced empty audio output".to_string(),
            ));
        }

        Ok(output.stdout)
    }
}
