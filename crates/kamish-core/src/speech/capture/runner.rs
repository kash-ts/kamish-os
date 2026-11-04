//! The continuous listening loop.
//!
//! Wires an [`AudioSource`] to the existing speech machinery:
//!
//! ```text
//! microphone ──► VAD ──► wake word ──► utterance assembly ──► STT ──► AI ──► TTS
//! ```
//!
//! The loop is deliberately conservative about hardware: it never reports a listening
//! state it cannot back with a frame source, and it never invents a transcript.

use super::AudioSource;
use crate::speech::models::WakeWordEvent;
use crate::speech::SpeechManager;
use crate::system::EventBus;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

/// Tunables for one listening session.
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// Language passed to the STT provider.
    pub language: String,
    /// Maximum utterance length in frames (~20 ms each) before forced cut-off.
    pub max_utterance_frames: usize,
    /// Consecutive silent frames that end an utterance.
    pub silence_tail_frames: usize,
    /// Frames captured *before* the wake word to avoid clipping the first syllable.
    pub pre_roll_frames: usize,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            language: "ru".to_string(),
            // 10 s of speech at 20 ms/frame.
            max_utterance_frames: 500,
            // ~600 ms of trailing silence closes a phrase.
            silence_tail_frames: 30,
            pre_roll_frames: 10,
        }
    }
}

/// Live statistics and state of the capture loop, exposed through the status API.
#[derive(Debug, Default)]
pub struct CaptureStats {
    pub frames_total: AtomicU64,
    pub wake_words_total: AtomicU64,
    pub utterances_total: AtomicU64,
    pub listening: AtomicBool,
    pub source_name: Mutex<String>,
    pub last_error: Mutex<Option<String>>,
    pub last_transcript: Mutex<Option<String>>,
    pub last_reply: Mutex<Option<String>>,
}

impl CaptureStats {
    fn set_source(&self, name: &str) {
        if let Ok(mut s) = self.source_name.lock() {
            *s = name.to_string();
        }
    }

    fn set_error(&self, msg: impl Into<String>) {
        if let Ok(mut e) = self.last_error.lock() {
            *e = Some(msg.into());
        }
    }

    fn clear_error(&self) {
        if let Ok(mut e) = self.last_error.lock() {
            *e = None;
        }
    }
}

/// Drives one audio source until stopped.
pub struct CaptureLoop {
    config: CaptureConfig,
    pub stats: Arc<CaptureStats>,
}

impl CaptureLoop {
    pub fn new(config: CaptureConfig) -> Self {
        Self {
            config,
            stats: Arc::new(CaptureStats::default()),
        }
    }

    /// Build from environment overrides.
    pub fn from_env() -> Self {
        let mut config = CaptureConfig::default();
        if let Ok(lang) = std::env::var("KAMISH_SPEECH_LANG") {
            config.language = lang;
        }
        Self::new(config)
    }

    /// Run until `shutdown` becomes true or the source ends.
    ///
    /// Returns the number of utterances dispatched to the AI orchestrator.
    pub async fn run(
        &self,
        mut source: Box<dyn AudioSource>,
        speech: Arc<SpeechManager>,
        state: crate::common::AppState,
        events: Arc<EventBus>,
        shutdown: Arc<AtomicBool>,
    ) -> u64 {
        self.stats.set_source(source.name());
        self.stats.listening.store(true, Ordering::Relaxed);
        self.stats.clear_error();

        info!("Capture loop started (source: {})", source.name());
        events.emit(
            "speech_capture_started",
            serde_json::json!({ "source": source.name() }),
        );

        let mut dispatched = 0u64;
        let mut pre_roll: std::collections::VecDeque<Vec<i16>> =
            std::collections::VecDeque::with_capacity(self.config.pre_roll_frames);
        // Start *disarmed*: the loop waits for a wake word before listening to a command.
        // (Previously this was seeded from `wakeword_enabled`, which is `true` by default
        // and therefore skipped wake word detection entirely.)
        let mut armed = false;
        let mut utterance: Vec<Vec<i16>> = Vec::new();
        let mut silent_run = 0usize;

        loop {
            if shutdown.load(Ordering::Relaxed) {
                debug!("Capture loop received shutdown signal");
                break;
            }

            let frame = match source.next_frame().await {
                Ok(Some(f)) => f,
                Ok(None) => {
                    info!("Audio source '{}' ended the stream", source.name());
                    break;
                }
                Err(e) => {
                    let msg = e.to_string();
                    warn!("Capture loop read error: {msg}");
                    self.stats.set_error(msg.clone());
                    events.emit("speech_capture_error", serde_json::json!({ "error": msg }));
                    break;
                }
            };

            self.stats.frames_total.fetch_add(1, Ordering::Relaxed);

            // Always feed the wake word spotter: it is the gate for arming.
            let wake = speech.process_pcm_frame(&frame);

            if !armed {
                pre_roll.push_back(frame);
                if pre_roll.len() > self.config.pre_roll_frames {
                    pre_roll.pop_front();
                }

                if let Some(event) = wake {
                    armed = true;
                    self.stats.wake_words_total.fetch_add(1, Ordering::Relaxed);
                    utterance.clear();
                    utterance.extend(pre_roll.drain(..));
                    silent_run = 0;

                    info!("Wake word spotted: '{}'", event.keyword);
                    events.emit(
                        "wake_word_detected",
                        serde_json::json!({
                            "keyword": event.keyword,
                            "score": event.score,
                            "timestamp_ms": event.timestamp_ms,
                        }),
                    );
                    self.emit_state(&events, "listening_command");
                }
                continue;
            }

            // Armed: accumulate speech, close the phrase on a trailing silence run.
            utterance.push(frame);
            let silent =
                !speech.vad_is_speech(utterance.last().map(|v| v.as_slice()).unwrap_or(&[]));
            if silent {
                silent_run += 1;
            } else {
                silent_run = 0;
            }

            let too_long = utterance.len() >= self.config.max_utterance_frames;
            let phrase_done =
                silent_run >= self.config.silence_tail_frames && utterance.len() > silent_run;

            if phrase_done || too_long {
                self.stats.utterances_total.fetch_add(1, Ordering::Relaxed);
                let dispatch = self
                    .handle_utterance(&utterance, &speech, &state, &events)
                    .await;
                if dispatch {
                    dispatched += 1;
                }
                utterance.clear();
                silent_run = 0;
                // Disarm again: the next phrase must be preceded by a fresh wake word.
                armed = false;
                self.emit_state(&events, "waiting_wake_word");
            }
        }

        self.stats.listening.store(false, Ordering::Relaxed);
        let _ = source.stop().await;
        self.emit_state(&events, "stopped");
        info!("Capture loop stopped after {dispatched} utterance(s)");
        dispatched
    }

