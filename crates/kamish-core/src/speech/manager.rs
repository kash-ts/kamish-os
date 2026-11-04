use crate::common::{AppError, AppResult, AppState};
use crate::speech::models::{
    RecognizeResponse, SimulateWakeWordResponse, SpeechStatus, SttResult, TtsVoice,
    UpdateSpeechConfigRequest, WakeWordEvent,
};
use crate::speech::stt::{
    CloudFreeSttProvider, MockSttProvider, SherpaOnnxSttProvider, VoskSttProvider,
};
use crate::speech::traits::{SttProvider, TtsProvider, VadDetector, WakeWordDetector};
use crate::speech::tts::{
    CloudFreeTtsProvider, OfflineFallbackTtsProvider, PiperTtsProvider, RhVoiceTtsProvider,
};
use crate::speech::vad::EnergyVadDetector;
use crate::speech::wakeword::KeywordSpotter;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tracing::{debug, warn};

/// Result of TTS synthesis holding raw audio bytes and format details.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SynthesizedAudio {
    pub bytes: Vec<u8>,
    pub format: &'static str, // "wav" or "mp3"
    pub provider: String,
    pub voice: String,
}

/// Central Speech Manager orchestrating STT, TTS, VAD, Wake Word, and AI loop.
pub struct SpeechManager {
    // STT Providers
    pub mock_stt: Arc<MockSttProvider>,
    pub vosk_stt: Arc<VoskSttProvider>,
    pub sherpa_stt: Arc<SherpaOnnxSttProvider>,
    pub cloud_stt: Arc<CloudFreeSttProvider>,

    // TTS Providers
    pub offline_tts: Arc<OfflineFallbackTtsProvider>,
    pub cloud_tts: Arc<CloudFreeTtsProvider>,
    pub piper_tts: Arc<PiperTtsProvider>,
    pub rhvoice_tts: Arc<RhVoiceTtsProvider>,

    // Processing Engines
    vad: Mutex<EnergyVadDetector>,
    wakeword: Mutex<KeywordSpotter>,

    active_stt_provider: RwLock<String>,
    active_tts_provider: RwLock<String>,
    active_voice: RwLock<String>,

    vad_enabled: AtomicBool,
    wakeword_enabled: AtomicBool,
    /// Set by the capture supervisor once an audio source is actually running.
    /// Never hardcoded: a `false` here means no microphone is being read.
    microphone_available: AtomicBool,
    /// Set by the capture supervisor while PCM frames are being pulled.
    microphone_listening: AtomicBool,

    last_wake_word: RwLock<Option<WakeWordEvent>>,
}

impl Default for SpeechManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SpeechManager {
    pub fn new() -> Self {
        let mock_stt = Arc::new(MockSttProvider::new());
        let vosk_stt = Arc::new(VoskSttProvider::new());
        let sherpa_stt = Arc::new(SherpaOnnxSttProvider::new());
        let cloud_stt = Arc::new(CloudFreeSttProvider::new());

        let offline_tts = Arc::new(OfflineFallbackTtsProvider::new());
        let cloud_tts = Arc::new(CloudFreeTtsProvider::new());
        let piper_tts = Arc::new(PiperTtsProvider::new());
        let rhvoice_tts = Arc::new(RhVoiceTtsProvider::new());

        let vad = Mutex::new(EnergyVadDetector::new());
        let wakeword = Mutex::new(KeywordSpotter::new());

        // Default to offline fallback / local TTS, and mock STT (or vosk if available)
        let default_tts = "offline_fallback".to_string();
        let default_stt = "mock".to_string();
        let default_voice = "offline_assistant".to_string();

        Self {
            mock_stt,
            vosk_stt,
            sherpa_stt,
            cloud_stt,
            offline_tts,
            cloud_tts,
            piper_tts,
            rhvoice_tts,
            vad,
            wakeword,
            active_stt_provider: RwLock::new(default_stt),
            active_tts_provider: RwLock::new(default_tts),
            active_voice: RwLock::new(default_voice),
            vad_enabled: AtomicBool::new(true),
            wakeword_enabled: AtomicBool::new(true),
            // Honest defaults: nothing is claimed until a capture source is started.
            microphone_available: AtomicBool::new(false),
            microphone_listening: AtomicBool::new(false),
            last_wake_word: RwLock::new(None),
        }
    }

