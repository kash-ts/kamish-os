pub mod cloud_free;
pub mod offline;
pub mod piper;
pub mod rhvoice;

pub use cloud_free::CloudFreeTtsProvider;
pub use offline::OfflineFallbackTtsProvider;
pub use piper::PiperTtsProvider;
pub use rhvoice::RhVoiceTtsProvider;
