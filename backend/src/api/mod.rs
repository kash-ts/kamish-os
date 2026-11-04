pub mod ai;
pub mod alarms;
pub mod auth;
pub mod bluetooth;
pub mod events;
pub mod music;
pub mod network;
pub mod reminders;
pub mod setup;
pub mod speech;
pub mod status;
pub mod system;
pub mod vpn;
pub mod youtube_auth;

use crate::common::AppState;
use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::json;
use std::path::PathBuf;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

pub fn resolve_static_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("STATIC_DIR") {
        let path = PathBuf::from(dir);
        if path.exists() {
            return Some(path);
        }
    }

    let candidates = [
        PathBuf::from("../frontend/dist"),
        PathBuf::from("frontend/dist"),
        PathBuf::from("dist"),
    ];

    for candidate in candidates {
        if candidate.exists() && candidate.join("index.html").exists() {
            return Some(candidate);
        }
    }

    None
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_routes = Router::new()
        // Status
        .route("/v1/status", get(status::get_status))
        // Alarms
        .route(
            "/v1/alarms",
            post(alarms::create_alarm_handler).get(alarms::list_alarms_handler),
        )
        .route("/v1/alarms/{id}", delete(alarms::delete_alarm_handler))
        .route("/v1/alarms/{id}/toggle", post(alarms::toggle_alarm_handler))
        // Reminders
        .route(
            "/v1/reminders",
            post(reminders::create_reminder_handler).get(reminders::list_reminders_handler),
        )
        .route(
            "/v1/reminders/{id}",
            delete(reminders::delete_reminder_handler),
        )
        // AI Assistant & Tool Registry
        .route("/v1/ai/prompt", post(ai::prompt_handler))
        .route("/v1/ai/tools", get(ai::list_tools_handler))
        .route("/v1/ai/sessions/{id}", get(ai::get_session_handler))
        .route("/v1/ai/status", get(ai::get_ai_status_handler))
        .route("/v1/ai/config", post(ai::update_ai_config_handler))
        .route("/v1/ai/probe", post(ai::probe_ai_handler))
        // Antigravity CLI install & authorization (panel-driven)
        .route("/v1/ai/agy/status", get(ai::agy_status_handler))
        .route("/v1/ai/agy/install", post(ai::agy_install_handler))
        .route("/v1/ai/agy/login/begin", post(ai::agy_login_begin_handler))
        .route(
            "/v1/ai/agy/login/submit",
            post(ai::agy_login_submit_handler),
        )
        // Music & Playback
        .route("/v1/music/search", get(music::search_handler))
        .route("/v1/music/current", get(music::get_current_handler))
        .route("/v1/music/history", get(music::get_history_handler))
        .route("/v1/music/favorites", get(music::get_favorites_handler))
        .route("/v1/music/play", post(music::play_handler))
        .route("/v1/music/pause", post(music::pause_handler))
        .route("/v1/music/resume", post(music::resume_handler))
        .route("/v1/music/stop", post(music::stop_handler))
        .route("/v1/music/like", post(music::like_handler))
        .route("/v1/music/skip", post(music::skip_handler))
        .route(
            "/v1/music/recommendations",
            get(music::get_recommendations_handler),
        )
        .route("/v1/music/radio", get(music::get_radio_handler))
        .route(
            "/v1/music/lavalink/status",
            get(music::lavalink_status_handler),
        )
        .route(
            "/v1/music/lavalink/config",
            post(music::lavalink_config_handler),
        )
        // YouTube OAuth (device flow, panel-driven)
        .route(
            "/v1/music/youtube/auth/status",
            get(youtube_auth::yt_auth_status_handler),
        )
        .route(
            "/v1/music/youtube/auth/begin",
            post(youtube_auth::yt_auth_begin_handler),
        )
        .route(
            "/v1/music/youtube/auth/finish",
            post(youtube_auth::yt_auth_finish_handler),
        )
        .route(
            "/v1/music/youtube/auth/refresh",
            post(youtube_auth::yt_auth_refresh_handler),
        )
        .route(
            "/v1/music/youtube/auth/logout",
            post(youtube_auth::yt_auth_logout_handler),
        )
        // Speech & Voice System (Stage 6)
        .route("/v1/speech/status", get(speech::get_speech_status_handler))
        .route("/v1/speech/voices", get(speech::list_voices_handler))
        .route(
            "/v1/speech/config",
            post(speech::update_speech_config_handler).put(speech::update_speech_config_handler),
        )
        .route(
            "/v1/speech/synthesize",
            post(speech::synthesize_handler).get(speech::synthesize_handler),
        )
        .route(
            "/v1/speech/tts",
            post(speech::synthesize_handler).get(speech::synthesize_handler),
        )
        .route("/v1/speech/recognize", post(speech::recognize_handler))
        .route("/v1/speech/stt", post(speech::recognize_handler))
        .route(
            "/v1/speech/wakeword/simulate",
            post(speech::simulate_wakeword_handler),
        )
        // Microphone capture lifecycle (Stage 6)
        .route(
            "/v1/speech/capture/status",
            get(speech::capture_status_handler),
        )
        .route(
            "/v1/speech/capture/start",
            post(speech::capture_start_handler),
        )
        .route(
            "/v1/speech/capture/stop",
            post(speech::capture_stop_handler),
        )
        // Bluetooth A2DP Sink (Stage 7)
        .route("/v1/bluetooth/status", get(bluetooth::get_bluetooth_status))
        .route(
            "/v1/bluetooth/scan",
            post(bluetooth::scan_bluetooth_handler),
        )
        .route(
            "/v1/bluetooth/pair",
            post(bluetooth::pair_bluetooth_handler),
        )
        .route(
            "/v1/bluetooth/connect",
            post(bluetooth::connect_bluetooth_handler),
        )
        .route(
            "/v1/bluetooth/disconnect",
            post(bluetooth::disconnect_bluetooth_handler),
        )
        .route(
            "/v1/bluetooth/discoverable",
            post(bluetooth::set_discoverable_handler),
        )
        // Network Manager & Wi-Fi / AP (Stage 7)
        .route("/v1/network/status", get(network::get_network_status))
        .route("/v1/network/wifi/scan", get(network::scan_wifi_handler))
        .route(
            "/v1/network/wifi/connect",
            post(network::connect_wifi_handler),
        )
        .route(
            "/v1/network/wifi/disconnect",
            post(network::disconnect_wifi_handler),
        )
        .route("/v1/network/ap/start", post(network::start_ap_handler))
        .route("/v1/network/ap/stop", post(network::stop_ap_handler))
        // VPN / VLESS Tunnel (Stage 7)
        .route("/v1/vpn/status", get(vpn::get_vpn_status))
        .route("/v1/vpn/connect", post(vpn::connect_vpn_handler))
        .route("/v1/vpn/disconnect", post(vpn::disconnect_vpn_handler))
        .route("/v1/vpn/config", post(vpn::configure_vpn_handler))
        // Hardware & Audio Diagnostics (Stage 8)
        .route("/v1/system/hardware", get(system::get_hardware_status))
        .route("/v1/system/audio/volume", post(system::set_audio_volume))
        .route("/v1/system/audio/route", post(system::set_audio_route))
        .route("/v1/system/audio/ceiling", post(system::set_volume_ceiling))
        .route(
            "/v1/system/audio/test",
            post(system::play_audio_test_handler),
        )
        // KAMISH OS: A/B System Updates (RAUC) & Dropbear SSH
        .route(
            "/v1/system/update/status",
            get(system::get_update_status_handler),
        )
        .route(
            "/v1/system/update/upload",
            post(system::upload_update_bundle_handler),
        )
        .route(
            "/v1/system/update/install",
            post(system::install_update_handler),
        )
        .route("/v1/system/ssh/status", get(system::get_ssh_status_handler))
        .route("/v1/system/ssh/toggle", post(system::toggle_ssh_handler))
        // Auth Subsystem (Requirement R1)
        .route("/v1/auth/login", post(auth::login_handler))
        .route(
            "/v1/auth/change-password",
            post(auth::change_password_handler),
        )
        .route("/v1/auth/logout", post(auth::logout_handler))
        .route("/v1/auth/session", get(auth::session_handler))
        // Setup Wizard Subsystem (Requirement R2)
        .route("/v1/setup/status", get(setup::get_setup_status_handler))
        .route("/v1/setup/step", post(setup::save_setup_step_handler))
        .route("/v1/setup/complete", post(setup::complete_setup_handler))
        .route(
            "/v1/setup/diagnostics",
            get(setup::get_setup_diagnostics_handler),
        )
        .route(
            "/v1/setup/simulate-degradation",
            post(setup::simulate_degradation_handler),
        )
        // WebSocket Real-Time Events (Stage 7)
        .route("/v1/events", get(events::ws_events_handler))
        .fallback(api_fallback_handler)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::auth::middleware::auth_middleware,
        ));

    let mut router = Router::new().nest("/api", api_routes);

    if let Some(static_dir) = resolve_static_dir() {
        let index_file = static_dir.join("index.html");
        let serve_dir = ServeDir::new(&static_dir).fallback(ServeFile::new(&index_file));
        router = router.fallback_service(serve_dir);
    } else {
        router = router.fallback(default_fallback_handler);
    }

    router
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

async fn api_fallback_handler() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "Endpoint not found",
            "code": "NOT_FOUND"
        })),
    )
}

async fn default_fallback_handler() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "Not found",
            "code": "NOT_FOUND"
        })),
    )
}
