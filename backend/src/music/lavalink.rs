//! Lavalink v4 Client for multi-source music streaming.
//! Supports YouTube, Yandex Music (via LavaSrc plugin), and SoundCloud.

use crate::music::models::SearchResult;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Configuration parameters for connecting to Lavalink v4.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LavalinkConfig {
    pub host: String,
    pub port: u16,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yandex_music_token: Option<String>,
}

impl Default for LavalinkConfig {
    fn default() -> Self {
        Self {
            host: std::env::var("LAVALINK_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("LAVALINK_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(2333),
            password: std::env::var("LAVALINK_PASSWORD")
                .unwrap_or_else(|_| "youshallnotpass".to_string()),
            yandex_music_token: std::env::var("YANDEX_MUSIC_TOKEN")
                .ok()
                .filter(|t| !t.trim().is_empty()),
        }
    }
}

/// Runtime status of the Lavalink node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LavalinkStatus {
    pub connected: bool,
    pub host: String,
    pub port: u16,
    pub version: Option<String>,
    pub sources: Vec<String>,
    pub plugins: Vec<String>,
    pub yandex_music_configured: bool,
}

/// Lavalink v4 REST API client.
#[derive(Clone)]
pub struct LavalinkClient {
    client: reqwest::Client,
    config: Arc<RwLock<LavalinkConfig>>,
}

impl Default for LavalinkClient {
    fn default() -> Self {
        Self::new(LavalinkConfig::default())
    }
}

impl LavalinkClient {
    /// Create a new Lavalink client instance with given configuration.
    pub fn new(config: LavalinkConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        Self {
            client,
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Retrieve the current client configuration.
    pub async fn get_config(&self) -> LavalinkConfig {
        self.config.read().await.clone()
    }

    /// Update client configuration in memory.
    pub async fn update_config(&self, new_cfg: LavalinkConfig) {
        let mut w = self.config.write().await;
        *w = new_cfg;
    }

    /// Base URL of the Lavalink node.
    pub async fn base_url(&self) -> String {
        let cfg = self.config.read().await;
        format!("http://{}:{}", cfg.host, cfg.port)
    }

    /// Check if the Lavalink server is reachable and authorized.
    pub async fn is_alive(&self) -> bool {
        let (host, port, password) = {
            let cfg = self.config.read().await;
            (cfg.host.clone(), cfg.port, cfg.password.clone())
        };

        let url = format!("http://{}:{}/v4/info", host, port);
        match self
            .client
            .get(&url)
            .header("Authorization", password)
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Query node information and active plugin status.
    pub async fn get_status(&self) -> LavalinkStatus {
        let cfg = self.config.read().await.clone();
        let url = format!("http://{}:{}/v4/info", cfg.host, cfg.port);

        let resp = self
            .client
            .get(&url)
            .header("Authorization", &cfg.password)
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                if let Ok(info) = r.json::<serde_json::Value>().await {
                    let version = info
                        .get("version")
                        .and_then(|v| v.get("semver"))
                        .and_then(|s| s.as_str())
                        .map(ToString::to_string);

                    let mut sources = Vec::new();
                    if let Some(srcs) = info.get("sourceManagers").and_then(|s| s.as_array()) {
                        for s in srcs {
                            if let Some(name) = s.as_str() {
                                sources.push(name.to_string());
                            }
                        }
                    }

                    let mut plugins = Vec::new();
                    if let Some(plugs) = info.get("plugins").and_then(|p| p.as_array()) {
                        for p in plugs {
                            if let Some(name) = p.get("name").and_then(|n| n.as_str()) {
                                plugins.push(name.to_string());
                            }
                        }
                    }

                    LavalinkStatus {
                        connected: true,
                        host: cfg.host,
                        port: cfg.port,
                        version,
                        sources,
                        plugins,
                        yandex_music_configured: cfg.yandex_music_token.is_some(),
                    }
                } else {
                    LavalinkStatus {
                        connected: true,
                        host: cfg.host,
                        port: cfg.port,
                        version: None,
                        sources: vec![
                            "youtube".to_string(),
                            "yandexmusic".to_string(),
                            "soundcloud".to_string(),
                        ],
                        plugins: vec!["youtube-plugin".to_string(), "lavasrc-plugin".to_string()],
                        yandex_music_configured: cfg.yandex_music_token.is_some(),
                    }
                }
            }
            _ => LavalinkStatus {
                connected: false,
                host: cfg.host,
                port: cfg.port,
                version: None,
                sources: vec![],
                plugins: vec![],
                yandex_music_configured: cfg.yandex_music_token.is_some(),
            },
        }
    }

    /// Load tracks from Lavalink via `/v4/loadtracks`.
    /// Identifiers can be:
    /// - `ytsearch:<query>` (YouTube)
    /// - `ytmsearch:<query>` (YouTube Music)
    /// - `ymsearch:<query>` (Yandex Music via LavaSrc)
    /// - `scsearch:<query>` (SoundCloud)
    /// - Direct URLs (YouTube, Yandex Music, SoundCloud)
    pub async fn load_tracks(&self, identifier: &str) -> Vec<SearchResult> {
        let (host, port, password) = {
            let cfg = self.config.read().await;
            (cfg.host.clone(), cfg.port, cfg.password.clone())
        };

        let url = format!("http://{}:{}/v4/loadtracks", host, port);
        let res = match self
            .client
            .get(&url)
            .header("Authorization", password)
            .query(&[("identifier", identifier)])
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                debug!("Lavalink /v4/loadtracks HTTP status: {}", r.status());
                return Vec::new();
            }
            Err(e) => {
                debug!("Lavalink request to {url} failed: {e}");
                return Vec::new();
            }
        };

