//! WebSocket real-time event support.
//!
//! Provides a `/api/v1/ws` endpoint that upgrades HTTP connections to WebSocket.
//! Events are distributed via a `tokio::sync::broadcast` channel for fan-out to
//! all connected clients.
//!
//! ## Authentication and what a client may see
//!
//! Clients may authenticate via a query parameter `?token=...` containing a
//! valid session token. A token that is present must be valid, unexpired and
//! belong to an active user, or the upgrade is rejected with
//! `401 Unauthorized`. A connection without a token is accepted, because the
//! lobby display is a legitimate unauthenticated consumer of live occupancy.
//!
//! Whatever the connection, **an event never carries someone else's
//! identity**. Occupancy detail (lot, slot, counts) is broadcast to every
//! subscriber; the `user_id` on a booking event reaches only the user it
//! belongs to. See `WsEvent::redacted_for`.
//!
//! This module previously documented two mutually exclusive contracts —
//! that tokenless upgrades were rejected, and that they "receive only public
//! events" — and implemented neither: every subscriber received every event
//! verbatim, `user_id` included.
//!
//! ## Heartbeat
//!
//! The server sends a WebSocket `Ping` frame every 30 seconds. If a client
//! misses 3 consecutive pongs the connection is terminated.

use axum::{
    Json,
    extract::{
        Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use parkhub_common::ApiResponse;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::AppState;

/// Capacity of the broadcast channel. Slow readers that fall behind will miss
/// messages (lagged), which is acceptable for real-time UI updates.
const BROADCAST_CAPACITY: usize = 256;

/// Heartbeat interval in seconds.
const HEARTBEAT_INTERVAL_SECS: u64 = 30;

/// Maximum number of consecutive missed pongs before disconnecting the client.
const MAX_MISSED_PONGS: u8 = 3;

// ─────────────────────────────────────────────────────────────────────────────
// Event types
// ─────────────────────────────────────────────────────────────────────────────

/// Event types that can be broadcast over the WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WsEventType {
    BookingCreated,
    BookingCancelled,
    OccupancyChanged,
    AnnouncementPublished,
    SlotStatusChange,
}

/// A WebSocket event message sent to connected clients.
///
/// The `data` field carries event-specific detail as freeform JSON.
/// This allows adding new event types without changing the wire format.
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

    /// Create a `BookingCreated` event.
    pub fn booking_created(lot_id: &str, slot_id: &str, user_id: &str) -> Self {
        Self::new(
            WsEventType::BookingCreated,
            serde_json::json!({
                "lot_id": lot_id,
                "slot_id": slot_id,
                "user_id": user_id,
            }),
        )
    }

    /// Return this event as it may be shown to `viewer`.
    ///
    /// Occupancy detail — which lot, which slot, how many free — is the
    /// point of the feed and is broadcast to everyone, including the
    /// unauthenticated lobby display. A `user_id` is not: it is only ever
    /// forwarded to the user it identifies.
    ///
    /// Every subscriber previously received every event verbatim, so any
    /// client that could reach the endpoint got credential-free, real-time,
    /// per-employee presence tracking by pseudonymous id.
    #[must_use]
    pub fn redacted_for(mut self, viewer: Option<Uuid>) -> Self {
        let Some(subject) = self
            .data
            .get("user_id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
        else {
            return self; // carries no identity
        };

        let is_own = viewer.is_some_and(|v| v.to_string() == subject);

        if !is_own && let Some(object) = self.data.as_object_mut() {
            object.remove("user_id");
        }

        self
    }

    /// Create a `BookingCancelled` event.
    pub fn booking_cancelled(lot_id: &str, slot_id: &str) -> Self {
        Self::new(
            WsEventType::BookingCancelled,
            serde_json::json!({
                "lot_id": lot_id,
                "slot_id": slot_id,
            }),
        )
    }

    /// Create an `OccupancyChanged` event.
    pub fn occupancy_update(lot_id: &str, available: u32, total: u32) -> Self {
        Self::new(
            WsEventType::OccupancyChanged,
            serde_json::json!({
                "lot_id": lot_id,
                "available": available,
                "total": total,
            }),
        )
    }

    /// Create an `AnnouncementPublished` event.
    pub fn announcement_published(id: &str, title: &str) -> Self {
        Self::new(
            WsEventType::AnnouncementPublished,
            serde_json::json!({
                "id": id,
                "title": title,
            }),
        )
    }

    /// Create a `SlotStatusChange` event.
    pub fn slot_status_change(lot_id: &str, slot_id: &str, status: &str) -> Self {
        Self::new(
            WsEventType::SlotStatusChange,
            serde_json::json!({
                "lot_id": lot_id,
                "slot_id": slot_id,
                "status": status,
            }),
        )
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
        self.sender.send(event).unwrap_or_default()
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
// Query params for auth
// ─────────────────────────────────────────────────────────────────────────────

/// Query parameters for the WebSocket upgrade endpoint.
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    /// Session token for authentication (optional — allows unauthenticated
    /// connections for public occupancy display).
    pub token: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// WebSocket handler
// ─────────────────────────────────────────────────────────────────────────────

type SharedState = Arc<RwLock<AppState>>;

/// Handler for GET /api/v1/ws — upgrades to WebSocket.
///
/// A `?token=...` query parameter is optional. When present it must resolve
/// to a valid, unexpired session for an active user. The resolved user is
/// carried into the socket so that events belonging to other users can have
/// their identity stripped before they are forwarded.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
    Query(params): Query<WsQuery>,
) -> Result<impl IntoResponse, (StatusCode, Json<ApiResponse<()>>)> {
    // Validate token if provided, and remember who the caller is.
    let mut viewer: Option<Uuid> = None;

    if let Some(ref token) = params.token {
        let state_guard = state.read().await;
        match state_guard.db.get_session(token).await {
            Ok(Some(s)) if !s.is_expired() => {
                // Valid session — check user is active
                match state_guard.db.get_user(&s.user_id.to_string()).await {
                    Ok(Some(u)) if u.is_active => {
                        debug!(user_id = %s.user_id, "WebSocket authenticated");
                        viewer = Some(s.user_id);
                    }
                    _ => {
                        return Err((
                            StatusCode::UNAUTHORIZED,
                            Json(ApiResponse::error(
                                "UNAUTHORIZED",
                                "Invalid or disabled user",
                            )),
                        ));
                    }
                }
            }
            _ => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(ApiResponse::error(
                        "UNAUTHORIZED",
                        "Invalid or expired token",
                    )),
                ));
            }
        }
    }

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, viewer)))
}

