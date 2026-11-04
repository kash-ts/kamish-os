use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use kamish_core::{audio::alert::set_audio_muted, build_app, config::Config};
use serde_json::{json, Value};
use tokio_tungstenite::connect_async;
use tower::ServiceExt;

async fn setup_app() -> (axum::Router, kamish_core::common::AppState) {
    set_audio_muted(true);
    let mut config = Config::from_env();
    config.database_url = "sqlite::memory:?cache=shared".to_string();

    let (app, state) = build_app(&config)
        .await
        .expect("Failed to build app for system integration tests");

    (app, state)
}

// ==========================================
// 1. BLUETOOTH A2DP SINK TESTS
// ==========================================

#[tokio::test]
async fn test_bluetooth_status_endpoint() {
    set_audio_muted(true);
    let (app, _) = setup_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/bluetooth/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["adapter_enabled"].is_boolean());
    assert!(json["discoverable"].is_boolean());
    assert!(json["paired_devices"].is_array());
}

#[tokio::test]
async fn test_bluetooth_scan_endpoint() {
    set_audio_muted(true);
    let (app, _) = setup_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bluetooth/scan")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "duration_secs": 1 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"].as_str().unwrap(), "scanning_complete");
    assert!(json["devices_found"].is_array());
}

#[tokio::test]
async fn test_bluetooth_pair_and_connect_lifecycle() {
    set_audio_muted(true);
    let (app, _) = setup_app().await;

    // 1. Pair new device
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bluetooth/pair")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "mac": "AC:37:43:88:19:22" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"].as_str().unwrap(), "paired");
    assert_eq!(json["mac"].as_str().unwrap(), "AC:37:43:88:19:22");

    // 2. Connect to device
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bluetooth/connect")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "mac": "AC:37:43:88:19:22" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"].as_str().unwrap(), "connected");
    assert_eq!(json["profile"].as_str().unwrap(), "A2DP Sink");

    // 3. Disconnect device
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bluetooth/disconnect")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"].as_str().unwrap(), "disconnected");

    // 4. Set discoverable
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bluetooth/discoverable")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "discoverable": false, "timeout_secs": 60 }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"].as_str().unwrap(), "ok");
    assert_eq!(json["discoverable"].as_bool().unwrap(), false);
}

// ==========================================
// 2. NETWORK SUBSYSTEM TESTS
// ==========================================

#[tokio::test]
async fn test_network_status_endpoint() {
    set_audio_muted(true);
    let (app, _) = setup_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/network/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(!json["primary_interface"].as_str().unwrap().is_empty());
    assert!(json["wifi"].is_object());
    assert_eq!(json["ap_mode"]["active"].as_bool().unwrap(), false);
}

#[tokio::test]
async fn test_network_wifi_scan_endpoint() {
    set_audio_muted(true);
    let (app, _) = setup_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/network/wifi/scan")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    let count = json["count"].as_u64().unwrap();
    assert_eq!(count, json["networks"].as_array().unwrap().len() as u64);
}

#[tokio::test]
async fn test_network_wifi_connect_and_disconnect() {
    set_audio_muted(true);
    let (app, _) = setup_app().await;

    // Connect successfully
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/network/wifi/connect")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "ssid": "Office_Guest",
                        "password": "ValidSecretPassword"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"].as_str().unwrap(), "connected");
    assert_eq!(json["ssid"].as_str().unwrap(), "Office_Guest");

    // Authentication failure test
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/network/wifi/connect")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "ssid": "SecureEnterprise",
                        "password": "wrongpassword"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"].as_str().unwrap(), "NETWORK_AUTH_ERROR");

    // Disconnect
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/network/wifi/disconnect")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"].as_str().unwrap(), "disconnected");
}

#[tokio::test]
async fn test_network_ap_mode_lifecycle() {
    set_audio_muted(true);
    let (app, _) = setup_app().await;

    // Start AP mode
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/network/ap/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"].as_str().unwrap(), "ap_started");
    assert_eq!(json["ssid"].as_str().unwrap(), "Kamish-Setup");
    assert_eq!(json["gateway_ip"].as_str().unwrap(), "192.168.4.1");

    // Check status in AP mode
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/network/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["ap_mode"]["active"].as_bool().unwrap());
    assert_eq!(json["ip_address"].as_str().unwrap(), "192.168.4.1");

    // Stop AP mode
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/network/ap/stop")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"].as_str().unwrap(), "ap_stopped");
}

