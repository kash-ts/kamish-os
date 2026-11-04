//! Speech Manager module.
//!
//! Provides modular provider interfaces for Speech-To-Text (STT) and Text-To-Speech (TTS),
//! energy & zero-crossing rate Voice Activity Detection (VAD), Wake Word keyword spotting,
//! and central speech-to-AI dispatch orchestration.

pub mod capture;
pub mod manager;
pub mod models;
pub mod stt;
pub mod traits;
pub mod tts;
pub mod vad;
pub mod wakeword;

pub use capture::{
    ArecordAudioSource, AudioSource, CaptureConfig, CaptureLoop, CaptureStats, CaptureSupervisor,
    MockAudioSource, SourceKind,
};
pub use manager::{SpeechManager, SynthesizedAudio};
pub use models::{
    RecognizeRequest, RecognizeResponse, SimulateWakeWordRequest, SimulateWakeWordResponse,
    SpeechStatus, SttResult, SynthesizeRequest, SynthesizeResponse, TtsVoice,
    UpdateSpeechConfigRequest, WakeWordEvent,
};
pub use stt::{CloudFreeSttProvider, MockSttProvider, SherpaOnnxSttProvider, VoskSttProvider};
pub use traits::{SttProvider, TtsProvider, VadDetector, WakeWordDetector};
pub use tts::{
    CloudFreeTtsProvider, OfflineFallbackTtsProvider, PiperTtsProvider, RhVoiceTtsProvider,
};
pub use vad::EnergyVadDetector;
pub use wakeword::KeywordSpotter;
