use crate::state::WebState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use tracing::{debug, error};

pub async fn ws_events_handler(
    ws: WebSocketUpgrade,
    State(state): State<WebState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: WebState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.ipc.subscribe_events();

    // Send initial handshake
    let init_msg = serde_json::json!({
        "event": "connected",
        "data": {
            "version": state.version,
        }
    });
    if sender
        .send(Message::Text(init_msg.to_string().into()))
        .await
        .is_err()
    {
        return;
    }

    // Task forwarding events from IPC subscriber to WebSocket client
    let send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let json = match serde_json::to_string(&event) {
                Ok(j) => j,
                Err(e) => {
                    error!("Failed to serialize CoreEvent: {}", e);
                    continue;
                }
            };
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    });

    // Task reading client messages (handles ping/pong and detects disconnect)
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => break,
                Message::Ping(_payload) => {
                    // Axum automatically replies to pings
                }
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
    debug!("WebSocket client disconnected");
}
