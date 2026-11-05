pub mod ai;
pub mod alarms;
pub mod audio;
pub mod auth;
pub mod bluetooth;
pub mod music;
pub mod network;
pub mod reminders;
pub mod setup;
pub mod speech;
pub mod status;
pub mod system;
pub mod vpn;

use crate::auth::auth_middleware;
use crate::events::ws_events_handler;
use crate::state::WebState;
use axum::{
    middleware,
    routing::{delete, get, patch, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

pub fn create_router(state: WebState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_routes = Router::new()
        // Auth
        .route("/auth/login", post(auth::login_handler))
        .route("/auth/logout", post(auth::logout_handler))
        .route("/auth/change-password", post(auth::change_password_handler))
        .route("/auth/session", get(auth::session_handler))
        // Status
        .route("/status", get(status::get_status))
        .route("/status/hardware", get(status::get_hardware))
        // Audio
        .route("/audio/status", get(audio::get_audio_status))
        .route("/audio/volume", post(audio::set_volume_handler))
        // Alarms
        .route(
            "/alarms",
            get(alarms::list_alarms_handler).post(alarms::create_alarm_handler),
        )
        .route("/alarms/{id}", delete(alarms::delete_alarm_handler))
        .route("/alarms/{id}/toggle", patch(alarms::toggle_alarm_handler))
        // Reminders
        .route(
            "/reminders",
            get(reminders::list_reminders_handler).post(reminders::create_reminder_handler),
        )
        .route(
            "/reminders/{id}",
            delete(reminders::delete_reminder_handler),
        )
        // Music
        .route("/music/search", get(music::search_handler))
        .route("/music/current", get(music::get_current_handler))
        .route("/music/play", post(music::play_handler))
        .route("/music/pause", post(music::pause_handler))
        .route("/music/resume", post(music::resume_handler))
        .route("/music/stop", post(music::stop_handler))
        .route("/music/skip", post(music::skip_handler))
        .route("/music/history", get(music::get_history_handler))
        .route(
            "/music/recommendations",
            get(music::get_recommendations_handler),
        )
        .route(
            "/music/youtube-auth/status",
            get(music::get_youtube_auth_status),
        )
        .route("/music/youtube-auth/begin", post(music::start_youtube_auth))
        .route("/music/youtube-auth/poll", post(music::poll_youtube_auth))
        // Speech
        .route("/speech/status", get(speech::get_speech_status_handler))
        .route("/speech/config", post(speech::update_speech_config_handler))
        .route("/speech/synthesize", post(speech::synthesize_handler))
        .route("/speech/capture/start", post(speech::start_capture_handler))
        .route("/speech/capture/stop", post(speech::stop_capture_handler))
        // Network
        .route("/network/status", get(network::get_network_status))
        .route("/network/scan", post(network::scan_wifi_handler))
        .route("/network/connect", post(network::connect_wifi_handler))
        .route("/network/ap/start", post(network::start_ap_handler))
        .route("/network/ap/stop", post(network::stop_ap_handler))
        // Bluetooth
        .route("/bluetooth/status", get(bluetooth::get_bluetooth_status))
        .route("/bluetooth/scan", post(bluetooth::scan_bluetooth_handler))
        .route(
            "/bluetooth/connect",
            post(bluetooth::connect_bluetooth_handler),
        )
        .route(
            "/bluetooth/disconnect",
            post(bluetooth::disconnect_bluetooth_handler),
        )
        // VPN
        .route("/vpn/status", get(vpn::get_vpn_status))
        .route("/vpn/configure", post(vpn::configure_vpn_handler))
        .route("/vpn/connect", post(vpn::connect_vpn_handler))
        .route("/vpn/disconnect", post(vpn::disconnect_vpn_handler))
        // Setup
        .route("/setup/status", get(setup::get_setup_status_handler))
        .route("/setup/step", post(setup::save_setup_step_handler))
        .route("/setup/diagnostics", post(setup::run_diagnostics_handler))
        // AI
        .route("/ai/prompt", post(ai::prompt_handler))
        .route("/ai/status", get(ai::get_ai_status))
        // System
        .route("/system/reboot", post(system::reboot_handler))
        .route("/system/shutdown", post(system::shutdown_handler))
        .route("/system/updates", get(system::check_updates_handler))
        // Events WebSocket
        .route("/events", get(ws_events_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // Static assets & SPA fallback
    let frontend_dist = std::env::var("FRONTEND_DIST")
        .or_else(|_| std::env::var("STATIC_DIR"))
        .unwrap_or_else(|_| "crates/kamish-web/frontend/dist".to_string());
    let index_html = format!("{}/index.html", frontend_dist);
    let static_service = ServeDir::new(&frontend_dist).fallback(ServeFile::new(index_html));

    Router::new()
        .nest("/api/v1", api_routes.clone())
        .nest("/v1", api_routes)
        .fallback_service(static_service)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
