use crate::audio::output::DEFAULT_SAFE_VOLUME_CEILING;
use crate::common::AppState;
use crate::setup::models::StepStatus;
use crate::system::hardware;
use kamish_ipc::{CoreRequest, CoreResponse, RequestHandler};
use tracing::{debug, info};

pub struct CoreIpcHandler {
    state: AppState,
}

impl CoreIpcHandler {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl RequestHandler for CoreIpcHandler {
    async fn handle_request(&self, request: CoreRequest) -> CoreResponse {
        debug!(
            "Core received IPC request: {:?}",
            std::mem::discriminant(&request)
        );
        match request {
            // Status & Hardware Telemetry
            CoreRequest::GetStatus => {
                let routing = self.state.audio.get_routing_info().await;
                let telemetry = hardware::get_hardware_telemetry(routing);
                CoreResponse::ok(serde_json::json!({
                    "version": self.state.version,
                    "uptime_seconds": self.state.start_time.elapsed().as_secs(),
                    "cpu_cores": telemetry.cpu_cores,
                    "cpu_temp_celsius": telemetry.cpu_temp_celsius,
                    "memory_used_bytes": telemetry.memory_used_bytes,
                    "memory_total_bytes": telemetry.memory_total_bytes,
                    "storage_used_bytes": telemetry.storage_used_bytes,
                    "storage_total_bytes": telemetry.storage_total_bytes,
                    "throttled": telemetry.throttled,
                    "safe_volume_ceiling": DEFAULT_SAFE_VOLUME_CEILING,
                    "board_name": telemetry.model,
                }))
            }
            CoreRequest::GetHardwareTelemetry => {
                let routing = self.state.audio.get_routing_info().await;
                let telemetry = hardware::get_hardware_telemetry(routing);
                CoreResponse::ok(telemetry)
            }

            // Audio & Playback
            CoreRequest::GetAudioStatus => {
                let vol = self.state.media.get_volume();
                let is_muted = self.state.audio.is_muted();
                CoreResponse::ok(serde_json::json!({
                    "volume": vol,
                    "max_volume": DEFAULT_SAFE_VOLUME_CEILING,
                    "muted": is_muted,
                    "sink": "default",
                }))
            }
            CoreRequest::SetVolume { volume } => {
                // Strict enforcement of Safe Volume Ceiling <= 85%
                let safe_vol = volume.min(DEFAULT_SAFE_VOLUME_CEILING);
                self.state
                    .media
                    .set_volume(safe_vol, Some(&self.state.db))
                    .await;
                self.state.audio.set_volume(safe_vol);
                self.state
                    .events
                    .emit("volume_changed", serde_json::json!({ "volume": safe_vol }));
                CoreResponse::ok(serde_json::json!({
                    "volume": safe_vol,
                    "max_volume": DEFAULT_SAFE_VOLUME_CEILING,
                }))
            }
            CoreRequest::SetVolumeCeiling { ceiling } => {
                // The ceiling can be lowered by the user but never above the hard 85% limit.
                let safe_ceiling = ceiling.clamp(1, DEFAULT_SAFE_VOLUME_CEILING);
                self.state.audio.set_safe_ceiling(safe_ceiling);
                CoreResponse::ok(serde_json::json!({
                    "success": true,
                    "requested_volume": self.state.audio.get_volume(),
                    "applied_volume": self.state.audio.get_volume(),
                    "safe_ceiling": safe_ceiling,
                }))
            }
            CoreRequest::ListAudioDevices => {
                // Restore the persisted selection after a restart.
                for (kind, key) in [
                    ("output", "audio:output_device"),
                    ("input", "audio:input_device"),
                ] {
                    let current = if kind == "output" {
                        crate::audio::devices::selected_output()
                    } else {
                        crate::audio::devices::selected_input()
                    };
                    if current.is_none() {
                        if let Ok(Some((val,))) = sqlx::query_as::<_, (String,)>(
                            "SELECT value FROM device_state WHERE key = ?",
                        )
                        .bind(key)
                        .fetch_optional(&self.state.db)
                        .await
                        {
                            crate::audio::devices::set_selected(kind, &val);
                        }
                    }
                }
                CoreResponse::ok(crate::audio::devices::list_devices().await)
            }
            CoreRequest::SetAudioDevice { kind, id } => {
                if !crate::audio::devices::set_selected(&kind, &id) {
                    return CoreResponse::err(
                        "validation_error",
                        "kind должен быть 'output' или 'input'",
                    );
                }
                let key = format!("audio:{kind}_device");
                let _ = sqlx::query(
                    r#"
                    INSERT INTO device_state (key, value, updated_at)
                    VALUES (?, ?, ?)
                    ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
                    "#,
                )
                .bind(&key)
                .bind(&id)
                .bind(chrono::Utc::now().to_rfc3339())
                .execute(&self.state.db)
                .await;
                CoreResponse::ok(serde_json::json!({ "kind": kind, "id": id }))
            }
            CoreRequest::PlayTestTone => {
                if self.state.audio.is_muted() {
                    return CoreResponse::err("audio_muted", "Звук отключён");
                }
                self.state
                    .audio
                    .play_test_tone(crate::audio::alert::AlertType::Reminder)
                    .await;
                CoreResponse::ok(serde_json::json!({ "status": "played" }))
            }
            CoreRequest::PlayTrack { title, uri, artist } => {
                let mut t = crate::music::models::Track::new(
                    &title,
                    artist.as_deref().unwrap_or("Kamish Artist"),
                    "manual",
                );
                t.uri = uri;
                self.state
                    .media
                    .play_track(t.clone(), Some(&self.state.db))
                    .await;
                CoreResponse::ok(t)
            }
            CoreRequest::Pause => {
                self.state.media.pause(Some(&self.state.db)).await;
                CoreResponse::ok(serde_json::json!({ "status": "paused" }))
            }
            CoreRequest::Resume => {
                self.state.media.resume(Some(&self.state.db)).await;
                CoreResponse::ok(serde_json::json!({ "status": "playing" }))
            }
            CoreRequest::Stop => {
                self.state.media.stop(Some(&self.state.db)).await;
                CoreResponse::ok(serde_json::json!({ "status": "stopped" }))
            }
            CoreRequest::Next => {
                let skipped = self.state.media.skip(None, Some(&self.state.db)).await;
                CoreResponse::ok(serde_json::json!({ "skipped": skipped }))
            }
            CoreRequest::Previous => CoreResponse::ok(serde_json::json!({ "status": "previous" })),
            CoreRequest::GetPlaybackState => {
                let current = self.state.media.get_playback_state().await;
                CoreResponse::ok(current)
            }
            CoreRequest::SearchMusic {
                query,
                source,
                limit,
            } => {
                let results = self
                    .state
                    .media
                    .search(
                        &query,
                        source.as_deref().unwrap_or("all"),
                        limit,
                        Some(&self.state.db),
                    )
                    .await;
                CoreResponse::ok(results)
            }
            CoreRequest::GetMusicHistory { limit } => {
                match self
                    .state
                    .media
                    .get_history(&self.state.db, limit as i64)
                    .await
                {
                    Ok(history) => CoreResponse::ok(history),
                    Err(e) => CoreResponse::err("db_error", e.to_string()),
                }
            }
            CoreRequest::GetRecommendations { limit } => {
                match self
                    .state
                    .media
                    .get_recommendations(&self.state.db, limit)
                    .await
                {
                    Ok(recs) => CoreResponse::ok(recs),
                    Err(e) => CoreResponse::err("db_error", e.to_string()),
                }
            }
            CoreRequest::GetYouTubeAuthStatus => {
                let status = self.state.youtube_auth.status().await;
                CoreResponse::ok(status)
            }
            CoreRequest::StartYouTubeAuth => {
                let client_id = std::env::var("YOUTUBE_CLIENT_ID").unwrap_or_default();
                let client_secret = std::env::var("YOUTUBE_CLIENT_SECRET").unwrap_or_default();
                match self
                    .state
                    .youtube_auth
                    .begin(&client_id, &client_secret)
                    .await
                {
                    Ok(resp) => CoreResponse::ok(resp),
                    Err(e) => CoreResponse::err("youtube_auth_error", e.to_string()),
                }
            }
            CoreRequest::PollYouTubeAuth => {
                let status = self.state.youtube_auth.status().await;
                CoreResponse::ok(status)
            }

            // Scheduler
            CoreRequest::ListAlarms => {
                match crate::scheduler::repository::list_alarms(&self.state.db).await {
                    Ok(alarms) => CoreResponse::ok(alarms),
                    Err(e) => CoreResponse::err("db_error", e.to_string()),
                }
            }
            CoreRequest::CreateAlarm {
                time,
                days,
                sound: _,
                enabled: _,
                label,
            } => {
                let req = crate::scheduler::models::CreateAlarmRequest {
                    time,
                    label,
                    recurrence: if days.is_empty() {
                        None
                    } else {
                        Some("daily".to_string())
                    },
                    timezone: None,
                    enabled: Some(true),
                };
                match crate::scheduler::repository::create_alarm(&self.state.db, &req).await {
                    Ok(alarm) => {
                        self.state.scheduler.notify_modified();
                        CoreResponse::ok(alarm)
                    }
                    Err(e) => CoreResponse::err("scheduler_error", e.to_string()),
                }
            }
            CoreRequest::UpdateAlarm {
                id,
                time: _,
                days: _,
                sound: _,
                enabled,
                label: _,
            } => {
                if let Some(en) = enabled {
                    let _ =
                        crate::scheduler::repository::toggle_alarm(&self.state.db, &id, Some(en))
                            .await;
                }
                self.state.scheduler.notify_modified();
                CoreResponse::ok(serde_json::json!({ "id": id, "updated": true }))
            }
            CoreRequest::DeleteAlarm { id } => {
                match crate::scheduler::repository::delete_alarm(&self.state.db, &id).await {
                    Ok(deleted) => {
                        self.state.scheduler.notify_modified();
                        CoreResponse::ok(serde_json::json!({ "deleted": deleted }))
                    }
                    Err(e) => CoreResponse::err("scheduler_error", e.to_string()),
                }
            }
            CoreRequest::ListReminders => {
                match crate::scheduler::repository::list_reminders(&self.state.db, None).await {
                    Ok(reminders) => CoreResponse::ok(reminders),
                    Err(e) => CoreResponse::err("db_error", e.to_string()),
                }
            }
            CoreRequest::CreateReminder { text, trigger_at } => {
                let req = crate::scheduler::models::CreateReminderRequest {
                    text,
                    trigger_time: trigger_at,
                    timezone: None,
                };
                match crate::scheduler::repository::create_reminder(&self.state.db, &req).await {
                    Ok(reminder) => {
                        self.state.scheduler.notify_modified();
                        CoreResponse::ok(reminder)
                    }
                    Err(e) => CoreResponse::err("scheduler_error", e.to_string()),
                }
            }
            CoreRequest::DeleteReminder { id } => {
                match crate::scheduler::repository::delete_reminder(&self.state.db, &id).await {
                    Ok(deleted) => {
                        self.state.scheduler.notify_modified();
                        CoreResponse::ok(serde_json::json!({ "deleted": deleted }))
                    }
                    Err(e) => CoreResponse::err("scheduler_error", e.to_string()),
                }
            }

            // Speech & VAD
            CoreRequest::GetSpeechStatus => {
                let status = self.state.speech.get_status();
                CoreResponse::ok(status)
            }
            CoreRequest::SetWakeWordDetection { enabled } => {
                let req = crate::speech::models::UpdateSpeechConfigRequest {
                    active_stt_provider: None,
                    active_tts_provider: None,
                    active_voice: None,
                    vad_enabled: None,
                    wakeword_enabled: Some(enabled),
                };
                let _ = self.state.speech.update_config(req);
                CoreResponse::ok(self.state.speech.get_status())
            }
            CoreRequest::SetVadThreshold { threshold } => {
                let req = crate::speech::models::UpdateSpeechConfigRequest {
                    active_stt_provider: None,
                    active_tts_provider: None,
                    active_voice: None,
                    vad_enabled: Some(threshold > 0.0),
                    wakeword_enabled: None,
                };
                let _ = self.state.speech.update_config(req);
                CoreResponse::ok(self.state.speech.get_status())
            }
            CoreRequest::StartCapture => {
                let started = self.state.capture.start(self.state.clone()).await;
                CoreResponse::ok(serde_json::json!({ "capture_started": started }))
            }
            CoreRequest::StopCapture => {
                self.state.capture.stop().await;
                CoreResponse::ok(serde_json::json!({ "capture_started": false }))
            }
            CoreRequest::SynthesizeSpeech { text } => {
                match self.state.speech.synthesize(&text, None, None, None).await {
                    Ok(resp) => CoreResponse::ok(resp),
                    Err(e) => CoreResponse::err("tts_error", e.to_string()),
                }
            }

            // AI
            CoreRequest::ExecutePrompt { prompt, session_id } => {
                match self
                    .state
                    .ai
                    .ask(&prompt, session_id.as_deref(), &self.state)
                    .await
                {
                    Ok(reply) => CoreResponse::ok(reply),
                    Err(e) => CoreResponse::err("ai_error", e.to_string()),
                }
            }
            CoreRequest::GetAgyStatus => {
                let status = self.state.agy.status().await;
                CoreResponse::ok(status)
            }
            CoreRequest::ClearAiSession { session_id } => {
                let _ = sqlx::query("DELETE FROM ai_sessions WHERE id = ?")
                    .bind(&session_id)
                    .execute(&self.state.db)
                    .await;
                CoreResponse::ok(serde_json::json!({ "cleared": true }))
            }

            // Network & Wi-Fi
            CoreRequest::GetNetworkStatus => {
                let status = self.state.network.get_status().await;
                CoreResponse::ok(status)
            }
            CoreRequest::ScanWifi => {
                let res = self.state.network.scan_wifi().await;
                CoreResponse::ok(res)
            }
            CoreRequest::ConnectWifi { ssid, password } => {
                let req = crate::network::WifiConnectRequest { ssid, password };
                match self.state.network.connect_wifi(req).await {
                    Ok(res) => CoreResponse::ok(res),
                    Err(e) => CoreResponse::err("network_error", e.to_string()),
                }
            }
            CoreRequest::DisconnectWifi => match self.state.network.disconnect_wifi().await {
                Ok(res) => CoreResponse::ok(res),
                Err(e) => CoreResponse::err("network_error", e.to_string()),
            },
            CoreRequest::StartSetupAp => match self.state.network.start_ap().await {
                Ok(res) => CoreResponse::ok(res),
                Err(e) => CoreResponse::err("network_error", e.to_string()),
            },
            CoreRequest::StopSetupAp => match self.state.network.stop_ap().await {
                Ok(res) => CoreResponse::ok(res),
                Err(e) => CoreResponse::err("network_error", e.to_string()),
            },

            // Bluetooth
            CoreRequest::GetBluetoothStatus => {
                let status = self.state.bluetooth.get_status().await;
                CoreResponse::ok(status)
            }
            CoreRequest::StartBluetoothScan => {
                let res = self.state.bluetooth.scan(10).await;
                CoreResponse::ok(res)
            }
            CoreRequest::ConnectBluetoothDevice { address } => {
                match self.state.bluetooth.connect(&address).await {
                    Ok(res) => CoreResponse::ok(res),
                    Err(e) => CoreResponse::err("bluetooth_error", e.to_string()),
                }
            }
            CoreRequest::DisconnectBluetoothDevice { .. } => {
                match self.state.bluetooth.disconnect().await {
                    Ok(res) => CoreResponse::ok(res),
                    Err(e) => CoreResponse::err("bluetooth_error", e.to_string()),
                }
            }

            // VPN
            CoreRequest::GetVpnStatus => {
                let status = self.state.vpn.get_status().await;
                CoreResponse::ok(status)
            }
            CoreRequest::SaveVpnConfig { vless_url } => {
                let req = crate::vpn::VpnConfigRequest {
                    subscription_url: None,
                    vless_uri: Some(vless_url),
                };
                match self.state.vpn.configure(req).await {
                    Ok(res) => CoreResponse::ok(res),
                    Err(e) => CoreResponse::err("vpn_error", e.to_string()),
                }
            }
            CoreRequest::ConnectVpn => match self.state.vpn.connect().await {
                Ok(res) => CoreResponse::ok(res),
                Err(e) => CoreResponse::err("vpn_error", e.to_string()),
            },
            CoreRequest::DisconnectVpn => match self.state.vpn.disconnect().await {
                Ok(res) => CoreResponse::ok(res),
                Err(e) => CoreResponse::err("vpn_error", e.to_string()),
            },

            // Setup & Diagnostics
            CoreRequest::GetSetupStatus => {
                let is_completed =
                    crate::setup::SetupRepository::is_setup_completed(&self.state.db)
                        .await
                        .unwrap_or(false);
                let completed_at = crate::setup::SetupRepository::get_completed_at(&self.state.db)
                    .await
                    .unwrap_or(None);
                let current_step = crate::setup::SetupRepository::get_current_step(&self.state.db)
                    .await
                    .unwrap_or_else(|_| "network".to_string());
                let (steps, degraded_components) =
                    self.state.diagnostics.probe_subsystems(&self.state).await;
                CoreResponse::ok(serde_json::json!({
                    "is_completed": is_completed,
                    "completed_at": completed_at,
                    "current_step": current_step,
                    "steps": steps,
                    "degraded_components": degraded_components,
                }))
            }
            CoreRequest::RunDiagnostics => {
                let (steps, degraded) = self.state.diagnostics.probe_subsystems(&self.state).await;
                CoreResponse::ok(serde_json::json!({
                    "steps": steps,
                    "degraded": degraded,
                }))
            }
            CoreRequest::CompleteSetupStep { step, data } => {
                let step_name = match step {
                    0 => "auth",
                    1 => "network",
                    2 => "audio",
                    3 => "voice",
                    4 => "ai",
                    _ => "completed",
                };
                let _ = crate::setup::SetupRepository::save_step(
                    &self.state.db,
                    step_name,
                    StepStatus::Completed,
                    Some(data),
                )
                .await;
                CoreResponse::ok(serde_json::json!({
                    "step": step_name,
                    "status": "completed"
                }))
            }

            CoreRequest::CompleteSetup => {
                match crate::setup::SetupRepository::set_setup_completed(&self.state.db, true).await
                {
                    Ok(at) => CoreResponse::ok(serde_json::json!({
                        "status": "completed",
                        "is_completed": true,
                        "completed_at": at,
                    })),
                    Err(e) => CoreResponse::err("setup_error", e.to_string()),
                }
            }

            // System operations
            CoreRequest::TriggerReboot => {
                tokio::spawn(async {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    let _ = tokio::process::Command::new("reboot").status().await;
                });
                CoreResponse::ok(serde_json::json!({ "rebooting": true }))
            }
            CoreRequest::TriggerShutdown => {
                tokio::spawn(async {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    let _ = tokio::process::Command::new("poweroff").status().await;
                });
                CoreResponse::ok(serde_json::json!({ "shutting_down": true }))
            }
            CoreRequest::CheckUpdates => {
                let status = crate::system::update::get_update_status();
                CoreResponse::ok(status)
            }
            CoreRequest::PerformUpdate => {
                let status = crate::system::update::get_update_status();
                CoreResponse::ok(status)
            }

            // Raw generic hook
            CoreRequest::RawCommand { command, payload } => {
                info!("Handling raw command: {}", command);
                CoreResponse::ok(serde_json::json!({
                    "command": command,
                    "received": payload,
                    "status": "acknowledged"
                }))
            }
        }
    }
}
