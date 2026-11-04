use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use kamish_core::{build_app, config::Config};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn test_status_endpoint_returns_ok() {
    kamish_core::audio::alert::set_audio_muted(true);
    let mut config = Config::from_env();
    config.database_url = "sqlite::memory:".to_string();

    let (app, _state) = build_app(&config)
        .await
        .expect("Failed to build app with in-memory database");

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).expect("Failed to parse JSON response");

    assert_eq!(json["status"], "ok");
    assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(json["database"]["connected"], true);
    assert!(json["database"]["applied_migrations"].as_i64().unwrap() >= 1);
    assert_eq!(json["subsystems"]["audio"], "ready");
    assert_eq!(json["subsystems"]["scheduler"], "ready");
    assert!(json["device"]["cpu_count"].as_u64().unwrap() > 0);
    assert!(json["device"]["memory_total_bytes"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_404_fallback() {
    kamish_core::audio::alert::set_audio_muted(true);
    let mut config = Config::from_env();
    config.database_url = "sqlite::memory:".to_string();

    let (app, _state) = build_app(&config).await.expect("Failed to build app");

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/non_existent_route")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "NOT_FOUND");
}
