//! Customizable Admin Dashboard Widgets handlers.
//!
//! Per-user widget layout with draggable/resizable widget cards.
//!
//! - `GET /api/v1/admin/widgets` — get user's widget layout
//! - `PUT /api/v1/admin/widgets` — save widget layout
//! - `GET /api/v1/admin/widgets/data/{widget_id}` — get data for a specific widget

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use parkhub_common::{ApiResponse, UserRole};

use super::{AuthUser, SharedState};

// ═══════════════════════════════════════════════════════════════════════════════
// TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// Available widget types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    OccupancyChart,
    RevenueSummary,
    RecentBookings,
    UserGrowth,
    BookingHeatmap,
    ActiveAlerts,
    MaintenanceStatus,
    EvChargingStatus,
}

#[allow(dead_code)]
impl WidgetType {
    /// All available widget types
    pub const ALL: &[WidgetType] = &[
        Self::OccupancyChart,
        Self::RevenueSummary,
        Self::RecentBookings,
        Self::UserGrowth,
        Self::BookingHeatmap,
        Self::ActiveAlerts,
        Self::MaintenanceStatus,
        Self::EvChargingStatus,
    ];

    /// Get the display name of the widget
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::OccupancyChart => "Occupancy Chart",
            Self::RevenueSummary => "Revenue Summary",
            Self::RecentBookings => "Recent Bookings",
            Self::UserGrowth => "User Growth",
            Self::BookingHeatmap => "Booking Heatmap",
            Self::ActiveAlerts => "Active Alerts",
            Self::MaintenanceStatus => "Maintenance Status",
            Self::EvChargingStatus => "EV Charging Status",
        }
    }
}

/// Position and size of a widget on the dashboard grid
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WidgetPosition {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// A widget entry in the user's layout
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WidgetEntry {
    pub id: String,
    pub widget_type: WidgetType,
    pub position: WidgetPosition,
    pub visible: bool,
}

/// The full widget layout for a user
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WidgetLayout {
    pub user_id: Uuid,
    pub widgets: Vec<WidgetEntry>,
}

/// Request to save a widget layout
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct SaveWidgetLayoutRequest {
    pub widgets: Vec<WidgetEntry>,
}

/// Widget data response — generic container
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WidgetData {
    pub widget_id: String,
    pub widget_type: WidgetType,
    pub title: String,
    pub data: serde_json::Value,
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Default widget layout for new admins
fn default_layout(user_id: Uuid) -> WidgetLayout {
    WidgetLayout {
        user_id,
        widgets: vec![
            WidgetEntry {
                id: "w1".to_string(),
                widget_type: WidgetType::OccupancyChart,
                position: WidgetPosition {
                    x: 0,
                    y: 0,
                    w: 6,
                    h: 4,
                },
                visible: true,
            },
            WidgetEntry {
                id: "w2".to_string(),
                widget_type: WidgetType::RevenueSummary,
                position: WidgetPosition {
                    x: 6,
                    y: 0,
                    w: 6,
                    h: 4,
                },
                visible: true,
            },
            WidgetEntry {
                id: "w3".to_string(),
                widget_type: WidgetType::RecentBookings,
                position: WidgetPosition {
                    x: 0,
                    y: 4,
                    w: 4,
                    h: 3,
                },
                visible: true,
            },
            WidgetEntry {
                id: "w4".to_string(),
                widget_type: WidgetType::UserGrowth,
                position: WidgetPosition {
                    x: 4,
                    y: 4,
                    w: 4,
                    h: 3,
                },
                visible: true,
            },
            WidgetEntry {
                id: "w5".to_string(),
                widget_type: WidgetType::ActiveAlerts,
                position: WidgetPosition {
                    x: 8,
                    y: 4,
                    w: 4,
                    h: 3,
                },
                visible: true,
            },
        ],
    }
}

/// Check if user is admin
async fn is_admin(state: &crate::AppState, auth_user: &AuthUser) -> bool {
    match state.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(u)) => u.role == UserRole::Admin || u.role == UserRole::SuperAdmin,
        _ => false,
    }
}