    /// Returns current comprehensive status of speech subsystem
    pub fn get_status(&self) -> SpeechStatus {
        let active_stt = self
            .active_stt_provider
            .read()
            .map(|l| l.clone())
            .unwrap_or_else(|_| "mock".to_string());

        let active_tts = self
            .active_tts_provider
            .read()
            .map(|l| l.clone())
            .unwrap_or_else(|_| "offline_fallback".to_string());

        let active_voice = self
            .active_voice
            .read()
            .map(|l| l.clone())
            .unwrap_or_else(|_| "offline_assistant".to_string());

        let mut available_stt = Vec::new();
        if self.mock_stt.is_available() {
            available_stt.push("mock".to_string());
        }
        if self.vosk_stt.is_available() {
            available_stt.push("vosk".to_string());
        }
        if self.sherpa_stt.is_available() {
            available_stt.push("sherpa_onnx".to_string());
        }

        let mut available_tts = Vec::new();
        if self.offline_tts.is_available() {
            available_tts.push("offline_fallback".to_string());
        }
        if self.piper_tts.is_available() {
            available_tts.push("piper".to_string());
        }
        if self.rhvoice_tts.is_available() {
            available_tts.push("rhvoice".to_string());
        }

        let last_wake = self.last_wake_word.read().ok().and_then(|l| l.clone());

        SpeechStatus {
            microphone_available: self.microphone_available.load(Ordering::Relaxed),
            microphone_listening: self.microphone_listening.load(Ordering::Relaxed),
            active_stt_provider: active_stt,
            active_tts_provider: active_tts,
            active_voice,
            vad_enabled: self.vad_enabled.load(Ordering::Relaxed),
            wakeword_enabled: self.wakeword_enabled.load(Ordering::Relaxed),
            available_stt_providers: available_stt,
            available_tts_providers: available_tts,
            last_wake_word: last_wake,
        }
    }

    /// Returns list of all available TTS voices across all configured providers
    pub fn list_voices(&self) -> Vec<TtsVoice> {
        let mut voices = Vec::new();
        voices.extend(self.offline_tts.available_voices());
        voices.extend(self.piper_tts.available_voices());
        voices.extend(self.rhvoice_tts.available_voices());
        voices.extend(self.cloud_tts.available_voices());
        voices
    }

