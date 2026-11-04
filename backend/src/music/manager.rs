use crate::database::DbPool;
use crate::music::models::{
    HistoryItem, PlaybackState, PlaybackStatus, RadioStation, RecommendationItem, SearchResult,
    Track, TrackInfo,
};
use crate::music::player::MpvPlayer;
use crate::music::radio::RadioService;
use crate::music::recommendation::RecommendationEngine;
use crate::music::repository::MusicRepository;
use crate::music::search::SearchService;
use crate::music::ytmusic::YtMusicClient;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

pub struct MusicManager {
    player: Arc<MpvPlayer>,
    queue: RwLock<Vec<Track>>,
    event_bus: std::sync::RwLock<Option<crate::system::EventBus>>,
    ytmusic: YtMusicClient,
}

impl Default for MusicManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MusicManager {
    pub fn new() -> Self {
        Self {
            player: Arc::new(MpvPlayer::new()),
            queue: RwLock::new(Vec::new()),
            event_bus: std::sync::RwLock::new(None),
            ytmusic: YtMusicClient::default(),
        }
    }

    pub fn set_event_bus(&self, bus: crate::system::EventBus) {
        if let Ok(mut lock) = self.event_bus.write() {
            *lock = Some(bus);
        }
    }

    /// Set volume (0-100) and persist to SQLite if pool is provided.
    pub async fn set_volume(&self, volume: u8, pool: Option<&DbPool>) -> u8 {
        let vol = self.player.set_volume(volume).await;

        if let Some(pool) = pool {
            let val = vol.to_string();
            let _ = sqlx::query(
                "INSERT INTO device_state (key, value, updated_at) VALUES ('volume', ?, CURRENT_TIMESTAMP)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            )
            .bind(&val)
            .execute(pool)
            .await;
        }

        if let Ok(guard) = self.event_bus.read() {
            if let Some(bus) = guard.as_ref() {
                bus.emit("volume_changed", serde_json::json!({ "volume": vol }));
            }
        }

        vol
    }

    /// Get current volume.
    pub fn get_volume(&self) -> u8 {
        self.player.get_volume()
    }

    /// Play a specific Track.
    pub async fn play_track(&self, track: Track, pool: Option<&DbPool>) {
        if let Some(pool) = pool {
            let _ = MusicRepository::upsert_track(pool, &track).await;

            let track_json = serde_json::to_string(&track).unwrap_or_default();
            let _ = sqlx::query(
                "INSERT INTO device_state (key, value, updated_at) VALUES ('current_track', ?, CURRENT_TIMESTAMP)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            )
            .bind(&track_json)
            .execute(pool)
            .await;

            let _ = sqlx::query(
                "INSERT INTO device_state (key, value, updated_at) VALUES ('playback_status', 'playing', CURRENT_TIMESTAMP)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            )
            .execute(pool)
            .await;

            let _ = MusicRepository::record_history(pool, &track.id, 0, false).await;
        }

        if let Ok(guard) = self.event_bus.read() {
            if let Some(bus) = guard.as_ref() {
                bus.emit(
                    "playback_status",
                    serde_json::json!({ "status": "playing" }),
                );
                bus.emit(
                    "track_changed",
                    serde_json::json!({
                        "track": {
                            "id": track.id,
                            "title": track.title,
                            "artist": track.artist,
                            "duration_secs": track.duration_secs,
                        }
                    }),
                );
            }
        }

        self.player.play(track).await;
    }

    /// Backward-compatible play method taking TrackInfo.
    pub async fn play(&self, track_info: TrackInfo, pool: Option<&DbPool>) {
        let track: Track = track_info.into();
        self.play_track(track, pool).await;
    }

    /// Play music by optional query, track_id, or direct stream URI.
    pub async fn play_by_query_or_uri(
        &self,
        query: Option<&str>,
        uri: Option<&str>,
        pool: Option<&DbPool>,
    ) -> Track {
        let track = match (query, uri) {
            (Some(q), _) if !q.trim().is_empty() => {
                // If query is an ID existing in DB or Radio
                if let Some(pool) = pool {
                    if let Ok(Some(existing)) = MusicRepository::get_track(pool, q.trim()).await {
                        existing
                    } else {
                        // Check if it matches a radio station
                        let radio_matches = RadioService::search_stations(q.trim());
                        if let Some(station) = radio_matches.first() {
                            station.to_track()
                        } else {
                            // Check catalog search
                            let search_res =
                                SearchService::search(q.trim(), "all", 1, Some(pool)).await;
                            if let Some(res) = search_res.first() {
                                Track {
                                    id: res.id.clone(),
                                    title: res.title.clone(),
                                    artist: res.artist.clone(),
                                    duration_secs: res.duration_secs,
                                    source: res.source.clone(),
                                    uri: res.uri.clone(),
                                    extra_json: None,
                                    created_at: None,
                                }
                            } else {
                                Track::new(q.trim(), "Kamish Music", "search")
                            }
                        }
                    }
                } else {
                    Track::new(q.trim(), "Kamish Music", "search")
                }
            }
            (_, Some(u)) if !u.trim().is_empty() => {
                let mut t = Track::new("Streaming Audio", "Online Stream", "stream");
                t.uri = Some(u.trim().to_string());
                t
            }
            _ => {
                // No query/uri provided: resume paused track if present
                if let Some(cur) = self.player.get_current_track().await {
                    self.resume(pool).await;
                    return cur;
                }

                // Default station: Chillhop Radio
                RadioService::get_curated_stations()[0].to_track()
            }
        };

        self.play_track(track.clone(), pool).await;
        track
    }

    /// Pause current playback.
    pub async fn pause(&self, pool: Option<&DbPool>) {
        self.player.pause().await;

        if let Some(pool) = pool {
            let _ = sqlx::query(
                "INSERT INTO device_state (key, value, updated_at) VALUES ('playback_status', 'paused', CURRENT_TIMESTAMP)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            )
            .execute(pool)
            .await;
        }

        if let Ok(guard) = self.event_bus.read() {
            if let Some(bus) = guard.as_ref() {
                bus.emit("playback_status", serde_json::json!({ "status": "paused" }));
            }
        }
    }

    /// Resume playback.
    pub async fn resume(&self, pool: Option<&DbPool>) {
        self.player.resume().await;

        if let Some(pool) = pool {
            let _ = sqlx::query(
                "INSERT INTO device_state (key, value, updated_at) VALUES ('playback_status', 'playing', CURRENT_TIMESTAMP)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            )
            .execute(pool)
            .await;
        }

        if let Ok(guard) = self.event_bus.read() {
            if let Some(bus) = guard.as_ref() {
                bus.emit(
                    "playback_status",
                    serde_json::json!({ "status": "playing" }),
                );
            }
        }
    }

    /// Stop playback completely.
    pub async fn stop(&self, pool: Option<&DbPool>) {
        self.player.stop().await;

        if let Some(pool) = pool {
            let _ = sqlx::query(
                "INSERT INTO device_state (key, value, updated_at) VALUES ('playback_status', 'stopped', CURRENT_TIMESTAMP)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            )
            .execute(pool)
            .await;
        }

        if let Ok(guard) = self.event_bus.read() {
            if let Some(bus) = guard.as_ref() {
                bus.emit(
                    "playback_status",
                    serde_json::json!({ "status": "stopped" }),
                );
            }
        }
    }

    /// Skip current track and record penalty in database.
    pub async fn skip(&self, position_secs: Option<u64>, pool: Option<&DbPool>) -> Option<String> {
        let current_track = self.player.get_current_track().await;
        let pos = match position_secs {
            Some(p) => p,
            None => self.player.get_playback_state().await.position_secs,
        };

        let mut skipped_id = None;
        if let Some(track) = current_track {
            skipped_id = Some(track.id.clone());
            if let Some(pool) = pool {
                let _ = MusicRepository::record_skip(pool, &track.id, pos).await;
            }
        }

        // Check if there is next track in queue
        let next_track = {
            let mut q = self.queue.write().await;
            if !q.is_empty() {
                Some(q.remove(0))
            } else {
                None
            }
        };

        if let Some(next) = next_track {
            self.play_track(next, pool).await;
        } else {
            self.stop(pool).await;
        }

        skipped_id
    }

    /// Get current playback status.
    pub async fn get_status(&self) -> PlaybackStatus {
        self.player.get_status().await
    }

    /// Get current track info.
    pub async fn get_current_track(&self) -> Option<TrackInfo> {
        self.player.get_current_track().await.map(TrackInfo::from)
    }

    /// Get current Track.
    pub async fn get_current(&self) -> Option<Track> {
        self.player.get_current_track().await
    }

    /// Get full playback state.
    pub async fn get_playback_state(&self) -> PlaybackState {
        self.player.get_playback_state().await
    }

    /// Toggle like status for a track.
    pub async fn toggle_like(&self, track_id: &str, pool: &DbPool) -> Result<bool> {
        MusicRepository::toggle_like(pool, track_id).await
    }

    /// Set explicit like status for a track.
    pub async fn set_like(&self, track_id: &str, liked: bool, pool: &DbPool) -> Result<()> {
        MusicRepository::set_like(pool, track_id, liked).await
    }

    /// Check if track is liked.
    pub async fn is_liked(&self, track_id: &str, pool: &DbPool) -> Result<bool> {
        MusicRepository::is_liked(pool, track_id).await
    }

    /// List user's favorite tracks.
    pub async fn get_favorites(&self, pool: &DbPool, limit: i64) -> Result<Vec<Track>> {
        MusicRepository::list_favorites(pool, limit).await
    }

    /// List recent listening history.
    pub async fn get_history(&self, pool: &DbPool, limit: i64) -> Result<Vec<HistoryItem>> {
        MusicRepository::list_history(pool, limit).await
    }

    /// Get recommendations based on listening history, likes, and skips.
    pub async fn get_recommendations(
        &self,
        pool: &DbPool,
        limit: usize,
    ) -> Result<Vec<RecommendationItem>> {
        let (
            liked_tracks,
            liked_artists,
            play_counts,
            completion_counts,
            skip_counts,
            artist_skips,
            mut candidate_tracks,
        ) = MusicRepository::get_recommendation_data(pool).await?;

        // If candidate catalog is small, also add radio station tracks
        for station in RadioService::get_curated_stations() {
            let track = station.to_track();
            if !candidate_tracks.iter().any(|t| t.id == track.id) {
                candidate_tracks.push(track);
            }
        }

        let recommendations = RecommendationEngine::score_tracks(
            candidate_tracks,
            &liked_tracks,
            &liked_artists,
            &play_counts,
            &completion_counts,
            &skip_counts,
            &artist_skips,
            limit,
        );

        Ok(recommendations)
    }

    /// Search tracks and radio.
    pub async fn search(
        &self,
        query: &str,
        source: &str,
        limit: usize,
        pool: Option<&DbPool>,
    ) -> Vec<SearchResult> {
        SearchService::search(query, source, limit, pool).await
    }

    /// List available radio stations.
    pub fn get_radio_stations(&self) -> Vec<RadioStation> {
        RadioService::get_curated_stations()
    }

    /// Access the YouTube Music client
    pub fn ytmusic(&self) -> &YtMusicClient {
        &self.ytmusic
    }

    /// Rate track directly on YouTube Music (LIKE / DISLIKE / INDIFFERENT)
    pub async fn rate_ytmusic(&self, video_id: &str, rating: &str) -> bool {
        self.ytmusic.rate_track(video_id, rating).await
    }

    /// Load persisted device state and seed initial catalog.
    pub async fn load_from_db(&self, pool: &DbPool) {
        // Seed default tracks if library is empty
        let _ = MusicRepository::seed_default_tracks_if_empty(pool).await;

        if let Ok(Some(row)) =
            sqlx::query_as::<_, (String,)>("SELECT value FROM device_state WHERE key = 'volume'")
                .fetch_optional(pool)
                .await
        {
            if let Ok(vol) = row.0.parse::<u8>() {
                let _ = self.player.set_volume(vol).await;
            }
        }

        if let Ok(Some(row)) = sqlx::query_as::<_, (String,)>(
            "SELECT value FROM device_state WHERE key = 'current_track'",
        )
        .fetch_optional(pool)
        .await
        {
            if let Ok(track) = serde_json::from_str::<Track>(&row.0) {
                // Preload current track into player without autoplaying on boot
                debug!("Loaded persisted current track: {}", track.title);
            }
        }
    }
}
