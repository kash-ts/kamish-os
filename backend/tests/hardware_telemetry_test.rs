//! Stage 8 Hardware Telemetry & Audio Output Interface Integration Tests.
//!
//! CRITICAL RULE: In all test files, ALWAYS invoke `kamish_core::audio::alert::set_audio_muted(true);`.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use kamish_core::audio::alert::set_audio_muted;
use kamish_core::audio::{AudioOutputManager, AudioRoute, DEFAULT_SAFE_VOLUME_CEILING};
use kamish_core::build_app;
use kamish_core::config::Config;
use kamish_core::system::hardware::{
    detect_board_model_from_sources, detect_respeaker_with_sources, is_cpu_throttled,
    parse_asound_cards, parse_sysfs_temperature, read_cpu_freq_mhz_from_path,
    read_cpu_temperature_from_path,
};
use serde_json::{json, Value};
use std::fs;
use tower::ServiceExt;

#[tokio::test]
async fn test_sysfs_temperature_parser() {
    set_audio_muted(true);

    // Millidegrees standard format on Linux
    assert_eq!(parse_sysfs_temperature("45000\n"), Some(45.0));
    assert_eq!(parse_sysfs_temperature("68250"), Some(68.25));

    // Direct degrees format
    assert_eq!(parse_sysfs_temperature("52\n"), Some(52.0));

    // Invalid formats
    assert_eq!(parse_sysfs_temperature(""), None);
    assert_eq!(parse_sysfs_temperature("abc"), None);
    assert_eq!(parse_sysfs_temperature("-200"), None);
    assert_eq!(parse_sysfs_temperature("9999999"), None);

    // Test with temporary mock sysfs file
    let temp_file = std::env::temp_dir().join(format!("mock_thermal_{}.txt", uuid::Uuid::new_v4()));
    fs::write(&temp_file, "55400\n").unwrap();
    let read_temp = read_cpu_temperature_from_path(&temp_file);
    let _ = fs::remove_file(&temp_file);

    assert_eq!(read_temp, Some(55.4));
}

#[tokio::test]
async fn test_cpu_throttling_check() {
    set_audio_muted(true);

    assert!(!is_cpu_throttled(Some(45.0)));
    assert!(!is_cpu_throttled(Some(79.9)));
    assert!(is_cpu_throttled(Some(80.0)));
    assert!(is_cpu_throttled(Some(85.5)));
    assert!(!is_cpu_throttled(None));
}

#[tokio::test]
async fn test_cpu_freq_parser() {
    set_audio_muted(true);

    let temp_freq = std::env::temp_dir().join(format!("mock_freq_{}.txt", uuid::Uuid::new_v4()));
    fs::write(&temp_freq, "1512000\n").unwrap(); // 1.512 GHz in kHz
    let freq_mhz = read_cpu_freq_mhz_from_path(&temp_freq);
    let _ = fs::remove_file(&temp_freq);

    assert_eq!(freq_mhz, Some(1512));
}

#[tokio::test]
async fn test_respeaker_detector_parser() {
    set_audio_muted(true);

    // 1. Asound cards content containing ReSpeaker XVF3800
    let mock_asound = r#"
 0 [H618HDMI       ]: H618HDMI - H618HDMI
                      H618HDMI
 1 [ArrayUAC10     ]: USB-Audio - ReSpeaker XVF3800 4-Mic Array
                      Seeed Technology Co.,Ltd ReSpeaker XVF3800 4-Mic Array at usb-sunxi-ehci-1
"#;
    let card = parse_asound_cards(mock_asound);
    assert!(card.is_some());
    assert!(card.unwrap().contains("ReSpeaker"));

    // Generic asound without ReSpeaker
    let generic_asound = r#"
 0 [PCH            ]: HDA-Intel - HDA Intel PCH
                      HDA Intel PCH at 0xdf320000 irq 145
"#;
    assert_eq!(parse_asound_cards(generic_asound), None);

    // 2. Sysfs USB detector with mock directory
    let temp_usb_dir = std::env::temp_dir().join(format!("mock_usb_{}", uuid::Uuid::new_v4()));
    let dev_dir = temp_usb_dir.join("1-1");
    fs::create_dir_all(&dev_dir).unwrap();
    fs::write(dev_dir.join("idVendor"), "2886\n").unwrap();
    fs::write(dev_dir.join("idProduct"), "0018\n").unwrap();

    let info = detect_respeaker_with_sources(Some(mock_asound), &temp_usb_dir);
    let _ = fs::remove_dir_all(&temp_usb_dir);

    assert!(info.detected);
    assert_eq!(info.usb_vendor_id.as_deref(), Some("2886"));
    assert_eq!(info.usb_product_id.as_deref(), Some("0018"));
    assert_eq!(info.channels, 4);
}

#[tokio::test]
async fn test_board_model_detector() {
    set_audio_muted(true);

    // Device tree model
    let dt = "Orange Pi Zero 3\0";
    let model = detect_board_model_from_sources(Some(dt), None);
    assert_eq!(model, "Orange Pi Zero 3");

    // Release file
    let release = "BOARD=orangepizero3\nBOARD_NAME=\"Orange Pi Zero 3\"\nVERSION=24.2.1\n";
    let model2 = detect_board_model_from_sources(None, Some(release));
    assert_eq!(model2, "Orange Pi Zero 3");

    // Fallback
    let fallback = detect_board_model_from_sources(None, None);
    assert!(fallback.contains("Generic Linux"));
}

