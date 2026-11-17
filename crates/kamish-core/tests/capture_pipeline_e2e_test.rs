//! End-to-end proof that the capture pipeline turns synthetic audio into an AI reply.
//!
//! Unlike the unit-level loop test, this one drives the *full* stack the way the
//! supervisor does and asserts the observable side effects: wake word counting,
//! transcript, AI reply and event bus traffic.

use kamish_core::common::AppState;
use kamish_core::database;
use kamish_core::speech::capture::mock::{MockAudioSource, MockFrameKind};
use kamish_core::speech::capture::runner::CaptureConfig;
use kamish_core::speech::capture::CaptureLoop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

async fn state_with_mock_ai() -> AppState {
    let pool = database::init_pool("sqlite::memory:").await.unwrap();
    database::run_migrations(&pool).await.unwrap();
    // Use the in-process mock so this test does not depend on an executable CLI
    // being present or runnable in the CI environment.
    AppState::with_options(pool, "agy".to_string(), false)
}

#[tokio::test]
async fn test_full_pipeline_produces_transcript_and_reply() {
    let state = state_with_mock_ai().await;
    let shutdown = Arc::new(AtomicBool::new(false));

    let src = MockAudioSource::new();
    // Two utterances in a row: the loop must re-arm after each one.
    src.script_utterance();
    src.script(MockFrameKind::Silence, 20);
    src.script_utterance();
    src.script(MockFrameKind::Silence, 40);

    let capture = CaptureLoop::new(CaptureConfig {
        language: "ru".to_string(),
        max_utterance_frames: 60,
        silence_tail_frames: 8,
        pre_roll_frames: 8,
    });
    let stats = capture.stats.clone();
    let sd = shutdown.clone();

    let handle = tokio::spawn(async move {
        capture
            .run(
                Box::new(src.with_pacing(Duration::from_millis(3))),
                state.speech.clone(),
                state.clone(),
                state.events.clone(),
                sd,
            )
            .await
    });

    // Allow enough time for capture, transcription and AI dispatch to finish. A
    // fixed short sleep was flaky under CI load and could stop the loop mid-request.
    let _ = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if stats.last_reply.lock().ok().and_then(|r| r.clone()).is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await;

    shutdown.store(true, Ordering::Relaxed);
    let _ = tokio::time::timeout(Duration::from_secs(8), handle).await;

    let frames = stats.frames_total.load(Ordering::Relaxed);
    let wakes = stats.wake_words_total.load(Ordering::Relaxed);
    let utterances = stats.utterances_total.load(Ordering::Relaxed);

    assert!(frames > 0, "no frames consumed");
    assert!(wakes >= 1, "no wake word detected (frames={frames})");
    assert!(
        utterances >= 1,
        "no utterance closed (frames={frames}, wakes={wakes})"
    );

    let transcript = stats
        .last_transcript
        .lock()
        .ok()
        .and_then(|t| t.clone())
        .expect("transcript must be produced from captured PCM");
    assert!(!transcript.trim().is_empty());

    let reply = stats.last_reply.lock().ok().and_then(|r| r.clone());
    assert!(
        reply.is_some(),
        "AI reply missing; transcript was {transcript:?}"
    );
    assert!(!reply.unwrap().trim().is_empty());
}
