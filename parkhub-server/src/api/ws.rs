//! WebSocket real-time event support.
//!
//! Provides a `/api/v1/ws` endpoint that upgrades HTTP connections to WebSocket.
//! Events are distributed via a `tokio::sync::broadcast` channel for fan-out to
//! all connected clients.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, warn};

use crate::AppState;

/// Capacity of the broadcast channel. Slow readers that fall behind will miss
/// messages (lagged), which is acceptable for real-time UI updates.
const BROADCAST_CAPACITY: usize = 256;

/// Heartbeat interval in seconds.
const HEARTBEAT_INTERVAL_SECS: u64 = 30;

// ─────────────────────────────────────────────────────────────────────────────
// Event types
// ─────────────────────────────────────────────────────────────────────────────

/// Event types that can be broadcast over the WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WsEventType {
    BookingCreated,
    BookingCancelled,
    OccupancyChanged,
}

/// A WebSocket event message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEvent {
    pub event: WsEventType,
    pub data: serde_json::Value,
    pub timestamp: String,
}

impl WsEvent {
    /// Create a new event with the current UTC timestamp.
    pub fn new(event: WsEventType, data: serde_json::Value) -> Self {
        Self {
            event,
            data,
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Broadcast channel
// ─────────────────────────────────────────────────────────────────────────────

/// Holds the broadcast sender for WebSocket events.
#[derive(Debug, Clone)]
pub struct EventBroadcaster {
    sender: broadcast::Sender<WsEvent>,
}

impl EventBroadcaster {
    /// Create a new broadcaster with the default channel capacity.
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self { sender }
    }

    /// Broadcast an event to all connected WebSocket clients.
    /// Returns the number of receivers that will get the message.
    /// Returns 0 if there are no active subscribers (which is fine).
    pub fn broadcast(&self, event: WsEvent) -> usize {
        match self.sender.send(event) {
            Ok(n) => n,
            Err(_) => 0, // No active receivers
        }
    }

    /// Subscribe to receive events.
    pub fn subscribe(&self) -> broadcast::Receiver<WsEvent> {
        self.sender.subscribe()
    }

    /// Get the current number of active receivers.
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// WebSocket handler
// ─────────────────────────────────────────────────────────────────────────────

type SharedState = Arc<RwLock<AppState>>;

/// Handler for GET /api/v1/ws — upgrades to WebSocket.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Manages a single WebSocket connection: subscribes to the broadcast channel,
/// forwards events to the client, and sends periodic pings.
async fn handle_socket(socket: WebSocket, state: SharedState) {
    let broadcaster = {
        let s = state.read().await;
        s.ws_events.clone()
    };
    let mut rx = broadcaster.subscribe();

    let (mut sender, mut receiver) = socket.split();

    use futures_util::{SinkExt, StreamExt};

    // Heartbeat timer
    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(
        HEARTBEAT_INTERVAL_SECS,
    ));

    debug!("WebSocket client connected");

    loop {
        tokio::select! {
            // Forward broadcast events to this client
            event = rx.recv() => {
                match event {
                    Ok(ws_event) => {
                        if let Ok(json) = serde_json::to_string(&ws_event) {
                            if sender.send(Message::Text(json.into())).await.is_err() {
                                break; // Client disconnected
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("WebSocket client lagged, skipped {n} messages");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break; // Channel closed
                    }
                }
            }

            // Send ping heartbeat
            _ = heartbeat.tick() => {
                if sender.send(Message::Ping(vec![].into())).await.is_err() {
                    break; // Client disconnected
                }
            }

            // Handle incoming messages from client (pong, close)
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Pong(_))) => {
                        // Client is alive
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break; // Client disconnected
                    }
                    Some(Err(_)) => {
                        break; // Connection error
                    }
                    _ => {} // Ignore text/binary from client
                }
            }
        }
    }

    debug!("WebSocket client disconnected");
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_serialization() {
        let event = WsEvent::new(
            WsEventType::BookingCreated,
            serde_json::json!({"booking_id": "abc-123", "slot": 5}),
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event\":\"booking_created\""));
        assert!(json.contains("\"booking_id\":\"abc-123\""));
        assert!(json.contains("\"timestamp\""));

        // Roundtrip
        let parsed: WsEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.event, WsEventType::BookingCreated);
    }

    #[test]
    fn event_types_serialize_snake_case() {
        let cases = vec![
            (WsEventType::BookingCreated, "\"booking_created\""),
            (WsEventType::BookingCancelled, "\"booking_cancelled\""),
            (WsEventType::OccupancyChanged, "\"occupancy_changed\""),
        ];
        for (variant, expected) in cases {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected);
        }
    }

    #[test]
    fn broadcaster_no_receivers() {
        let broadcaster = EventBroadcaster::new();
        let event = WsEvent::new(
            WsEventType::OccupancyChanged,
            serde_json::json!({"lot_id": "lot-1", "available": 42}),
        );
        assert_eq!(broadcaster.broadcast(event), 0);
        assert_eq!(broadcaster.receiver_count(), 0);
    }

    #[test]
    fn broadcaster_fan_out() {
        let broadcaster = EventBroadcaster::new();
        let mut rx1 = broadcaster.subscribe();
        let mut rx2 = broadcaster.subscribe();

        let event = WsEvent::new(
            WsEventType::BookingCancelled,
            serde_json::json!({"booking_id": "xyz-789"}),
        );

        let count = broadcaster.broadcast(event.clone());
        assert_eq!(count, 2);

        let received1 = rx1.try_recv().unwrap();
        assert_eq!(received1.event, WsEventType::BookingCancelled);

        let received2 = rx2.try_recv().unwrap();
        assert_eq!(received2.event, WsEventType::BookingCancelled);
    }

    #[test]
    fn broadcaster_lagged_receiver() {
        let (sender, _) = broadcast::channel::<WsEvent>(2);
        let broadcaster = EventBroadcaster { sender };

        let mut rx = broadcaster.subscribe();

        for i in 0..3 {
            broadcaster.broadcast(WsEvent::new(
                WsEventType::OccupancyChanged,
                serde_json::json!({"seq": i}),
            ));
        }

        match rx.try_recv() {
            Err(broadcast::error::TryRecvError::Lagged(n)) => {
                assert_eq!(n, 1);
            }
            other => panic!("Expected Lagged, got {:?}", other),
        }

        let event = rx.try_recv().unwrap();
        assert_eq!(event.data["seq"], 2);
    }

    #[test]
    fn broadcaster_default_impl() {
        let b = EventBroadcaster::default();
        assert_eq!(b.receiver_count(), 0);
    }

    #[test]
    fn broadcaster_dropped_receiver() {
        let broadcaster = EventBroadcaster::new();
        let rx = broadcaster.subscribe();
        assert_eq!(broadcaster.receiver_count(), 1);

        drop(rx);
        assert_eq!(broadcaster.receiver_count(), 0);

        let event = WsEvent::new(
            WsEventType::BookingCreated,
            serde_json::json!({}),
        );
        assert_eq!(broadcaster.broadcast(event), 0);
    }
}
