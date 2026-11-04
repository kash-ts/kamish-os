//! Music Manager module.
//! Handles track search, playback queue, favorites, listening history, skips,
//! internet radio, local library, headless MPV playback, and custom recommendation engine.

pub mod lavalink;
pub mod manager;
pub mod models;
pub mod player;
pub mod radio;
pub mod recommendation;
pub mod repository;
pub mod search;
pub mod youtube_auth;
pub mod ytmusic;

pub use lavalink::{LavalinkClient, LavalinkConfig, LavalinkStatus};
pub use manager::MusicManager;
pub use models::{
    HistoryItem, PlaybackState, PlaybackStatus, Playlist, QueueItem, RadioStation,
    RecommendationItem, SearchResult, Track, TrackInfo,
};
pub use player::MpvPlayer;
pub use radio::RadioService;
pub use recommendation::RecommendationEngine;
pub use repository::MusicRepository;
pub use search::SearchService;
pub use youtube_auth::{DeviceCodeChallenge, YouTubeAuth, YouTubeAuthStatus, YouTubeTokens};

/// Alias for backwards compatibility with earlier stages
pub type MediaState = MusicManager;
