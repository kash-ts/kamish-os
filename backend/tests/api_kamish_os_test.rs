use axum::http::StatusCode;
use http_body_util::BodyExt;
use kamish_core::{build_app, config::Config};
use serde_json::Value;
use tower::ServiceExt;

async fn setup_test_app() -> axum::Router {
    kamish_core::audio::alert::set_audio_muted(true);
    let mut config = Config::from_env();
    config.database_url = "sqlite::memory:".to_string();
    let (app, _state) = build_app(&config)
        .await
        .expect("Failed to build app with in-memory database");
    app
}

#[tokio::test]
async fn test_update_status_endpoint() {
    let app = setup_test_app().await;

    let req = axum::http::Request::builder()
        .method("GET")
        .uri("/api/v1/system/update/status")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json.get("active_slot").is_some());
    assert!(json.get("boot_primary").is_some());
    assert!(json.get("installed_version").is_some());
    assert!(json.get("is_updating").is_some());
}

#[tokio::test]
async fn test_ssh_status_and_toggle_endpoint() {
    let app = setup_test_app().await;

    // 1. Initial Status check
    let req = axum::http::Request::builder()
        .method("GET")
        .uri("/api/v1/system/ssh/status")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let status_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(status_json["user"], "kamish");
    assert_eq!(status_json["root_login_allowed"], false);

    // 2. Enable SSH
    let enable_payload = serde_json::json!({
        "enable": true,
        "ssh_key": null
    });

    let req_enable = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/system/ssh/toggle")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&enable_payload).unwrap(),
        ))
        .unwrap();

    let res_enable = app.clone().oneshot(req_enable).await.unwrap();
    assert_eq!(res_enable.status(), StatusCode::OK);

    let body_enable = res_enable.into_body().collect().await.unwrap().to_bytes();
    let enable_json: Value = serde_json::from_slice(&body_enable).unwrap();
    assert_eq!(enable_json["enabled"], true);
    assert_eq!(enable_json["user"], "kamish");
    assert!(enable_json["temp_password"].is_string());
    let temp_pass = enable_json["temp_password"].as_str().unwrap();
    assert_eq!(temp_pass.len(), 12);

    // 3. Disable SSH
    let disable_payload = serde_json::json!({
        "enable": false
    });

    let req_disable = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/system/ssh/toggle")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&disable_payload).unwrap(),
        ))
        .unwrap();

    let res_disable = app.oneshot(req_disable).await.unwrap();
    assert_eq!(res_disable.status(), StatusCode::OK);

    let body_disable = res_disable.into_body().collect().await.unwrap().to_bytes();
    let disable_json: Value = serde_json::from_slice(&body_disable).unwrap();
    assert_eq!(disable_json["enabled"], false);
    assert!(disable_json["temp_password"].is_null());
}

#[tokio::test]
async fn test_update_install_endpoint() {
    let app = setup_test_app().await;

    let payload = serde_json::json!({
        "bundle_path": "/tmp/test_firmware.raucb"
    });

    let req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/system/update/install")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&payload).unwrap(),
        ))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "installing");
    assert!(json["target_slot"] == "A" || json["target_slot"] == "B");
}
