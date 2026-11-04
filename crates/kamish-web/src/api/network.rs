use crate::error::WebResult;
use crate::state::WebState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use kamish_ipc::CoreRequest;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WifiConnectPayload {
    pub ssid: String,
    pub password: Option<String>,
}

pub async fn get_network_status(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let status = state.call_core(CoreRequest::GetNetworkStatus).await?;
    Ok((StatusCode::OK, Json(status)))
}

pub async fn scan_wifi_handler(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let networks = state.call_core(CoreRequest::ScanWifi).await?;
    Ok((StatusCode::OK, Json(networks)))
}

pub async fn connect_wifi_handler(
    State(state): State<WebState>,
    Json(payload): Json<WifiConnectPayload>,
) -> WebResult<impl IntoResponse> {
    let res = state
        .call_core(CoreRequest::ConnectWifi {
            ssid: payload.ssid,
            password: payload.password,
        })
        .await?;
    Ok((StatusCode::OK, Json(res)))
}

pub async fn start_ap_handler(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let res = state.call_core(CoreRequest::StartSetupAp).await?;
    Ok((StatusCode::OK, Json(res)))
}

pub async fn stop_ap_handler(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let res = state.call_core(CoreRequest::StopSetupAp).await?;
    Ok((StatusCode::OK, Json(res)))
}
