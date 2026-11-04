use crate::auth::AuthService;
use crate::error::{WebError, WebResult};
use kamish_ipc::{CoreRequest, CoreResponse, IpcClient};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::time::Instant;

#[derive(Clone)]
pub struct WebState {
    pub db: SqlitePool,
    pub auth: Arc<AuthService>,
    pub ipc: Arc<IpcClient>,
    pub start_time: Instant,
    pub version: &'static str,
}

impl WebState {
    pub fn new(db: SqlitePool, ipc: IpcClient, auth_enabled: bool) -> Self {
        Self {
            db: db.clone(),
            auth: Arc::new(AuthService::new(db, auth_enabled)),
            ipc: Arc::new(ipc),
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION"),
        }
    }

    /// Helper to forward request to kamish-core and unwrap JSON payload
    pub async fn call_core(&self, request: CoreRequest) -> WebResult<serde_json::Value> {
        match self.ipc.call(request).await {
            Ok(CoreResponse::Ok(val)) => Ok(val),
            Ok(CoreResponse::Error { code, message }) => {
                if code == "core_offline" || code == "core_disconnected" {
                    Err(WebError::CoreUnavailable(message))
                } else if code == "not_found" {
                    Err(WebError::NotFound(message))
                } else if code == "unauthorized" {
                    Err(WebError::Unauthorized(message))
                } else if code == "validation" {
                    Err(WebError::Validation(message))
                } else {
                    Err(WebError::Internal(anyhow::anyhow!(
                        "Core error [{}]: {}",
                        code,
                        message
                    )))
                }
            }
            Err(e) => Err(WebError::CoreUnavailable(format!(
                "Failed to communicate with kamish-core: {}",
                e
            ))),
        }
    }
}