/// Manages a single WebSocket connection: subscribes to the broadcast channel,
/// forwards events to the client, and sends periodic pings.
async fn handle_socket(socket: WebSocket, state: SharedState, viewer: Option<Uuid>) {
    use futures_util::{SinkExt, StreamExt};

    let broadcaster = {
        let s = state.read().await;
        s.ws_events.clone()
    };
    let mut rx = broadcaster.subscribe();

    let (mut sender, mut receiver) = socket.split();

    // Send initial occupancy snapshot for all lots
    {
        let s = state.read().await;
        if let Ok(lots) = s.db.list_parking_lots().await {
            for lot in &lots {
                if let Ok(slots) = s.db.list_slots_by_lot(&lot.id.to_string()).await {
                    let total = u32::try_from(slots.len()).unwrap_or(u32::MAX);
                    let available = u32::try_from(
                        slots
                            .iter()
                            .filter(|sl| sl.status == parkhub_common::SlotStatus::Available)
                            .count(),
                    )
                    .unwrap_or(u32::MAX);
                    let snapshot = WsEvent::occupancy_update(&lot.id.to_string(), available, total);
                    if let Ok(json) = serde_json::to_string(&snapshot)
                        && sender.send(Message::Text(json.into())).await.is_err()
                    {
                        return; // Client disconnected during snapshot
                    }
                }
            }
        }
    }

    // Heartbeat timer
    let mut heartbeat =
        tokio::time::interval(std::time::Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
    let mut missed_pongs: u8 = 0;

    debug!("WebSocket client connected");

    loop {
        tokio::select! {
            // Forward broadcast events to this client
            event = rx.recv() => {
                match event {
                    Ok(ws_event) => {
                        // Strip identity that does not belong to this client.
                        let ws_event = ws_event.redacted_for(viewer);
                        if let Ok(json) = serde_json::to_string(&ws_event)
                            && sender.send(Message::Text(json.into())).await.is_err() {
                                break; // Client disconnected
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
                if missed_pongs >= MAX_MISSED_PONGS {
                    warn!("WebSocket client missed {missed_pongs} pongs, disconnecting");
                    break;
                }
                if sender.send(Message::Ping(vec![].into())).await.is_err() {
                    break; // Client disconnected
                }
                missed_pongs += 1;
            }

            // Handle incoming messages from client (pong, close)
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Pong(_))) => {
                        missed_pongs = 0;
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break; // Client disconnected
                    }
                    Some(Err(_)) => {
                        break; // Connection error
                    }
                    _ => {} // Text/binary from client — ignore
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
            (
                WsEventType::AnnouncementPublished,
                "\"announcement_published\"",
            ),
            (WsEventType::SlotStatusChange, "\"slot_status_change\""),
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
        assert_eq!(event.data["seq"], 1);
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

        let event = WsEvent::new(WsEventType::BookingCreated, serde_json::json!({}));
        assert_eq!(broadcaster.broadcast(event), 0);
    }

    #[test]
    fn booking_created_event_factory() {
        let event = WsEvent::booking_created("lot-1", "slot-2", "user-3");
        assert_eq!(event.event, WsEventType::BookingCreated);
        assert_eq!(event.data["lot_id"], "lot-1");
        assert_eq!(event.data["slot_id"], "slot-2");
        assert_eq!(event.data["user_id"], "user-3");
    }

    #[test]
    fn booking_cancelled_event_factory() {
        let event = WsEvent::booking_cancelled("lot-1", "slot-2");
        assert_eq!(event.event, WsEventType::BookingCancelled);
        assert_eq!(event.data["lot_id"], "lot-1");
        assert_eq!(event.data["slot_id"], "slot-2");
    }

    #[test]
    fn occupancy_update_event_factory() {
        let event = WsEvent::occupancy_update("lot-1", 5, 10);
        assert_eq!(event.event, WsEventType::OccupancyChanged);
        assert_eq!(event.data["lot_id"], "lot-1");
        assert_eq!(event.data["available"], 5);
        assert_eq!(event.data["total"], 10);
    }

    #[test]
    fn announcement_published_event_factory() {
        let event = WsEvent::announcement_published("ann-1", "Important Notice");
        assert_eq!(event.event, WsEventType::AnnouncementPublished);
        assert_eq!(event.data["id"], "ann-1");
        assert_eq!(event.data["title"], "Important Notice");
    }

    #[test]
    fn slot_status_change_event_factory() {
        let event = WsEvent::slot_status_change("lot-1", "slot-2", "maintenance");
        assert_eq!(event.event, WsEventType::SlotStatusChange);
        assert_eq!(event.data["lot_id"], "lot-1");
        assert_eq!(event.data["slot_id"], "slot-2");
        assert_eq!(event.data["status"], "maintenance");
    }

    #[test]
    fn event_roundtrip_all_types() {
        let events = vec![
            WsEvent::booking_created("l", "s", "u"),
            WsEvent::booking_cancelled("l", "s"),
            WsEvent::occupancy_update("l", 1, 2),
            WsEvent::announcement_published("a", "t"),
            WsEvent::slot_status_change("l", "s", "ok"),
        ];
        for event in events {
            let json = serde_json::to_string(&event).unwrap();
            let parsed: WsEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed.event, event.event);
            assert_eq!(parsed.data, event.data);
        }
    }

    #[test]
    fn ws_query_deserialize_with_token() {
        let q: WsQuery = serde_json::from_str(r#"{"token":"abc123"}"#).unwrap();
        assert_eq!(q.token.as_deref(), Some("abc123"));
    }

    #[test]
    fn ws_query_deserialize_without_token() {
        let q: WsQuery = serde_json::from_str("{}").unwrap();
        assert!(q.token.is_none());
    }
}

#[cfg(test)]
mod redaction_tests {
    use super::*;

    fn user(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    #[test]
    fn the_owner_of_a_booking_event_still_sees_their_own_id() {
        let owner = user(1);
        let event = WsEvent::booking_created("lot-1", "slot-1", &owner.to_string());

        let seen = event.redacted_for(Some(owner));

        assert_eq!(seen.data["user_id"], owner.to_string());
    }

    #[test]
    fn another_authenticated_user_does_not_see_the_id() {
        let owner = user(1);
        let event = WsEvent::booking_created("lot-1", "slot-1", &owner.to_string());

        let seen = event.redacted_for(Some(user(2)));

        assert!(seen.data.get("user_id").is_none(), "leaked to another user: {}", seen.data);
    }

    /// The lobby display connects without a token. It needs occupancy, not
    /// identities.
    #[test]
    fn an_unauthenticated_client_does_not_see_the_id() {
        let event = WsEvent::booking_created("lot-1", "slot-1", &user(1).to_string());

        let seen = event.redacted_for(None);

        assert!(seen.data.get("user_id").is_none(), "leaked to an anonymous client: {}", seen.data);
    }

    #[test]
    fn occupancy_detail_survives_redaction_for_everyone() {
        let event = WsEvent::booking_created("lot-1", "slot-7", &user(1).to_string());

        let seen = event.redacted_for(None);

        assert_eq!(seen.data["lot_id"], "lot-1");
        assert_eq!(seen.data["slot_id"], "slot-7");
    }

    #[test]
    fn events_without_an_identity_are_untouched() {
        let event = WsEvent::occupancy_update("lot-1", 3, 10);

        let seen = event.clone().redacted_for(None);

        assert_eq!(seen.data, event.data);
    }
}