        let body_val = match res.json::<serde_json::Value>().await {
            Ok(v) => v,
            Err(e) => {
                debug!("Failed to decode Lavalink JSON response: {e}");
                return Vec::new();
            }
        };

        let mut out = Vec::new();
        let load_type = body_val
            .get("loadType")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let data = body_val.get("data");

        match load_type {
            "search" => {
                if let Some(arr) = data.and_then(|d| d.as_array()) {
                    for item in arr {
                        if let Some(sr) = Self::parse_track_item(item) {
                            out.push(sr);
                        }
                    }
                }
            }
            "track" => {
                if let Some(item) = data {
                    if let Some(sr) = Self::parse_track_item(item) {
                        out.push(sr);
                    }
                }
            }
            "playlist" => {
                if let Some(tracks) = data
                    .and_then(|d| d.get("tracks"))
                    .and_then(|t| t.as_array())
                {
                    for item in tracks {
                        if let Some(sr) = Self::parse_track_item(item) {
                            out.push(sr);
                        }
                    }
                }
            }
            "empty" => {
                debug!("Lavalink returned empty results for: {identifier}");
            }
            "error" => {
                let msg = data
                    .and_then(|d| d.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown error");
                warn!("Lavalink load error: {msg}");
            }
            other => {
                debug!("Unhandled Lavalink loadType: {other}");
            }
        }

        out
    }

    /// Convert a Lavalink Track JSON node to a unified `SearchResult`.
    pub fn parse_track_item(val: &serde_json::Value) -> Option<SearchResult> {
        let info = val.get("info")?;
        let title = info.get("title")?.as_str()?.to_string();
        let author = info.get("author")?.as_str()?.to_string();
        let identifier = info.get("identifier")?.as_str()?.to_string();
        let length_ms = info.get("length").and_then(|l| l.as_u64());
        let uri = info
            .get("uri")
            .and_then(|u| u.as_str())
            .map(ToString::to_string);
        let artwork_url = info
            .get("artworkUrl")
            .and_then(|a| a.as_str())
            .map(ToString::to_string);
        let raw_source = info
            .get("sourceName")
            .and_then(|s| s.as_str())
            .unwrap_or("lavalink");

        let normalized_source = match raw_source {
            "youtube" => "youtube".to_string(),
            "yandexmusic" | "yandex" => "yandexmusic".to_string(),
            "soundcloud" => "soundcloud".to_string(),
            other => other.to_string(),
        };

        Some(SearchResult {
            id: format!("{}:{}", normalized_source, identifier),
            title,
            artist: author,
            duration_secs: length_ms.map(|ms| ms / 1000),
            source: normalized_source,
            uri,
            thumbnail_url: artwork_url,
        })
    }

