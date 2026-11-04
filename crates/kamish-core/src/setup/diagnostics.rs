use crate::common::{AppResult, AppState};
use crate::setup::models::{DegradedComponentInfo, SetupStepInfo, StepHealthInfo, StepStatus};
use crate::setup::repository::SetupRepository;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Default)]
pub struct DiagnosticsEngine {
    simulated_degradations: Arc<RwLock<HashMap<String, String>>>,
}

impl DiagnosticsEngine {
    pub fn new() -> Self {
        Self {
            simulated_degradations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn simulate_degradation(
        &self,
        component: &str,
        degraded: bool,
        error_reason: Option<&str>,
    ) -> AppResult<()> {
        let mut map = self.simulated_degradations.write().await;
        if degraded {
            let reason = error_reason
                .unwrap_or("Simulated subsystem degradation")
                .to_string();
            map.insert(component.to_string(), reason);
        } else {
            map.remove(component);
        }
        Ok(())
    }

    pub async fn get_simulated_degradation(&self, component: &str) -> Option<String> {
        let map = self.simulated_degradations.read().await;
        map.get(component).cloned()
    }

    pub async fn probe_subsystems(
        &self,
        state: &AppState,
    ) -> (Vec<SetupStepInfo>, Vec<DegradedComponentInfo>) {
        let mut steps = Vec::new();
        let mut degraded_components = Vec::new();

        // 1. Network step probe
        let (mut net_status, net_data, net_updated) =
            SetupRepository::get_step_status(&state.db, "network")
                .await
                .unwrap_or((StepStatus::Pending, None, None));

        let net_info = state.network.get_status().await;
        let wifi_connected = net_info.wifi.as_ref().map(|w| w.connected).unwrap_or(false);
        let mut net_healthy =
            wifi_connected || net_info.ethernet.connected || net_info.ap_mode.active;
        let mut net_msg = if wifi_connected {
            format!(
                "Подключено к Wi-Fi ({})",
                net_info.ip_address.as_deref().unwrap_or("IP получен")
            )
        } else if net_info.ethernet.connected {
            format!(
                "Подключено по Ethernet ({})",
                net_info.ip_address.as_deref().unwrap_or("IP получен")
            )
        } else if net_info.ap_mode.active {
            "Активна точка доступа для настройки Kamish-Setup".to_string()
        } else {
            "Сеть не подключена".to_string()
        };

        if let Some(reason) = self.get_simulated_degradation("network").await {
            net_healthy = false;
            net_msg = reason.clone();
            net_status = StepStatus::Degraded;
            degraded_components.push(DegradedComponentInfo {
                component: "network".to_string(),
                title: "Сетевое подключение (Wi-Fi / Ethernet)".to_string(),
                recovery_step: "network".to_string(),
                recovery_url: "/setup?step=network&recovery=true".to_string(),
                error_message: reason,
                severity: "warning".to_string(),
                can_fallback: true,
                fallback_action_description: Some(
                    "Включить режим точки доступа Kamish-Setup".to_string(),
                ),
            });
        }

        steps.push(SetupStepInfo {
            id: "network".to_string(),
            title: "Сетевое подключение (Wi-Fi / Ethernet)".to_string(),
            status: net_status,
            is_mandatory: true,
            health: StepHealthInfo {
                healthy: net_healthy,
                message: net_msg,
            },
            data: net_data.or_else(|| {
                Some(serde_json::json!({
                    "primary_interface": net_info.primary_interface,
                    "ip_address": net_info.ip_address,
                    "wifi_connected": wifi_connected,
                    "ap_active": net_info.ap_mode.active,
                }))
            }),
            updated_at: net_updated,
        });

        // 3. Audio step probe
        let (mut audio_status, audio_data, audio_updated) =
            SetupRepository::get_step_status(&state.db, "audio")
                .await
                .unwrap_or((StepStatus::Pending, None, None));

        let routing = state.audio.get_routing_info().await;
        let mut audio_healthy = true;
        let mut audio_msg = format!(
            "Аудиовыход: {}, громкость: {}% (лимит: {}%)",
            routing.output_device, routing.volume, routing.safe_volume_ceiling
        );

        if let Some(reason) = self.get_simulated_degradation("audio").await {
            audio_healthy = false;
            audio_msg = reason.clone();
            audio_status = StepStatus::Degraded;
            degraded_components.push(DegradedComponentInfo {
                component: "audio".to_string(),
                title: "Аудиосистема и Safe Volume Ceiling".to_string(),
                recovery_step: "audio".to_string(),
                recovery_url: "/setup?step=audio&recovery=true".to_string(),
                error_message: reason,
                severity: "error".to_string(),
                can_fallback: true,
                fallback_action_description: Some(
                    "Сбросить громкость на безопасный уровень 50%".to_string(),
                ),
            });
        }

        steps.push(SetupStepInfo {
            id: "audio".to_string(),
            title: "Аудиосистема и Safe Volume Ceiling".to_string(),
            status: audio_status,
            is_mandatory: true,
            health: StepHealthInfo {
                healthy: audio_healthy,
                message: audio_msg,
            },
            data: audio_data.or_else(|| {
                Some(serde_json::json!({
                    "volume": routing.volume,
                    "safe_volume_ceiling": routing.safe_volume_ceiling,
                    "output_device": routing.output_device,
                    "mixer_status": routing.mixer_status,
                }))
            }),
            updated_at: audio_updated,
        });

        // 4. Voice step probe
        let (mut voice_status, voice_data, voice_updated) =
            SetupRepository::get_step_status(&state.db, "voice")
                .await
                .unwrap_or((StepStatus::Pending, None, None));

        let speech_status = state.speech.get_status();
        let mut voice_healthy = speech_status.microphone_available;
        let mut voice_msg = format!(
            "Микрофон: {}, STT: {}, TTS: {}",
            if speech_status.microphone_available {
                "подключен"
            } else {
                "не обнаружен"
            },
            speech_status.active_stt_provider,
            speech_status.active_tts_provider
        );

        if let Some(reason) = self.get_simulated_degradation("voice").await {
            voice_healthy = false;
            voice_msg = reason.clone();
            voice_status = StepStatus::Degraded;
            degraded_components.push(DegradedComponentInfo {
                component: "voice".to_string(),
                title: "Микрофонный массив ReSpeaker и синтез речи".to_string(),
                recovery_step: "voice".to_string(),
                recovery_url: "/setup?step=voice&recovery=true".to_string(),
                error_message: reason,
                severity: "warning".to_string(),
                can_fallback: true,
                fallback_action_description: Some(
                    "Переключить на автономный генератор WAV-тонов AlertSynth".to_string(),
                ),
            });
        }

        steps.push(SetupStepInfo {
            id: "voice".to_string(),
            title: "Микрофонный массив ReSpeaker и синтез речи".to_string(),
            status: voice_status,
            is_mandatory: true,
            health: StepHealthInfo {
                healthy: voice_healthy,
                message: voice_msg,
            },
            data: voice_data.or_else(|| {
                Some(serde_json::json!({
                    "microphone_available": speech_status.microphone_available,
                    "active_stt": speech_status.active_stt_provider,
                    "active_tts": speech_status.active_tts_provider,
                }))
            }),
            updated_at: voice_updated,
        });

        // 5. AI step probe
        let (mut ai_status, ai_data, ai_updated) =
            SetupRepository::get_step_status(&state.db, "ai")
                .await
                .unwrap_or((StepStatus::Pending, None, None));

        let ai_readiness = state.ai.get_readiness().await;
        let mut ai_healthy = ai_readiness.ready;
        let mut ai_msg = if ai_readiness.ready {
            format!(
                "AI оркестратор готов (режим: {}, версия: {})",
                ai_readiness.mode,
                ai_readiness.cli_version.as_deref().unwrap_or("local")
            )
        } else {
            ai_readiness
                .error_reason
                .clone()
                .unwrap_or_else(|| "AI CLI недоступен".to_string())
        };

        if let Some(reason) = self.get_simulated_degradation("ai").await {
            ai_healthy = false;
            ai_msg = reason.clone();
            ai_status = StepStatus::Degraded;
            degraded_components.push(DegradedComponentInfo {
                component: "ai".to_string(),
                title: "AI Orchestrator (Antigravity CLI / Mock)".to_string(),
                recovery_step: "ai".to_string(),
                recovery_url: "/setup?step=ai&recovery=true".to_string(),
                error_message: reason,
                severity: "warning".to_string(),
                can_fallback: true,
                fallback_action_description: Some(
                    "Переключить на автономный оффлайн-режим (Mock AI Engine)".to_string(),
                ),
            });
        }

        steps.push(SetupStepInfo {
            id: "ai".to_string(),
            title: "AI Orchestrator (Antigravity CLI / Mock)".to_string(),
            status: ai_status,
            is_mandatory: true,
            health: StepHealthInfo {
                healthy: ai_healthy,
                message: ai_msg,
            },
            data: ai_data.or_else(|| {
                Some(serde_json::json!({
                    "ready": ai_readiness.ready,
                    "mode": ai_readiness.mode,
                    "cli_path": ai_readiness.cli_path,
                    "cli_version": ai_readiness.cli_version,
                }))
            }),
            updated_at: ai_updated,
        });

        (steps, degraded_components)
    }
}
