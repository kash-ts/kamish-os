//! Audio Manager module.
//! Audio input (ReSpeaker XVF3800), audio output (GT-112 adapter),
//! Bluetooth A2DP Sink routing, system mixer, and alert notifications.

pub mod alert;
pub mod output;

pub use output::{AudioOutputManager, AudioRoute, DEFAULT_SAFE_VOLUME_CEILING};

/// Adjust system mixer volume synchronously/in background.
pub fn set_system_volume_sync(volume: u8) {
    let vol = volume.min(100);
    std::thread::spawn(move || {
        let _ = std::process::Command::new("amixer")
            .args(["sset", "Master", &format!("{vol}%")])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    });
}

/// Adjust system mixer volume using ALSA amixer (if available).
/// Safe and non-blocking; logs debug on failure in headless/test environments.
pub async fn set_system_volume(volume: u8) {
    let vol = volume.min(100);
    tokio::task::spawn_blocking(move || {
        let status = std::process::Command::new("amixer")
            .args(["sset", "Master", &format!("{vol}%")])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        match status {
            Ok(s) if s.success() => {
                tracing::debug!("System mixer volume set to {}%", vol);
            }
            Ok(s) => {
                tracing::debug!("amixer exited with status {s}");
            }
            Err(e) => {
                tracing::debug!("amixer not available ({e}), virtual volume updated");
            }
        }
    });
}
