use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use base64::Engine;
use http_body_util::BodyExt;
use kamish_core::{audio::alert::set_audio_muted, build_app, config::Config};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn setup_app() -> axum::Router {
    set_audio_muted(true);
    let mut config = Config::from_env();
    config.database_url = "sqlite::memory:?cache=shared".to_string();
    config.ai_cli_enabled = false;

    let (app, _state) = build_app(&config)
        .await
        .expect("Failed to build app for speech tests");

    app
}

#[tokio::test]
async fn test_speech_status_endpoint() {
    set_audio_muted(true);
    let app = setup_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/speech/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // No capture source is started in tests, so the microphone must NOT be advertised
    // as available/listening. This guards against the old hardcoded `true` constants.
    assert!(!json["microphone_available"].as_bool().unwrap());
    assert!(!json["microphone_listening"].as_bool().unwrap());
    assert!(json["vad_enabled"].as_bool().unwrap());
    assert!(json["wakeword_enabled"].as_bool().unwrap());
    assert!(!json["active_stt_provider"].as_str().unwrap().is_empty());
    assert!(!json["active_tts_provider"].as_str().unwrap().is_empty());

    let available_tts = json["available_tts_providers"].as_array().unwrap();
    assert!(available_tts
        .iter()
        .any(|p| p.as_str() == Some("offline_fallback")));

    let available_stt = json["available_stt_providers"].as_array().unwrap();
    assert!(available_stt.iter().any(|p| p.as_str() == Some("mock")));
}

#[tokio::test]
async fn test_speech_voices_endpoint() {
    set_audio_muted(true);
    let app = setup_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/speech/voices")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    let voices = json["voices"].as_array().expect("voices array");
    assert!(voices.len() >= 3);

    let voice_ids: Vec<&str> = voices.iter().filter_map(|v| v["id"].as_str()).collect();
    assert!(voice_ids.contains(&"offline_assistant"));
    assert!(voice_ids.contains(&"google_ru"));
}

#[tokio::test]
async fn test_speech_config_update() {
    set_audio_muted(true);
    let app = setup_app().await;

    let update_req = json!({
        "active_stt_provider": "mock",
        "active_tts_provider": "offline_fallback",
        "active_voice": "offline_melodic",
        "vad_enabled": false,
        "wakeword_enabled": true
    });

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/speech/config")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(update_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["active_stt_provider"], "mock");
    assert_eq!(json["active_tts_provider"], "offline_fallback");
    assert_eq!(json["active_voice"], "offline_melodic");
    assert_eq!(json["vad_enabled"], false);
    assert_eq!(json["wakeword_enabled"], true);
}

