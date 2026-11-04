//! VPN Manager module.
//! Controls VLESS / Happ / Reality tunnel configuration, status, and diagnostics.

pub mod manager;
pub mod models;

pub use manager::VpnManager;
pub use models::{
    VpnConfigRequest, VpnConfigResponse, VpnConnectResponse, VpnDisconnectResponse, VpnStats,
    VpnStatus,
};
