//! Audio capture module.
//!
//! Provides a pluggable [`AudioSource`] abstraction over microphone input, so the
//! rest of the speech pipeline never talks to ALSA directly. Two implementations ship:
//!
//! * [`MockAudioSource`] — deterministic synthetic frames for x86_64/WSL and tests where
//!   no physical microphone exists. Never claims real hardware.
//! * [`ArecordAudioSource`] — real capture via the `arecord` ALSA utility, used on the
//!   Linux / Buildroot target.
//!
//! The [`CaptureLoop`] glues a source to VAD, wake-word spotting, STT, the AI
//! orchestrator and TTS playback.

pub mod arecord;
pub mod mock;
pub mod runner;
pub mod supervisor;

pub use arecord::ArecordAudioSource;
pub use mock::MockAudioSource;
pub use runner::{CaptureConfig, CaptureLoop, CaptureStats, CaptureStatus};
pub use supervisor::{CaptureSupervisor, SourceKind};

use crate::common::AppResult;
use std::future::Future;
use std::pin::Pin;

/// Sample rate used across the whole speech pipeline (Hz).
pub const CAPTURE_SAMPLE_RATE: u32 = 16000;

/// Number of channels captured (mono).
pub const CAPTURE_CHANNELS: u16 = 1;

/// Size of one processing frame in samples (~20 ms at 16 kHz).
pub const FRAME_SAMPLES: usize = 320;

/// A pluggable microphone input.
///
/// Implementations must deliver fixed-size PCM frames of [`FRAME_SAMPLES`] samples,
/// 16-bit signed little-endian, mono, at [`CAPTURE_SAMPLE_RATE`].
/// Object-safe boxed future used by [`AudioSource`] so it can live behind `dyn`.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait AudioSource: Send + Sync + 'static {
    /// Stable identifier for status reporting (e.g. "arecord", "mock").
    fn name(&self) -> &str;

    /// Whether this source can actually capture right now.
    ///
    /// Must return `false` when the underlying device or binary is missing — never
    /// pretend a microphone exists.
    fn is_available(&self) -> bool;

    /// Human-readable reason the source is unavailable, if any.
    fn unavailable_reason(&self) -> Option<String> {
        None
    }

    /// Block until the next PCM frame is ready.
    ///
    /// Returns `Ok(Some(frame))` with exactly [`FRAME_SAMPLES`] samples, or
    /// `Ok(None)` when the stream ended cleanly.
    fn next_frame(&mut self) -> BoxFuture<'_, AppResult<Option<Vec<i16>>>>;

    /// Stop capturing and release the device.
    fn stop(&mut self) -> BoxFuture<'_, AppResult<()>>;
}
