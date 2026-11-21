//! Hardware Power Button (PEK / gpio-keys) monitoring and multi-click gesture engine.
//!
//! Universal for Linux / Buildroot embedded systems:
//! Automatically detects the input event node via `/proc/bus/input/devices` or `/sys/class/input/`
//! without hardcoding device numbers (supports AXP8191/AXP2101 PEK, gpio-keys, etc.).
//!
//! Gestures:
//! - 1 short click: Graceful system shutdown (systemctl poweroff)
//! - 2 clicks: Reserved / Audio Mute toggle
//! - 3 clicks in quick succession: System Factory Reset (clears user DBs/configs & reboots)

use crate::common::CoreState;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::time::Instant;
use tracing::{error, info, warn};

/// Standard Linux input event codes
const EV_KEY: u16 = 1;
const KEY_POWER: u16 = 116;

/// Raw Linux input_event structure (24 bytes on 64-bit systems)
#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct LinuxInputEvent {
    tv_sec: i64,
    tv_usec: i64,
    event_type: u16,
    event_code: u16,
    event_value: i32,
}

impl LinuxInputEvent {
    fn from_bytes(buf: &[u8]) -> Option<Self> {
        if buf.len() < 24 {
            return None;
        }
        let tv_sec = i64::from_ne_bytes(buf[0..8].try_into().ok()?);
        let tv_usec = i64::from_ne_bytes(buf[8..16].try_into().ok()?);
        let event_type = u16::from_ne_bytes(buf[16..18].try_into().ok()?);
        let event_code = u16::from_ne_bytes(buf[18..20].try_into().ok()?);
        let event_value = i32::from_ne_bytes(buf[20..24].try_into().ok()?);
        Some(Self {
            tv_sec,
            tv_usec,
            event_type,
            event_code,
            event_value,
        })
    }
}