    /// Search specifically on YouTube via Lavalink.
    pub async fn search_youtube(&self, query: &str) -> Vec<SearchResult> {
        self.load_tracks(&format!("ytsearch:{query}")).await
    }

    /// Search specifically on Yandex Music via Lavalink LavaSrc.
    pub async fn search_yandex(&self, query: &str) -> Vec<SearchResult> {
        self.load_tracks(&format!("ymsearch:{query}")).await
    }

    /// Search specifically on SoundCloud via Lavalink.
    pub async fn search_soundcloud(&self, query: &str) -> Vec<SearchResult> {
        self.load_tracks(&format!("scsearch:{query}")).await
    }

    /// Parallel multi-source search across YouTube, Yandex Music, and SoundCloud.
    pub async fn search_all_sources(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let (yt_res, ym_res, sc_res) = tokio::join!(
            self.search_youtube(query),
            self.search_yandex(query),
            self.search_soundcloud(query)
        );

        let mut combined = Vec::new();
        let max_len = yt_res.len().max(ym_res.len()).max(sc_res.len());

        // Interleave results fairly across providers
        for i in 0..max_len {
            if let Some(item) = ym_res.get(i) {
                combined.push(item.clone());
            }
            if let Some(item) = yt_res.get(i) {
                combined.push(item.clone());
            }
            if let Some(item) = sc_res.get(i) {
                combined.push(item.clone());
            }
            if combined.len() >= limit {
                break;
            }
        }

        combined.truncate(limit);
        combined
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lavalink_config_defaults() {
        let cfg = LavalinkConfig::default();
        assert_eq!(cfg.port, 2333);
        assert_eq!(cfg.password, "youshallnotpass");
    }

    #[test]
    fn test_parse_track_item() {
        let json_str = r#"{
            "encoded": "QAAAbAI...",
            "info": {
                "identifier": "dQw4w9WgXcQ",
                "isSeekable": true,
                "author": "Rick Astley",
                "length": 212000,
                "isStream": false,
                "position": 0,
                "title": "Never Gonna Give You Up",
                "uri": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
                "artworkUrl": "https://i.ytimg.com/vi/dQw4w9WgXcQ/hqdefault.jpg",
                "isrc": null,
                "sourceName": "youtube"
            }
        }"#;

        let val: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let track = LavalinkClient::parse_track_item(&val).expect("parsed track");

        assert_eq!(track.id, "youtube:dQw4w9WgXcQ");
        assert_eq!(track.title, "Never Gonna Give You Up");
        assert_eq!(track.artist, "Rick Astley");
        assert_eq!(track.duration_secs, Some(212));
        assert_eq!(track.source, "youtube");
        assert_eq!(
            track.uri,
            Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string())
        );
    }

    #[test]
    fn test_parse_yandex_track_item() {
        let json_str = r#"{
            "encoded": "QAAAbAI...",
            "info": {
                "identifier": "123456",
                "isSeekable": true,
                "author": "Кино",
                "length": 255000,
                "isStream": false,
                "position": 0,
                "title": "Группа крови",
                "uri": "https://music.yandex.ru/track/123456",
                "artworkUrl": "https://avatars.yandex.net/get-music-content/123/hq",
                "isrc": null,
                "sourceName": "yandexmusic"
            }
        }"#;

        let val: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let track = LavalinkClient::parse_track_item(&val).expect("parsed yandex track");

        assert_eq!(track.id, "yandexmusic:123456");
        assert_eq!(track.title, "Группа крови");
        assert_eq!(track.artist, "Кино");
        assert_eq!(track.source, "yandexmusic");
        assert_eq!(track.duration_secs, Some(255));
    }
}
