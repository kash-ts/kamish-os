pub mod events;
pub mod hardware;
pub mod power_button;
pub mod ssh;
pub mod update;

pub use events::{EventBus, SystemEvent};
pub use hardware::{
    detect_board_model, detect_respeaker, get_hardware_telemetry, is_cpu_throttled,
    parse_asound_cards, parse_sysfs_temperature, read_cpu_freq_mhz, read_cpu_temperature,
    read_storage_usage, AudioRoutingInfo, HardwareTelemetry, RespeakerInfo,
};
use serde::Serialize;
pub use ssh::{get_ssh_status, toggle_ssh, SshStatus, SshToggleRequest, SshToggleResponse};
use sysinfo::System;
pub use update::{
    get_update_status, inspect_bundle, install_bundle, UpdateBundleInfo, UpdateInstallResult,
    UpdateStatus,
};

#[derive(Debug, Serialize, Clone)]
pub struct DeviceInfo {
    pub hostname: String,
    pub local_url: String,
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub arch: String,
    pub cpu_count: usize,
    pub memory_total_bytes: u64,
    pub memory_used_bytes: u64,
    pub storage_total_bytes: u64,
    pub storage_used_bytes: u64,
    pub system_uptime_secs: u64,
    pub cpu_temp_celsius: Option<f32>,
    pub hardware_model: String,
}

pub fn get_device_info() -> DeviceInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    let host_name_raw = std::env::var("KAMISH_HOSTNAME")
        .ok()
        .unwrap_or_else(|| "kamish".to_string());
    let clean_hostname =
        if host_name_raw.is_empty() || host_name_raw == "localhost" || host_name_raw == "unknown" {
            "kamish".to_string()
        } else {
            host_name_raw.to_lowercase().replace('_', "-")
        };
    let port = std::env::var("PORT").unwrap_or_else(|_| "80".to_string());
    let port_suffix = if port == "80" {
        "".to_string()
    } else {
        format!(":{port}")
    };
    let local_url = if clean_hostname == "kamish" {
        format!("http://kamish.local{port_suffix}")
    } else {
        format!("http://{clean_hostname}.local{port_suffix}")
    };
    let hostname = if host_name_raw.is_empty() || host_name_raw == "localhost" {
        "kamish".to_string()
    } else {
        host_name_raw
    };
    let os_name = System::name().unwrap_or_else(|| "Linux".to_string());
    let os_version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
    let kernel_version = System::kernel_version().unwrap_or_else(|| "Unknown".to_string());
    let arch = System::cpu_arch();

    let cpu_count = sys.cpus().len();
    let memory_total_bytes = sys.total_memory();
    let memory_used_bytes = sys.used_memory();
    let (storage_total_bytes, storage_used_bytes, _) = hardware::read_storage_usage();
    let system_uptime_secs = System::uptime();
    let cpu_temp_celsius = read_cpu_temperature();
    let hardware_model = detect_board_model();

    DeviceInfo {
        hostname,
        local_url,
        os_name,
        os_version,
        kernel_version,
        arch,
        cpu_count,
        memory_total_bytes,
        memory_used_bytes,
        storage_total_bytes,
        storage_used_bytes,
        system_uptime_secs,
        cpu_temp_celsius,
        hardware_model,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info() {
        let info = get_device_info();
        assert!(!info.hostname.is_empty());
        assert!(info.cpu_count > 0);
        assert!(info.memory_total_bytes > 0);
    }
}
