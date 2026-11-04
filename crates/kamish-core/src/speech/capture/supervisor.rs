//! Capture supervisor: owns the background listening task lifecycle.
//!
//! The supervisor decides *which* [`AudioSource`] to use, keeps at most one loop
//! running, and mirrors the real state into [`SpeechManager`] so `/api/v1/speech/status`
//! never reports a microphone that is not being read.
//!
//! Source selection is explicit and honest:
//!
//! * `KAMISH_AUDIO_SOURCE=arecord|mock` forces a specific source.
//! * Otherwise a real `arecord` device is preferred, and the mock is used only when
//!   explicitly allowed via `KAMISH_ALLOW_MOCK_MIC=1`.

use super::runner::{snapshot, CaptureConfig, CaptureLoop, CaptureStatus};
use super::{ArecordAudioSource, AudioSource, MockAudioSource};
use crate::common::AppState;
use crate::speech::SpeechManager;
use crate::system::EventBus;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tracing::{info, warn};

/// Which source the supervisor should attempt to open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// Real ALSA capture through `arecord`.
    Arecord,
    /// Synthetic frames, for dev environments without a device.
    Mock,
}

impl SourceKind {
    /// Resolve from environment with sane, explicit precedence.
    pub fn from_env() -> Self {
        match std::env::var("KAMISH_AUDIO_SOURCE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "mock" => SourceKind::Mock,
            "arecord" | "alsa" | "real" => SourceKind::Arecord,
            _ => {
                if ArecordAudioSource::from_env().is_available() {
                    SourceKind::Arecord
                } else if std::env::var("KAMISH_ALLOW_MOCK_MIC")
                    .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false)
                {
                    SourceKind::Mock
                } else {
                    // No device and mock not permitted: stay stopped rather than fake it.
                    SourceKind::Arecord
                }
            }
        }
    }

    fn build(self) -> Box<dyn AudioSource> {
        match self {
            SourceKind::Arecord => Box::new(ArecordAudioSource::from_env()),
            SourceKind::Mock => Box::new(MockAudioSource::new()),
        }
    }
}

/// Manages the single capture task.
pub struct CaptureSupervisor {
    speech: Arc<SpeechManager>,
    events: Arc<EventBus>,
    loop_handle: Mutex<Option<JoinHandle<()>>>,
    shutdown: Mutex<Option<Arc<AtomicBool>>>,
    stats: Mutex<Option<Arc<crate::speech::CaptureStats>>>,
    last_kind: Mutex<Option<SourceKind>>,
}

impl CaptureSupervisor {
    pub fn new(speech: Arc<SpeechManager>, events: Arc<EventBus>) -> Self {
        Self {
            speech,
            events,
            loop_handle: Mutex::new(None),
            shutdown: Mutex::new(None),
            stats: Mutex::new(None),
            last_kind: Mutex::new(None),
        }
    }

    /// Whether the audio source requested by the environment can be opened at all.
    pub fn source_available(&self) -> bool {
        match SourceKind::from_env() {
            SourceKind::Arecord => ArecordAudioSource::from_env().is_available(),
            SourceKind::Mock => true,
        }
    }

    /// Start listening. Returns `false` if a loop is already running.
    pub async fn start(&self, state: AppState) -> bool {
        {
            let handle = self.loop_handle.lock().ok();
            if let Some(h) = handle.as_ref().and_then(|g| g.as_ref()) {
                if !h.is_finished() {
                    info!("Capture supervisor: loop already running, ignoring start");
                    return false;
                }
            }
        }

        let kind = SourceKind::from_env();
        let source = kind.build();

        if !source.is_available() {
            let reason = source
                .unavailable_reason()
                .unwrap_or_else(|| "audio source unavailable".to_string());
            warn!("Capture supervisor: cannot start ({reason})");
            self.speech.set_capture_state(false, false);
            self.events.emit(
                "speech_capture_error",
                serde_json::json!({ "error": reason, "source": source.name() }),
            );
            return false;
        }

        let capture = Arc::new(CaptureLoop::from_env());
        let stats = capture.stats.clone();
        let shutdown = Arc::new(AtomicBool::new(false));
        let speech = self.speech.clone();
        let events = self.events.clone();

        self.speech.set_capture_state(true, true);

        let shutdown_for_task = shutdown.clone();
        let speech_for_task = speech.clone();
        let events_for_task = events.clone();
        let source_name = source.name().to_string();

        let handle = tokio::spawn(async move {
            let result = capture
                .run(
                    source,
                    speech_for_task.clone(),
                    state,
                    events_for_task,
                    shutdown_for_task,
                )
                .await;
            // The loop ended (shutdown, EOF, or error): reflect reality, do not keep
            // advertising a listening microphone.
            speech_for_task.set_capture_state(false, false);
            info!("Capture supervisor: loop finished ({result} utterance(s) dispatched)");
        });

        if let Ok(mut g) = self.loop_handle.lock() {
            *g = Some(handle);
        }
        if let Ok(mut g) = self.shutdown.lock() {
            *g = Some(shutdown);
        }
        if let Ok(mut g) = self.stats.lock() {
            *g = Some(stats);
        }
        if let Ok(mut g) = self.last_kind.lock() {
            *g = Some(kind);
        }

        info!("Capture supervisor: started ({source_name})");
        true
    }

    /// Signal the loop to stop and wait for it to unwind.
    pub async fn stop(&self) -> bool {
        let had_loop = {
            let shutdown = self.shutdown.lock().ok().and_then(|g| g.clone());
            match shutdown {
                Some(flag) => {
                    flag.store(true, Ordering::Relaxed);
                    true
                }
                None => false,
            }
        };

        if !had_loop {
            self.speech.set_capture_state(false, false);
            return false;
        }

        let handle = self.loop_handle.lock().ok().and_then(|mut g| g.take());
        if let Some(h) = handle {
            // Bound the wait: a blocking read on a dead device must not hang shutdown.
            let _ = tokio::time::timeout(std::time::Duration::from_secs(3), h).await;
        }

        self.speech.set_capture_state(false, false);
        if let Ok(mut g) = self.shutdown.lock() {
            *g = None;
        }
        info!("Capture supervisor: stopped");
        true
    }

    /// Whether a loop is currently running.
    pub fn is_running(&self) -> bool {
        self.loop_handle
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|h| !h.is_finished()))
            .unwrap_or(false)
    }

    /// Snapshot for the status API.
    pub fn status(&self) -> CaptureStatus {
        let kind = self.last_kind.lock().ok().and_then(|g| *g);
        let fallback = match kind {
            Some(SourceKind::Mock) => "mock",
            Some(SourceKind::Arecord) => "arecord",
            None => "none",
        };

        let stats = self.stats.lock().ok().and_then(|g| g.clone());
        let available = self.source_available() && self.is_running();

        match stats {
            Some(s) => {
                let mut st = snapshot(&s, fallback, available);
                if !self.is_running() {
                    st.listening = false;
                }
                st
            }
            None => CaptureStatus {
                source: fallback.to_string(),
                available,
                listening: false,
                frames_total: 0,
                wake_words_total: 0,
                utterances_total: 0,
                last_error: None,
                last_transcript: None,
                last_reply: None,
            },
        }
    }
}

/// Re-export so callers can build a `CaptureConfig` explicitly.
pub type Config = CaptureConfig;
