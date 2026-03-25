//! EV Charging Station Management handlers.
//!
//! Manage EV chargers in parking lots: list chargers, start/stop sessions,
//! session history, and admin charger management.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::models::{
    ChargingSession, ChargingSessionStatus, ConnectorType, EvCharger, EvChargerStatus,
};
use parkhub_common::ApiResponse;

use super::{check_admin, AuthUser, SharedState};

/// Request to start a charging session
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct StartChargingRequest {
    pub booking_id: Option<Uuid>,
}

/// Request to add a charger (admin)
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AddChargerRequest {
    pub lot_id: Uuid,
    pub label: String,
    pub connector_type: ConnectorType,
    pub power_kw: f64,
    pub location_hint: Option<String>,
}

/// Admin charger utilization stats
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ChargerUtilizationStats {
    pub total_chargers: i32,
    pub available: i32,
    pub in_use: i32,
    pub offline: i32,
    pub total_sessions: i32,
    pub total_kwh: f64,
}

/// `GET /api/v1/lots/:id/chargers` — list EV chargers in a lot
#[utoipa::path(
    get,
    path = "/api/v1/lots/{id}/chargers",
    tag = "EV Charging",
    summary = "List chargers in a lot",
    security(("bearer_auth" = []))
)]
pub async fn list_lot_chargers(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(lot_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<Vec<EvCharger>>>) {
    let state_guard = state.read().await;

    match state_guard.db.list_chargers_by_lot(&lot_id).await {
        Ok(chargers) => (StatusCode::OK, Json(ApiResponse::success(chargers))),
        Err(e) => {
            tracing::error!("Failed to list chargers: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list chargers",
                )),
            )
        }
    }
}

/// `POST /api/v1/chargers/:id/start` — start charging session
#[utoipa::path(
    post,
    path = "/api/v1/chargers/{id}/start",
    tag = "EV Charging",
    summary = "Start charging session",
    security(("bearer_auth" = []))
)]
pub async fn start_charging(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(charger_id): Path<String>,
    Json(req): Json<StartChargingRequest>,
) -> (StatusCode, Json<ApiResponse<ChargingSession>>) {
    let state_guard = state.read().await;

    // Check charger exists and is available
    let mut charger = match state_guard.db.get_charger(&charger_id).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Charger not found")),
            );
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    if charger.status != EvChargerStatus::Available {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::error(
                "CHARGER_UNAVAILABLE",
                "Charger is not available",
            )),
        );
    }

    let session = ChargingSession {
        id: Uuid::new_v4(),
        charger_id: charger.id,
        user_id: auth_user.user_id,
        booking_id: req.booking_id,
        start_time: Utc::now(),
        end_time: None,
        kwh_consumed: 0.0,
        status: ChargingSessionStatus::Active,
        created_at: Utc::now(),
    };

    charger.status = EvChargerStatus::InUse;

    if let Err(e) = state_guard.db.save_charger(&charger).await {
        tracing::error!("Failed to update charger status: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to start charging",
            )),
        );
    }

    if let Err(e) = state_guard.db.save_charging_session(&session).await {
        tracing::error!("Failed to save charging session: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to save session")),
        );
    }

    (StatusCode::CREATED, Json(ApiResponse::success(session)))
}

