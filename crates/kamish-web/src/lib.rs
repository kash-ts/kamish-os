pub mod api;
pub mod auth;
pub mod database;
pub mod error;
pub mod events;
pub mod state;

pub use error::{WebError, WebResult};
use kamish_ipc::IpcClient;
pub use state::WebState;
use tracing::info;

pub async fn build_web_app(
    database_url: &str,
    ipc_client: IpcClient,
    auth_enabled: bool,
) -> anyhow::Result<(axum::Router, WebState)> {
    let pool = database::init_web_pool(database_url).await?;
    let state = WebState::new(pool, ipc_client, auth_enabled);

    // Initialize default admin user if not present
    state.auth.init_default_admin().await?;

    let router = api::create_router(state.clone());
    info!("kamish-web router successfully built");
    Ok((router, state))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use kamish_ipc::{CoreRequest, CoreResponse};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_web_status_forwarded_to_core() {
        let mock_ipc = IpcClient::mock(|req| match req {
            CoreRequest::GetStatus => CoreResponse::ok(serde_json::json!({
                "version": "0.1.0",
                "uptime_seconds": 42,
                "cpu_temp_c": 45.5,
                "safe_volume_ceiling": 85,
            })),
            _ => CoreResponse::err("unknown", "unknown request"),
        });

        let (app, _) = build_web_app("sqlite::memory:", mock_ipc, false)
            .await
            .unwrap();

        let req = Request::builder()
            .uri("/api/v1/status")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(val["version"], "0.1.0");
        assert_eq!(val["safe_volume_ceiling"], 85);
    }

    #[tokio::test]
    async fn test_web_core_offline_returns_503() {
        let mock_ipc = IpcClient::mock(|_| {
            CoreResponse::err(
                "core_offline",
                "Ядро колонки недоступно или перезапускается",
            )
        });

        let (app, _) = build_web_app("sqlite::memory:", mock_ipc, false)
            .await
            .unwrap();

        let req = Request::builder()
            .uri("/api/v1/status")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(val["code"], "CORE_OFFLINE");
    }
}
