//! Real microphone capture via the ALSA `arecord` utility (Buildroot / Linux target).
//!
//! `arecord` is used instead of a native ALSA or `cpal` binding because it is present
//! on the Buildroot image, needs no extra build-time C dependencies, and streams raw
//! PCM on stdout where it is trivially framed. The source reports itself unavailable
//! when the binary or the device is missing, so the caller can fall back honestly.

use super::{AudioSource, BoxFuture, CAPTURE_CHANNELS, CAPTURE_SAMPLE_RATE, FRAME_SAMPLES};
use crate::common::{AppError, AppResult};
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tracing::{debug, info};

/// Captures PCM from a microphone device using `arecord`.
pub struct ArecordAudioSource {
    /// ALSA device name, e.g. `default`, `plughw:1,0`, or `hw:CARD=XVF3800`.
    device: String,
    /// Located binary path, or `None` if `arecord` is not installed.
    bin_path: Option<String>,
    child: Option<Child>,
    /// Bytes-per-sample for 16-bit mono.
    bytes_per_frame: usize,
}

impl ArecordAudioSource {
    pub fn new(device: impl Into<String>) -> Self {
        let device = device.into();
        Self {
            bin_path: Self::find_binary(),
            device,
            child: None,
            bytes_per_frame: FRAME_SAMPLES * 2, // i16 = 2 bytes
        }
    }

    /// Read the ALSA capture device from the environment, defaulting to `default`.
    pub fn from_env() -> Self {
        let device = std::env::var("KAMISH_MIC_DEVICE").unwrap_or_else(|_| "default".to_string());
        Self::new(device)
    }

    fn find_binary() -> Option<String> {
        for candidate in ["arecord", "/usr/bin/arecord", "/usr/local/bin/arecord"] {
            if std::path::Path::new(candidate).is_file() {
                return Some(candidate.to_string());
            }
            if let Ok(path_var) = std::env::var("PATH") {
                for dir in std::env::split_paths(&path_var) {
                    let full = dir.join(candidate);
                    if full.is_file() {
                        return Some(full.to_string_lossy().to_string());
                    }
                }
            }
        }
        None
    }

    /// The configured ALSA device name.
    pub fn device(&self) -> &str {
        &self.device
    }

    fn spawn(&mut self) -> AppResult<()> {
        let bin = self
            .bin_path
            .clone()
            .ok_or_else(|| AppError::Config("arecord binary not found".to_string()))?;

        let mut cmd = Command::new(&bin);
        cmd.arg("-q")
            .arg("-D")
            .arg(&self.device)
            .arg("-f")
            .arg("S16_LE") // 16-bit signed little-endian
            .arg("-r")
            .arg(CAPTURE_SAMPLE_RATE.to_string())
            .arg("-c")
            .arg(CAPTURE_CHANNELS.to_string())
            .arg("-t")
            .arg("raw")
            .arg("-") // stream to stdout
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::null());

        let child = cmd
            .spawn()
            .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to spawn arecord: {e}")))?;

        info!(
            "Audio capture started: arecord -D {} -f S16_LE -r {} -c {}",
            self.device, CAPTURE_SAMPLE_RATE, CAPTURE_CHANNELS
        );

        self.child = Some(child);
        Ok(())
    }
}

impl AudioSource for ArecordAudioSource {
    fn name(&self) -> &str {
        "arecord"
    }

    fn is_available(&self) -> bool {
        self.bin_path.is_some()
    }

    fn unavailable_reason(&self) -> Option<String> {
        if self.bin_path.is_none() {
            Some("arecord not installed (install alsa-utils)".to_string())
        } else {
            None
        }
    }

    fn next_frame(&mut self) -> BoxFuture<'_, AppResult<Option<Vec<i16>>>> {
        Box::pin(async move {
            if !self.is_available() {
                return Err(AppError::Config(
                    "arecord is not available on this host".to_string(),
                ));
            }

            if self.child.is_none() {
                self.spawn()?;
            }

            let stdout = match self.child.as_mut().and_then(|c| c.stdout.as_mut()) {
                Some(s) => s,
                None => {
                    return Err(AppError::Internal(anyhow::anyhow!(
                        "arecord stdout pipe is unavailable"
                    )))
                }
            };

            let mut buf = vec![0u8; self.bytes_per_frame];
            match stdout.read_exact(&mut buf).await {
                Ok(_) => {
                    let samples: Vec<i16> = buf
                        .chunks_exact(2)
                        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
                        .collect();
                    Ok(Some(samples))
                }
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    // Device disappeared or stream closed: report a clean end of stream so
                    // the capture loop can restart instead of spinning on a dead pipe.
                    debug!("arecord stream ended (EOF)");
                    self.child = None;
                    Ok(None)
                }
                Err(e) => Err(AppError::Internal(anyhow::anyhow!(
                    "failed reading from arecord: {e}"
                ))),
            }
        })
    }

    fn stop(&mut self) -> BoxFuture<'_, AppResult<()>> {
        Box::pin(async move {
            if let Some(mut child) = self.child.take() {
                let _ = child.kill().await;
                let _ = child.wait().await;
                info!("Audio capture stopped (device {})", self.device);
            }
            Ok(())
        })
    }
}

impl Drop for ArecordAudioSource {
    fn drop(&mut self) {
        // Best-effort: never leave an orphaned arecord process behind.
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arecord_availability_is_honest() {
        let src = ArecordAudioSource::new("default");
        // Availability must exactly track binary presence — no optimistic defaults.
        let expect = ArecordAudioSource::find_binary().is_some();
        assert_eq!(src.is_available(), expect);
        if !expect {
            assert!(src.unavailable_reason().is_some());
        }
    }

    #[test]
    fn test_arecord_respects_device_from_env() {
        // SAFETY: test-local env manipulation, single-threaded assertion below.
        unsafe {
            std::env::set_var("KAMISH_MIC_DEVICE", "plughw:1,0");
        }
        let src = ArecordAudioSource::from_env();
        assert_eq!(src.device(), "plughw:1,0");
        unsafe {
            std::env::remove_var("KAMISH_MIC_DEVICE");
        }
        let fallback = ArecordAudioSource::from_env();
        assert_eq!(fallback.device(), "default");
    }
}
