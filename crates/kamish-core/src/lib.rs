pub mod ai;
pub mod audio;
pub mod bluetooth;
pub mod common;
pub mod config;
pub mod database;
pub mod ipc_server;
pub mod music;
pub mod network;
pub mod scheduler;
pub mod setup;
pub mod speech;
pub mod system;
pub mod vpn;

pub use common::{AppError, AppResult, AppState, CoreState};
pub use config::Config;
pub use ipc_server::CoreIpcHandler;
use kamish_ipc::{resolve_socket_path, CoreEvent, IpcServer};
use tracing::info;

/// Initializes core subsystems and binds the IPC Unix socket server
pub async fn start_core_daemon(config: &Config) -> AppResult<(CoreState, IpcServer)> {
    let pool = database::init_pool(&config.database_url).await?;
    database::run_migrations(&pool).await?;

    let state = AppState::with_options(
        pool.clone(),
        config.ai_cli_path.clone(),
        config.ai_cli_enabled,
    );

    // Load persisted volume & state
    state.media.load_from_db(&pool).await;

    // Spawn background scheduler
    let scheduler = state.scheduler.clone();
    tokio::spawn(async move {
        scheduler.run().await;
    });

    // Spawn continuous hardware monitor (throttling check at >= 80C)
    let hw_events = state.events.clone();
    tokio::spawn(async move {
        loop {
            let temp = system::hardware::read_cpu_temperature();
            let is_throttling = system::hardware::is_cpu_throttled(temp);
            if is_throttling {
                hw_events.emit(
                    "hardware_warning",
                    serde_json::json!({
                        "cpu_temp_c": temp.unwrap_or(80.0),
                        "throttled": true,
                    }),
                );
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    // Spawn diagnostic LED worker
    let led = std::sync::Arc::new(system::LedController::new());
    let initial_pattern = if state.setup.is_completed().await.unwrap_or(false) {
        system::LedPattern::Ready
    } else {
        system::LedPattern::SetupAp
    };
    let led_pattern = std::sync::Arc::new(tokio::sync::RwLock::new(initial_pattern));
    let led_running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    led.start_worker(led_pattern.clone(), led_running.clone());

    // Connect LED state to system events
    let mut led_events = state.events.subscribe();
    let pattern_ref = led_pattern.clone();
    tokio::spawn(async move {
        while let Ok(bus_event) = led_events.recv().await {
            match bus_event.event.as_str() {
                "hardware_warning" => {
                    *pattern_ref.write().await = system::LedPattern::Error;
                }
                "setup_step_updated" => {
                    *pattern_ref.write().await = system::LedPattern::SetupAp;
                }
                "setup_completed" => {
                    *pattern_ref.write().await = system::LedPattern::Ready;
                }
                "wakeword_detected" => {
                    *pattern_ref.write().await = system::LedPattern::Listening;
                }
                "speech_completed" => {
                    *pattern_ref.write().await = system::LedPattern::Ready;
                }
                _ => {}
            }
        }
    });

    // Spawn hardware power button gesture monitor (1 click: shutdown, 3 clicks: factory reset)
    system::power_button::start_power_button_monitor(state.clone());

    // Bind IPC Server
    let socket_path = resolve_socket_path();
    let ipc_server = IpcServer::bind(&socket_path)
        .map_err(|e| AppError::System(format!("Failed to bind IPC socket: {}", e)))?;

    // Bridge event bus to IPC broadcast
    let mut bus_rx = state.events.subscribe();
    let ipc_broadcaster = ipc_server.event_sender();
    tokio::spawn(async move {
        while let Ok(bus_event) = bus_rx.recv().await {
            let core_event = match bus_event.event.as_str() {
                "volume_changed" => {
                    let level = bus_event
                        .payload
                        .get("volume")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(50) as u8;
                    CoreEvent::VolumeChanged { level }
                }
                "hardware_warning" => {
                    let temp = bus_event
                        .payload
                        .get("cpu_temp_c")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(80.0) as f32;
                    let throttled = bus_event
                        .payload
                        .get("throttled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    CoreEvent::HardwareWarning {
                        cpu_temp_c: temp,
                        throttled,
                    }
                }
                _ => CoreEvent::Generic {
                    event_name: bus_event.event,
                    payload: bus_event.payload,
                },
            };
            let _ = ipc_broadcaster.send(core_event);
        }
    });

    // Bridge scheduler events to IPC broadcast
    let mut sched_rx = state.scheduler.subscribe();
    let sched_broadcaster = ipc_server.event_sender();
    tokio::spawn(async move {
        while let Ok(event) = sched_rx.recv().await {
            let ipc_event = match event {
                scheduler::SchedulerEvent::AlarmTriggered(alarm) => CoreEvent::AlarmTriggered {
                    id: alarm.id,
                    time: alarm.time,
                    label: alarm.label,
                },
                scheduler::SchedulerEvent::ReminderTriggered(reminder) => {
                    CoreEvent::ReminderTriggered {
                        id: reminder.id,
                        text: reminder.text,
                    }
                }
            };
            let _ = sched_broadcaster.send(ipc_event);
        }
    });

    // Autostart capture if configured
    let autostart = std::env::var("KAMISH_AUTOSTART_CAPTURE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if autostart {
        let capture = state.capture.clone();
        let boot_state = state.clone();
        tokio::spawn(async move {
            if capture.start(boot_state).await {
                info!("Microphone capture autostarted");
            } else {
                info!("Microphone capture skipped: no hardware device");
            }
        });
    }

    info!("kamish-core initialized successfully");
    Ok((state, ipc_server))
}
