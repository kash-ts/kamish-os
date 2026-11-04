use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use kamish_core::{build_app, config::Config};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn test_frontend_static_serving_and_fallback() {
    kamish_core::audio::alert::set_audio_muted(true);
    let mut config = Config::from_env();
    config.database_url = "sqlite::memory:".to_string();

    let (app, _state) = build_app(&config)
        .await
        .expect("Failed to build app with in-memory database");

    // Test GET / serves index.html
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains("Kamish Control Panel") || body_str.contains("id=\"root\""));

    // Test GET /alarms (client-side routing fallback) serves index.html
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/alarms")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains("id=\"root\""));

    // Test API route remains functional
    let response = app
        .clone()
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

    // Test unknown API route returns 404 JSON, NOT index.html!
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/unknown_route")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).expect("Failed to parse JSON response");
    assert_eq!(json["code"], "NOT_FOUND");
}
