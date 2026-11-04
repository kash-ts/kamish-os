use crate::common::{AppResult, AppState};
use crate::vpn::{
    VpnConfigRequest, VpnConfigResponse, VpnConnectResponse, VpnDisconnectResponse, VpnStatus,
};
use axum::{extract::State, Json};

pub async fn get_vpn_status(State(state): State<AppState>) -> Json<VpnStatus> {
    let status = state.vpn.get_status().await;
    Json(status)
}

pub async fn connect_vpn_handler(
    State(state): State<AppState>,
) -> AppResult<Json<VpnConnectResponse>> {
    let res = state.vpn.connect().await?;
    Ok(Json(res))
}

pub async fn disconnect_vpn_handler(
    State(state): State<AppState>,
) -> AppResult<Json<VpnDisconnectResponse>> {
    let res = state.vpn.disconnect().await?;
    Ok(Json(res))
}

pub async fn configure_vpn_handler(
    State(state): State<AppState>,
    Json(req): Json<VpnConfigRequest>,
) -> AppResult<Json<VpnConfigResponse>> {
    let res = state.vpn.configure(req).await?;
    Ok(Json(res))
}
