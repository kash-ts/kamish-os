use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnStatus {
    pub enabled: bool,
    pub status: String,
    pub protocol: String,
    pub server_address: Option<String>,
    pub ping_ms: Option<u64>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connected_since: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct VpnConfigRequest {
    pub subscription_url: Option<String>,
    pub vless_uri: Option<String>,
}

// Custom Debug implementation to NEVER log UUIDs, secrets or subscription URLs with tokens
impl std::fmt::Debug for VpnConfigRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sub_masked = self
            .subscription_url
            .as_ref()
            .map(|_| "[REDACTED_SUBSCRIPTION_URL]");
        let vless_masked = self.vless_uri.as_ref().map(|_| "[REDACTED_VLESS_URI]");

        f.debug_struct("VpnConfigRequest")
            .field("subscription_url", &sub_masked)
            .field("vless_uri", &vless_masked)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnConfigResponse {
    pub status: String,
    pub server: String,
    pub sni: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnConnectResponse {
    pub status: String,
    pub ping_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnDisconnectResponse {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VpnStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub ping_ms: Option<u64>,
}
