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
    config.database_url = format!("sqlite::memory:");
    build_app(&config).await.expect("Failed to build test app")
}

#[tokio::test]
async fn test_create_and_list_alarms() {
    let (app, _) = test_app().await;

    // 1. Create alarm
    let req_body = json!({
        "time": "07:30",
        "recurrence": "weekdays",
        "label": "Morning Standup",
        "timezone": "Europe/Moscow",
        "enabled": true
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/alarms")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let created: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(created["time"], "07:30");
    assert_eq!(created["recurrence"], "weekdays");
    assert_eq!(created["label"], "Morning Standup");
    assert_eq!(created["timezone"], "Europe/Moscow");
    assert_eq!(created["enabled"], true);
    let alarm_id = created["id"].as_str().unwrap().to_string();

    // 2. List alarms
    let list_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/alarms")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_res.status(), StatusCode::OK);
    let list_bytes = list_res.into_body().collect().await.unwrap().to_bytes();
    let list: Vec<Value> = serde_json::from_slice(&list_bytes).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["id"], alarm_id);

    // 3. Toggle alarm
    let toggle_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/alarms/{alarm_id}/toggle"))
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(toggle_res.status(), StatusCode::OK);
    let toggle_bytes = toggle_res.into_body().collect().await.unwrap().to_bytes();
    let toggled: Value = serde_json::from_slice(&toggle_bytes).unwrap();
    assert_eq!(toggled["enabled"], false);

    // 4. Delete alarm
    let del_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/alarms/{alarm_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(del_res.status(), StatusCode::OK);

    // 5. Verify deleted from list
    let list_after_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/alarms")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let list_after_bytes = list_after_res
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let list_after: Vec<Value> = serde_json::from_slice(&list_after_bytes).unwrap();
    assert_eq!(list_after.len(), 0);
}

#[tokio::test]
async fn test_alarm_validation_errors() {
    let (app, _) = test_app().await;

    // Invalid time
    let bad_time_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/alarms")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"time": "25:70"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_time_res.status(), StatusCode::BAD_REQUEST);

    // Invalid recurrence
    let bad_rec_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/alarms")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"time": "08:00", "recurrence": "every_month"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_rec_res.status(), StatusCode::BAD_REQUEST);

    // Invalid timezone
    let bad_tz_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/alarms")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"time": "08:00", "timezone": "NonExistent/City"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_tz_res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_alarm_not_found() {
    let (app, _) = test_app().await;

    // Delete non-existent
    let del_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/alarms/non-existent-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_res.status(), StatusCode::NOT_FOUND);

    // Toggle non-existent
    let toggle_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/alarms/non-existent-id/toggle")
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(toggle_res.status(), StatusCode::NOT_FOUND);
}