/// Generate sample widget data
fn generate_widget_data(widget_type: &WidgetType) -> serde_json::Value {
    match widget_type {
        WidgetType::OccupancyChart => serde_json::json!({
            "current": 42,
            "total": 100,
            "percentage": 42.0,
            "trend": "up",
        }),
        WidgetType::RevenueSummary => serde_json::json!({
            "today": 1250.50,
            "this_week": 8750.00,
            "this_month": 35000.00,
            "currency": "EUR",
        }),
        WidgetType::RecentBookings => serde_json::json!({
            "bookings": [
                { "user": "Alice", "slot": "A1", "time": "08:00-18:00" },
                { "user": "Bob", "slot": "B3", "time": "09:00-17:00" },
            ],
            "count": 2,
        }),
        WidgetType::UserGrowth => serde_json::json!({
            "total_users": 156,
            "new_this_month": 12,
            "active_today": 45,
        }),
        WidgetType::BookingHeatmap => serde_json::json!({
            "peak_hour": "09:00",
            "peak_day": "Monday",
            "avg_daily": 35,
        }),
        WidgetType::ActiveAlerts => serde_json::json!({
            "alerts": [],
            "count": 0,
        }),
        WidgetType::MaintenanceStatus => serde_json::json!({
            "scheduled": 0,
            "active": 0,
            "completed_this_week": 2,
        }),
        WidgetType::EvChargingStatus => serde_json::json!({
            "total_chargers": 8,
            "in_use": 3,
            "available": 5,
            "energy_today_kwh": 125.5,
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/admin/widgets` — get user's widget layout
#[utoipa::path(get, path = "/api/v1/admin/widgets", tag = "Admin Widgets",
    summary = "Get widget layout",
    description = "Get the current admin user's dashboard widget layout.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Widget layout"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn get_widget_layout(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<WidgetLayout>>) {
    let state_guard = state.read().await;

    if !is_admin(&state_guard, &auth_user).await {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    let key = format!("widget_layout:{}", auth_user.user_id);
    let layout = match state_guard.db.get_setting(&key).await {
        Ok(Some(json_str)) => serde_json::from_str::<WidgetLayout>(&json_str)
            .unwrap_or_else(|_| default_layout(auth_user.user_id)),
        _ => default_layout(auth_user.user_id),
    };

    (StatusCode::OK, Json(ApiResponse::success(layout)))
}

/// `PUT /api/v1/admin/widgets` — save widget layout
#[utoipa::path(put, path = "/api/v1/admin/widgets", tag = "Admin Widgets",
    summary = "Save widget layout",
    description = "Save the admin user's dashboard widget layout (positions, sizes, visibility).",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Layout saved"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn save_widget_layout(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<SaveWidgetLayoutRequest>,
) -> (StatusCode, Json<ApiResponse<WidgetLayout>>) {
    let state_guard = state.read().await;

    if !is_admin(&state_guard, &auth_user).await {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    let layout = WidgetLayout {
        user_id: auth_user.user_id,
        widgets: req.widgets,
    };

    let key = format!("widget_layout:{}", auth_user.user_id);
    let json_str = match serde_json::to_string(&layout) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to serialize widget layout: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Serialization error")),
            );
        }
    };

    if let Err(e) = state_guard.db.set_setting(&key, &json_str).await {
        tracing::error!("Failed to save widget layout: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to save layout")),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(layout)))
}

/// `GET /api/v1/admin/widgets/data/{widget_id}` — get data for a specific widget
#[utoipa::path(get, path = "/api/v1/admin/widgets/data/{widget_id}", tag = "Admin Widgets",
    summary = "Get widget data",
    description = "Get the data for a specific dashboard widget.",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Widget data"),
        (status = 404, description = "Widget not found"),
        (status = 403, description = "Admin access required"),
    )
)]
pub async fn get_widget_data(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(widget_id): Path<String>,
) -> (StatusCode, Json<ApiResponse<WidgetData>>) {
    let state_guard = state.read().await;

    if !is_admin(&state_guard, &auth_user).await {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error("FORBIDDEN", "Admin access required")),
        );
    }

    // Parse widget_id to determine type
    let widget_type = match widget_id.as_str() {
        "occupancy_chart" => WidgetType::OccupancyChart,
        "revenue_summary" => WidgetType::RevenueSummary,
        "recent_bookings" => WidgetType::RecentBookings,
        "user_growth" => WidgetType::UserGrowth,
        "booking_heatmap" => WidgetType::BookingHeatmap,
        "active_alerts" => WidgetType::ActiveAlerts,
        "maintenance_status" => WidgetType::MaintenanceStatus,
        "ev_charging_status" => WidgetType::EvChargingStatus,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Widget not found")),
            );
        }
    };

    let data = generate_widget_data(&widget_type);

    (
        StatusCode::OK,
        Json(ApiResponse::success(WidgetData {
            widget_id,
            widget_type: widget_type.clone(),
            title: widget_type.display_name().to_string(),
            data,
        })),
    )
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_widget_type_serialize() {
        assert_eq!(
            serde_json::to_string(&WidgetType::OccupancyChart).unwrap(),
            "\"occupancy_chart\""
        );
        assert_eq!(
            serde_json::to_string(&WidgetType::RevenueSummary).unwrap(),
            "\"revenue_summary\""
        );
        assert_eq!(
            serde_json::to_string(&WidgetType::RecentBookings).unwrap(),
            "\"recent_bookings\""
        );
        assert_eq!(
            serde_json::to_string(&WidgetType::EvChargingStatus).unwrap(),
            "\"ev_charging_status\""
        );
    }

    #[test]
    fn test_widget_type_deserialize() {
        let t: WidgetType = serde_json::from_str("\"occupancy_chart\"").unwrap();
        assert_eq!(t, WidgetType::OccupancyChart);
        let t: WidgetType = serde_json::from_str("\"ev_charging_status\"").unwrap();
        assert_eq!(t, WidgetType::EvChargingStatus);
    }

    #[test]
    fn test_widget_type_all() {
        assert_eq!(WidgetType::ALL.len(), 8);
    }

    #[test]
    fn test_widget_type_display_name() {
        assert_eq!(WidgetType::OccupancyChart.display_name(), "Occupancy Chart");
        assert_eq!(
            WidgetType::EvChargingStatus.display_name(),
            "EV Charging Status"
        );
    }

    #[test]
    fn test_widget_position_serialize() {
        let pos = WidgetPosition {
            x: 0,
            y: 0,
            w: 6,
            h: 4,
        };
        let json = serde_json::to_string(&pos).unwrap();
        assert!(json.contains("\"x\":0"));
        assert!(json.contains("\"w\":6"));
    }

    #[test]
    fn test_widget_entry_serialize() {
        let entry = WidgetEntry {
            id: "w1".to_string(),
            widget_type: WidgetType::OccupancyChart,
            position: WidgetPosition {
                x: 0,
                y: 0,
                w: 6,
                h: 4,
            },
            visible: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"id\":\"w1\""));
        assert!(json.contains("\"widget_type\":\"occupancy_chart\""));
        assert!(json.contains("\"visible\":true"));
    }

    #[test]
    fn test_widget_layout_roundtrip() {
        let layout = default_layout(Uuid::nil());
        let json = serde_json::to_string(&layout).unwrap();
        let parsed: WidgetLayout = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.widgets.len(), 5);
        assert_eq!(parsed.widgets[0].widget_type, WidgetType::OccupancyChart);
    }

    #[test]
    fn test_default_layout_has_expected_widgets() {
        let layout = default_layout(Uuid::new_v4());
        assert_eq!(layout.widgets.len(), 5);
        assert!(layout.widgets.iter().all(|w| w.visible));
        assert_eq!(layout.widgets[0].id, "w1");
        assert_eq!(layout.widgets[1].id, "w2");
    }

    #[test]
    fn test_save_widget_layout_request_deserialize() {
        let json = r#"{"widgets":[{"id":"w1","widget_type":"occupancy_chart","position":{"x":0,"y":0,"w":6,"h":4},"visible":true}]}"#;
        let req: SaveWidgetLayoutRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.widgets.len(), 1);
        assert_eq!(req.widgets[0].widget_type, WidgetType::OccupancyChart);
    }

    #[test]
    fn test_generate_widget_data_occupancy() {
        let data = generate_widget_data(&WidgetType::OccupancyChart);
        assert!(data.get("current").is_some());
        assert!(data.get("total").is_some());
        assert!(data.get("percentage").is_some());
    }

    #[test]
    fn test_generate_widget_data_revenue() {
        let data = generate_widget_data(&WidgetType::RevenueSummary);
        assert!(data.get("today").is_some());
        assert!(data.get("currency").is_some());
    }

    #[test]
    fn test_generate_widget_data_all_types() {
        for widget_type in WidgetType::ALL {
            let data = generate_widget_data(widget_type);
            assert!(
                data.is_object(),
                "Widget data for {:?} should be an object",
                widget_type
            );
        }
    }

    #[test]
    fn test_widget_data_serialize() {
        let data = WidgetData {
            widget_id: "occupancy_chart".to_string(),
            widget_type: WidgetType::OccupancyChart,
            title: "Occupancy Chart".to_string(),
            data: serde_json::json!({"current": 42}),
        };
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"widget_id\":\"occupancy_chart\""));
        assert!(json.contains("\"title\":\"Occupancy Chart\""));
    }
}