// ==========================================
// 3. VPN / VLESS SUBSYSTEM TESTS
// ==========================================

#[tokio::test]
async fn test_vpn_status_endpoint() {
    set_audio_muted(true);
    let (app, _) = setup_app().await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/vpn/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(json["enabled"].as_bool().unwrap());
    assert_eq!(json["status"].as_str().unwrap(), "connected");
    assert_eq!(json["protocol"].as_str().unwrap(), "VLESS-Reality");
    assert_eq!(
        json["server_address"].as_str().unwrap(),
        "nl-ams-01.tunnel.net:443"
    );
    assert_eq!(json["ping_ms"].as_u64().unwrap(), 48);
    assert!(json["bytes_sent"].as_u64().unwrap() > 0);
    assert!(json["bytes_received"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_vpn_connect_and_disconnect() {
    set_audio_muted(true);
    let (app, _) = setup_app().await;

    // Disconnect
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vpn/disconnect")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"].as_str().unwrap(), "disconnected");

    // Connect
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vpn/connect")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"].as_str().unwrap(), "connected");
    assert_eq!(json["ping_ms"].as_u64().unwrap(), 48);
}

#[tokio::test]
async fn test_vpn_config_vless_uri_parsing_and_security() {
    set_audio_muted(true);
    let (app, _) = setup_app().await;

    // Valid VLESS URI
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vpn/config")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "vless_uri": "vless://b8313e9a-7a1f-4b08@amsterdam.relay.com:443?security=reality&sni=gateway.microsoft.com&fp=chrome#Netherlands-01"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"].as_str().unwrap(), "configured");
    assert_eq!(json["server"].as_str().unwrap(), "amsterdam.relay.com:443");
    assert_eq!(json["sni"].as_str().unwrap(), "gateway.microsoft.com");

    // Invalid VLESS URI
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/vpn/config")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "vless_uri": "invalid_scheme://no_credentials"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"].as_str().unwrap(), "CONFIG_ERROR");
}

// ==========================================
// 4. WEBSOCKET REAL-TIME EVENTS TESTS
// ==========================================

#[tokio::test]
async fn test_websocket_realtime_events_broadcasting() {
    set_audio_muted(true);
    let mut config = Config::from_env();
    config.database_url = "sqlite::memory:?cache=shared".to_string();

    let (router, state) = build_app(&config)
        .await
        .expect("Failed to build app for WS test");

    // Bind real TCP listener to test full WebSocket upgrade over the wire
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind TCP listener for test");
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    let ws_url = format!("ws://{}/api/v1/events", addr);
    let (mut ws_stream, _) = connect_async(&ws_url)
        .await
        .expect("Failed to connect to WebSocket events endpoint");

    // 1. Emit Bluetooth state change and verify receipt
    state.bluetooth.connect("FC:58:FA:12:34:56").await.unwrap();

    let msg = tokio::time::timeout(std::time::Duration::from_secs(3), ws_stream.next())
        .await
        .expect("Timeout waiting for WebSocket message")
        .expect("Stream closed unexpectedly")
        .expect("WebSocket error");

    let text = msg.to_text().expect("Expected text message");
    let event: Value = serde_json::from_str(text).unwrap();
    assert_eq!(event["event"].as_str().unwrap(), "bluetooth_state");
    assert!(!event["payload"]["device_name"].as_str().unwrap().is_empty());

    // 2. Emit Volume changed and verify receipt
    state.media.set_volume(82, None).await;

    let msg = tokio::time::timeout(std::time::Duration::from_secs(3), ws_stream.next())
        .await
        .expect("Timeout waiting for volume message")
        .expect("Stream closed unexpectedly")
        .expect("WebSocket error");

    let text = msg.to_text().expect("Expected text message");
    let event: Value = serde_json::from_str(text).unwrap();
    assert_eq!(event["event"].as_str().unwrap(), "volume_changed");
    assert_eq!(event["payload"]["volume"].as_u64().unwrap(), 82);

    // 3. Test ping/pong message
    ws_stream
        .send(tokio_tungstenite::tungstenite::Message::Ping(
            "test_ping".as_bytes().to_vec().into(),
        ))
        .await
        .expect("Failed to send ping");

    // Close socket cleanly
    let _ = ws_stream.close(None).await;
}