/// `POST /api/v1/chargers/:id/stop` — stop charging session
#[utoipa::path(
    post,
    path = "/api/v1/chargers/{id}/stop",
    tag = "EV Charging",
    summary = "Stop charging session",
    security(("bearer_auth" = []))
)]
pub async fn stop_charging(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(charger_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<ChargingSession>>) {
    let state_guard = state.read().await;

    // Find active session for this charger and user
    let sessions = match state_guard
        .db
        .list_charging_sessions_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to list sessions: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Internal server error")),
            );
        }
    };

    let mut session = match sessions.into_iter().find(|s| {
        s.charger_id.to_string() == charger_id && s.status == ChargingSessionStatus::Active
    }) {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    "NO_ACTIVE_SESSION",
                    "No active charging session found",
                )),
            );
        }
    };

    // Calculate duration-based kWh (simplified)
    let duration_hours = (Utc::now() - session.start_time).num_minutes() as f64 / 60.0;
    let charger = state_guard.db.get_charger(&charger_id).await.ok().flatten();
    let power_kw = charger.as_ref().map(|c| c.power_kw).unwrap_or(7.4);

    session.end_time = Some(Utc::now());
    session.kwh_consumed = (duration_hours * power_kw * 0.85).max(0.1); // 85% efficiency
    session.status = ChargingSessionStatus::Completed;

    if let Err(e) = state_guard.db.save_charging_session(&session).await {
        tracing::error!("Failed to update session: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to stop session")),
        );
    }

    // Release charger
    if let Some(mut c) = charger {
        c.status = EvChargerStatus::Available;
        let _ = state_guard.db.save_charger(&c).await;
    }

    (StatusCode::OK, Json(ApiResponse::success(session)))
}

/// `GET /api/v1/chargers/sessions` — user's charging history
#[utoipa::path(
    get,
    path = "/api/v1/chargers/sessions",
    tag = "EV Charging",
    summary = "User charging history",
    security(("bearer_auth" = []))
)]
pub async fn charging_history(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<Vec<ChargingSession>>>) {
    let state_guard = state.read().await;

    match state_guard
        .db
        .list_charging_sessions_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(sessions) => (StatusCode::OK, Json(ApiResponse::success(sessions))),
        Err(e) => {
            tracing::error!("Failed to list sessions: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "SERVER_ERROR",
                    "Failed to list sessions",
                )),
            )
        }
    }
}

/// `GET /api/v1/admin/chargers` — admin: all chargers with utilization
#[utoipa::path(
    get,
    path = "/api/v1/admin/chargers",
    tag = "Admin",
    summary = "Admin charger overview",
    security(("bearer_auth" = []))
)]
pub async fn admin_charger_overview(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<ChargerUtilizationStats>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let chargers = state_guard.db.list_all_chargers().await.unwrap_or_default();
    let sessions = state_guard
        .db
        .list_all_charging_sessions()
        .await
        .unwrap_or_default();

    let stats = ChargerUtilizationStats {
        total_chargers: chargers.len() as i32,
        available: chargers
            .iter()
            .filter(|c| c.status == EvChargerStatus::Available)
            .count() as i32,
        in_use: chargers
            .iter()
            .filter(|c| c.status == EvChargerStatus::InUse)
            .count() as i32,
        offline: chargers
            .iter()
            .filter(|c| c.status == EvChargerStatus::Offline)
            .count() as i32,
        total_sessions: sessions.len() as i32,
        total_kwh: sessions.iter().map(|s| s.kwh_consumed).sum(),
    };

    (StatusCode::OK, Json(ApiResponse::success(stats)))
}

