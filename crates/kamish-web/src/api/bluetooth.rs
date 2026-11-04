use crate::error::WebResult;
use crate::state::WebState;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use kamish_ipc::CoreRequest;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BluetoothDevicePayload {
    pub mac: Option<String>,
    pub address: Option<String>,
}

pub async fn get_bluetooth_status(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let status = state.call_core(CoreRequest::GetBluetoothStatus).await?;
    Ok((StatusCode::OK, Json(status)))
}

pub async fn scan_bluetooth_handler(State(state): State<WebState>) -> WebResult<impl IntoResponse> {
    let devices = state.call_core(CoreRequest::StartBluetoothScan).await?;
    Ok((StatusCode::OK, Json(devices)))
}

pub async fn connect_bluetooth_handler(
    State(state): State<WebState>,
    Json(payload): Json<BluetoothDevicePayload>,
) -> WebResult<impl IntoResponse> {
    let address = payload
        .mac
        .or(payload.address)
        .unwrap_or_else(|| "".to_string());
    let res = state
        .call_core(CoreRequest::ConnectBluetoothDevice { address })
        .await?;
    Ok((StatusCode::OK, Json(res)))
}

pub async fn disconnect_bluetooth_handler(
    State(state): State<WebState>,
    payload: Option<Json<BluetoothDevicePayload>>,
) -> WebResult<impl IntoResponse> {
    let address = payload
        .and_then(|Json(p)| p.mac.or(p.address))
        .unwrap_or_default();
    let res = state
        .call_core(CoreRequest::DisconnectBluetoothDevice { address })
        .await?;
    Ok((StatusCode::OK, Json(res)))
}
