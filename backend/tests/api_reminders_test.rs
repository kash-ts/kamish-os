use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use kamish_core::{audio::alert::set_audio_muted, build_app, config::Config};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn test_app() -> (axum::Router, kamish_core::common::AppState) {
    set_audio_muted(true);
    let mut config = Config::from_env();
    config.database_url = "sqlite::memory:".to_string();
    build_app(&config).await.expect("Failed to build test app")
}

#[tokio::test]
async fn test_create_list_and_delete_reminders() {
    let (app, _) = test_app().await;

    // 1. Create reminder
    let req_body = json!({
        "trigger_time": "2026-11-01T15:30:00Z",
        "text": "Turn off the stove",
        "timezone": "UTC"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/reminders")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let created: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(created["text"], "Turn off the stove");
    assert_eq!(created["trigger_time"], "2026-11-01T15:30:00Z");
    assert_eq!(created["status"], "pending");
    let reminder_id = created["id"].as_str().unwrap().to_string();

    // 2. List reminders (all)
    let list_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/reminders")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_res.status(), StatusCode::OK);
    let list_bytes = list_res.into_body().collect().await.unwrap().to_bytes();
    let list: Vec<Value> = serde_json::from_slice(&list_bytes).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["id"], reminder_id);

    // 3. List reminders with filter: status=pending
    let pending_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/reminders?status=pending")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(pending_res.status(), StatusCode::OK);
    let pending_bytes = pending_res.into_body().collect().await.unwrap().to_bytes();
    let pending_list: Vec<Value> = serde_json::from_slice(&pending_bytes).unwrap();
    assert_eq!(pending_list.len(), 1);

    // 4. List reminders with filter: status=completed (should be empty)
    let completed_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/reminders?status=completed")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(completed_res.status(), StatusCode::OK);
    let completed_bytes = completed_res
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let completed_list: Vec<Value> = serde_json::from_slice(&completed_bytes).unwrap();
    assert_eq!(completed_list.len(), 0);

    // 5. Delete reminder
    let del_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/reminders/{reminder_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(del_res.status(), StatusCode::OK);

    // 6. Delete again -> 404
    let del_again_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/reminders/{reminder_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_again_res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_reminder_validation_errors() {
    let (app, _) = test_app().await;

    // Empty text
    let empty_text_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/reminders")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"trigger_time": "2026-11-01T15:30:00Z", "text": "   "}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(empty_text_res.status(), StatusCode::BAD_REQUEST);

    // Invalid trigger_time format
    let bad_time_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/reminders")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"trigger_time": "tomorrow at noon", "text": "Something"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_time_res.status(), StatusCode::BAD_REQUEST);

    // Invalid timezone
    let bad_tz_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/reminders")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"trigger_time": "2026-11-01T15:30:00Z", "text": "Valid", "timezone": "Invalid/Zone"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_tz_res.status(), StatusCode::BAD_REQUEST);
}
