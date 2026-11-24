//! Status & Diagnostic LED Controller for Kamish OS (Orange Pi 4 Pro / Allwinner A733).
//!
//! Controls onboard LEDs via Linux sysfs (`/sys/class/leds/`).
//! Provides visual feedback for headless operation:
//! - Booting / Initializing: Rapid blink (5 Hz, 100ms on / 100ms off)
//! - Setup AP Mode: Steady blink (1 Hz, 500ms on / 500ms off)
//! - Ready / Idle: Solid ON (system healthy, listening for "Камыш")
//! - Voice Active / Dialog: Double-blink (100ms on / 100ms off / 100ms on / 700ms off)
//! - Offline / No Network: Slow pulse (100ms on / 2900ms off)
//! - Error / Hardware Throttling: Rapid strobe (50ms on / 50ms off)

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedPattern {
    Off,
    SolidOn,
    Booting,   // 100ms ON / 100ms OFF (5 Hz)
    SetupAp,   // 500ms ON / 500ms OFF (1 Hz)
    Ready,     // Solid ON
    Listening, // 100ms ON, 100ms OFF, 100ms ON, 700ms OFF
    Offline,   // 100ms ON, 2900ms OFF
    Error,     // 50ms ON, 50ms OFF
}

pub struct LedController {
    sysfs_path: Option<PathBuf>,
    mock_mode: bool,
}

impl LedController {
    pub fn new() -> Self {
        let (sysfs_path, mock_mode) = Self::detect_led();
        if mock_mode {
            debug!("LedController initialized in mock mode (no hardware sysfs LEDs found).");
        } else if let Some(ref path) = sysfs_path {
            info!("LedController attached to sysfs LED: {}", path.display());
            // Set trigger to none so user-space has full direct control
            let trigger_file = path.join("trigger");
            let _ = fs::write(&trigger_file, "none\n");
        }

        Self {
            sysfs_path,
            mock_mode,
        }
    }

    fn detect_led() -> (Option<PathBuf>, bool) {
        if std::env::var("KAMISH_MOCK_HARDWARE").is_ok() {
            return (None, true);
        }

        let leds_dir = Path::new("/sys/class/leds");
        if let Ok(entries) = fs::read_dir(leds_dir) {
            let mut candidates = Vec::new();
            for entry in entries.flatten() {
                let p = entry.path();
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name.contains("green")
                    || name.contains("status")
                    || name.contains("run")
                    || name.contains("act")
                {
                    candidates.push(p);
                }
            }
            if let Some(first) = candidates.into_iter().next() {
                return (Some(first), false);
            }
            // Fallback: pick any available LED directory if present
            if let Ok(entries2) = fs::read_dir(leds_dir) {
                for entry in entries2.flatten() {
                    return (Some(entry.path()), false);
                }
            }
        }

        (None, true)
    }

    pub fn set_brightness(&self, on: bool) {
        if self.mock_mode {
            return;
        }
        if let Some(ref path) = self.sysfs_path {
            let val = if on { "1\n" } else { "0\n" };
            let _ = fs::write(path.join("brightness"), val);
        }
    }

    /// Spawns an asynchronous background task that continuously runs the specified pattern.
    /// Can be cancelled or updated via the shared atomic pattern state.
    pub fn start_worker(
        self: Arc<Self>,
        current_pattern: Arc<tokio::sync::RwLock<LedPattern>>,
        running: Arc<AtomicBool>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("Status LED diagnostic worker loop started.");
            while running.load(Ordering::Relaxed) {
                let pattern = *current_pattern.read().await;
                match pattern {
                    LedPattern::Off => {
                        self.set_brightness(false);
                        sleep(Duration::from_millis(500)).await;
                    }
                    LedPattern::SolidOn | LedPattern::Ready => {
                        self.set_brightness(true);
                        sleep(Duration::from_millis(500)).await;
                    }
                    LedPattern::Booting => {
                        self.set_brightness(true);
                        sleep(Duration::from_millis(100)).await;
                        self.set_brightness(false);
                        sleep(Duration::from_millis(100)).await;
                    }
                    LedPattern::SetupAp => {
                        self.set_brightness(true);
                        sleep(Duration::from_millis(500)).await;
                        self.set_brightness(false);
                        sleep(Duration::from_millis(500)).await;
                    }
                    LedPattern::Listening => {
                        self.set_brightness(true);
                        sleep(Duration::from_millis(100)).await;
                        self.set_brightness(false);
                        sleep(Duration::from_millis(100)).await;
                        self.set_brightness(true);
                        sleep(Duration::from_millis(100)).await;
                        self.set_brightness(false);
                        sleep(Duration::from_millis(700)).await;
                    }
                    LedPattern::Offline => {
                        self.set_brightness(true);
                        sleep(Duration::from_millis(100)).await;
                        self.set_brightness(false);
                        sleep(Duration::from_millis(2900)).await;
                    }
                    LedPattern::Error => {
                        self.set_brightness(true);
                        sleep(Duration::from_millis(50)).await;
                        self.set_brightness(false);
                        sleep(Duration::from_millis(50)).await;
                    }
                }
            }
            self.set_brightness(false);
            info!("Status LED diagnostic worker loop stopped.");
        })
    }
}

impl Default for LedController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_led_controller_mock_mode() {
        let led = LedController::new();
        // On non-Linux or test env without /sys/class/leds, mock_mode must be active
        led.set_brightness(true);
        led.set_brightness(false);
    }

    #[tokio::test]
    async fn test_led_worker_lifecycle() {
        let led = Arc::new(LedController::new());
        let pattern = Arc::new(tokio::sync::RwLock::new(LedPattern::Booting));
        let running = Arc::new(AtomicBool::new(true));

        let handle = led.start_worker(pattern.clone(), running.clone());
        tokio::time::sleep(Duration::from_millis(50)).await;

        *pattern.write().await = LedPattern::Ready;
        tokio::time::sleep(Duration::from_millis(50)).await;

        running.store(false, Ordering::Relaxed);
        let _ = tokio::time::timeout(Duration::from_millis(500), handle).await;
    }
}
