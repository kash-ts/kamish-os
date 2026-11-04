use crate::database::DbPool;
use crate::music::models::{HistoryItem, Track};
use anyhow::Result;
use std::collections::{HashMap, HashSet};

pub struct MusicRepository;

impl MusicRepository {
    /// Insert or update a track in the catalog.
    pub async fn upsert_track(pool: &DbPool, track: &Track) -> Result<()> {
        let dur = track.duration_secs.map(|d| d as i64).unwrap_or(0);
        let uri = track.uri.as_deref().unwrap_or("");
        sqlx::query(
            r#"
            INSERT INTO music_tracks (id, title, artist, duration_secs, source, uri, extra_json)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                artist = excluded.artist,
                duration_secs = excluded.duration_secs,
                source = excluded.source,
                uri = excluded.uri,
                extra_json = excluded.extra_json
            "#,
        )
        .bind(&track.id)
        .bind(&track.title)
        .bind(&track.artist)
        .bind(dur)
        .bind(&track.source)
        .bind(uri)
        .bind(&track.extra_json)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Retrieve a track by ID.
    pub async fn get_track(pool: &DbPool, id: &str) -> Result<Option<Track>> {
        let row = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                Option<i64>,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
            ),
        >(
            r#"
            SELECT id, title, artist, duration_secs, source, uri, extra_json, created_at
            FROM music_tracks
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| Track {
            id: r.0,
            title: r.1,
            artist: r.2,
            duration_secs: r.3.map(|d| d as u64),
            source: r.4,
            uri: r.5,
            extra_json: r.6,
            created_at: r.7,
        }))
    }

    /// Search tracks in catalog matching query in title or artist.
    pub async fn search_tracks(pool: &DbPool, query: &str, limit: i64) -> Result<Vec<Track>> {
        let pattern = format!("%{}%", query.trim());
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                Option<i64>,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
            ),
        >(
            r#"
            SELECT id, title, artist, duration_secs, source, uri, extra_json, created_at
            FROM music_tracks
            WHERE title LIKE ? OR artist LIKE ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Track {
                id: r.0,
                title: r.1,
                artist: r.2,
                duration_secs: r.3.map(|d| d as u64),
                source: r.4,
                uri: r.5,
                extra_json: r.6,
                created_at: r.7,
            })
            .collect())
    }

    /// List all catalog tracks.
    pub async fn list_all_tracks(pool: &DbPool, limit: i64) -> Result<Vec<Track>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                Option<i64>,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
            ),
        >(
            r#"
            SELECT id, title, artist, duration_secs, source, uri, extra_json, created_at
            FROM music_tracks
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Track {
                id: r.0,
                title: r.1,
                artist: r.2,
                duration_secs: r.3.map(|d| d as u64),
                source: r.4,
                uri: r.5,
                extra_json: r.6,
                created_at: r.7,
            })
            .collect())
    }

    /// Check if a track is liked.
    pub async fn is_liked(pool: &DbPool, track_id: &str) -> Result<bool> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(1) FROM music_likes WHERE track_id = ?")
            .bind(track_id)
            .fetch_one(pool)
            .await?;

        Ok(count.0 > 0)
    }

    /// Toggle like status for a track. Returns true if now liked, false if unliked.
    pub async fn toggle_like(pool: &DbPool, track_id: &str) -> Result<bool> {
        let currently_liked = Self::is_liked(pool, track_id).await?;
        if currently_liked {
            sqlx::query("DELETE FROM music_likes WHERE track_id = ?")
                .bind(track_id)
                .execute(pool)
                .await?;
            Ok(false)
        } else {
            sqlx::query(
                "INSERT INTO music_likes (track_id, liked_at) VALUES (?, CURRENT_TIMESTAMP)
                 ON CONFLICT(track_id) DO NOTHING",
            )
            .bind(track_id)
            .execute(pool)
            .await?;
            Ok(true)
        }
    }

    /// Set explicit like status.
    pub async fn set_like(pool: &DbPool, track_id: &str, liked: bool) -> Result<()> {
        if liked {
            sqlx::query(
                "INSERT INTO music_likes (track_id, liked_at) VALUES (?, CURRENT_TIMESTAMP)
                 ON CONFLICT(track_id) DO NOTHING",
            )
            .bind(track_id)
            .execute(pool)
            .await?;
        } else {
            sqlx::query("DELETE FROM music_likes WHERE track_id = ?")
                .bind(track_id)
                .execute(pool)
                .await?;
        }
        Ok(())
    }

    /// List liked tracks ordered by like time descending.
    pub async fn list_favorites(pool: &DbPool, limit: i64) -> Result<Vec<Track>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                Option<i64>,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
            ),
        >(
            r#"
            SELECT t.id, t.title, t.artist, t.duration_secs, t.source, t.uri, t.extra_json, t.created_at
            FROM music_tracks t
            INNER JOIN music_likes l ON t.id = l.track_id
            ORDER BY l.liked_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Track {
                id: r.0,
                title: r.1,
                artist: r.2,
                duration_secs: r.3.map(|d| d as u64),
                source: r.4,
                uri: r.5,
                extra_json: r.6,
                created_at: r.7,
            })
            .collect())
    }

    /// Record a playback event into music_history.
    pub async fn record_history(
        pool: &DbPool,
        track_id: &str,
        playback_duration_secs: u64,
        completed: bool,
    ) -> Result<i64> {
        let res = sqlx::query(
            r#"
            INSERT INTO music_history (track_id, playback_duration_secs, completed)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(track_id)
        .bind(playback_duration_secs as i64)
        .bind(completed)
        .execute(pool)
        .await?;

        Ok(res.last_insert_rowid())
    }

    /// List recent listening history with joined track info.
    pub async fn list_history(pool: &DbPool, limit: i64) -> Result<Vec<HistoryItem>> {
        let rows = sqlx::query_as::<
            _,
            (
                i64,
                String,
                String,
                i64,
                bool,
                Option<String>,
                Option<String>,
                Option<i64>,
                Option<String>,
                Option<String>,
                Option<String>,
            ),
        >(
            r#"
            SELECT 
                h.id, h.track_id, h.played_at, h.playback_duration_secs, h.completed,
                t.title, t.artist, t.duration_secs, t.source, t.uri, t.extra_json
            FROM music_history h
            LEFT JOIN music_tracks t ON h.track_id = t.id
            ORDER BY h.played_at DESC, h.id DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        let items = rows
            .into_iter()
            .map(|r| {
                let track = r.5.map(|title| Track {
                    id: r.1.clone(),
                    title,
                    artist: r.6.unwrap_or_else(|| "Unknown Artist".to_string()),
                    duration_secs: r.7.map(|d| d as u64),
                    source: r.8.unwrap_or_else(|| "history".to_string()),
                    uri: r.9,
                    extra_json: r.10,
                    created_at: None,
                });

                HistoryItem {
                    id: r.0,
                    track_id: r.1,
                    track,
                    played_at: r.2,
                    playback_duration_secs: r.3.max(0) as u64,
                    completed: r.4,
                }
            })
            .collect();

        Ok(items)
    }

    /// Record a skip event for recommendation penalty calculation.
    pub async fn record_skip(pool: &DbPool, track_id: &str, position_secs: u64) -> Result<i64> {
        let res = sqlx::query(
            r#"
            INSERT INTO music_skips (track_id, position_secs)
            VALUES (?, ?)
            "#,
        )
        .bind(track_id)
        .bind(position_secs as i64)
        .execute(pool)
        .await?;

        Ok(res.last_insert_rowid())
    }

    /// Extract stats needed by the Recommendation Engine.
    pub async fn get_recommendation_data(
        pool: &DbPool,
    ) -> Result<(
        HashSet<String>,        // liked_track_ids
        HashSet<String>,        // liked_artists (lowercase)
        HashMap<String, usize>, // play_counts (track_id -> count)
        HashMap<String, usize>, // completion_counts (track_id -> count)
        HashMap<String, usize>, // skip_counts (track_id -> count)
        HashMap<String, usize>, // artist_skips (artist_lower -> count)
        Vec<Track>,             // candidate tracks
    )> {
        // Liked track IDs
        let liked_rows = sqlx::query_as::<_, (String,)>("SELECT track_id FROM music_likes")
            .fetch_all(pool)
            .await?;
        let liked_tracks: HashSet<String> = liked_rows.into_iter().map(|r| r.0).collect();

        // All catalog tracks
        let all_tracks = Self::list_all_tracks(pool, 500).await?;

        // Liked artists
        let mut liked_artists = HashSet::new();
        for t in &all_tracks {
            if liked_tracks.contains(&t.id) {
                liked_artists.insert(t.artist.to_lowercase());
            }
        }

        // History plays & completions
        let hist_rows =
            sqlx::query_as::<_, (String, bool)>("SELECT track_id, completed FROM music_history")
                .fetch_all(pool)
                .await?;
        let mut play_counts = HashMap::new();
        let mut completion_counts = HashMap::new();
        for (tid, completed) in hist_rows {
            *play_counts.entry(tid.clone()).or_insert(0) += 1;
            if completed {
                *completion_counts.entry(tid).or_insert(0) += 1;
            }
        }

        // Skips
        let skip_rows = sqlx::query_as::<_, (String,)>("SELECT track_id FROM music_skips")
            .fetch_all(pool)
            .await?;
        let mut skip_counts = HashMap::new();
        for (tid,) in skip_rows {
            *skip_counts.entry(tid).or_insert(0) += 1;
        }

        // Artist skips
        let mut artist_skips = HashMap::new();
        for t in &all_tracks {
            if let Some(&skips) = skip_counts.get(&t.id) {
                *artist_skips.entry(t.artist.to_lowercase()).or_insert(0) += skips;
            }
        }

        Ok((
            liked_tracks,
            liked_artists,
            play_counts,
            completion_counts,
            skip_counts,
            artist_skips,
            all_tracks,
        ))
    }

    /// Seed default curated tracks if music_tracks table is empty.
    pub async fn seed_default_tracks_if_empty(pool: &DbPool) -> Result<()> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(1) FROM music_tracks")
            .fetch_one(pool)
            .await?;

        if count.0 > 0 {
            return Ok(());
        }

        let defaults = vec![
            Track {
                id: "seed-chillhop-01".to_string(),
                title: "Coffee & Rain Lofi Beats".to_string(),
                artist: "Chillhop Beats".to_string(),
                duration_secs: Some(174),
                source: "local".to_string(),
                uri: Some("https://streams.ilovemusic.de/iloveradio17.mp3".to_string()),
                extra_json: None,
                created_at: None,
            },
            Track {
                id: "seed-ambient-02".to_string(),
                title: "Groove Salad Ambient Flow".to_string(),
                artist: "SomaFM".to_string(),
                duration_secs: Some(215),
                source: "radio".to_string(),
                uri: Some("https://ice1.somafm.com/groovesalad-128-mp3".to_string()),
                extra_json: None,
                created_at: None,
            },
            Track {
                id: "seed-queen-03".to_string(),
                title: "Bohemian Rhapsody".to_string(),
                artist: "Queen".to_string(),
                duration_secs: Some(354),
                source: "catalog".to_string(),
                uri: Some("https://www.youtube.com/watch?v=fJ9rUzIMcZQ".to_string()),
                extra_json: None,
                created_at: None,
            },
            Track {
                id: "seed-queen-04".to_string(),
                title: "Don't Stop Me Now".to_string(),
                artist: "Queen".to_string(),
                duration_secs: Some(209),
                source: "catalog".to_string(),
                uri: Some("https://www.youtube.com/watch?v=HgzGwKwLmgM".to_string()),
                extra_json: None,
                created_at: None,
            },
            Track {
                id: "seed-synth-05".to_string(),
                title: "Neon Horizon Synthwave".to_string(),
                artist: "Nightwave Plaza".to_string(),
                duration_secs: Some(240),
                source: "radio".to_string(),
                uri: Some("https://radio.plaza.one/mp3".to_string()),
                extra_json: None,
                created_at: None,
            },
            Track {
                id: "seed-classic-06".to_string(),
                title: "Clair de Lune".to_string(),
                artist: "Claude Debussy".to_string(),
                duration_secs: Some(302),
                source: "catalog".to_string(),
                uri: Some("https://media-ice.musicradio.com/ClassicFMMP3".to_string()),
                extra_json: None,
                created_at: None,
            },
            Track {
                id: "seed-jazz-07".to_string(),
                title: "Autumn Leaves Jazz Trio".to_string(),
                artist: "Jazz Radio".to_string(),
                duration_secs: Some(275),
                source: "radio".to_string(),
                uri: Some("https://jazzradio.ice.infomaniak.ch/jazzradio-high.mp3".to_string()),
                extra_json: None,
                created_at: None,
            },
        ];

        for track in defaults {
            Self::upsert_track(pool, &track).await?;
        }

        tracing::info!("Seeded default catalog tracks in music_tracks");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    async fn create_test_pool() -> DbPool {
        let pool = database::init_pool("sqlite::memory:").await.unwrap();
        database::run_migrations(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_track_crud_and_favorites() {
        let pool = create_test_pool().await;

        let track = Track::new("Repository Test Song", "Rustacean", "test");
        MusicRepository::upsert_track(&pool, &track).await.unwrap();

        let loaded = MusicRepository::get_track(&pool, &track.id).await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().title, "Repository Test Song");

        // Favorites
        assert!(!MusicRepository::is_liked(&pool, &track.id).await.unwrap());
        let liked = MusicRepository::toggle_like(&pool, &track.id)
            .await
            .unwrap();
        assert!(liked);
        assert!(MusicRepository::is_liked(&pool, &track.id).await.unwrap());

        let favs = MusicRepository::list_favorites(&pool, 10).await.unwrap();
        assert_eq!(favs.len(), 1);
        assert_eq!(favs[0].id, track.id);

        let unliked = MusicRepository::toggle_like(&pool, &track.id)
            .await
            .unwrap();
        assert!(!unliked);
        assert!(!MusicRepository::is_liked(&pool, &track.id).await.unwrap());
    }

    #[tokio::test]
    async fn test_history_and_skips() {
        let pool = create_test_pool().await;

        let track = Track::new("History Test Song", "History Artist", "test");
        MusicRepository::upsert_track(&pool, &track).await.unwrap();

        // Record history
        let hist_id = MusicRepository::record_history(&pool, &track.id, 120, true)
            .await
            .unwrap();
        assert!(hist_id > 0);

        let history = MusicRepository::list_history(&pool, 10).await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].track_id, track.id);
        assert!(history[0].completed);

        // Record skip
        let skip_id = MusicRepository::record_skip(&pool, &track.id, 15)
            .await
            .unwrap();
        assert!(skip_id > 0);

        let (liked, _, plays, completions, skips, _, _) =
            MusicRepository::get_recommendation_data(&pool)
                .await
                .unwrap();
        assert!(liked.is_empty());
        assert_eq!(*plays.get(&track.id).unwrap(), 1);
        assert_eq!(*completions.get(&track.id).unwrap(), 1);
        assert_eq!(*skips.get(&track.id).unwrap(), 1);
    }
}
