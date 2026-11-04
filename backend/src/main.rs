use kamish_core::{build_app, config::Config};
use std::net::SocketAddr;
use tokio::signal;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level)),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!(
        "Starting Kamish backend v{} in [{}] mode",
        env!("CARGO_PKG_VERSION"),
        config.env
    );

    let (app, state) = build_app(&config).await?;

    let addr: SocketAddr = config
        .server_addr()
        .parse()
        .expect("Invalid server address");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Server listening on http://{}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    state.scheduler.stop();
    info!("Server shutdown completed");
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
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received SIGINT (Ctrl+C), starting graceful shutdown");
        },
        _ = terminate => {
            info!("Received SIGTERM, starting graceful shutdown");
        },
    }
}