#[tokio::test]
async fn test_audio_output_manager_safe_volume_ceiling() {
    set_audio_muted(true);

    let mgr = AudioOutputManager::new();

    // Default configuration
    assert_eq!(mgr.get_volume(), 50);
    assert_eq!(mgr.get_safe_ceiling(), DEFAULT_SAFE_VOLUME_CEILING);
    assert_eq!(mgr.get_route().await, AudioRoute::Gt112Default);

    // Requesting volume below ceiling
    let applied = mgr.set_volume(70);
    assert_eq!(applied, 70);
    assert_eq!(mgr.get_volume(), 70);

    // Requesting volume ABOVE ceiling (95 > 85)
    let applied_clamped = mgr.set_volume(95);
    assert_eq!(applied_clamped, 85);
    assert_eq!(mgr.get_volume(), 85);

    // Lowering safe ceiling to 60 clamps active volume down to 60
    mgr.set_safe_ceiling(60);
    assert_eq!(mgr.get_safe_ceiling(), 60);
    assert_eq!(mgr.get_volume(), 60);

    // Raising ceiling back to 85 retains existing volume (doesn't arbitrarily jump)
    mgr.set_safe_ceiling(85);
    assert_eq!(mgr.get_safe_ceiling(), 85);
    assert_eq!(mgr.get_volume(), 60);

    // Route switching
    mgr.set_route(AudioRoute::BluetoothSink).await;
    assert_eq!(mgr.get_route().await, AudioRoute::BluetoothSink);
    assert!(mgr.active_device_name().await.contains("Bluetooth"));

    mgr.set_route(AudioRoute::Gt112Default).await;
    assert_eq!(mgr.get_route().await, AudioRoute::Gt112Default);
    assert!(mgr.active_device_name().await.contains("GT-112"));

    // Routing info snapshot
    let routing = mgr.get_routing_info().await;
    assert_eq!(routing.volume, 60);
    assert_eq!(routing.safe_volume_ceiling, 85);
}

#[tokio::test]
async fn test_hardware_telemetry_endpoint() {
    set_audio_muted(true);

    let mut config = Config::from_env();
    config.database_url = "sqlite::memory:".to_string();

    let (app, _state) = build_app(&config).await.expect("Failed to build app");

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/system/hardware")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert!(!json["model"].as_str().unwrap().is_empty());
    assert!(!json["cpu_arch"].as_str().unwrap().is_empty());
    assert!(json["cpu_cores"].as_u64().unwrap() > 0);
    assert!(json["memory_total_bytes"].as_u64().unwrap() > 0);
    assert!(json["audio_routing"]["output_device"]
        .as_str()
        .unwrap()
        .contains("GT-112"));
    assert_eq!(json["audio_routing"]["safe_volume_ceiling"], 85);
}

#[tokio::test]
async fn test_hardware_audio_controls_and_ceiling() {
    set_audio_muted(true);

    let mut config = Config::from_env();
    config.database_url = "sqlite::memory:".to_string();

    let (app, _state) = build_app(&config).await.expect("Failed to build app");

    // 1. Set volume via API
    let vol_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/system/audio/volume")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "volume": 75 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(vol_res.status(), StatusCode::OK);
    let vol_body = vol_res.into_body().collect().await.unwrap().to_bytes();
    let vol_json: Value = serde_json::from_slice(&vol_body).unwrap();
    assert_eq!(vol_json["applied_volume"], 75);

    // 2. Volume exceeding safe ceiling is clamped
    let clamp_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/system/audio/volume")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "volume": 100 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(clamp_res.status(), StatusCode::OK);
    let clamp_body = clamp_res.into_body().collect().await.unwrap().to_bytes();
    let clamp_json: Value = serde_json::from_slice(&clamp_body).unwrap();
    assert_eq!(clamp_json["applied_volume"], 85);

    // 3. Switch route via API
    let route_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/system/audio/route")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "route": "bluetooth_sink" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(route_res.status(), StatusCode::OK);
    let route_body = route_res.into_body().collect().await.unwrap().to_bytes();
    let route_json: Value = serde_json::from_slice(&route_body).unwrap();
    assert_eq!(route_json["route"], "bluetooth_sink");
    assert!(route_json["description"]
        .as_str()
        .unwrap()
        .contains("Bluetooth"));

    // 4. Set safe ceiling via API
    let ceil_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/system/audio/ceiling")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "ceiling": 80 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ceil_res.status(), StatusCode::OK);
    let ceil_body = ceil_res.into_body().collect().await.unwrap().to_bytes();
    let ceil_json: Value = serde_json::from_slice(&ceil_body).unwrap();
    assert_eq!(ceil_json["safe_ceiling"], 80);

    // 5. Invalid ceiling (0 or >100) returns 400 Bad Request
    let err_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/system/audio/ceiling")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "ceiling": 0 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(err_res.status(), StatusCode::BAD_REQUEST);

    // 6. Verify status endpoint includes hardware telemetry
    let status_res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status_res.status(), StatusCode::OK);
    let status_body = status_res.into_body().collect().await.unwrap().to_bytes();
    let status_json: Value = serde_json::from_slice(&status_body).unwrap();
    assert!(status_json["hardware"]["model"].is_string());
    assert_eq!(
        status_json["hardware"]["audio_routing"]["safe_volume_ceiling"],
        80
    );
}
