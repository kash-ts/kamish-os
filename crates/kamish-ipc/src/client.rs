use crate::codec::{read_message, write_message};
use crate::protocol::{CoreEvent, CoreRequest, CoreResponse, IpcMessage};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{BufReader, BufWriter};
use tokio::net::UnixStream;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex, RwLock};
use tracing::{debug, warn};
use uuid::Uuid;

type PendingMap = Arc<Mutex<HashMap<String, oneshot::Sender<CoreResponse>>>>;

#[derive(Clone)]
pub struct IpcClient {
    socket_path: String,
    tx: Arc<RwLock<Option<mpsc::Sender<IpcMessage>>>>,
    pending: PendingMap,
    event_tx: broadcast::Sender<CoreEvent>,
    mock_handler: Option<Arc<dyn Fn(CoreRequest) -> CoreResponse + Send + Sync>>,
}

impl IpcClient {
    pub fn new(socket_path: String) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        let client = Self {
            socket_path,
            tx: Arc::new(RwLock::new(None)),
            pending: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
            mock_handler: None,
        };
        client.spawn_connection_manager();
        client
    }

    /// Creates a mock IPC client for unit testing without a real Unix domain socket
    pub fn mock<F>(handler: F) -> Self
    where
        F: Fn(CoreRequest) -> CoreResponse + Send + Sync + 'static,
    {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            socket_path: "/mock/core.sock".to_string(),
            tx: Arc::new(RwLock::new(None)),
            pending: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
            mock_handler: Some(Arc::new(handler)),
        }
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<CoreEvent> {
        self.event_tx.subscribe()
    }

    pub fn emit_mock_event(&self, event: CoreEvent) {
        let _ = self.event_tx.send(event);
    }

    fn spawn_connection_manager(&self) {
        let socket_path = self.socket_path.clone();
        let tx_holder = self.tx.clone();
        let pending = self.pending.clone();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            loop {
                // If not connected, attempt to connect
                let is_connected = tx_holder.read().await.is_some();
                if !is_connected {
                    match UnixStream::connect(&socket_path).await {
                        Ok(stream) => {
                            debug!("Connected to IPC socket at {}", socket_path);
                            let (read_half, write_half) = stream.into_split();
                            let mut reader = BufReader::new(read_half);
                            let mut writer = BufWriter::new(write_half);

                            let (out_tx, mut out_rx) = mpsc::channel::<IpcMessage>(64);
                            *tx_holder.write().await = Some(out_tx);

                            let tx_holder_writer = tx_holder.clone();
                            let writer_task = tokio::spawn(async move {
                                while let Some(msg) = out_rx.recv().await {
                                    if let Err(e) = write_message(&mut writer, &msg).await {
                                        warn!("Failed to write message to IPC socket: {}", e);
                                        break;
                                    }
                                }
                                *tx_holder_writer.write().await = None;
                            });

                            let pending_reader = pending.clone();
                            let event_tx_reader = event_tx.clone();
                            let tx_holder_reader = tx_holder.clone();

                            loop {
                                match read_message(&mut reader).await {
                                    Ok(Some(IpcMessage::Response { id, response })) => {
                                        let mut map = pending_reader.lock().await;
                                        if let Some(sender) = map.remove(&id) {
                                            let _ = sender.send(response);
                                        }
                                    }
                                    Ok(Some(IpcMessage::Event { event })) => {
                                        let _ = event_tx_reader.send(event);
                                    }
                                    Ok(Some(IpcMessage::Request { .. })) => {
                                        debug!("Received unexpected request from core");
                                    }
                                    Ok(None) => {
                                        // Server closed socket
                                        break;
                                    }
                                    Err(e) => {
                                        warn!("Error reading IPC response: {}", e);
                                        break;
                                    }
                                }
                            }

                            // Connection dropped: clear holder and fail all pending requests
                            *tx_holder_reader.write().await = None;
                            let _ = writer_task.await;
                            let mut map = pending.lock().await;
                            for (_id, sender) in map.drain() {
                                let _ = sender.send(CoreResponse::err(
                                    "core_disconnected",
                                    "Connection to kamish-core was lost",
                                ));
                            }
                        }
                        Err(_) => {
                            // Core not ready yet, wait before reconnect
                        }
                    }
                }

                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });
    }

    /// Sends a request to kamish-core and awaits the typed response
    pub async fn call(&self, request: CoreRequest) -> Result<CoreResponse> {
        if let Some(ref mock) = self.mock_handler {
            return Ok(mock(request));
        }

        let sender = {
            let guard = self.tx.read().await;
            guard.clone()
        };

        let tx = match sender {
            Some(tx) => tx,
            None => {
                // Return descriptive error when core is offline
                return Ok(CoreResponse::err(
                    "core_offline",
                    "Ядро колонки недоступно или перезапускается",
                ));
            }
        };

        let id = Uuid::new_v4().to_string();
        let (resp_tx, resp_rx) = oneshot::channel();

        {
            let mut map = self.pending.lock().await;
            map.insert(id.clone(), resp_tx);
        }

        let req_msg = IpcMessage::Request {
            id: id.clone(),
            request,
        };

        if let Err(e) = tx.send(req_msg).await {
            let mut map = self.pending.lock().await;
            map.remove(&id);
            return Err(anyhow!("Failed to send IPC request: {}", e));
        }

        match tokio::time::timeout(Duration::from_secs(10), resp_rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Ok(CoreResponse::err(
                "connection_closed",
                "Connection closed while waiting for response",
            )),
            Err(_) => {
                let mut map = self.pending.lock().await;
                map.remove(&id);
                Ok(CoreResponse::err("timeout", "IPC request timed out"))
            }
        }
    }
}
