//! Bluetooth module.
//! Controls BlueZ / bluetoothctl for A2DP Sink audio device management, pairing, and volume.

pub mod manager;
pub mod models;

pub use manager::BluetoothManager;
pub use models::{
    BluetoothConnectedDevice, BluetoothDevice, BluetoothPairedDevice, BluetoothStatus,
    ConnectRequest, ConnectResponse, DisconnectResponse, DiscoverableRequest, DiscoverableResponse,
    PairRequest, PairResponse, ScanRequest, ScanResponse,
};
