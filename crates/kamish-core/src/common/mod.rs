pub mod error;

use crate::ai::{AgyManager, AiOrchestrator};
use crate::audio::AudioOutputManager;
use crate::bluetooth::BluetoothManager;
use crate::music::{MediaState, YouTubeAuth};
use crate::network::NetworkManager;
use crate::scheduler::{SchedulerEngine, SchedulerHandle};
use crate::setup::DiagnosticsEngine;
use crate::speech::{CaptureSupervisor, SpeechManager};
use crate::system::EventBus;
use crate::vpn::VpnManager;
pub use error::{AppError, AppResult};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::time::Instant;

/// Core application state across background services and IPC handlers
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub scheduler: SchedulerHandle,
    pub media: Arc<MediaState>,
    pub youtube_auth: Arc<YouTubeAuth>,
    pub ai: Arc<AiOrchestrator>,
    pub agy: Arc<AgyManager>,
    pub speech: Arc<SpeechManager>,
    pub capture: Arc<CaptureSupervisor>,
    pub audio: Arc<AudioOutputManager>,
    pub bluetooth: Arc<BluetoothManager>,
    pub network: Arc<NetworkManager>,
    pub vpn: Arc<VpnManager>,
    pub events: Arc<EventBus>,
    pub diagnostics: Arc<DiagnosticsEngine>,
    pub start_time: Instant,
    pub version: &'static str,
}

pub type CoreState = AppState;

impl AppState {
    pub fn new(db: SqlitePool) -> Self {
        Self::with_options(db, "agy".to_string(), true)
    }

    pub fn with_options(db: SqlitePool, ai_cli_path: String, ai_cli_enabled: bool) -> Self {
        let events = Arc::new(EventBus::default());
        let scheduler = SchedulerEngine::new(db.clone());
        let media = Arc::new(MediaState::new());
        media.set_event_bus((*events).clone());
        let ai = Arc::new(AiOrchestrator::new(db.clone(), ai_cli_path, ai_cli_enabled));
        let agy = Arc::new(AgyManager::new());
        let youtube_auth = Arc::new(YouTubeAuth::new());
        let speech = Arc::new(SpeechManager::new());
        let capture = Arc::new(CaptureSupervisor::new(speech.clone(), events.clone()));
        let audio = Arc::new(AudioOutputManager::new());
        let bluetooth = Arc::new(BluetoothManager::with_event_bus(Some((*events).clone())));
        let network = Arc::new(NetworkManager::with_event_bus(Some((*events).clone())));
        let vpn = Arc::new(VpnManager::with_event_bus(Some((*events).clone())));
        let diagnostics = Arc::new(DiagnosticsEngine::new());

        Self {
            db,
            scheduler,
            media,
            youtube_auth,
            ai,
            agy,
            speech,
            capture,
            audio,
            bluetooth,
            network,
            vpn,
            events,
            diagnostics,
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION"),
        }
    }
}
