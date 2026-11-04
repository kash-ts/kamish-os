use crate::common::{AppResult, AppState};
use crate::network::{
    ApStartResponse, ApStopResponse, NetworkStatus, WifiConnectRequest, WifiConnectResponse,
    WifiDisconnectResponse, WifiScanResponse,
};
use axum::{extract::State, Json};

pub async fn get_network_status(State(state): State<AppState>) -> Json<NetworkStatus> {
    let status = state.network.get_status().await;
    Json(status)
}

pub async fn scan_wifi_handler(State(state): State<AppState>) -> Json<WifiScanResponse> {
    let res = state.network.scan_wifi().await;
    Json(res)
}

pub async fn connect_wifi_handler(
    State(state): State<AppState>,
    Json(req): Json<WifiConnectRequest>,
) -> AppResult<Json<WifiConnectResponse>> {
    let res = state.network.connect_wifi(req).await?;
    Ok(Json(res))
}

pub async fn disconnect_wifi_handler(
    State(state): State<AppState>,
) -> AppResult<Json<WifiDisconnectResponse>> {
    let res = state.network.disconnect_wifi().await?;
    Ok(Json(res))
}

pub async fn start_ap_handler(State(state): State<AppState>) -> AppResult<Json<ApStartResponse>> {
    let res = state.network.start_ap().await?;
    Ok(Json(res))
}

pub async fn stop_ap_handler(State(state): State<AppState>) -> AppResult<Json<ApStopResponse>> {
    let res = state.network.stop_ap().await?;
    Ok(Json(res))
}
