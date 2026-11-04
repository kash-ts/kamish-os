use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

impl std::fmt::Display for PlaybackStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlaybackStatus::Playing => write!(f, "playing"),
            PlaybackStatus::Paused => write!(f, "paused"),
            PlaybackStatus::Stopped => write!(f, "stopped"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<u64>,
    pub source: String, // "youtube", "radio", "local", "mock", "catalog"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

impl Track {
    pub fn new(
        title: impl Into<String>,
        artist: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            artist: artist.into(),
            duration_secs: None,
            source: source.into(),
            uri: None,
            extra_json: None,
            created_at: None,
        }
    }
}

/// Backward compatibility structure for existing code
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrackInfo {
    pub id: Option<String>,
    pub title: String,
    pub artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl From<Track> for TrackInfo {
    fn from(t: Track) -> Self {
        Self {
            id: Some(t.id),
            title: t.title,
            artist: t.artist,
            uri: t.uri,
            duration_secs: t.duration_secs,
            source: Some(t.source),
        }
    }
}

impl From<TrackInfo> for Track {
    fn from(ti: TrackInfo) -> Self {
        Self {
            id: ti.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            title: ti.title,
            artist: ti.artist,
            duration_secs: ti.duration_secs,
            source: ti.source.unwrap_or_else(|| "search".to_string()),
            uri: ti.uri,
            extra_json: None,
            created_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaybackState {
    pub status: PlaybackStatus,
    pub current_track: Option<Track>,
    pub volume: u8,
    pub position_secs: u64,
    pub duration_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<u64>,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QueueItem {
    pub id: String,
    pub track: Track,
    pub position: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecommendationItem {
    pub track: Track,
    pub score: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RadioStation {
    pub id: String,
    pub name: String,
    pub genre: String,
    pub stream_uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon_url: Option<String>,
}

impl RadioStation {
    pub fn to_track(&self) -> Track {
        Track {
            id: format!("radio:{}", self.id),
            title: self.name.clone(),
            artist: format!("Radio ({})", self.genre),
            duration_secs: None,
            source: "radio".to_string(),
            uri: Some(self.stream_uri.clone()),
            extra_json: serde_json::to_string(self).ok(),
            created_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: i64,
    pub track_id: String,
    pub track: Option<Track>,
    pub played_at: String,
    pub playback_duration_secs: u64,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub tracks: Vec<Track>,
}
