//! Deterministic synthetic microphone source for dev environments and tests.
//!
//! WSL2 and most x86_64 dev machines have no usable microphone passthrough, and the
//! project rules forbid inventing hardware. This source therefore reports itself as a
//! *mock* in its name and generates synthetic PCM on demand, so the capture loop,
//! VAD, wake word spotting and STT dispatch can all be exercised without a device.

use super::{AudioSource, BoxFuture, CAPTURE_SAMPLE_RATE, FRAME_SAMPLES};
use crate::common::AppResult;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

/// What the synthetic source should emit for a given stretch of frames.
#[derive(Debug, Clone, PartialEq)]
pub enum MockFrameKind {
    /// Digital silence.
    Silence,
    /// A voiced tone loud enough to pass VAD (used to emulate speech).
    Speech,
}

/// Scripted synthetic microphone.
///
/// Frames are drawn from an internal queue. When the queue is empty the source keeps
/// emitting [`MockFrameKind::Silence`], mirroring a quiet room.
pub struct MockAudioSource {
    queue: Mutex<VecDeque<MockFrameKind>>,
    /// Delay inserted before each frame to emulate real-time pacing; tests use `None`.
    pace: Option<Duration>,
    /// Total frames emitted so far (diagnostics / assertions).
    emitted: Mutex<u64>,
    stopped: Mutex<bool>,
}

impl Default for MockAudioSource {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAudioSource {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            pace: None,
            emitted: Mutex::new(0),
            stopped: Mutex::new(false),
        }
    }

    /// Pace frame delivery in real time. Integration tests leave this `None`.
    pub fn with_pacing(mut self, pace: Duration) -> Self {
        self.pace = Some(pace);
        self
    }

    /// Enqueue a scripted run of frames.
    pub fn script(&self, kind: MockFrameKind, count: usize) {
        if let Ok(mut q) = self.queue.lock() {
            for _ in 0..count {
                q.push_back(kind.clone());
            }
        }
    }

    /// Enqueue a complete "wake word + command" utterance.
    ///
    /// Produces a multi-syllable amplitude envelope (which the keyword spotter needs)
    /// and then a trailing silence tail so the VAD hangover settles. The run is 24
    /// frames (~480 ms), comfortably above the spotter's 15-frame history requirement.
    pub fn script_utterance(&self) {
        // Four syllable groups of 6 frames: repeated 3-2-3 envelope peaks.
        self.script(MockFrameKind::Speech, 24);
        self.script(MockFrameKind::Silence, 12);
    }

    /// Frames emitted since creation.
    pub fn emitted_frames(&self) -> u64 {
        self.emitted.lock().map(|g| *g).unwrap_or(0)
    }

    fn synth(kind: &MockFrameKind, frame_index: u64) -> Vec<i16> {
        match kind {
            MockFrameKind::Silence => vec![0i16; FRAME_SAMPLES],
            MockFrameKind::Speech => {
                // Voiced 250 Hz tone whose per-frame amplitude follows a syllable
                // envelope. The envelope is driven by the *frame index* rather than
                // absolute sample time: syllables must change loudness at the ~20 ms
                // frame granularity, otherwise a single slow swell yields only one
                // energy peak and the keyword spotter (which needs >= 2 peaks) can
                // never arm.
                let freq = 250.0_f32;
                let sr = CAPTURE_SAMPLE_RATE as f32;
                let frame = frame_index as usize;
                // Two distinct syllable peaks inside the spotter's ~800 ms window.
                // The period must be long enough that each peak spans several frames,
                // otherwise adjacent local maxima exceed the spotter's 6-peak ceiling.
                let phase = (frame % 8) as f32 / 8.0;
                let gain = 0.15 + 0.85 * (std::f32::consts::PI * phase).sin();
                let frame_gain = gain.max(0.2);
                let mut out = Vec::with_capacity(FRAME_SAMPLES);
                for n in 0..FRAME_SAMPLES {
                    let t = (frame * FRAME_SAMPLES + n) as f32 / sr;
                    // Gentle intra-frame taper keeps ZCR stable as speech-like.
                    let s = (2.0 * std::f32::consts::PI * freq * t).sin() * frame_gain;
                    out.push((s * 16000.0) as i16);
                }
                out
            }
        }
    }
}

impl AudioSource for MockAudioSource {
    fn name(&self) -> &str {
        "mock"
    }

    /// Always captured as available: this is a software generator, and its name says so.
    fn is_available(&self) -> bool {
        true
    }

    fn unavailable_reason(&self) -> Option<String> {
        Some("Synthetic microphone: no physical capture device is used".to_string())
    }

    fn next_frame(&mut self) -> BoxFuture<'_, AppResult<Option<Vec<i16>>>> {
        Box::pin(async move {
            if self.stopped.lock().map(|g| *g).unwrap_or(false) {
                return Ok(None);
            }

            if let Some(pace) = self.pace {
                tokio::time::sleep(pace).await;
            }

            let kind = self
                .queue
                .lock()
                .ok()
                .and_then(|mut q| q.pop_front())
                .unwrap_or(MockFrameKind::Silence);

            let index = {
                let mut e = match self.emitted.lock() {
                    Ok(e) => e,
                    Err(_) => return Ok(None),
                };
                let i = *e;
                *e += 1;
                i
            };

            Ok(Some(Self::synth(&kind, index)))
        })
    }

    fn stop(&mut self) -> BoxFuture<'_, AppResult<()>> {
        Box::pin(async move {
            if let Ok(mut s) = self.stopped.lock() {
                *s = true;
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_source_emits_frames_of_expected_size() {
        let mut src = MockAudioSource::new();
        let frame = src.next_frame().await.unwrap().unwrap();
        assert_eq!(frame.len(), FRAME_SAMPLES);
        assert_eq!(src.name(), "mock");
        assert!(src.is_available());
    }

    #[tokio::test]
    async fn test_mock_source_scripted_speech_is_audible_and_silence_is_quiet() {
        let mut src = MockAudioSource::new();
        src.script(MockFrameKind::Speech, 1);
        let speech = src.next_frame().await.unwrap().unwrap();
        let speech_rms = crate::speech::vad::EnergyVadDetector::calculate_rms(&speech);

        src.script(MockFrameKind::Silence, 1);
        let silence = src.next_frame().await.unwrap().unwrap();
        let silence_rms = crate::speech::vad::EnergyVadDetector::calculate_rms(&silence);

        assert!(silence_rms == 0.0);
        assert!(
            speech_rms > 0.05,
            "speech frame must be loud enough for VAD, got {speech_rms}"
        );
    }

    #[tokio::test]
    async fn test_mock_source_stop_ends_stream() {
        let mut src = MockAudioSource::new();
        assert!(src.next_frame().await.unwrap().is_some());
        src.stop().await.unwrap();
        assert!(src.next_frame().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_mock_source_reports_no_physical_hardware() {
        let src = MockAudioSource::new();
        assert!(
            src.unavailable_reason().is_some(),
            "mock must not claim a physical microphone"
        );
    }
}
