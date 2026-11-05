//! Stage 8 Hardware Telemetry & Audio Output Interface Integration Tests.
//!
//! CRITICAL RULE: In all test files, ALWAYS invoke `kamish_core::audio::alert::set_audio_muted(true);`.

use kamish_core::audio::alert::set_audio_muted;
use kamish_core::audio::{AudioOutputManager, AudioRoute, DEFAULT_SAFE_VOLUME_CEILING};
use kamish_core::system::hardware::{
    detect_board_model_from_sources, detect_respeaker_with_sources, is_cpu_throttled,
    parse_asound_cards, parse_sysfs_temperature, read_cpu_freq_mhz_from_path,
    read_cpu_temperature_from_path,
};
use std::fs;

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

    assert!(!is_cpu_throttled(Some(79.9)));
    assert!(!is_cpu_throttled(Some(55.0)));
    assert!(is_cpu_throttled(Some(80.0)));
    assert!(is_cpu_throttled(Some(85.5)));
    assert!(!is_cpu_throttled(None));
}

#[tokio::test]
async fn test_respeaker_xvf3800_card_parser() {
    set_audio_muted(true);

    let sample_cards = r#"
 0 [audiocodec     ]: audiocodec - audiocodec
                      audiocodec
 1 [ArrayUAC10     ]: USB-Audio - Seeed ReSpeaker XVF3800
                      Seeed Technology Co., Ltd. Seeed ReSpeaker XVF3800 at usb-5200000.usb-1
"#;
    let card = parse_asound_cards(sample_cards);
    assert!(card.is_some());
    assert!(card.unwrap().contains("Seeed ReSpeaker XVF3800"));

    let empty_cards = " 0 [audiocodec ]: audiocodec - audiocodec\n";
    assert_eq!(parse_asound_cards(empty_cards), None);
}

#[tokio::test]
async fn test_respeaker_detection_with_sources() {
    set_audio_muted(true);

    let temp_dir = std::env::temp_dir().join(format!("mock_sysfs_usb_{}", uuid::Uuid::new_v4()));
    let dev_dir = temp_dir.join("1-1");
    fs::create_dir_all(&dev_dir).unwrap();

    fs::write(dev_dir.join("idVendor"), "2886\n").unwrap();
    fs::write(dev_dir.join("idProduct"), "0018\n").unwrap();

    let info = detect_respeaker_with_sources(None, &temp_dir);
    let _ = fs::remove_dir_all(&temp_dir);

    assert!(info.detected);
    assert_eq!(info.usb_vendor_id, Some("2886".to_string()));
    assert_eq!(info.usb_product_id, Some("0018".to_string()));
    assert_eq!(info.channels, 4);
}

#[tokio::test]
async fn test_board_model_detection() {
    set_audio_muted(true);

    let dt = Some("OrangePi 4 Pro\0");
    let model = detect_board_model_from_sources(dt, None);
    assert_eq!(model, "OrangePi 4 Pro");

    let os_release = "NAME=\"Buildroot\"\nBOARD_NAME=\"OrangePi 4 Pro\"\n";
    let model2 = detect_board_model_from_sources(None, Some(os_release));
    assert_eq!(model2, "OrangePi 4 Pro");

    let generic = detect_board_model_from_sources(None, None);
    assert!(generic.contains("Generic Linux"));
}

#[tokio::test]
async fn test_cpufreq_sysfs_parser() {
    set_audio_muted(true);

    let temp_file = std::env::temp_dir().join(format!("mock_cpufreq_{}.txt", uuid::Uuid::new_v4()));
    fs::write(&temp_file, "1512000\n").unwrap();
    let freq = read_cpu_freq_mhz_from_path(&temp_file);
    let _ = fs::remove_file(&temp_file);

    assert_eq!(freq, Some(1512));
}

#[tokio::test]
async fn test_audio_output_manager_safe_ceiling() {
    set_audio_muted(true);

    let mgr = AudioOutputManager::new();

    // Default safe volume ceiling is 85%
    assert_eq!(mgr.get_safe_ceiling(), DEFAULT_SAFE_VOLUME_CEILING);
    assert_eq!(mgr.get_volume(), 50);

    // Setting volume <= 85 succeeds exactly
    assert_eq!(mgr.set_volume(70), 70);
    assert_eq!(mgr.get_volume(), 70);

    // Setting volume > 85 is clamped to 85
    assert_eq!(mgr.set_volume(90), 85);
    assert_eq!(mgr.get_volume(), 85);

    assert_eq!(mgr.set_volume(100), 85);
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
