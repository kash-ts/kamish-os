use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast;

/// Real-time system event sent across internal bus and WebSocket clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub event: String,
    pub timestamp: String,
    pub payload: Value,
}

impl SystemEvent {
    pub fn new(event: impl Into<String>, payload: Value) -> Self {
        Self {
            event: event.into(),
            timestamp: Utc::now().to_rfc3339(),
            payload,
        }
    }
}

/// Central event broadcaster for real-time WebSocket notifications
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<SystemEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    pub fn publish(&self, event: SystemEvent) {
        let _ = self.tx.send(event);
    }

    pub fn emit(&self, event_type: impl Into<String>, payload: Value) {
        self.publish(SystemEvent::new(event_type, payload));
    }

    pub fn subscribe(&self) -> broadcast::Receiver<SystemEvent> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(128)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_event_bus_publish_and_subscribe() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();

        bus.emit("test_event", json!({ "status": "ok", "value": 42 }));

        let received = rx.recv().await.expect("Failed to receive event");
        assert_eq!(received.event, "test_event");
        assert_eq!(received.payload["status"], "ok");
        assert_eq!(received.payload["value"], 42);
        assert!(!received.timestamp.is_empty());
    }
}
