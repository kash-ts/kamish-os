use crate::common::{AppError, AppResult};
use crate::speech::models::TtsVoice;
use crate::speech::traits::TtsProvider;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{debug, warn};

/// RHVoice lightweight TTS provider.
///
/// Well-suited for low-resource ARM64 boards (Orange Pi Zero 3) due to minimal CPU & RAM usage.
pub struct RhVoiceTtsProvider {
    bin_path: Option<PathBuf>,
    http_url: Option<String>,
}

impl Default for RhVoiceTtsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl RhVoiceTtsProvider {
    pub fn new() -> Self {
        let http_url = std::env::var("RHVOICE_HTTP_URL").ok();
        let bin_candidate = std::env::var("RHVOICE_BIN")
            .map(PathBuf::from)
            .ok()
            .or_else(Self::find_binary);

        Self {
            bin_path: bin_candidate,
            http_url,
        }
    }

    fn find_binary() -> Option<PathBuf> {
        let candidates = [
            "rhvoice-client",
            "RHVoice-test",
            "/usr/bin/rhvoice-client",
            "/usr/local/bin/rhvoice-client",
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

impl TtsProvider for RhVoiceTtsProvider {
    fn name(&self) -> &str {
        "rhvoice"
    }

    fn is_available(&self) -> bool {
        self.bin_path.is_some() || self.http_url.is_some()
    }

    fn available_voices(&self) -> Vec<TtsVoice> {
        vec![
            TtsVoice {
                id: "rhvoice_aleksandr".to_string(),
                name: "RHVoice Aleksandr (Russian)".to_string(),
                language: "ru".to_string(),
                provider: "rhvoice".to_string(),
                gender: Some("male".to_string()),
            },
            TtsVoice {
                id: "rhvoice_elena".to_string(),
                name: "RHVoice Elena (Russian)".to_string(),
                language: "ru".to_string(),
                provider: "rhvoice".to_string(),
                gender: Some("female".to_string()),
            },
            TtsVoice {
                id: "rhvoice_anna".to_string(),
                name: "RHVoice Anna (Russian)".to_string(),
                language: "ru".to_string(),
                provider: "rhvoice".to_string(),
                gender: Some("female".to_string()),
            },
            TtsVoice {
                id: "rhvoice_alan".to_string(),
                name: "RHVoice Alan (English)".to_string(),
                language: "en".to_string(),
                provider: "rhvoice".to_string(),
                gender: Some("male".to_string()),
            },
        ]
    }

    async fn synthesize(&self, text: &str, voice: Option<&str>, speed: f32) -> AppResult<Vec<u8>> {
        let selected_voice = match voice {
            Some("rhvoice_elena") | Some("elena") => "Elena",
            Some("rhvoice_anna") | Some("anna") => "Anna",
            Some("rhvoice_alan") | Some("alan") => "Alan",
            _ => "Aleksandr",
        };

        // Try HTTP first if configured
        if let Some(ref url) = self.http_url {
            let client = reqwest::Client::new();
            let res = client
                .get(format!("{}/say", url))
                .query(&[
                    ("text", text),
                    ("voice", selected_voice),
                    ("format", "wav"),
                    ("rate", &format!("{:.1}", speed * 100.0)),
                ])
                .send()
                .await;

            if let Ok(resp) = res {
                if resp.status().is_success() {
                    let bytes = resp.bytes().await.map_err(|e| {
                        AppError::System(format!("Failed reading RHVoice HTTP response: {e}"))
                    })?;
                    return Ok(bytes.to_vec());
                }
            }
        }

        // Try local binary
        let bin = self.bin_path.as_ref().ok_or_else(|| {
            AppError::Config(
                "RHVoice binary ('rhvoice-client' or 'RHVoice-test') not found on host".to_string(),
            )
        })?;

        let mut cmd = Command::new(bin);
        cmd.arg("-s")
            .arg(selected_voice)
            .arg("-r")
            .arg(format!("{:.1}", (speed - 1.0) * 50.0))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!("Spawning RHVoice command: {:?}", cmd);
        let mut child = cmd
            .spawn()
            .map_err(|e| AppError::System(format!("Failed to spawn RHVoice process: {e}")))?;

        if let Some(mut stdin) = child.stdin.take() {
            let text_bytes = text.as_bytes().to_vec();
            tokio::spawn(async move {
                if let Err(e) = stdin.write_all(&text_bytes).await {
                    warn!("Failed writing to RHVoice stdin: {}", e);
                }
            });
        }

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| AppError::System(format!("RHVoice execution error: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::System(format!(
                "RHVoice exited with code {:?}: {}",
                output.status.code(),
                stderr
            )));
        }

        if output.stdout.is_empty() {
            return Err(AppError::System(
                "RHVoice produced empty audio output".to_string(),
            ));
        }

        Ok(output.stdout)
    }
}
