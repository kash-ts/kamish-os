use crate::audio::alert::is_audio_muted;
use crate::audio::set_system_volume;
use crate::music::models::{PlaybackState, PlaybackStatus, Track};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info};

struct PlayerInternalState {
    status: PlaybackStatus,
    current_track: Option<Track>,
    started_at: Option<Instant>,
    accumulated_position_secs: u64,
}

pub struct MpvPlayer {
    state: RwLock<PlayerInternalState>,
    volume: AtomicU8,
    ipc_path: PathBuf,
    mpv_process: Mutex<Option<Child>>,
}

impl Default for MpvPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MpvPlayer {
    pub fn new() -> Self {
        let unique_id = uuid::Uuid::new_v4().to_string();
        let ipc_path =
            std::env::temp_dir().join(format!("kamish_core_mpv_{}.sock", &unique_id[..8]));

        Self {
            state: RwLock::new(PlayerInternalState {
                status: PlaybackStatus::Stopped,
                current_track: None,
                started_at: None,
                accumulated_position_secs: 0,
            }),
            volume: AtomicU8::new(50),
            ipc_path,
            mpv_process: Mutex::new(None),
        }
    }

    /// Ensure MPV subprocess is running in idle mode with IPC server socket.
    async fn ensure_mpv_running(&self) -> bool {
        if is_audio_muted() {
            return false;
        }

        let mut child_guard = self.mpv_process.lock().await;
        if let Some(ref mut child) = *child_guard {
            // Check if process is still alive
            match child.try_wait() {
                Ok(None) => return true, // running
                _ => {
                    *child_guard = None;
                }
            }
        }

        // Clean up stale socket file if it exists
        let _ = tokio::fs::remove_file(&self.ipc_path).await;

        let ipc_arg = format!("--input-ipc-server={}", self.ipc_path.to_string_lossy());
        let mut cmd = Command::new("mpv");
        cmd.arg("--idle=yes")
            .arg("--no-video")
            .arg(&ipc_arg)
            .arg("--ao=pulse,alsa,null")
            .arg("--ytdl=yes");

        let cookies_path = std::env::var("KAMISH_YOUTUBE_COOKIES_PATH")
            .unwrap_or_else(|_| "/var/lib/kamish/youtube-cookies.txt".to_string());
        if std::path::Path::new(&cookies_path).exists() {
            cmd.arg(format!("--ytdl-raw-options=cookies={cookies_path}"));
        }

        let spawn_res = cmd
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        match spawn_res {
            Ok(child) => {
                *child_guard = Some(child);
                // Give MPV a brief moment to create the unix socket
                for _ in 0..10 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    if self.ipc_path.exists() {
                        debug!("MPV daemon started with IPC: {:?}", self.ipc_path);
                        return true;
                    }
                }
                debug!("MPV process started, socket creation pending");
                true
            }
            Err(e) => {
                debug!("MPV binary not available or failed to spawn: {e}. Using virtual playback.");
                false
            }
        }
    }

    /// Send a JSON command to the MPV IPC Unix domain socket.
    async fn send_ipc_command(&self, cmd: serde_json::Value) -> bool {
        if is_audio_muted() || !self.ipc_path.exists() {
            return false;
        }

        let stream_res = UnixStream::connect(&self.ipc_path).await;
        match stream_res {
            Ok(mut stream) => {
                let mut data = serde_json::to_vec(&cmd).unwrap_or_default();
                data.push(b'\n');
                if let Err(e) = stream.write_all(&data).await {
                    debug!("Failed to write to MPV IPC socket: {e}");
                    false
                } else {
                    true
                }
            }
            Err(e) => {
                debug!("Cannot connect to MPV IPC socket: {e}");
                false
            }
        }
    }

    /// Play a track (URL, stream, or query).
    pub async fn play(&self, track: Track) {
        let uri = track
            .uri
            .clone()
            .unwrap_or_else(|| format!("ytdl://{}", track.title));

        {
            let mut st = self.state.write().await;
            st.status = PlaybackStatus::Playing;
            st.current_track = Some(track.clone());
            st.started_at = Some(Instant::now());
            st.accumulated_position_secs = 0;
        }

        if !is_audio_muted() && self.ensure_mpv_running().await {
            // Load file in MPV
            self.send_ipc_command(serde_json::json!({
                "command": ["loadfile", uri, "replace"]
            }))
            .await;

            // Apply current volume
            let vol = self.volume.load(Ordering::Relaxed);
            self.send_ipc_command(serde_json::json!({
                "command": ["set_property", "volume", vol]
            }))
            .await;
        }

        info!("Playback started: {} - {}", track.artist, track.title);
    }

    /// Pause current playback.
    pub async fn pause(&self) {
        {
            let mut st = self.state.write().await;
            if st.status == PlaybackStatus::Playing {
                if let Some(started) = st.started_at {
                    st.accumulated_position_secs += started.elapsed().as_secs();
                }
                st.started_at = None;
                st.status = PlaybackStatus::Paused;
            }
        }

        if !is_audio_muted() {
            self.send_ipc_command(serde_json::json!({
                "command": ["set_property", "pause", true]
            }))
            .await;
        }

        debug!("Playback paused");
    }

    /// Resume playback.
    pub async fn resume(&self) {
        {
            let mut st = self.state.write().await;
            if st.status == PlaybackStatus::Paused {
                st.started_at = Some(Instant::now());
                st.status = PlaybackStatus::Playing;
            }
        }

        if !is_audio_muted() {
            self.send_ipc_command(serde_json::json!({
                "command": ["set_property", "pause", false]
            }))
            .await;
        }

        debug!("Playback resumed");
    }

    /// Stop playback completely.
    pub async fn stop(&self) {
        {
            let mut st = self.state.write().await;
            st.status = PlaybackStatus::Stopped;
            st.current_track = None;
            st.started_at = None;
            st.accumulated_position_secs = 0;
        }

        if !is_audio_muted() {
            self.send_ipc_command(serde_json::json!({
                "command": ["stop"]
            }))
            .await;
        }

        debug!("Playback stopped");
    }

    /// Set volume (0-100).
    pub async fn set_volume(&self, vol: u8) -> u8 {
        let volume = vol.min(100);
        self.volume.store(volume, Ordering::Relaxed);
        set_system_volume(volume).await;

        if !is_audio_muted() {
            self.send_ipc_command(serde_json::json!({
                "command": ["set_property", "volume", volume]
            }))
            .await;
        }

        volume
    }

    /// Get volume.
    pub fn get_volume(&self) -> u8 {
        self.volume.load(Ordering::Relaxed)
    }

    /// Get current playback status.
    pub async fn get_status(&self) -> PlaybackStatus {
        let st = self.state.read().await;
        st.status
    }

    /// Get current track.
    pub async fn get_current_track(&self) -> Option<Track> {
        let st = self.state.read().await;
        st.current_track.clone()
    }

    /// Get full playback state including elapsed position.
    pub async fn get_playback_state(&self) -> PlaybackState {
        let st = self.state.read().await;
        let position_secs = match st.status {
            PlaybackStatus::Playing => {
                let elapsed = st.started_at.map(|t| t.elapsed().as_secs()).unwrap_or(0);
                st.accumulated_position_secs + elapsed
            }
            PlaybackStatus::Paused => st.accumulated_position_secs,
            PlaybackStatus::Stopped => 0,
        };

        let duration_secs = st.current_track.as_ref().and_then(|t| t.duration_secs);

        PlaybackState {
            status: st.status,
            current_track: st.current_track.clone(),
            volume: self.volume.load(Ordering::Relaxed),
            position_secs,
            duration_secs,
        }
    }
}

impl Drop for MpvPlayer {
    fn drop(&mut self) {
        if self.ipc_path.exists() {
            let _ = std::fs::remove_file(&self.ipc_path);
        }
    }
}
