use crate::common::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use tracing::{debug, warn};

pub async fn ws_events_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    debug!("New WebSocket client connected for real-time events");
    let mut rx = state.events.subscribe();

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(ev) => {
                        let text = match serde_json::to_string(&ev) {
                            Ok(t) => t,
                            Err(e) => {
                                warn!("Failed to serialize event: {e}");
                                continue;
                            }
                        };
                        if socket.send(Message::Text(text.into())).await.is_err() {
                            // Client disconnected
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("WebSocket subscriber lagged by {n} messages");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    Some(Ok(_)) => {
                        // Client sent message, keep connection alive
                    }
                    Some(Err(_)) => {
                        break;
                    }
                }
            }
        }
    }

    debug!("WebSocket client disconnected");
}
