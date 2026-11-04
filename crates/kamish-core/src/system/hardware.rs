//! Hardware telemetry and peripheral detection for Orange Pi Zero 3,
//! Seeed ReSpeaker XVF3800, and audio routing.

use serde::{Deserialize, Serialize};
use std::path::Path;
use sysinfo::System;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AudioRoutingInfo {
    pub input_device: String,
    pub input_detected: bool,
    pub output_device: String,
    pub mixer_status: String,
    pub volume: u8,
    pub safe_volume_ceiling: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RespeakerInfo {
    pub detected: bool,
    pub card_name: Option<String>,
    pub usb_vendor_id: Option<String>,
    pub usb_product_id: Option<String>,
    pub channels: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HardwareTelemetry {
    pub model: String,
    pub cpu_arch: String,
    pub cpu_cores: usize,
    pub cpu_temp_celsius: Option<f32>,
    pub cpu_freq_mhz: Option<u64>,
    pub throttled: bool,
    pub memory_total_bytes: u64,
    pub memory_used_bytes: u64,
    pub storage_total_bytes: u64,
    pub storage_used_bytes: u64,
    pub storage_available_bytes: u64,
    pub respeaker: RespeakerInfo,
    pub audio_routing: AudioRoutingInfo,
}

/// Parse temperature value from sysfs content.
/// On Linux, `/sys/class/thermal/thermal_zone0/temp` contains millidegrees (e.g. "45000" = 45.0°C).
pub fn parse_sysfs_temperature(content: &str) -> Option<f32> {
    let trimmed = content.trim();
    if let Ok(milli) = trimmed.parse::<i64>() {
        if (1_000..=150_000).contains(&milli) {
            Some((milli as f32) / 1000.0)
        } else if (0..=150).contains(&milli) {
            Some(milli as f32)
        } else {
            None
        }
    } else {
        None
    }
}

/// Read CPU temperature from a specific sysfs file.
pub fn read_cpu_temperature_from_path(path: &Path) -> Option<f32> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|c| parse_sysfs_temperature(&c))
}

/// Read CPU temperature with multiple fallbacks:
/// 1. `/sys/class/thermal/thermal_zone0..4/temp`
/// 2. `/sys/class/hwmon/hwmon0..4/temp1_input`
/// 3. `sysinfo::Components`
pub fn read_cpu_temperature() -> Option<f32> {
    // 1. Sysfs thermal zones (standard on Allwinner / Linux)
    for zone in 0..5 {
        let p = format!("/sys/class/thermal/thermal_zone{zone}/temp");
        if let Some(t) = read_cpu_temperature_from_path(Path::new(&p)) {
            return Some(t);
        }
    }

    // 2. Sysfs hwmon inputs
    for hwmon in 0..5 {
        let p = format!("/sys/class/hwmon/hwmon{hwmon}/temp1_input");
        if let Some(t) = read_cpu_temperature_from_path(Path::new(&p)) {
            return Some(t);
        }
    }

    // 3. Fallback via sysinfo::Components
    let components = sysinfo::Components::new_with_refreshed_list();
    for comp in components.iter() {
        let label = comp.label().to_lowercase();
        if label.contains("cpu")
            || label.contains("core")
            || label.contains("soc")
            || label.contains("package")
        {
            if let Some(temp) = comp.temperature() {
                return Some(temp);
            }
        }
    }
    if let Some(comp) = components.first() {
        if let Some(temp) = comp.temperature() {
            return Some(temp);
        }
    }

    None
}

/// Parse `/proc/asound/cards` to find Seeed ReSpeaker XVF3800.
pub fn parse_asound_cards(content: &str) -> Option<String> {
    for line in content.lines() {
        let lower = line.to_lowercase();
        if lower.contains("respeaker")
            || lower.contains("xvf3800")
            || lower.contains("arrayuac10")
            || lower.contains("seeed")
        {
            return Some(line.trim().to_string());
        }
    }
    None
}

/// Detect Seeed USB device (Vendor ID 0x2886) in `/sys/bus/usb/devices`.
pub fn detect_respeaker_usb_in_dir(usb_dir: &Path) -> Option<(String, String)> {
    if !usb_dir.exists() {
        return None;
    }
    if let Ok(entries) = std::fs::read_dir(usb_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let vendor_path = path.join("idVendor");
            let product_path = path.join("idProduct");
            if vendor_path.exists() {
                if let Ok(vendor) = std::fs::read_to_string(&vendor_path) {
                    if vendor.trim().eq_ignore_ascii_case("2886") {
                        let product = std::fs::read_to_string(&product_path)
                            .unwrap_or_else(|_| "unknown".to_string())
                            .trim()
                            .to_string();
                        return Some((vendor.trim().to_string(), product));
                    }
                }
            }
        }
    }
    None
}

