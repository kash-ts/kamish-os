use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub primary_interface: String,
    pub ip_address: Option<String>,
    pub subnet_mask: Option<String>,
    pub gateway: Option<String>,
    pub dns: Vec<String>,
    pub ethernet: EthernetStatus,
    pub wifi: Option<WifiStatus>,
    pub ap_mode: ApStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthernetStatus {
    pub connected: bool,
    pub speed_mbps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiStatus {
    pub connected: bool,
    pub ssid: Option<String>,
    pub bssid: Option<String>,
    pub signal_level_percent: Option<u8>,
    pub frequency_mhz: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApStatus {
    pub active: bool,
    pub ssid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub bssid: String,
    pub signal_percent: u8,
    pub security: String,
    pub channel: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiScanResponse {
    pub count: usize,
    pub networks: Vec<WifiNetwork>,
}

#[derive(Clone, Deserialize)]
pub struct WifiConnectRequest {
    pub ssid: String,
    pub password: Option<String>,
}

// Custom Debug implementation to NEVER log passwords
impl std::fmt::Debug for WifiConnectRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WifiConnectRequest")
            .field("ssid", &self.ssid)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiConnectResponse {
    pub status: String,
    pub ssid: String,
    pub ip_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiDisconnectResponse {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApStartResponse {
    pub status: String,
    pub ssid: String,
    pub gateway_ip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApStopResponse {
    pub status: String,
}
