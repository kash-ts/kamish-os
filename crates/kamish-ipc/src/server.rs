use crate::codec::{read_message, write_message};
use crate::protocol::{CoreEvent, CoreRequest, CoreResponse, IpcMessage};
use anyhow::{Context, Result};
use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{BufReader, BufWriter};
use tokio::net::UnixListener;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info, warn};

pub trait RequestHandler: Send + Sync + 'static {
    fn handle_request(&self, request: CoreRequest) -> impl Future<Output = CoreResponse> + Send;
}

impl<F, Fut> RequestHandler for F
where
    F: Fn(CoreRequest) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = CoreResponse> + Send + 'static,
{
    fn handle_request(&self, request: CoreRequest) -> impl Future<Output = CoreResponse> + Send {
        (self)(request)
    }
}

pub struct IpcServer {
    listener: UnixListener,
    event_tx: broadcast::Sender<CoreEvent>,
    socket_path: String,
}

impl IpcServer {
    pub fn bind(socket_path: &str) -> Result<Self> {
        let path = Path::new(socket_path);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }

        let listener = UnixListener::bind(socket_path)
            .with_context(|| format!("Failed to bind IPC socket at {}", socket_path))?;

        let (event_tx, _) = broadcast::channel(256);

        info!("IPC Server listening on Unix socket: {}", socket_path);
        Ok(Self {
            listener,
            event_tx,
            socket_path: socket_path.to_string(),
        })
    }

    pub fn event_sender(&self) -> broadcast::Sender<CoreEvent> {
        self.event_tx.clone()
    }

    pub fn broadcast_event(&self, event: CoreEvent) {
        let _ = self.event_tx.send(event);
    }

    pub async fn run<H: RequestHandler>(self, handler: Arc<H>) -> Result<()> {
        loop {
            match self.listener.accept().await {
                Ok((stream, _addr)) => {
                    let (read_half, write_half) = stream.into_split();
                    let mut reader = BufReader::new(read_half);
                    let mut writer = BufWriter::new(write_half);
                    let handler_clone = handler.clone();
                    let mut event_rx = self.event_tx.subscribe();

                    // Channel for writing outgoing messages to writer half
                    let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<IpcMessage>(64);

                    // Writer task
                    let writer_task = tokio::spawn(async move {
                        while let Some(msg) = outgoing_rx.recv().await {
                            if let Err(e) = write_message(&mut writer, &msg).await {
                                warn!("Failed to write to IPC client: {}", e);
                                break;
                            }
                        }
                    });

                    // Event broadcaster task to this client
                    let event_outgoing_tx = outgoing_tx.clone();
                    let event_task = tokio::spawn(async move {
                        while let Ok(event) = event_rx.recv().await {
                            if event_outgoing_tx
                                .send(IpcMessage::Event { event })
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                    });

                    // Reader & request dispatcher task
                    tokio::spawn(async move {
                        loop {
                            match read_message(&mut reader).await {
                                Ok(Some(IpcMessage::Request { id, request })) => {
                                    let resp = handler_clone.handle_request(request).await;
                                    if outgoing_tx
                                        .send(IpcMessage::Response { id, response: resp })
                                        .await
                                        .is_err()
                                    {
                                        break;
                                    }
                                }
                                Ok(Some(_)) => {
                                    debug!("Unexpected non-request message from client");
                                }
                                Ok(None) => {
                                    // Client disconnected cleanly
                                    break;
                                }
                                Err(e) => {
                                    warn!("Error reading IPC client stream: {}", e);
                                    break;
                                }
                            }
                        }
                        drop(outgoing_tx);
                        let _ = writer_task.await;
                        event_task.abort();
                    });
                }
                Err(e) => {
                    error!("IPC Server accept error: {}", e);
                }
            }
        }
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
