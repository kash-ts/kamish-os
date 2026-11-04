use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BluetoothStatus {
    pub adapter_enabled: bool,
    pub discoverable: bool,
    pub connected_device: Option<BluetoothConnectedDevice>,
    pub paired_devices: Vec<BluetoothPairedDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BluetoothConnectedDevice {
    pub mac: String,
    pub name: String,
    pub connected: bool,
    pub codec: Option<String>,
    pub volume: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BluetoothPairedDevice {
    pub mac: String,
    pub name: String,
    pub trusted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BluetoothDevice {
    pub mac: String,
    pub name: String,
    pub rssi: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScanRequest {
    #[serde(default = "default_scan_duration")]
    pub duration_secs: u64,
}

fn default_scan_duration() -> u64 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResponse {
    pub status: String,
    pub devices_found: Vec<BluetoothDevice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PairRequest {
    pub mac: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairResponse {
    pub status: String,
    pub mac: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConnectRequest {
    pub mac: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectResponse {
    pub status: String,
    pub mac: String,
    pub profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisconnectResponse {
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscoverableRequest {
    #[serde(alias = "enabled")]
    pub discoverable: Option<bool>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverableResponse {
    pub status: String,
    pub discoverable: bool,
}