/// Detect Seeed ReSpeaker XVF3800 given optional `/proc/asound/cards` content and sysfs USB path.
pub fn detect_respeaker_with_sources(
    asound_content: Option<&str>,
    usb_dir: &Path,
) -> RespeakerInfo {
    let card_name = asound_content.and_then(parse_asound_cards);
    let usb_info = detect_respeaker_usb_in_dir(usb_dir);

    let detected = card_name.is_some() || usb_info.is_some();
    let (usb_vendor_id, usb_product_id) = match usb_info {
        Some((v, p)) => (Some(v), Some(p)),
        None => (None, None),
    };

    RespeakerInfo {
        detected,
        card_name,
        usb_vendor_id,
        usb_product_id,
        channels: if detected { 4 } else { 0 },
    }
}

/// Detect Seeed ReSpeaker XVF3800 on live host.
pub fn detect_respeaker() -> RespeakerInfo {
    let asound_cards = std::fs::read_to_string("/proc/asound/cards").ok();
    detect_respeaker_with_sources(asound_cards.as_deref(), Path::new("/sys/bus/usb/devices"))
}

/// Detect board hardware model from device tree or release files.
pub fn detect_board_model_from_sources(
    device_tree_model: Option<&str>,
    release_info: Option<&str>,
) -> String {
    if let Some(model) = device_tree_model {
        let clean = model.trim_matches('\0').trim();
        if !clean.is_empty() {
            return clean.to_string();
        }
    }
    if let Some(info) = release_info {
        for line in info.lines() {
            if let Some(stripped) = line.strip_prefix("BOARD_NAME=") {
                let name = stripped.trim_matches('"').trim();
                if !name.is_empty() {
                    return name.to_string();
                }
            } else if let Some(stripped) = line.strip_prefix("PRETTY_NAME=") {
                let name = stripped.trim_matches('"').trim();
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
    }
    format!("Generic Linux ({})", std::env::consts::ARCH)
}

/// Detect host board model on live host.
pub fn detect_board_model() -> String {
    let dt_model = std::fs::read_to_string("/proc/device-tree/model").ok();
    let release = std::fs::read_to_string("/etc/os-release")
        .or_else(|_| std::fs::read_to_string("/etc/buildroot-release"))
        .ok();
    detect_board_model_from_sources(dt_model.as_deref(), release.as_deref())
}

/// Read current CPU frequency in MHz from cpufreq sysfs.
pub fn read_cpu_freq_mhz_from_path(path: &Path) -> Option<u64> {
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(khz) = content.trim().parse::<u64>() {
            return Some(khz / 1000);
        }
    }
    None
}

/// Read CPU frequency from live system.
pub fn read_cpu_freq_mhz() -> Option<u64> {
    read_cpu_freq_mhz_from_path(Path::new(
        "/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq",
    ))
}

/// Read storage statistics: (total_bytes, used_bytes, available_bytes).
pub fn read_storage_usage() -> (u64, u64, u64) {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    if disks.is_empty() {
        return (0, 0, 0);
    }
    if let Some(root) = disks.iter().find(|d| d.mount_point() == Path::new("/")) {
        let total = root.total_space();
        let avail = root.available_space();
        let used = total.saturating_sub(avail);
        return (total, used, avail);
    }
    let first = &disks[0];
    let total = first.total_space();
    let avail = first.available_space();
    let used = total.saturating_sub(avail);
    (total, used, avail)
}

/// Check if CPU thermal throttling is active (temp >= 80°C).
pub fn is_cpu_throttled(temp: Option<f32>) -> bool {
    if let Some(t) = temp {
        t >= 80.0
    } else {
        false
    }
}

/// Gather complete hardware telemetry snapshot.
pub fn get_hardware_telemetry(audio_routing: AudioRoutingInfo) -> HardwareTelemetry {
    let mut sys = System::new_all();
    sys.refresh_all();

    let model = detect_board_model();
    let cpu_arch = System::cpu_arch();
    let cpu_cores = sys.cpus().len();
    let memory_total_bytes = sys.total_memory();
    let memory_used_bytes = sys.used_memory();
    let (storage_total_bytes, storage_used_bytes, storage_available_bytes) = read_storage_usage();

    let cpu_temp_celsius = read_cpu_temperature();
    let cpu_freq_mhz = read_cpu_freq_mhz();
    let throttled = is_cpu_throttled(cpu_temp_celsius);
    let respeaker = detect_respeaker();

    HardwareTelemetry {
        model,
        cpu_arch,
        cpu_cores,
        cpu_temp_celsius,
        cpu_freq_mhz,
        throttled,
        memory_total_bytes,
        memory_used_bytes,
        storage_total_bytes,
        storage_used_bytes,
        storage_available_bytes,
        respeaker,
        audio_routing,
    }
}
