use crate::bluetooth::{
    ConnectRequest, ConnectResponse, DisconnectResponse, DiscoverableRequest, DiscoverableResponse,
    PairRequest, PairResponse, ScanRequest, ScanResponse,
};
use crate::common::{AppResult, AppState};
use axum::{extract::State, Json};

pub async fn get_bluetooth_status(
    State(state): State<AppState>,
) -> Json<crate::bluetooth::BluetoothStatus> {
    let status = state.bluetooth.get_status().await;
    Json(status)
}

pub async fn scan_bluetooth_handler(
    State(state): State<AppState>,
    body: Option<Json<ScanRequest>>,
) -> Json<ScanResponse> {
    let duration = body.map(|b| b.duration_secs).unwrap_or(10);
    let res = state.bluetooth.scan(duration).await;
    Json(res)
}

pub async fn pair_bluetooth_handler(
    State(state): State<AppState>,
    Json(req): Json<PairRequest>,
) -> AppResult<Json<PairResponse>> {
    let res = state.bluetooth.pair(&req.mac).await?;
    Ok(Json(res))
}

pub async fn connect_bluetooth_handler(
    State(state): State<AppState>,
    Json(req): Json<ConnectRequest>,
) -> AppResult<Json<ConnectResponse>> {
    let res = state.bluetooth.connect(&req.mac).await?;
    Ok(Json(res))
}

pub async fn disconnect_bluetooth_handler(
    State(state): State<AppState>,
) -> AppResult<Json<DisconnectResponse>> {
    let res = state.bluetooth.disconnect().await?;
    Ok(Json(res))
}

pub async fn set_discoverable_handler(
    State(state): State<AppState>,
    Json(req): Json<DiscoverableRequest>,
) -> AppResult<Json<DiscoverableResponse>> {
    let enabled = req.discoverable.unwrap_or(true);
    let res = state
        .bluetooth
        .set_discoverable(enabled, req.timeout_secs)
        .await?;
    Ok(Json(res))
}