    fn emit_state(&self, events: &Arc<EventBus>, state: &str) {
        events.emit(
            "speech_capture_state",
            serde_json::json!({
                "state": state,
                "source": self.stats.source_name.lock().ok().map(|s| s.clone()).unwrap_or_default(),
                "frames_total": self.stats.frames_total.load(Ordering::Relaxed),
                "wake_words_total": self.stats.wake_words_total.load(Ordering::Relaxed),
                "utterances_total": self.stats.utterances_total.load(Ordering::Relaxed),
            }),
        );
    }

    /// Convert assembled PCM frames into 16-bit LE bytes, run STT, then AI + TTS.
    async fn handle_utterance(
        &self,
        frames: &[Vec<i16>],
        speech: &Arc<SpeechManager>,
        state: &crate::common::AppState,
        events: &Arc<EventBus>,
    ) -> bool {
        let samples: Vec<i16> = frames.iter().flat_map(|f| f.iter().copied()).collect();
        if samples.is_empty() {
            return false;
        }
        let mut pcm_bytes = Vec::with_capacity(samples.len() * 2);
        for s in &samples {
            pcm_bytes.extend_from_slice(&s.to_le_bytes());
        }

        let transcript = match speech
            .recognize(&pcm_bytes, Some(&self.config.language), None)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("STT failed for captured utterance: {e}");
                self.stats.set_error(e.to_string());
                return false;
            }
        };

        let text = transcript.text.trim().to_string();
        if text.is_empty() {
            debug!("Empty transcript, skipping dispatch");
            return false;
        }

        info!("Heard: \"{text}\" (provider: {})", transcript.provider);
        if let Ok(mut t) = self.stats.last_transcript.lock() {
            *t = Some(text.clone());
        }
        events.emit(
            "speech_transcript",
            serde_json::json!({
                "text": text,
                "confidence": transcript.confidence,
                "provider": transcript.provider,
            }),
        );

        // Route the recognized phrase through the AI orchestrator and speak the reply.
        match speech
            .recognize_and_dispatch(&pcm_bytes, Some(&self.config.language), None, true, state)
            .await
        {
            Ok(resp) => {
                if let Some(ai) = resp.ai_response {
                    if let Ok(mut r) = self.stats.last_reply.lock() {
                        *r = Some(ai.response.clone());
                    }
                    events.emit(
                        "speech_reply",
                        serde_json::json!({
                            "response": ai.response,
                            "spoken": resp.spoken_response,
                        }),
                    );
                }
                true
            }
            Err(e) => {
                warn!("AI dispatch failed for utterance: {e}");
                self.stats.set_error(e.to_string());
                false
            }
        }
    }
}

/// Public snapshot of capture state for the status endpoint.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CaptureStatus {
    pub source: String,
    pub available: bool,
    pub listening: bool,
    pub frames_total: u64,
    pub wake_words_total: u64,
    pub utterances_total: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_transcript: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_reply: Option<String>,
}

/// Snapshot helper reused by the API layer.
pub fn snapshot(stats: &CaptureStats, fallback_source: &str, available: bool) -> CaptureStatus {
    CaptureStatus {
        source: stats
            .source_name
            .lock()
            .ok()
            .map(|s| s.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| fallback_source.to_string()),
        available,
        listening: stats.listening.load(Ordering::Relaxed),
        frames_total: stats.frames_total.load(Ordering::Relaxed),
        wake_words_total: stats.wake_words_total.load(Ordering::Relaxed),
        utterances_total: stats.utterances_total.load(Ordering::Relaxed),
        last_error: stats.last_error.lock().ok().and_then(|e| e.clone()),
        last_transcript: stats.last_transcript.lock().ok().and_then(|t| t.clone()),
        last_reply: stats.last_reply.lock().ok().and_then(|r| r.clone()),
    }
}

/// Re-exported for callers that build wake events manually.
pub type WakeEvent = WakeWordEvent;
