//! Server-Sent Events (SSE) for realtime fleet updates (T-1946).
//!
//! Provides `GET /api/v1/events/fleet` — a one-way push channel for the three
//! fleet screens (Einchecken / EV / Tausch). Authenticated (JWT cookie or
//! bearer). Mutation handlers (`check_in_handler`, `swap_*`, `start/stop
//! charging`, `create/cancel guest booking`) call
//! `state.fleet_events.broadcast(event)` AFTER DB commit.
//!
//! ## Architecture
//!
//! - `FleetEventBroadcaster` wraps a `tokio::sync::broadcast::Sender<FleetEvent>`.
//! - The HTTP handler subscribes to a new receiver, serializes each incoming
//!   event as a JSON `data:` field, and keeps the connection alive with
//!   periodic comment heartbeats every 15 s (axum's built-in `KeepAlive`).
//! - Auth mirrors the session middleware: `Authorization: Bearer <token>` OR
//!   `Cookie: <AUTH_COOKIE_NAME>=<token>`. Rejected requests receive `401`.

use async_stream::stream;
use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
};
use futures_util::Stream;
use parkhub_common::{ApiResponse, FleetEvent, UserRole};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast};
use tracing::{debug, warn};

use crate::AppState;
use crate::api::auth::AUTH_COOKIE_NAME;

/// Broadcast channel capacity. Slow subscribers that fall behind skip
/// messages (`Lagged`) — acceptable for UI events that are also refreshed by
/// polling fallback on the client side.
const BROADCAST_CAPACITY: usize = 256;

/// Keep-alive heartbeat interval. Sent as an SSE comment (`:\n\n`) so idle
/// proxies (Render, Cloudflare) do not tear down the stream.
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);

// ─────────────────────────────────────────────────────────────────────────────
// Broadcaster
// ─────────────────────────────────────────────────────────────────────────────

/// Fan-out channel for `FleetEvent`s. Cloning is cheap — the inner sender is
/// already an `Arc` under the hood.
#[derive(Debug, Clone)]
pub struct FleetEventBroadcaster {
    sender: broadcast::Sender<FleetEvent>,
}

