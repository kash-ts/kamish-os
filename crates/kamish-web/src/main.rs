use kamish_ipc::{resolve_socket_path, IpcClient};
use kamish_web::build_web_app;
use std::net::SocketAddr;
use tokio::signal;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&log_level)))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;

    let default_db = if std::path::Path::new("/var/lib/kamish").is_dir() {
        "sqlite:/var/lib/kamish/web.db".to_string()
    } else {
        "sqlite:/tmp/kamish_web.db".to_string()
    };
    let database_url = std::env::var("DATABASE_URL").unwrap_or(default_db);

    let auth_enabled = std::env::var("AUTH_ENABLED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true);

    let socket_path = resolve_socket_path();
    info!("kamish-web connecting to core socket: {}", socket_path);
    let ipc_client = IpcClient::new(socket_path);

    info!(
        "Starting kamish-web server v{} on http://{}",
        env!("CARGO_PKG_VERSION"),
        addr
    );

    let (app, _state) = build_web_app(&database_url, ipc_client, auth_enabled).await?;

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("kamish-web server shutdown complete.");
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