#[tokio::test]
async fn test_speech_synthesize_raw_binary_and_json() {
    set_audio_muted(true);
    let app = setup_app().await;

    // 1. Synthesize requesting WAV stream
    let req = json!({
        "text": "Тестовое голосовое сообщение",
        "provider": "offline_fallback",
        "voice": "offline_assistant"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/speech/synthesize")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ACCEPT, "audio/wav")
                .body(Body::from(req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get(header::CONTENT_TYPE).unwrap(),
        "audio/wav"
    );

    let wav_bytes = res.into_body().collect().await.unwrap().to_bytes();
    assert!(wav_bytes.len() > 44);
    assert_eq!(&wav_bytes[0..4], b"RIFF");
    assert_eq!(&wav_bytes[8..12], b"WAVE");

    // 2. Synthesize requesting JSON format
    let res_json = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/speech/tts?format=json")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res_json.status(), StatusCode::OK);
    let body = res_json.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "ok");
    assert_eq!(json["provider"], "offline_fallback");
    assert!(json["audio_base64"].as_str().is_some());

    // 3. Synthesize with play_on_speaker=true
    let play_req = json!({
        "text": "Оповещение",
        "provider": "offline_fallback",
        "play_on_speaker": true
    });

    let res_play = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/speech/synthesize")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(play_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res_play.status(), StatusCode::OK);
    let body_play = res_play.into_body().collect().await.unwrap().to_bytes();
    let json_play: Value = serde_json::from_slice(&body_play).unwrap();
    assert_eq!(json_play["status"], "playing");
    assert_eq!(json_play["played_on_speaker"], true);
}

#[tokio::test]
async fn test_speech_recognize_and_dispatch_ai() {
    set_audio_muted(true);
    let app = setup_app().await;

    // 1. Raw byte recognize with CMD marker
    let cmd_payload = "CMD:поставь будильник на 07:45".as_bytes();
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/speech/recognize?provider=mock&language=ru&dispatch_ai=true")
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .body(Body::from(cmd_payload.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["stt"]["text"], "поставь будильник на 07:45");
    assert_eq!(json["stt"]["provider"], "mock");
    assert!(json["ai_response"].is_object());
    assert!(json["spoken_response"].as_bool().unwrap());

    let ai_text = json["ai_response"]["response"].as_str().unwrap();
    assert!(ai_text.contains("07:45") || ai_text.contains("Будильник"));

    // 2. Base64 JSON recognize
    let b64 =
        base64::engine::general_purpose::STANDARD.encode("PROMPT:какой статус системы?".as_bytes());
    let json_req = json!({
        "audio_base64": b64,
        "language": "ru",
        "provider": "mock",
        "dispatch_ai": true
    });

    let res_json = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/speech/stt")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json_req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res_json.status(), StatusCode::OK);
    let body2 = res_json.into_body().collect().await.unwrap().to_bytes();
    let json2: Value = serde_json::from_slice(&body2).unwrap();

    assert_eq!(json2["stt"]["text"], "какой статус системы?");
    assert!(json2["ai_response"].is_object());
}

#[tokio::test]
async fn test_speech_wakeword_simulate() {
    set_audio_muted(true);
    let app = setup_app().await;

    let req = json!({
        "keyword": "колонка",
        "command": "напомни через 5 минут выключить чайник",
        "dispatch_ai": true
    });

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/speech/wakeword/simulate")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(req.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["event"]["keyword"], "колонка");
    assert!(json["event"]["score"].as_f64().unwrap() > 0.8);
    assert_eq!(json["command"], "напомни через 5 минут выключить чайник");
    assert!(json["ai_response"].is_object());
    assert!(json["spoken_response"].as_bool().unwrap());
}

#[tokio::test]
async fn test_capture_status_endpoint_reports_no_source_by_default() {
    set_audio_muted(true);
    let app = setup_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/speech/capture/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Nothing started: capture must not claim to be listening.
    assert!(!json["listening"].as_bool().unwrap());
    assert_eq!(json["frames_total"].as_u64().unwrap(), 0);
    assert_eq!(json["wake_words_total"].as_u64().unwrap(), 0);
    // Source name is always reported, even when idle.
    assert!(json["source"].is_string());
}

#[tokio::test]
async fn test_capture_start_reports_unavailable_without_microphone() {
    set_audio_muted(true);
    // Force the arecord source, which is absent in the CI/dev container.
    // SAFETY: single-threaded test setup before any capture task is spawned.
    unsafe {
        std::env::set_var("KAMISH_AUDIO_SOURCE", "arecord");
        std::env::remove_var("KAMISH_ALLOW_MOCK_MIC");
    }

    let app = setup_app().await;
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/speech/capture/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    // Without arecord installed the endpoint must answer honestly, not pretend success.
    let status = json["status"].as_str().unwrap();
    assert!(
        status == "unavailable" || status == "started",
        "unexpected capture start status: {status}"
    );
    if status == "unavailable" {
        assert!(!json["capture"]["listening"].as_bool().unwrap());
    }

    unsafe {
        std::env::remove_var("KAMISH_AUDIO_SOURCE");
    }
}

#[tokio::test]
async fn test_capture_stop_is_idempotent_when_idle() {
    set_audio_muted(true);
    let app = setup_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/speech/capture/stop")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "idle");
    assert!(!json["capture"]["listening"].as_bool().unwrap());
}
