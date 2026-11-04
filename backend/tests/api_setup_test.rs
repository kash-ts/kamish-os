use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use kamish_core::{audio::alert::set_audio_muted, build_app, config::Config};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn test_setup_app() -> (axum::Router, kamish_core::common::AppState) {
    set_audio_muted(true);
    let mut config = Config::from_env();
    config.database_url = format!("sqlite::memory:");
    let (app, state) = build_app(&config).await.expect("Failed to build test app");
    (app, state)
}

#[tokio::test]
async fn test_setup_initial_status() {
    let (app, _) = test_setup_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/setup/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["is_completed"], false);
    assert_eq!(body["current_step"], "network");

    let steps = body["steps"].as_array().expect("steps should be an array");
    assert_eq!(steps.len(), 4);

    let step_ids: Vec<&str> = steps.iter().map(|s| s["id"].as_str().unwrap()).collect();
    assert_eq!(step_ids, vec!["network", "audio", "voice", "ai"]);

    assert!(body["degraded_components"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_setup_step_progression_and_safe_volume_ceiling() {
    let (app, state) = test_setup_app().await;

    // 1. Complete "network" step
    let net_step_body = json!({
        "step": "network",
        "status": "completed",
        "data": { "ssid": "Home-WiFi-5G", "ip_address": "192.168.1.100" }
    });

    let net_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/setup/step")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&net_step_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(net_res.status(), StatusCode::OK);
    let net_bytes = net_res.into_body().collect().await.unwrap().to_bytes();
    let net_json: Value = serde_json::from_slice(&net_bytes).unwrap();
    assert_eq!(net_json["next_step"], "audio");

    // 4. Complete "audio" step with volume > 85% to test Safe Volume Ceiling clamping
    let audio_step_body = json!({
        "step": "audio",
        "status": "completed",
        "data": { "volume": 95, "output_device": "GT-112 Line-Out" }
    });

    let audio_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/setup/step")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&audio_step_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(audio_res.status(), StatusCode::OK);
    let audio_bytes = audio_res.into_body().collect().await.unwrap().to_bytes();
    let audio_json: Value = serde_json::from_slice(&audio_bytes).unwrap();
    assert_eq!(audio_json["next_step"], "voice");

    // Verify audio volume clamped in DB
    let (_, audio_data, _) =
        kamish_core::setup::SetupRepository::get_step_status(&state.db, "audio")
            .await
            .unwrap();
    let clamped_vol = audio_data.unwrap()["volume"].as_u64().unwrap();
    assert_eq!(
        clamped_vol, 85,
        "Volume should be clamped to 85% safe ceiling"
    );

    // 5. Complete "voice" step
    let voice_step_body = json!({
        "step": "voice",
        "status": "completed",
        "data": { "microphone": "Seeed ReSpeaker XVF3800", "stt": "vosk", "tts": "piper" }
    });

    let voice_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/setup/step")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&voice_step_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(voice_res.status(), StatusCode::OK);
    let voice_bytes = voice_res.into_body().collect().await.unwrap().to_bytes();
    let voice_json: Value = serde_json::from_slice(&voice_bytes).unwrap();
    assert_eq!(voice_json["next_step"], "ai");

    // 6. Complete "ai" step
    let ai_step_body = json!({
        "step": "ai",
        "status": "completed",
        "data": { "mode": "cli", "version": "1.4.2" }
    });

    let ai_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/setup/step")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&ai_step_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(ai_res.status(), StatusCode::OK);
    let ai_bytes = ai_res.into_body().collect().await.unwrap().to_bytes();
    let ai_json: Value = serde_json::from_slice(&ai_bytes).unwrap();
    assert_eq!(ai_json["next_step"], "completed");

    // 7. Complete entire setup wizard
    let complete_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/setup/complete")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(complete_res.status(), StatusCode::OK);
    let complete_bytes = complete_res.into_body().collect().await.unwrap().to_bytes();
    let complete_json: Value = serde_json::from_slice(&complete_bytes).unwrap();
    assert_eq!(complete_json["is_completed"], true);

    // 8. Verify GET /api/v1/setup/status reflects completed state
    let status_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/setup/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(status_res.status(), StatusCode::OK);
    let status_bytes = status_res.into_body().collect().await.unwrap().to_bytes();
    let status_json: Value = serde_json::from_slice(&status_bytes).unwrap();
    assert_eq!(status_json["is_completed"], true);
}

#[tokio::test]
async fn test_setup_complete_premature_fails() {
    let (app, _) = test_setup_app().await;

    // Call /complete before completing any steps
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/setup/complete")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn test_setup_diagnostics_endpoint() {
    let (app, _) = test_setup_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/setup/diagnostics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body["timestamp"].is_string());
    assert!(body["subsystems"]["audio"].is_object());
    assert!(body["subsystems"]["network"].is_object());
    assert!(body["subsystems"]["voice"].is_object());
    assert!(body["subsystems"]["ai"].is_object());
}

#[tokio::test]
async fn test_simulate_degradation_and_recovery() {
    let (app, _) = test_setup_app().await;

    // 1. Inject simulated degradation for AI
    let deg_body = json!({
        "component": "ai",
        "degraded": true,
        "error_reason": "Simulated: Antigravity CLI binary exited with error 127"
    });

    let deg_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/setup/simulate-degradation")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&deg_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(deg_res.status(), StatusCode::OK);

    // 2. Check /setup/status reflects degraded state
    let status_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/setup/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(status_res.status(), StatusCode::OK);
    let status_bytes = status_res.into_body().collect().await.unwrap().to_bytes();
    let status_json: Value = serde_json::from_slice(&status_bytes).unwrap();

    let degraded = status_json["degraded_components"].as_array().unwrap();
    assert_eq!(degraded.len(), 1);
    assert_eq!(degraded[0]["component"], "ai");
    assert_eq!(degraded[0]["recovery_step"], "ai");
    assert_eq!(degraded[0]["can_fallback"], true);

    // Check that AI step status is "degraded"
    let steps = status_json["steps"].as_array().unwrap();
    let ai_step = steps.iter().find(|s| s["id"] == "ai").unwrap();
    assert_eq!(ai_step["status"], "degraded");

    // 3. Clear simulated degradation
    let clear_body = json!({
        "component": "ai",
        "degraded": false
    });

    let clear_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/setup/simulate-degradation")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&clear_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(clear_res.status(), StatusCode::OK);

    // 4. Verify degradation cleared
    let restored_status_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/setup/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let restored_bytes = restored_status_res
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let restored_json: Value = serde_json::from_slice(&restored_bytes).unwrap();
    assert!(restored_json["degraded_components"]
        .as_array()
        .unwrap()
        .is_empty());
}
