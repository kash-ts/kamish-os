//! Unified Audio Output Manager implementing the Section 9 specification.
//!
//! Controls ALSA/PipeWire output routing, enforces a safe hardware volume ceiling
//! to protect the GT-112 amplifier and driver from clipping, and provides
//! real-time routing diagnostics.

use super::alert::{is_audio_muted, play_alert_tone, set_audio_muted, AlertType};
use crate::system::hardware::{detect_respeaker, AudioRoutingInfo};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Default safe volume ceiling for the GT-112 speaker to prevent clipping.
pub const DEFAULT_SAFE_VOLUME_CEILING: u8 = 85;

/// Available physical and virtual audio output routes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioRoute {
    /// GT-112 portable acoustic system via analog/ALSA line out.
    Gt112Default,
    /// Bluetooth A2DP sink wireless audio output.
    BluetoothSink,
    /// Headphone / auxiliary analog jack output.
    Headphones,
    /// System default fallback audio device.
    SystemDefault,
}

impl Default for AudioRoute {
    fn default() -> Self {
        Self::Gt112Default
    }
}

impl AudioRoute {
    pub fn description(&self) -> &'static str {
        match self {
            Self::Gt112Default => "GT-112 (ALSA Analog / Line Out)",
            Self::BluetoothSink => "Bluetooth A2DP Sink (BlueALSA)",
            Self::Headphones => "Headphones (3.5mm Analog Out)",
            Self::SystemDefault => "System Default Audio Sink",
        }
    }
}

/// Unified Audio Output Manager.
pub struct AudioOutputManager {
    route: RwLock<AudioRoute>,
    current_volume: AtomicU8,
    safe_ceiling: AtomicU8,
    muted: AtomicBool,
}

impl Default for AudioOutputManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioOutputManager {
    /// Create a new AudioOutputManager with default safe settings (50% volume, 85% ceiling).
    pub fn new() -> Self {
        Self {
            route: RwLock::new(AudioRoute::Gt112Default),
            current_volume: AtomicU8::new(50),
            safe_ceiling: AtomicU8::new(DEFAULT_SAFE_VOLUME_CEILING),
            muted: AtomicBool::new(false),
        }
    }

    /// Get current volume setting (0..=100).
    pub fn get_volume(&self) -> u8 {
        self.current_volume.load(Ordering::Relaxed)
    }

    /// Get current safe volume ceiling (1..=100).
    pub fn get_safe_ceiling(&self) -> u8 {
        self.safe_ceiling.load(Ordering::Relaxed)
    }

    /// Set safe volume ceiling (1..=100).
    /// If current volume exceeds new ceiling, current volume is clamped down.
    pub fn set_safe_ceiling(&self, ceiling: u8) {
        let safe = ceiling.clamp(1, 100);
        self.safe_ceiling.store(safe, Ordering::Relaxed);

        let current = self.current_volume.load(Ordering::Relaxed);
        if current > safe {
            self.set_volume(safe);
        }
    }

    /// Set volume, strictly enforced by the safe volume ceiling.
    /// Returns the applied effective volume.
    pub fn set_volume(&self, requested: u8) -> u8 {
        let ceiling = self.safe_ceiling.load(Ordering::Relaxed);
        let applied = requested.min(ceiling);
        self.current_volume.store(applied, Ordering::Relaxed);

        // Adjust system mixer in non-blocking manner
        super::set_system_volume_sync(applied);
        debug!(
            "Audio output volume set: requested={}, applied={} (ceiling={})",
            requested, applied, ceiling
        );
        applied
    }

    /// Is output muted?
    pub fn is_muted(&self) -> bool {
        self.muted.load(Ordering::Relaxed) || is_audio_muted()
    }

    /// Is output muted by the user? Excludes the test-harness safety mute
    /// (`set_audio_muted`) so test environments still report a "ready" subsystem.
    pub fn is_user_muted(&self) -> bool {
        self.muted.load(Ordering::Relaxed)
    }

    /// Set mute status.
    pub fn set_muted(&self, muted: bool) {
        self.muted.store(muted, Ordering::Relaxed);
        set_audio_muted(muted);
    }

    /// Get active output route.
    pub async fn get_route(&self) -> AudioRoute {
        *self.route.read().await
    }

    /// Switch active audio output route.
    pub async fn set_route(&self, route: AudioRoute) {
        let mut r = self.route.write().await;
        *r = route;
        info!("Audio output route changed to: {}", route.description());
    }

    /// Get current human-readable active device name.
    pub async fn active_device_name(&self) -> String {
        self.route.read().await.description().to_string()
    }

    /// Generate an `AudioRoutingInfo` telemetry status object.
    pub async fn get_routing_info(&self) -> AudioRoutingInfo {
        let respeaker = detect_respeaker();
        let input_device = if respeaker.detected {
            format!(
                "Seeed ReSpeaker XVF3800 ({})",
                respeaker.card_name.as_deref().unwrap_or("4-Mic Array")
            )
        } else {
            "System Default / Fallback Microphone".to_string()
        };

        let output_device = self.active_device_name().await;
        let mixer_status = if self.is_muted() {
            "muted".to_string()
        } else {
            "active".to_string()
        };

        AudioRoutingInfo {
            input_device,
            input_detected: respeaker.detected,
            output_device,
            mixer_status,
            volume: self.get_volume(),
            safe_volume_ceiling: self.get_safe_ceiling(),
        }
    }

    /// Safely test audio output with alert tone (honors mute flag).
    pub async fn play_test_tone(&self, alert_type: AlertType) {
        if !self.is_muted() {
            play_alert_tone(alert_type).await;
        }
    }
}
