use crate::error::WebResult;
use crate::state::WebState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use kamish_ipc::CoreRequest;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct VpnConfigurePayload {
    pub vless_uri: Option<String>,
    pub vless_url: Option<String>,
}

pub async fn get_vpn_status(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let status = state.call_core(CoreRequest::GetVpnStatus).await?;
    Ok((StatusCode::OK, Json(status)))
}

pub async fn connect_vpn_handler(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let res = state.call_core(CoreRequest::ConnectVpn).await?;
    Ok((StatusCode::OK, Json(res)))
}

pub async fn disconnect_vpn_handler(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let res = state.call_core(CoreRequest::DisconnectVpn).await?;
    Ok((StatusCode::OK, Json(res)))
}

pub async fn configure_vpn_handler(
    State(state): State<WebState>,
    Json(payload): Json<VpnConfigurePayload>,
) -> WebResult<impl IntoResponse> {
    let uri = payload
        .vless_uri
        .or(payload.vless_url)
        .unwrap_or_else(|| "".to_string());
    let res = state
        .call_core(CoreRequest::SaveVpnConfig { vless_url: uri })
        .await?;
    Ok((StatusCode::OK, Json(res)))
}
