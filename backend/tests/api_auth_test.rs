use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use http_body_util::BodyExt;
use kamish_core::{audio::alert::set_audio_muted, build_app, config::Config};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn test_auth_app() -> (axum::Router, kamish_core::common::AppState) {
    set_audio_muted(true);
    let mut config = Config::from_env();
    config.database_url = format!("sqlite::memory:");
    let (app, state) = build_app(&config).await.expect("Failed to build test app");

    // Explicitly enable auth for this test suite
    state.auth.set_auth_enabled(true);
    (app, state)
}

#[tokio::test]
async fn test_login_invalid_credentials_returns_401() {
    let (app, _) = test_auth_app().await;

    let req_body = json!({
        "username": "admin",
        "password": "wrong_password"
    });

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["code"], "UNAUTHORIZED");
}

#[tokio::test]
async fn test_default_admin_login_success_must_change_password() {
    let (app, _) = test_auth_app().await;

    let req_body = json!({
        "username": "admin",
        "password": "admin"
    });

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&req_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    // Verify Set-Cookie header exists
    let cookies: Vec<_> = res.headers().get_all(header::SET_COOKIE).iter().collect();
    assert!(!cookies.is_empty(), "Expected Set-Cookie header");
    let cookie_str = cookies[0].to_str().unwrap();
    assert!(cookie_str.contains("kamish_session=") || cookie_str.contains("session_token="));

    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body["token"].is_string());
    assert_eq!(body["user"]["username"], "admin");
    assert_eq!(body["user"]["must_change_password"], true);
}

#[tokio::test]
async fn test_protected_routes_blocked_when_unauthenticated() {
    let (app, _) = test_auth_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/alarms")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["code"], "UNAUTHORIZED");
}

#[tokio::test]
async fn test_protected_routes_blocked_when_must_change_password_true() {
    let (app, _) = test_auth_app().await;

    // Login as default admin
    let login_body = json!({
        "username": "admin",
        "password": "admin"
    });

    let login_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body_bytes = login_res.into_body().collect().await.unwrap().to_bytes();
    let login_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    let token = login_json["token"].as_str().unwrap();

    // Try accessing protected resource (/api/v1/alarms)
    let alarms_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/alarms")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(alarms_res.status(), StatusCode::FORBIDDEN);
    let alarms_bytes = alarms_res.into_body().collect().await.unwrap().to_bytes();
    let alarms_json: Value = serde_json::from_slice(&alarms_bytes).unwrap();
    assert_eq!(alarms_json["code"], "PASSWORD_CHANGE_REQUIRED");

    // But auth endpoints (/api/v1/auth/session) must remain accessible
    let session_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/session")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(session_res.status(), StatusCode::OK);
    let session_bytes = session_res.into_body().collect().await.unwrap().to_bytes();
    let session_json: Value = serde_json::from_slice(&session_bytes).unwrap();
    assert_eq!(session_json["authenticated"], true);
    assert_eq!(session_json["user"]["must_change_password"], true);
}

#[tokio::test]
async fn test_status_endpoint_whitelisted() {
    let (app, _) = test_auth_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(body["status"].is_string());
}

#[tokio::test]
async fn test_change_password_validation_error() {
    let (app, _) = test_auth_app().await;

    // Login
    let login_body = json!({"username": "admin", "password": "admin"});
    let login_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body_bytes = login_res.into_body().collect().await.unwrap().to_bytes();
    let token = serde_json::from_slice::<Value>(&body_bytes).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string();

    // Password too short (< 4 chars)
    let change_body = json!({
        "old_password": "admin",
        "new_password": "12"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/change-password")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&change_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let res_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let res_json: Value = serde_json::from_slice(&res_bytes).unwrap();
    assert_eq!(res_json["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn test_change_password_wrong_old_password() {
    let (app, _) = test_auth_app().await;

    let login_body = json!({"username": "admin", "password": "admin"});
    let login_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body_bytes = login_res.into_body().collect().await.unwrap().to_bytes();
    let token = serde_json::from_slice::<Value>(&body_bytes).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string();

    let change_body = json!({
        "old_password": "wrong_old_password",
        "new_password": "new_secure_password"
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/change-password")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&change_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let res_bytes = res.into_body().collect().await.unwrap().to_bytes();
    let res_json: Value = serde_json::from_slice(&res_bytes).unwrap();
    assert_eq!(res_json["code"], "UNAUTHORIZED");
}

#[tokio::test]
async fn test_change_password_lifecycle_and_access_unlocked() {
    let (app, _) = test_auth_app().await;

    // 1. Login with initial admin credentials
    let login_body = json!({"username": "admin", "password": "admin"});
    let login_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body_bytes = login_res.into_body().collect().await.unwrap().to_bytes();
    let token = serde_json::from_slice::<Value>(&body_bytes).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string();

    // 2. Change password
    let change_body = json!({
        "old_password": "admin",
        "new_password": "secret_new_password_2026"
    });

    let change_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/change-password")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&change_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(change_res.status(), StatusCode::OK);
    let change_bytes = change_res.into_body().collect().await.unwrap().to_bytes();
    let change_json: Value = serde_json::from_slice(&change_bytes).unwrap();
    assert_eq!(change_json["must_change_password"], false);

    // 3. Old password "admin" should now be rejected
    let old_login_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(old_login_res.status(), StatusCode::UNAUTHORIZED);

    // 4. New password should successfully login
    let new_login_body = json!({
        "username": "admin",
        "password": "secret_new_password_2026"
    });
    let new_login_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&new_login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(new_login_res.status(), StatusCode::OK);
    let new_bytes = new_login_res
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let new_json: Value = serde_json::from_slice(&new_bytes).unwrap();
    let new_token = new_json["token"].as_str().unwrap();
    assert_eq!(new_json["user"]["must_change_password"], false);

    // 5. Protected resources (/api/v1/alarms) are now fully accessible!
    let alarms_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/alarms")
                .header("Authorization", format!("Bearer {new_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(alarms_res.status(), StatusCode::OK);

    // 6. Test cookie authentication
    let cookie_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/alarms")
                .header("Cookie", format!("kamish_session={new_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cookie_res.status(), StatusCode::OK);

    // 7. Logout invalidates session
    let logout_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header("Authorization", format!("Bearer {new_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logout_res.status(), StatusCode::OK);

    // 8. Session is now invalid
    let session_check_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/session")
                .header("Authorization", format!("Bearer {new_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(session_check_res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_test_bypass_header() {
    let (app, _) = test_auth_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/alarms")
                .header("X-Test-Bypass", "true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}
