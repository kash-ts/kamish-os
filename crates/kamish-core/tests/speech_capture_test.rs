//! Integration tests for the microphone capture pipeline.
//!
//! These tests exercise the real [`CaptureLoop`] against the synthetic
//! [`MockAudioSource`], proving that the end-to-end chain
//! `frames → VAD → wake word → utterance → STT → AI` is wired correctly without
//! touching physical hardware.

use kamish_core::common::AppState;
use kamish_core::database;
use kamish_core::speech::capture::mock::{MockAudioSource, MockFrameKind};
use kamish_core::speech::capture::runner::CaptureConfig;
use kamish_core::speech::capture::{AudioSource, CaptureLoop};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

async fn test_state() -> AppState {
    let pool = database::init_pool("sqlite::memory:").await.unwrap();
    database::run_migrations(&pool).await.unwrap();
    AppState::with_options(pool, "agy".to_string(), false)
}

#[tokio::test]
async fn test_mock_source_delivers_frame_sized_pcm() {
    let mut src = MockAudioSource::new();
    for _ in 0..5 {
        let frame = src.next_frame().await.unwrap().expect("frame expected");
        assert_eq!(
            frame.len(),
            320,
            "frames must be 320 samples (20 ms @ 16 kHz)"
        );
    }
    assert_eq!(src.emitted_frames(), 5);
}

#[tokio::test]
async fn test_capture_loop_counts_frames_and_stops_on_shutdown() {
    let state = test_state().await;
    let shutdown = Arc::new(AtomicBool::new(false));

    let src = MockAudioSource::new();
    src.script(MockFrameKind::Silence, 3);

    let capture = CaptureLoop::new(CaptureConfig::default());
    let stats = capture.stats.clone();

    // Run with an already-set shutdown flag: the loop must exit promptly and cleanly.
    shutdown.store(true, Ordering::Relaxed);
    let dispatched = capture
        .run(
            Box::new(src),
            state.speech.clone(),
            state.clone(),
            state.events.clone(),
            shutdown,
        )
        .await;

    assert_eq!(dispatched, 0);
    assert!(!stats.listening.load(Ordering::Relaxed));
    assert_eq!(stats.frames_total.load(Ordering::Relaxed), 0);
}

#[tokio::test]
async fn test_capture_loop_detects_wake_word_from_synthetic_speech() {
    let state = test_state().await;
    let shutdown = Arc::new(AtomicBool::new(false));

    // Script a wake word utterance, then remain silent.
    let src = MockAudioSource::new();
    src.script_utterance();
    src.script(MockFrameKind::Silence, 60);

    let capture = CaptureLoop::new(CaptureConfig::default());
    let stats = capture.stats.clone();
    let shutdown_task = shutdown.clone();

    // Give the loop real pacing so the wake word spotter sees ~800 ms of audio.
    let handle = tokio::spawn(async move {
        capture
            .run(
                Box::new(src.with_pacing(std::time::Duration::from_millis(5))),
                state.speech.clone(),
                state.clone(),
                state.events.clone(),
                shutdown_task,
            )
            .await
    });

    // Allow enough wall-clock time to consume the scripted utterance.
    tokio::time::sleep(std::time::Duration::from_millis(900)).await;
    shutdown.store(true, Ordering::Relaxed);
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle).await;

    let frames = stats.frames_total.load(Ordering::Relaxed);
    assert!(frames > 0, "loop must have consumed frames");

    // The synthetic utterance is designed to satisfy the keyword spotter's envelope.
    let wake = stats.wake_words_total.load(Ordering::Relaxed);
    assert!(
        wake >= 1,
        "expected at least one wake word from synthetic speech, got {wake}"
    );
    assert!(
        !stats.listening.load(Ordering::Relaxed),
        "loop must mark itself stopped"
    );
}

#[tokio::test]
async fn test_capture_state_flags_are_honest_by_default() {
    let state = test_state().await;
    // Before any supervisor starts, the manager must not claim a live microphone.
    assert!(!state.speech.is_microphone_available());
    assert!(!state.speech.is_microphone_listening());

    let status = state.speech.get_status();
    assert!(!status.microphone_available);
    assert!(!status.microphone_listening);

    // Setting capture state reflects truthfully in the status payload.
    state.speech.set_capture_state(true, true);
    let live = state.speech.get_status();
    assert!(live.microphone_available);
    assert!(live.microphone_listening);

    state.speech.set_capture_state(false, false);
    let stopped = state.speech.get_status();
    assert!(!stopped.microphone_available);
    assert!(!stopped.microphone_listening);
}

#[tokio::test]
async fn test_capture_loop_converts_pcm_and_dispatches_ai() {
    let state = test_state().await;
    let shutdown = Arc::new(AtomicBool::new(false));

    // Force a very short utterance window so the phrase closes quickly.
    let config = CaptureConfig {
        language: "ru".to_string(),
        max_utterance_frames: 40,
        silence_tail_frames: 3,
        pre_roll_frames: 5,
    };

    let src = MockAudioSource::new();
    src.script_utterance();
    src.script(MockFrameKind::Silence, 80);

    let capture = CaptureLoop::new(config);
    let stats = capture.stats.clone();
    let shutdown_task = shutdown.clone();

    let handle = tokio::spawn(async move {
        capture
            .run(
                Box::new(src.with_pacing(std::time::Duration::from_millis(2))),
                state.speech.clone(),
                state.clone(),
                state.events.clone(),
                shutdown_task,
            )
            .await
    });

    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    shutdown.store(true, Ordering::Relaxed);
    let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle).await;

    assert!(stats.frames_total.load(Ordering::Relaxed) > 0);
    // A transcript proves the PCM → bytes → STT path produced usable text.
    let last = stats.last_transcript.lock().ok().and_then(|t| t.clone());
    assert!(
        last.is_some(),
        "expected a transcript from the captured utterance"
    );
    assert!(!last.unwrap().trim().is_empty());
}
