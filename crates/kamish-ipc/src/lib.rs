pub mod client;
pub mod codec;
pub mod protocol;
pub mod server;

pub use client::IpcClient;
pub use protocol::{
    resolve_socket_path, CoreEvent, CoreRequest, CoreResponse, IpcMessage, DEFAULT_SOCKET_PATH,
    FALLBACK_SOCKET_PATH,
};
pub use server::{IpcServer, RequestHandler};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_ipc_request_response_and_events() {
        let dir = tempdir().unwrap();
        let sock_path = dir
            .path()
            .join("test_core.sock")
            .to_string_lossy()
            .to_string();

        let server = IpcServer::bind(&sock_path).unwrap();
        let event_broadcaster = server.event_sender();

        tokio::spawn(async move {
            server
                .run(Arc::new(|req: CoreRequest| async move {
                    match req {
                        CoreRequest::GetAudioStatus => CoreResponse::ok(serde_json::json!({
                            "volume": 60,
                            "max_volume": 85,
                            "muted": false
                        })),
                        CoreRequest::SetVolume { volume } => {
                            let vol = volume.min(85);
                            CoreResponse::ok(serde_json::json!({ "volume": vol }))
                        }
                        _ => CoreResponse::err("not_supported", "Action not supported in test"),
                    }
                }))
                .await
                .unwrap();
        });

        // Let server start
        tokio::time::sleep(Duration::from_millis(50)).await;

        let client = IpcClient::new(sock_path.clone());
        let mut event_sub = client.subscribe_events();

        // Wait for client to connect
        tokio::time::sleep(Duration::from_millis(150)).await;

        // 1. Test request/response
        let resp = client.call(CoreRequest::GetAudioStatus).await.unwrap();
        match resp {
            CoreResponse::Ok(val) => {
                assert_eq!(val["volume"], 60);
                assert_eq!(val["max_volume"], 85);
            }
            _ => panic!("Expected Ok response"),
        }

        let resp = client
            .call(CoreRequest::SetVolume { volume: 90 })
            .await
            .unwrap();
        match resp {
            CoreResponse::Ok(val) => {
                assert_eq!(val["volume"], 85);
            }
            _ => panic!("Expected volume capped at 85"),
        }

        // 2. Test event broadcast
        let _ = event_broadcaster.send(CoreEvent::VolumeChanged { level: 75 });
        let event = tokio::time::timeout(Duration::from_secs(1), event_sub.recv())
            .await
            .expect("Did not receive event in time")
            .expect("Event channel error");

        match event {
            CoreEvent::VolumeChanged { level } => assert_eq!(level, 75),
            _ => panic!("Unexpected event received"),
        }
    }

    #[tokio::test]
    async fn test_mock_client() {
        let client = IpcClient::mock(|req| match req {
            CoreRequest::GetStatus => CoreResponse::ok(serde_json::json!({ "version": "0.1.0" })),
            _ => CoreResponse::err("unhandled", "unhandled"),
        });

        let resp = client.call(CoreRequest::GetStatus).await.unwrap();
        match resp {
            CoreResponse::Ok(val) => assert_eq!(val["version"], "0.1.0"),
            _ => panic!("Expected Ok"),
        }
    }
}