    /// Update active provider and configuration
    pub fn update_config(&self, req: UpdateSpeechConfigRequest) -> AppResult<()> {
        if let Some(ref stt) = req.active_stt_provider {
            let valid = matches!(stt.as_str(), "mock" | "vosk" | "sherpa_onnx" | "cloud_free");
            if !valid {
                return Err(AppError::Config(format!("Unknown STT provider: {}", stt)));
            }
            if let Ok(mut lock) = self.active_stt_provider.write() {
                *lock = stt.clone();
            }
        }

        if let Some(ref tts) = req.active_tts_provider {
            let valid = matches!(
                tts.as_str(),
                "offline_fallback" | "cloud_free" | "piper" | "rhvoice"
            );
            if !valid {
                return Err(AppError::Config(format!("Unknown TTS provider: {}", tts)));
            }
            if let Ok(mut lock) = self.active_tts_provider.write() {
                *lock = tts.clone();
            }
        }

        if let Some(ref voice) = req.active_voice {
            if let Ok(mut lock) = self.active_voice.write() {
                *lock = voice.clone();
            }
        }

        if let Some(vad) = req.vad_enabled {
            self.vad_enabled.store(vad, Ordering::Relaxed);
        }

        if let Some(ww) = req.wakeword_enabled {
            self.wakeword_enabled.store(ww, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Synthesize speech text into audio bytes (with automatic fallback to offline engine)
    pub async fn synthesize(
        &self,
        text: &str,
        voice: Option<&str>,
        provider: Option<&str>,
        speed: Option<f32>,
    ) -> AppResult<SynthesizedAudio> {
        let spd = speed.unwrap_or(1.0);
        let active_prov = self
            .active_tts_provider
            .read()
            .map(|l| l.clone())
            .unwrap_or_else(|_| "offline_fallback".to_string());

        let target_prov = provider.unwrap_or(&active_prov);

        let active_vc = self
            .active_voice
            .read()
            .map(|l| l.clone())
            .unwrap_or_else(|_| "offline_assistant".to_string());

        let target_vc = voice.unwrap_or(&active_vc);

        // Attempt requested provider
        let primary_result = match target_prov {
            "cloud_free" => {
                if self.cloud_tts.is_available() {
                    self.cloud_tts
                        .synthesize(text, Some(target_vc), spd)
                        .await
                        .map(|bytes| {
                            let fmt = if bytes.starts_with(b"RIFF") {
                                "wav"
                            } else {
                                "mp3"
                            };
                            SynthesizedAudio {
                                bytes,
                                format: fmt,
                                provider: "cloud_free".to_string(),
                                voice: target_vc.to_string(),
                            }
                        })
                } else {
                    Err(AppError::Config("CloudFree TTS not available".to_string()))
                }
            }
            "piper" => {
                if self.piper_tts.is_available() {
                    self.piper_tts
                        .synthesize(text, Some(target_vc), spd)
                        .await
                        .map(|bytes| SynthesizedAudio {
                            bytes,
                            format: "wav",
                            provider: "piper".to_string(),
                            voice: target_vc.to_string(),
                        })
                } else {
                    Err(AppError::Config(
                        "Piper TTS binary not installed".to_string(),
                    ))
                }
            }
            "rhvoice" => {
                if self.rhvoice_tts.is_available() {
                    self.rhvoice_tts
                        .synthesize(text, Some(target_vc), spd)
                        .await
                        .map(|bytes| SynthesizedAudio {
                            bytes,
                            format: "wav",
                            provider: "rhvoice".to_string(),
                            voice: target_vc.to_string(),
                        })
                } else {
                    Err(AppError::Config(
                        "RHVoice TTS binary not installed".to_string(),
                    ))
                }
            }
            "offline_fallback" => self
                .offline_tts
                .synthesize(text, Some(target_vc), spd)
                .await
                .map(|bytes| SynthesizedAudio {
                    bytes,
                    format: "wav",
                    provider: "offline_fallback".to_string(),
                    voice: target_vc.to_string(),
                }),
            _ => Err(AppError::Config(format!(
                "Unknown TTS provider '{}'",
                target_prov
            ))),
        };

        match primary_result {
            Ok(audio) => Ok(audio),
            Err(err) => {
                warn!(
                    "TTS provider '{}' failed ({}); falling back to offline synthesizer",
                    target_prov, err
                );
                // Guarantee offline fallback succeeds
                let fallback_bytes = self
                    .offline_tts
                    .synthesize(text, Some("offline_assistant"), spd)
                    .await?;

                Ok(SynthesizedAudio {
                    bytes: fallback_bytes,
                    format: "wav",
                    provider: "offline_fallback".to_string(),
                    voice: "offline_assistant".to_string(),
                })
            }
        }
    }

    /// Play audio bytes via system sound driver, respecting test silence flags
    pub async fn play_audio_bytes(bytes: &[u8], is_mp3: bool) -> AppResult<()> {
        if crate::audio::alert::is_audio_muted() {
            debug!("Audio playback suppressed (muted/test mode)");
            return Ok(());
        }

        let audio_data = bytes.to_vec();
        tokio::task::spawn_blocking(move || {
            let temp_dir = std::env::temp_dir();
            let ext = if is_mp3 { "mp3" } else { "wav" };
            let temp_path: PathBuf =
                temp_dir.join(format!("speech_playback_{}.{}", uuid::Uuid::new_v4(), ext));

            if let Err(e) = std::fs::write(&temp_path, &audio_data) {
                warn!("Failed writing audio playback temp file: {}", e);
                return;
            }

            let path_str = temp_path.to_string_lossy().to_string();
            let players: Vec<(&str, Vec<&str>)> = if is_mp3 {
                vec![
                    ("mpv", vec!["--no-terminal", "--really-quiet", &path_str]),
                    ("pw-play", vec![&path_str]),
                    ("paplay", vec![&path_str]),
                ]
            } else {
                vec![
                    ("aplay", vec!["-q", &path_str]),
                    ("pw-play", vec![&path_str]),
                    ("paplay", vec![&path_str]),
                    ("mpv", vec!["--no-terminal", "--really-quiet", &path_str]),
                ]
            };

            for (cmd, args) in players {
                if let Ok(status) = std::process::Command::new(cmd).args(&args).status() {
                    if status.success() {
                        debug!("Audio played successfully via {}", cmd);
                        break;
                    }
                }
            }

            let _ = std::fs::remove_file(&temp_path);
        });

        Ok(())
    }

    /// Transcribe audio data using active or requested STT provider
    pub async fn recognize(
        &self,
        audio_data: &[u8],
        language: Option<&str>,
        provider: Option<&str>,
    ) -> AppResult<SttResult> {
        let lang = language.unwrap_or("ru");
        let active_p = self
            .active_stt_provider
            .read()
            .map(|l| l.clone())
            .unwrap_or_else(|_| "mock".to_string());

        let target_p = provider.unwrap_or(&active_p);

        let res = match target_p {
            "vosk" => {
                if self.vosk_stt.is_available() {
                    self.vosk_stt.recognize(audio_data, lang).await
                } else {
                    Err(AppError::Config(
                        "Vosk STT binary not available".to_string(),
                    ))
                }
            }
            "sherpa_onnx" => {
                if self.sherpa_stt.is_available() {
                    self.sherpa_stt.recognize(audio_data, lang).await
                } else {
                    Err(AppError::Config(
                        "Sherpa-onnx STT binary not available".to_string(),
                    ))
                }
            }
            "cloud_free" => {
                if self.cloud_stt.is_available() {
                    self.cloud_stt.recognize(audio_data, lang).await
                } else {
                    Err(AppError::Config(
                        "Cloud STT endpoint not reachable".to_string(),
                    ))
                }
            }
            "mock" => self.mock_stt.recognize(audio_data, lang).await,
            _ => Err(AppError::Config(format!(
                "Unknown STT provider '{}'",
                target_p
            ))),
        };

        match res {
            Ok(stt) => Ok(stt),
            Err(err) => {
                warn!(
                    "STT provider '{}' failed ({}); falling back to mock STT",
                    target_p, err
                );
                self.mock_stt.recognize(audio_data, lang).await
            }
        }
    }

    /// Record that an audio capture source is up and streaming.
    ///
    /// Called by the capture supervisor only when frames are genuinely being pulled;
    /// this is what makes `microphone_available` / `microphone_listening` truthful.
    pub fn set_capture_state(&self, available: bool, listening: bool) {
        self.microphone_available
            .store(available, Ordering::Relaxed);
        self.microphone_listening
            .store(listening, Ordering::Relaxed);
    }

    /// Whether the capture supervisor currently has a working source.
    pub fn is_microphone_available(&self) -> bool {
        self.microphone_available.load(Ordering::Relaxed)
    }

    /// Whether PCM frames are actively being consumed.
    pub fn is_microphone_listening(&self) -> bool {
        self.microphone_listening.load(Ordering::Relaxed)
    }

    /// Query the VAD for a single frame without touching the wake word spotter.
    ///
    /// Used by the capture loop to detect the end of an utterance after the wake word
    /// has already armed the pipeline.
    pub fn vad_is_speech(&self, frame: &[i16]) -> bool {
        if !self.vad_enabled.load(Ordering::Relaxed) {
            // With VAD disabled the pipeline never declares silence, so utterances are
            // bounded only by `max_utterance_frames`.
            return !frame.is_empty();
        }
        match self.vad.lock() {
            Ok(mut vad) => vad.is_speech(frame),
            Err(_) => false,
        }
    }

    /// Process PCM frame through VAD and Wake Word spotter
    pub fn process_pcm_frame(&self, frame: &[i16]) -> Option<WakeWordEvent> {
        if self.vad_enabled.load(Ordering::Relaxed) {
            if let Ok(mut vad) = self.vad.lock() {
                if !vad.is_speech(frame) {
                    return None;
                }
            }
        }

        if self.wakeword_enabled.load(Ordering::Relaxed) {
            if let Ok(mut ww) = self.wakeword.lock() {
                if let Some(event) = ww.process_frame(frame) {
                    if let Ok(mut last) = self.last_wake_word.write() {
                        *last = Some(event.clone());
                    }
                    return Some(event);
                }
            }
        }

        None
    }

    /// Simulate wake word spotting and optional AI query & vocal reply
    pub async fn simulate_wakeword(
        &self,
        keyword: Option<&str>,
        command: Option<&str>,
        dispatch_ai: bool,
        state: &AppState,
    ) -> AppResult<SimulateWakeWordResponse> {
        let event = if let Ok(ww) = self.wakeword.lock() {
            ww.simulate_trigger(keyword)
        } else {
            KeywordSpotter::new().simulate_trigger(keyword)
        };

        if let Ok(mut last) = self.last_wake_word.write() {
            *last = Some(event.clone());
        }

        let mut ai_res = None;
        let mut spoken = false;

        if let Some(cmd) = command {
            if dispatch_ai && !cmd.trim().is_empty() {
                match state.ai.ask(cmd, None, state).await {
                    Ok(resp) => {
                        // Automatically speak response via TTS
                        if let Ok(synth) = self.synthesize(&resp.response, None, None, None).await {
                            let is_mp3 = synth.format == "mp3";
                            let _ = Self::play_audio_bytes(&synth.bytes, is_mp3).await;
                            spoken = true;
                        }
                        ai_res = Some(resp);
                    }
                    Err(e) => {
                        warn!("AI ask error during wake word simulate: {}", e);
                    }
                }
            }
        }

        Ok(SimulateWakeWordResponse {
            event,
            command: command.map(|c| c.to_string()),
            ai_response: ai_res,
            spoken_response: spoken,
        })
    }

    /// Recognize audio and optionally dispatch recognized text into AI orchestrator and TTS
    pub async fn recognize_and_dispatch(
        &self,
        audio_data: &[u8],
        language: Option<&str>,
        provider: Option<&str>,
        dispatch_ai: bool,
        state: &AppState,
    ) -> AppResult<RecognizeResponse> {
        let stt_result = self.recognize(audio_data, language, provider).await?;

        let mut ai_resp = None;
        let mut spoken = false;

        if dispatch_ai && !stt_result.text.trim().is_empty() {
            match state.ai.ask(&stt_result.text, None, state).await {
                Ok(resp) => {
                    // Automatically synthesize and speak AI response
                    if let Ok(synth) = self.synthesize(&resp.response, None, None, None).await {
                        let is_mp3 = synth.format == "mp3";
                        let _ = Self::play_audio_bytes(&synth.bytes, is_mp3).await;
                        spoken = true;
                    }
                    ai_resp = Some(resp);
                }
                Err(e) => {
                    warn!("Failed dispatching recognized speech to AI: {}", e);
                }
            }
        }

        Ok(RecognizeResponse {
            stt: stt_result,
            ai_response: ai_resp,
            spoken_response: spoken,
        })
    }
}
