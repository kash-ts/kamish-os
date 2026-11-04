//! Network Manager module.
//! Controls Ethernet, Wi-Fi profile management, and Setup AP mode fallback.

pub mod manager;
pub mod models;

pub use manager::NetworkManager;
pub use models::{
    ApStartResponse, ApStatus, ApStopResponse, EthernetStatus, NetworkStatus, WifiConnectRequest,
    WifiConnectResponse, WifiDisconnectResponse, WifiNetwork, WifiScanResponse, WifiStatus,
};