impl FleetEventBroadcaster {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self { sender }
    }

    /// Broadcast an event. Returns the number of active subscribers that will
    /// receive it (0 if nobody is listening — that is not an error).
    pub fn broadcast(&self, event: FleetEvent) -> usize {
        self.sender.send(event).unwrap_or_default()
    }

    /// Subscribe a new receiver.
    pub fn subscribe(&self) -> broadcast::Receiver<FleetEvent> {
        self.sender.subscribe()
    }

    /// Number of currently connected subscribers.
    pub fn receiver_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for FleetEventBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RBAC visibility filter (Codex PR #378 finding)
// ─────────────────────────────────────────────────────────────────────────────

/// Decide whether a given `FleetEvent` should be forwarded to a specific
/// subscriber.
///
/// Rules:
/// - **Admin / SuperAdmin**: sees every event. Operations/oversight roles
///   need the full fleet view (check-ins, swaps, EV sessions, guest bookings).
/// - **Regular user (User / Premium)**: sees only events whose `user_id`
///   matches their own id.
/// - Events with `user_id == None` are treated as system-scoped and visible
///   to any authenticated subscriber — the broadcaster is already auth-gated
///   at the handler, so this preserves the historical "public among
///   authenticated users" semantics for non-attributed events.
///
/// This fix was introduced in response to the Codex review of PR #378, which
/// flagged the previous unconditional fan-out as an authorization leak
/// between tenants/users.
fn is_event_visible(event: &FleetEvent, viewer_id: &str, viewer_role: &UserRole) -> bool {
    match viewer_role {
        UserRole::Admin | UserRole::SuperAdmin => true,
        UserRole::User | UserRole::Premium => match event.user_id.as_deref() {
            Some(owner) => owner == viewer_id,
            None => true,
        },
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Handler
// ─────────────────────────────────────────────────────────────────────────────

type SharedState = Arc<RwLock<AppState>>;

/// Extract the session token either from the `Authorization: Bearer` header or
/// from the auth cookie.
fn extract_token(headers: &axum::http::HeaderMap) -> Option<String> {
    // Prefer Authorization header
    if let Some(v) = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        && let Some(rest) = v.strip_prefix("Bearer ")
        && !rest.is_empty()
    {
        return Some(rest.to_string());
    }
    // Fall back to cookie
    if let Some(cookies) = headers.get(header::COOKIE).and_then(|h| h.to_str().ok()) {
        for c in cookies.split(';') {
            let c = c.trim();
            if let Some(rest) = c.strip_prefix(&format!("{AUTH_COOKIE_NAME}="))
                && !rest.is_empty()
            {
                return Some(rest.to_string());
            }
        }
    }
    None
}

/// Handler for `GET /api/v1/events/fleet`.
///
/// Returns a text/event-stream that forwards every `FleetEvent` broadcast by
/// mutation handlers, prefixed with its `type` as the SSE `event:` field so
/// clients can use `EventSource.addEventListener("checkin.started", …)`.
pub async fn fleet_events_handler(
    State(state): State<SharedState>,
    headers: axum::http::HeaderMap,
) -> Result<
    Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>,
    (StatusCode, Json<ApiResponse<()>>),
> {
    let token = match extract_token(&headers) {
        Some(t) => t,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::error(
                    "UNAUTHORIZED",
                    "Missing or invalid authorization",
                )),
            ));
        }
    };

    // Validate session against DB (mirrors `auth_middleware`).
    // Capture viewer identity + role so the stream can filter events
    // per-subscriber (RBAC — Codex PR #378 finding).
    let (viewer_id, viewer_role) = {
        let state_guard = state.read().await;
        match state_guard.db.get_session(&token).await {
            Ok(Some(s)) if !s.is_expired() => {
                // Confirm user is still active.
                match state_guard.db.get_user(&s.user_id.to_string()).await {
                    Ok(Some(u)) if u.is_active => {
                        debug!(user_id = %s.user_id, role = ?u.role, "SSE authenticated");
                        (u.id.to_string(), u.role)
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
    };

    // Subscribe to the broadcast channel.
    let mut rx = {
        let s = state.read().await;
        s.fleet_events.subscribe()
    };

    let body = stream! {
        loop {
            match rx.recv().await {
                Ok(fleet_event) => {
                    // RBAC gate: drop events the current subscriber must not
                    // see (Codex PR #378). Skip serializes nothing — the SSE
                    // connection stays open and will receive future allowed
                    // events + keep-alives.
                    if !is_event_visible(&fleet_event, &viewer_id, &viewer_role) {
                        continue;
                    }
                    // Serialize payload to JSON; skip if somehow unserializable.
                    let Ok(json) = serde_json::to_string(&fleet_event) else { continue };
                    // Expose the event type as SSE `event:` so clients can
                    // `addEventListener("checkin.started", …)`.
                    let event_name = match fleet_event.event_type {
                        parkhub_common::FleetEventType::CheckinStarted => "checkin.started",
                        parkhub_common::FleetEventType::CheckinCompleted => "checkin.completed",
                        parkhub_common::FleetEventType::SwapRequested => "swap.requested",
                        parkhub_common::FleetEventType::SwapAccepted => "swap.accepted",
                        parkhub_common::FleetEventType::SwapDeclined => "swap.declined",
                        parkhub_common::FleetEventType::EvSessionStarted => "ev.session.started",
                        parkhub_common::FleetEventType::EvSessionStopped => "ev.session.stopped",
                        parkhub_common::FleetEventType::GuestCreated => "guest.created",
                        parkhub_common::FleetEventType::GuestCancelled => "guest.cancelled",
                    };
                    yield Ok(Event::default().event(event_name).data(json));
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("SSE client lagged, skipped {n} messages");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    };

    Ok(Sse::new(body).keep_alive(
        KeepAlive::new()
            .interval(KEEPALIVE_INTERVAL)
            .text("keep-alive"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use parkhub_common::UserRole;

    // ── RBAC visibility filter (T-1946 Codex finding) ────────────────────────
    //
    // Regression tests for: "Restrict fleet SSE fan-out to authorized event
    // scope." Prior to this fix `fleet_events_handler` forwarded every event
    // to every authenticated subscriber regardless of ownership.
    //
    // Rules enforced by `is_event_visible`:
    //   - Admin / SuperAdmin: sees everything.
    //   - Regular user (User / Premium): sees ONLY events where the event's
    //     `user_id` matches their own user id (plus any event carrying no
    //     `user_id` — those are system-scoped and already public).

    #[test]
    fn visibility_admin_sees_event_from_other_user() {
        let ev = FleetEvent::checkin_started("r-1", None, "other-user-id");
        assert!(is_event_visible(&ev, "admin-id", &UserRole::Admin));
    }

    #[test]
    fn visibility_super_admin_sees_event_from_other_user() {
        let ev = FleetEvent::swap_requested("r-2", None, "other-user-id");
        assert!(is_event_visible(&ev, "sa-id", &UserRole::SuperAdmin));
    }

    #[test]
    fn visibility_regular_user_blocked_from_other_users_event() {
        let ev = FleetEvent::checkin_started("r-3", None, "user-b");
        assert!(!is_event_visible(&ev, "user-a", &UserRole::User));
    }

    #[test]
    fn visibility_premium_user_blocked_from_other_users_event() {
        let ev = FleetEvent::ev_session_started("r-4", None, "user-b");
        assert!(!is_event_visible(&ev, "user-a", &UserRole::Premium));
    }

    #[test]
    fn visibility_regular_user_sees_own_event() {
        let ev = FleetEvent::checkin_completed("r-5", None, "user-a");
        assert!(is_event_visible(&ev, "user-a", &UserRole::User));
    }

    #[test]
    fn visibility_premium_user_sees_own_event() {
        let ev = FleetEvent::guest_created("r-6", None, "user-a");
        assert!(is_event_visible(&ev, "user-a", &UserRole::Premium));
    }

    #[test]
    fn visibility_regular_user_sees_system_event_without_user_id() {
        // Defensive: if an event is emitted with `user_id == None` (e.g. an
        // admin-triggered action that doesn't attribute to a specific user),
        // all authenticated subscribers should see it.  The broadcaster is
        // already auth-gated, so this is the historical "public among
        // authenticated users" behaviour.
        let ev = FleetEvent {
            event_type: parkhub_common::FleetEventType::CheckinStarted,
            resource_id: "r-7".to_string(),
            lot_id: None,
            user_id: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        assert!(is_event_visible(&ev, "user-a", &UserRole::User));
    }

    #[test]
    fn extract_token_reads_bearer_header() {
        let mut h = axum::http::HeaderMap::new();
        h.insert(header::AUTHORIZATION, "Bearer abc123".parse().unwrap());
        assert_eq!(extract_token(&h).as_deref(), Some("abc123"));
    }

    #[test]
    fn extract_token_reads_cookie() {
        let mut h = axum::http::HeaderMap::new();
        let cookie = format!("{AUTH_COOKIE_NAME}=cookie-token; other=1");
        h.insert(header::COOKIE, cookie.parse().unwrap());
        assert_eq!(extract_token(&h).as_deref(), Some("cookie-token"));
    }

    #[test]
    fn extract_token_returns_none_without_auth() {
        let h = axum::http::HeaderMap::new();
        assert_eq!(extract_token(&h), None);
    }

    #[test]
    fn extract_token_prefers_bearer_over_cookie() {
        let mut h = axum::http::HeaderMap::new();
        h.insert(header::AUTHORIZATION, "Bearer h-token".parse().unwrap());
        h.insert(
            header::COOKIE,
            format!("{AUTH_COOKIE_NAME}=c-token").parse().unwrap(),
        );
        assert_eq!(extract_token(&h).as_deref(), Some("h-token"));
    }

    #[tokio::test]
    async fn broadcaster_fans_out() {
        let b = FleetEventBroadcaster::new();
        let mut rx1 = b.subscribe();
        let mut rx2 = b.subscribe();
        let sent = b.broadcast(FleetEvent::checkin_started("r1", None, "u1"));
        assert_eq!(sent, 2);
        let _ = rx1.recv().await.unwrap();
        let _ = rx2.recv().await.unwrap();
    }
}