/// Dynamically locates the power key event node across any Buildroot / Linux distribution.
pub fn find_power_button_device() -> Option<PathBuf> {
    #[cfg(not(target_os = "linux"))]
    {
        None
    }

    #[cfg(target_os = "linux")]
    {
        // 1. Check /proc/bus/input/devices (most portable across Buildroot/Debian/Yocto)
        if let Ok(devices_content) = fs::read_to_string("/proc/bus/input/devices") {
            let mut current_name = String::new();
            for line in devices_content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("N: Name=") {
                    current_name = trimmed.trim_start_matches("N: Name=").replace('"', "").to_lowercase();
                } else if trimmed.starts_with("H: Handlers=") {
                    let handlers = trimmed.trim_start_matches("H: Handlers=");
                    let is_power_dev = current_name.contains("pek")
                        || current_name.contains("power")
                        || current_name.contains("pwr")
                        || current_name.contains("gpio-keys")
                        || current_name.contains("button");

                    if is_power_dev {
                        for part in handlers.split_whitespace() {
                            if part.starts_with("event") {
                                let path = PathBuf::from(format!("/dev/input/{}", part));
                                if path.exists() {
                                    info!(
                                        "Detected power button device via /proc/bus/input/devices: {} ({})",
                                        path.display(),
                                        current_name
                                    );
                                    return Some(path);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Scan /sys/class/input/event*/device/name
        if let Ok(entries) = fs::read_dir("/sys/class/input") {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.starts_with("event") {
                    let name_path = entry.path().join("device/name");
                    if let Ok(dev_name) = fs::read_to_string(&name_path) {
                        let dev_name_lower = dev_name.to_lowercase();
                        if dev_name_lower.contains("pek")
                            || dev_name_lower.contains("power")
                            || dev_name_lower.contains("pwr")
                            || dev_name_lower.contains("gpio-keys")
                        {
                            let node = PathBuf::from(format!("/dev/input/{}", file_name));
                            if node.exists() {
                                info!(
                                    "Detected power button device via sysfs: {} ({})",
                                    node.display(),
                                    dev_name.trim()
                                );
                                return Some(node);
                            }
                        }
                    }
                }
            }
        }

        // 3. Fallback to /dev/input/event0 if it exists
        let fallback = PathBuf::from("/dev/input/event0");
        if fallback.exists() {
            warn!("Falling back to default input event node: /dev/input/event0");
            return Some(fallback);
        }

        None
    }
}

/// Spawns the background hardware power button listener task.
pub fn start_power_button_monitor(state: CoreState) {
    tokio::spawn(async move {
        run_monitor_loop(state).await;
    });
}

async fn run_monitor_loop(state: CoreState) {
    info!("Starting Kamish hardware power button monitor service...");

    #[cfg(not(target_os = "linux"))]
    {
        info!("Power button monitor: disabled on non-Linux platform.");
        return;
    }

    #[cfg(target_os = "linux")]
    {
        use tokio::io::AsyncReadExt;

        let click_window = Duration::from_millis(700);

        loop {
            let dev_path = match find_power_button_device() {
                Some(p) => p,
                None => {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            info!("Monitoring power key events from: {}", dev_path.display());
            let mut file = match tokio::fs::File::open(&dev_path).await {
                Ok(f) => f,
                Err(e) => {
                    error!("Failed to open {}: {}. Retrying in 5s...", dev_path.display(), e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            let mut press_start: Option<Instant> = None;
            let mut click_count: u32 = 0;
            let mut buf = [0u8; 24];

            let window_sleep = tokio::time::sleep(Duration::from_secs(3600 * 24));
            tokio::pin!(window_sleep);

            loop {
                tokio::select! {
                    read_res = file.read_exact(&mut buf) => {
                        match read_res {
                            Ok(_) => {
                                if let Some(event) = LinuxInputEvent::from_bytes(&buf) {
                                    if event.event_type == EV_KEY && event.event_code == KEY_POWER {
                                        match event.event_value {
                                            1 => {
                                                // Key Press
                                                press_start = Some(Instant::now());
                                            }
                                            0 => {
                                                // Key Release
                                                if let Some(start) = press_start.take() {
                                                    let duration = start.elapsed();
                                                    if duration < Duration::from_millis(600) {
                                                        click_count += 1;
                                                        info!("Power button clicked (#{}, duration: {:?})", click_count, duration);
                                                        window_sleep.as_mut().reset(Instant::now() + click_window);
                                                    } else if duration >= Duration::from_secs(2) && duration < Duration::from_secs(7) {
                                                        info!("Power button long press detected: {:?}", duration);
                                                        state.events.emit("power_button_long_press", json!({
                                                            "duration_ms": duration.as_millis()
                                                        }));
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Error reading from {}: {}. Reconnecting...", dev_path.display(), e);
                                break;
                            }
                        }
                    }
                    _ = &mut window_sleep, if click_count > 0 => {
                        let final_clicks = click_count;
                        click_count = 0;
                        window_sleep.as_mut().reset(Instant::now() + Duration::from_secs(3600 * 24));

                        handle_gesture(&state, final_clicks).await;
                    }
                }
            }
        }
    }
}

/// Dispatches actions based on number of consecutive short clicks
async fn handle_gesture(state: &CoreState, clicks: u32) {
    match clicks {
        1 => {
            info!("Power Button Action: 1 click -> Graceful system shutdown");
            state.events.emit(
                "power_button_action",
                json!({
                    "action": "shutdown",
                    "clicks": 1,
                    "message": "Shutting down system gracefully"
                }),
            );
            execute_shutdown().await;
        }
        2 => {
            info!("Power Button Action: 2 clicks -> Toggle Mute / Status Announcement");
            state.events.emit(
                "power_button_action",
                json!({
                    "action": "toggle_mute",
                    "clicks": 2
                }),
            );
        }
        _ if clicks >= 3 => {
            info!(
                "Power Button Action: {} clicks -> FACTORY RESET TRIGGERED!",
                clicks
            );
            state.events.emit(
                "power_button_action",
                json!({
                    "action": "factory_reset",
                    "clicks": clicks,
                    "message": "Resetting Kamish OS to factory settings"
                }),
            );
            execute_factory_reset().await;
        }
        _ => {}
    }
}

/// Executes clean system poweroff
async fn execute_shutdown() {
    info!("Executing graceful system shutdown via systemctl poweroff...");
    let _ = tokio::process::Command::new("systemctl")
        .arg("poweroff")
        .status()
        .await;
}

/// Executes factory reset: cleans state databases and reboots into clean setup mode
async fn execute_factory_reset() {
    info!("Performing Kamish OS Factory Reset...");

    // 1. Remove user database files
    for db_path in &["/var/lib/kamish/core.db", "/var/lib/kamish/web.db"] {
        if Path::new(db_path).exists() {
            let _ = tokio::fs::remove_file(db_path).await;
            info!("Removed database file: {}", db_path);
        }
    }

    // 2. Remove saved Wi-Fi profiles if exists in overlay
    let wifi_conf = Path::new("/var/lib/kamish/wifi_credentials.json");
    if wifi_conf.exists() {
        let _ = tokio::fs::remove_file(wifi_conf).await;
        info!("Removed saved Wi-Fi credentials");
    }

    // 3. Sync disks and reboot
    info!("Syncing filesystem and rebooting into factory setup...");
    let _ = tokio::process::Command::new("sync").status().await;
    let _ = tokio::process::Command::new("systemctl")
        .arg("reboot")
        .status()
        .await;
}