/// `POST /api/v1/admin/chargers` — admin: add charger to lot
#[utoipa::path(
    post,
    path = "/api/v1/admin/chargers",
    tag = "Admin",
    summary = "Add charger to lot",
    security(("bearer_auth" = []))
)]
pub async fn admin_add_charger(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<AddChargerRequest>,
) -> (StatusCode, Json<ApiResponse<EvCharger>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let charger = EvCharger {
        id: Uuid::new_v4(),
        lot_id: req.lot_id,
        label: req.label,
        connector_type: req.connector_type,
        power_kw: req.power_kw,
        status: EvChargerStatus::Available,
        location_hint: req.location_hint,
        created_at: Utc::now(),
    };

    if let Err(e) = state_guard.db.save_charger(&charger).await {
        tracing::error!("Failed to save charger: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to add charger")),
        );
    }

    (StatusCode::CREATED, Json(ApiResponse::success(charger)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_charging_request_deserialize() {
        let json = r#"{"booking_id":"550e8400-e29b-41d4-a716-446655440000"}"#;
        let req: StartChargingRequest = serde_json::from_str(json).unwrap();
        assert!(req.booking_id.is_some());
    }

    #[test]
    fn test_start_charging_request_no_booking() {
        let json = r#"{}"#;
        let req: StartChargingRequest = serde_json::from_str(json).unwrap();
        assert!(req.booking_id.is_none());
    }

    #[test]
    fn test_add_charger_request_deserialize() {
        let json = r#"{
            "lot_id":"550e8400-e29b-41d4-a716-446655440000",
            "label":"Charger A1",
            "connector_type":"ccs",
            "power_kw":50.0,
            "location_hint":"Near entrance"
        }"#;
        let req: AddChargerRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.label, "Charger A1");
        assert_eq!(req.connector_type, ConnectorType::Ccs);
        assert_eq!(req.power_kw, 50.0);
        assert_eq!(req.location_hint.as_deref(), Some("Near entrance"));
    }

    #[test]
    fn test_connector_type_serialization() {
        assert_eq!(
            serde_json::to_string(&ConnectorType::Type2).unwrap(),
            "\"type2\""
        );
        assert_eq!(
            serde_json::to_string(&ConnectorType::Ccs).unwrap(),
            "\"ccs\""
        );
        assert_eq!(
            serde_json::to_string(&ConnectorType::Chademo).unwrap(),
            "\"chademo\""
        );
        assert_eq!(
            serde_json::to_string(&ConnectorType::Tesla).unwrap(),
            "\"tesla\""
        );
    }

    #[test]
    fn test_charger_status_serialization() {
        assert_eq!(
            serde_json::to_string(&EvChargerStatus::Available).unwrap(),
            "\"available\""
        );
        assert_eq!(
            serde_json::to_string(&EvChargerStatus::InUse).unwrap(),
            "\"in_use\""
        );
        assert_eq!(
            serde_json::to_string(&EvChargerStatus::Offline).unwrap(),
            "\"offline\""
        );
    }

    #[test]
    fn test_session_status_serialization() {
        assert_eq!(
            serde_json::to_string(&ChargingSessionStatus::Active).unwrap(),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&ChargingSessionStatus::Completed).unwrap(),
            "\"completed\""
        );
        assert_eq!(
            serde_json::to_string(&ChargingSessionStatus::Cancelled).unwrap(),
            "\"cancelled\""
        );
    }

    #[test]
    fn test_ev_charger_model_roundtrip() {
        let charger = EvCharger {
            id: Uuid::new_v4(),
            lot_id: Uuid::new_v4(),
            label: "Charger B2".to_string(),
            connector_type: ConnectorType::Type2,
            power_kw: 22.0,
            status: EvChargerStatus::Available,
            location_hint: Some("Floor -1, slot 42".to_string()),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&charger).unwrap();
        let back: EvCharger = serde_json::from_str(&json).unwrap();
        assert_eq!(back.label, "Charger B2");
        assert_eq!(back.power_kw, 22.0);
        assert_eq!(back.connector_type, ConnectorType::Type2);
    }

    #[test]
    fn test_charging_session_model_roundtrip() {
        let session = ChargingSession {
            id: Uuid::new_v4(),
            charger_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            booking_id: None,
            start_time: Utc::now(),
            end_time: None,
            kwh_consumed: 0.0,
            status: ChargingSessionStatus::Active,
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&session).unwrap();
        let back: ChargingSession = serde_json::from_str(&json).unwrap();
        assert_eq!(back.status, ChargingSessionStatus::Active);
        assert!(back.end_time.is_none());
    }

    #[test]
    fn test_utilization_stats_serialize() {
        let stats = ChargerUtilizationStats {
            total_chargers: 10,
            available: 6,
            in_use: 3,
            offline: 1,
            total_sessions: 150,
            total_kwh: 2500.5,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total_chargers\":10"));
        assert!(json.contains("\"total_kwh\":2500.5"));
    }
}
