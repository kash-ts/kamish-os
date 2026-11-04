use kamish_core::{config::Config, start_core_daemon, CoreIpcHandler};
use std::sync::Arc;
use tokio::signal;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env();

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level)),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!(
        "Starting kamish-core daemon v{} in [{}] mode",
        env!("CARGO_PKG_VERSION"),
        config.env
    );

    let (state, ipc_server) = start_core_daemon(&config).await?;
    let handler = Arc::new(CoreIpcHandler::new(state.clone()));

    // Run IPC server in background task
    let server_task = tokio::spawn(async move {
        if let Err(e) = ipc_server.run(handler).await {
            error!("IPC Server terminated with error: {}", e);
        }
    });

    info!("kamish-core daemon running 24/7. Waiting for shutdown signal...");
    shutdown_signal().await;

    info!("Shutting down kamish-core services...");
    state.scheduler.stop();
    server_task.abort();
    info!("kamish-core shutdown complete.");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
