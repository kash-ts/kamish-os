//! YouTube Music client for Kamish.
//! Integrates with the YtMusic microservice (`ytmusic_service.py` or native daemon).

use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YtMusicTrack {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub duration_seconds: Option<u64>,
    pub thumbnail: Option<String>,
    pub uri: String,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    success: bool,
    #[serde(default)]
    tracks: Vec<YtMusicTrack>,
}

#[derive(Debug, Deserialize)]
struct RateResponse {
    success: bool,
}

#[derive(Debug, Clone)]
pub struct YtMusicClient {
    script_path: String,
    python_bin: String,
}

impl Default for YtMusicClient {
    fn default() -> Self {
        Self::new("deploy/scripts/ytmusic_service.py")
    }
}

impl YtMusicClient {
    pub fn new(script_path: &str) -> Self {
        Self {
            script_path: script_path.to_string(),
            python_bin: "python3".to_string(),
        }
    }

    /// Search songs on YouTube Music
    pub async fn search_songs(&self, query: &str, limit: usize) -> Vec<YtMusicTrack> {
        let output = match tokio::time::timeout(
            Duration::from_secs(6),
            Command::new(&self.python_bin)
                .arg(&self.script_path)
                .arg("search")
                .arg(query)
                .arg(limit.to_string())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output(),
        )
        .await
        {
            Ok(Ok(out)) if out.status.success() => out,
            Ok(Ok(out)) => {
                warn!("ytmusic search failed with status: {:?}", out.status);
                return Vec::new();
            }
            Ok(Err(e)) => {
                warn!("ytmusic search execution error: {e}");
                return Vec::new();
            }
            Err(_) => {
                warn!("ytmusic search timed out");
                return Vec::new();
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        match serde_json::from_str::<SearchResponse>(stdout.trim()) {
            Ok(res) => {
                if res.success {
                    return res.tracks;
                }
            }
            Err(e) => {
                warn!("ytmusic search json parse error: {e}, output: {stdout}");
            }
        }
        Vec::new()
    }

    /// Like, dislike or reset rating for a track
    pub async fn rate_track(&self, video_id: &str, rating: &str) -> bool {
        let output = match tokio::time::timeout(
            Duration::from_secs(5),
            Command::new(&self.python_bin)
                .arg(&self.script_path)
                .arg("rate")
                .arg(video_id)
                .arg(rating)
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output(),
        )
        .await
        {
            Ok(Ok(out)) if out.status.success() => out,
            Ok(Ok(out)) => {
                warn!("ytmusic rate failed with status: {:?}", out.status);
                return false;
            }
            Ok(Err(e)) => {
                warn!("ytmusic rate execution error: {e}");
                return false;
            }
            Err(_) => {
                warn!("ytmusic rate timed out");
                return false;
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        match serde_json::from_str::<RateResponse>(stdout.trim()) {
            Ok(res) => {
                if res.success {
                    return true;
                }
            }
            Err(e) => {
                warn!("ytmusic rate json parse error: {e}, output: {stdout}");
            }
        }
        false
    }
}
