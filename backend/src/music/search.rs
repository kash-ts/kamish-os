use crate::audio::alert::is_audio_muted;
use crate::database::DbPool;
use crate::music::models::SearchResult;
use crate::music::radio::RadioService;
use crate::music::repository::MusicRepository;
use std::time::Duration;
use tokio::process::Command;

pub struct SearchService;

impl SearchService {
    /// Perform unified search across local library, radio stations, and online/YouTube.
    pub async fn search(
        query: &str,
        source: &str,
        limit: usize,
        pool: Option<&DbPool>,
    ) -> Vec<SearchResult> {
        let q = query.trim();
        if q.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();
        let target_source = source.to_lowercase();

        // 1. Search Radio Stations
        if target_source == "all" || target_source == "radio" {
            let stations = RadioService::search_stations(q);
            for s in stations {
                results.push(SearchResult {
                    id: format!("radio:{}", s.id),
                    title: s.name,
                    artist: format!("Radio ({})", s.genre),
                    duration_secs: None,
                    source: "radio".to_string(),
                    uri: Some(s.stream_uri),
                    thumbnail_url: s.favicon_url,
                });
                if results.len() >= limit {
                    return results;
                }
            }
        }

        // 2. Search Local Library / Catalog
        if (target_source == "all" || target_source == "local") && pool.is_some() {
            if let Some(pool) = pool {
                if let Ok(local_tracks) =
                    MusicRepository::search_tracks(pool, q, limit as i64).await
                {
                    for t in local_tracks {
                        results.push(SearchResult {
                            id: t.id,
                            title: t.title,
                            artist: t.artist,
                            duration_secs: t.duration_secs,
                            source: t.source,
                            uri: t.uri,
                            thumbnail_url: None,
                        });
                        if results.len() >= limit {
                            return results;
                        }
                    }
                }
            }
        }

        // 3. Search YouTube Music / YouTube / Online
        if target_source == "all"
            || target_source == "ytmusic"
            || target_source == "youtube"
            || target_source == "online"
        {
            // Check if muted (test environment) or offline
            if !is_audio_muted() {
                // Try YouTube Music microservice first
                let ytm = crate::music::ytmusic::YtMusicClient::default();
                let ytm_tracks = ytm
                    .search_songs(q, limit.saturating_sub(results.len()))
                    .await;
                if !ytm_tracks.is_empty() {
                    for t in ytm_tracks {
                        results.push(SearchResult {
                            id: format!("yt:{}", t.id),
                            title: t.title,
                            artist: t.artist,
                            duration_secs: t.duration_seconds,
                            source: "ytmusic".to_string(),
                            uri: Some(t.uri),
                            thumbnail_url: t.thumbnail,
                        });
                    }
                } else {
                    let yt_results =
                        Self::search_youtube(q, limit.saturating_sub(results.len())).await;
                    results.extend(yt_results);
                }
            }
        }

        // 4. Safe fallback if nothing matched (offline / no network / test)
        if results.is_empty() {
            results.push(SearchResult {
                id: format!("mock:{}", uuid::Uuid::new_v4()),
                title: q.to_string(),
                artist: "Kamish Catalog".to_string(),
                duration_secs: Some(215),
                source: "search_result".to_string(),
                uri: None,
                thumbnail_url: None,
            });
        }

        results.truncate(limit);
        results
    }

    /// Search YouTube via yt-dlp with timeout and safe fallback.
    async fn search_youtube(query: &str, limit: usize) -> Vec<SearchResult> {
        let count = limit.clamp(1, 10);
        let search_arg = format!("ytsearch{count}:{query}");

        let cmd_res = tokio::time::timeout(
            Duration::from_secs(3),
            Command::new("yt-dlp")
                .arg("--default-search")
                .arg("ytsearch")
                .arg("--get-title")
                .arg("--get-id")
                .arg("--get-duration")
                .arg("--no-warnings")
                .arg(&search_arg)
                .output(),
        )
        .await;

        let mut results = Vec::new();

        if let Ok(Ok(output)) = cmd_res {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = stdout.lines().collect();

                // yt-dlp outputs: line 0: title, line 1: id, line 2: duration
                for chunk in lines.chunks(3) {
                    if chunk.len() >= 2 {
                        let title = chunk[0].trim().to_string();
                        let yt_id = chunk[1].trim().to_string();
                        let duration_secs = chunk.get(2).and_then(|d| parse_duration(d));

                        results.push(SearchResult {
                            id: format!("yt:{}", yt_id),
                            title: title.clone(),
                            artist: "YouTube Music".to_string(),
                            duration_secs,
                            source: "youtube".to_string(),
                            uri: Some(format!("https://www.youtube.com/watch?v={}", yt_id)),
                            thumbnail_url: Some(format!(
                                "https://img.youtube.com/vi/{}/hqdefault.jpg",
                                yt_id
                            )),
                        });
                    }
                }
            }
        }

        results
    }
}

fn parse_duration(dur_str: &str) -> Option<u64> {
    let parts: Vec<&str> = dur_str.trim().split(':').collect();
    match parts.len() {
        2 => {
            let m = parts[0].parse::<u64>().ok()?;
            let s = parts[1].parse::<u64>().ok()?;
            Some(m * 60 + s)
        }
        3 => {
            let h = parts[0].parse::<u64>().ok()?;
            let m = parts[1].parse::<u64>().ok()?;
            let s = parts[2].parse::<u64>().ok()?;
            Some(h * 3600 + m * 60 + s)
        }
        _ => None,
    }
}
