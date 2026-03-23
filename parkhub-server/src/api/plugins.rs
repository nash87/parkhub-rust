//! Plugin/Extension System handlers.
//!
//! A modular plugin architecture that allows extending ParkHub with custom
//! integrations and automations.
//!
//! - `GET  /api/v1/admin/plugins`            — list installed plugins
//! - `PUT  /api/v1/admin/plugins/{id}/toggle` — enable/disable plugin
//! - `GET  /api/v1/admin/plugins/{id}/config` — get plugin config
//! - `PUT  /api/v1/admin/plugins/{id}/config` — update plugin config

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use parkhub_common::ApiResponse;

use super::SharedState;

// ═══════════════════════════════════════════════════════════════════════════════
// PLUGIN TRAIT & TYPES
// ═══════════════════════════════════════════════════════════════════════════════

/// Events that plugins can subscribe to
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginEvent {
    BookingCreated,
    BookingCancelled,
    UserRegistered,
    LotFull,
}

impl PluginEvent {
    /// All available plugin events
    pub const ALL: &[PluginEvent] = &[
        Self::BookingCreated,
        Self::BookingCancelled,
        Self::UserRegistered,
        Self::LotFull,
    ];

    /// Human-readable name for the event
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::BookingCreated => "Booking Created",
            Self::BookingCancelled => "Booking Cancelled",
            Self::UserRegistered => "User Registered",
            Self::LotFull => "Lot Full",
        }
    }
}

/// HTTP method for plugin-provided routes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

/// A route provided by a plugin
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PluginRoute {
    pub path: String,
    pub method: HttpMethod,
    pub description: String,
}

/// Plugin status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginStatus {
    Enabled,
    Disabled,
}

/// Core plugin trait — defines the contract for all plugins.
pub trait Plugin: Send + Sync {
    /// Unique plugin identifier (slug-style, e.g. "slack-notifier")
    fn id(&self) -> &str;
    /// Human-readable plugin name
    fn name(&self) -> &str;
    /// Semantic version string
    fn version(&self) -> &str;
    /// Short description of what the plugin does
    fn description(&self) -> &str;
    /// Author name
    fn author(&self) -> &str;
    /// Events this plugin subscribes to
    fn subscribed_events(&self) -> Vec<PluginEvent>;
    /// Handle an event — returns Ok(()) on success
    fn on_event(&self, event: &PluginEvent, payload: &serde_json::Value) -> Result<(), String>;
    /// Routes this plugin exposes (empty if none)
    fn routes(&self) -> Vec<PluginRoute>;
    /// Default configuration for this plugin
    fn default_config(&self) -> HashMap<String, serde_json::Value>;
}

/// Serializable plugin info (returned by API)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub status: PluginStatus,
    pub subscribed_events: Vec<PluginEvent>,
    pub routes: Vec<PluginRoute>,
    pub config: HashMap<String, serde_json::Value>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// BUILT-IN PLUGINS
// ═══════════════════════════════════════════════════════════════════════════════

/// Slack Notifier — sends notifications to a Slack channel on key events.
pub struct SlackNotifierPlugin;

impl Plugin for SlackNotifierPlugin {
    fn id(&self) -> &str {
        "slack-notifier"
    }
    fn name(&self) -> &str {
        "Slack Notifier"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "Sends notifications to a Slack channel when bookings are created or cancelled"
    }
    fn author(&self) -> &str {
        "ParkHub"
    }

    fn subscribed_events(&self) -> Vec<PluginEvent> {
        vec![
            PluginEvent::BookingCreated,
            PluginEvent::BookingCancelled,
            PluginEvent::LotFull,
        ]
    }

    fn on_event(&self, event: &PluginEvent, payload: &serde_json::Value) -> Result<(), String> {
        // In a real implementation this would POST to the configured Slack webhook URL.
        // For now, log the event.
        tracing::info!(
            plugin = "slack-notifier",
            event = ?event,
            payload = %payload,
            "Would send Slack notification"
        );
        Ok(())
    }

    fn routes(&self) -> Vec<PluginRoute> {
        vec![PluginRoute {
            path: "/api/v1/plugins/slack-notifier/test".to_string(),
            method: HttpMethod::Post,
            description: "Send a test notification to the configured Slack channel".to_string(),
        }]
    }

    fn default_config(&self) -> HashMap<String, serde_json::Value> {
        let mut cfg = HashMap::new();
        cfg.insert("webhook_url".to_string(), serde_json::json!(""));
        cfg.insert("channel".to_string(), serde_json::json!("#parking"));
        cfg.insert("notify_on_booking".to_string(), serde_json::json!(true));
        cfg.insert("notify_on_cancel".to_string(), serde_json::json!(true));
        cfg.insert("notify_on_lot_full".to_string(), serde_json::json!(true));
        cfg
    }
}

/// Auto-Assign Preferred Spot — automatically assigns a user's preferred parking
/// spot when they create a booking (if available).
pub struct AutoAssignPreferredSpotPlugin;

impl Plugin for AutoAssignPreferredSpotPlugin {
    fn id(&self) -> &str {
        "auto-assign-preferred"
    }
    fn name(&self) -> &str {
        "Auto-Assign Preferred Spot"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "Automatically assigns a user's preferred/favorite parking spot when creating a booking"
    }
    fn author(&self) -> &str {
        "ParkHub"
    }

    fn subscribed_events(&self) -> Vec<PluginEvent> {
        vec![PluginEvent::BookingCreated]
    }

    fn on_event(&self, event: &PluginEvent, payload: &serde_json::Value) -> Result<(), String> {
        if *event != PluginEvent::BookingCreated {
            return Ok(());
        }
        tracing::info!(
            plugin = "auto-assign-preferred",
            event = ?event,
            user_id = %payload.get("user_id").and_then(|v| v.as_str()).unwrap_or("unknown"),
            "Would auto-assign preferred spot"
        );
        Ok(())
    }

    fn routes(&self) -> Vec<PluginRoute> {
        vec![]
    }

    fn default_config(&self) -> HashMap<String, serde_json::Value> {
        let mut cfg = HashMap::new();
        cfg.insert("fallback_to_any".to_string(), serde_json::json!(true));
        cfg.insert(
            "respect_zone_preference".to_string(),
            serde_json::json!(true),
        );
        cfg
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// PLUGIN REGISTRY
// ═══════════════════════════════════════════════════════════════════════════════

/// Registry of all available plugins and their runtime state.
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
    status: HashMap<String, PluginStatus>,
    configs: HashMap<String, HashMap<String, serde_json::Value>>,
}

impl PluginRegistry {
    /// Create a new registry with built-in plugins loaded.
    pub fn new() -> Self {
        let plugins: Vec<Box<dyn Plugin>> = vec![
            Box::new(SlackNotifierPlugin),
            Box::new(AutoAssignPreferredSpotPlugin),
        ];

        let mut status = HashMap::new();
        let mut configs = HashMap::new();
        for p in &plugins {
            status.insert(p.id().to_string(), PluginStatus::Disabled);
            configs.insert(p.id().to_string(), p.default_config());
        }

        Self {
            plugins,
            status,
            configs,
        }
    }

    /// List all plugins with their current state.
    pub fn list(&self) -> Vec<PluginInfo> {
        self.plugins
            .iter()
            .map(|p| PluginInfo {
                id: p.id().to_string(),
                name: p.name().to_string(),
                version: p.version().to_string(),
                description: p.description().to_string(),
                author: p.author().to_string(),
                status: self
                    .status
                    .get(p.id())
                    .cloned()
                    .unwrap_or(PluginStatus::Disabled),
                subscribed_events: p.subscribed_events(),
                routes: p.routes(),
                config: self.configs.get(p.id()).cloned().unwrap_or_default(),
            })
            .collect()
    }

    /// Get a single plugin by ID.
    pub fn get(&self, id: &str) -> Option<PluginInfo> {
        self.list().into_iter().find(|p| p.id == id)
    }

    /// Toggle a plugin's enabled/disabled state. Returns the new status.
    pub fn toggle(&mut self, id: &str) -> Option<PluginStatus> {
        if !self.plugins.iter().any(|p| p.id() == id) {
            return None;
        }
        let current = self
            .status
            .get(id)
            .cloned()
            .unwrap_or(PluginStatus::Disabled);
        let new_status = match current {
            PluginStatus::Enabled => PluginStatus::Disabled,
            PluginStatus::Disabled => PluginStatus::Enabled,
        };
        self.status.insert(id.to_string(), new_status.clone());
        Some(new_status)
    }

    /// Enable a specific plugin.
    pub fn enable(&mut self, id: &str) -> bool {
        if !self.plugins.iter().any(|p| p.id() == id) {
            return false;
        }
        self.status.insert(id.to_string(), PluginStatus::Enabled);
        true
    }

    /// Disable a specific plugin.
    pub fn disable(&mut self, id: &str) -> bool {
        if !self.plugins.iter().any(|p| p.id() == id) {
            return false;
        }
        self.status.insert(id.to_string(), PluginStatus::Disabled);
        true
    }

    /// Get the configuration for a plugin.
    pub fn get_config(&self, id: &str) -> Option<HashMap<String, serde_json::Value>> {
        self.configs.get(id).cloned()
    }

    /// Update the configuration for a plugin. Merges with existing config.
    pub fn update_config(
        &mut self,
        id: &str,
        updates: HashMap<String, serde_json::Value>,
    ) -> Option<HashMap<String, serde_json::Value>> {
        if !self.plugins.iter().any(|p| p.id() == id) {
            return None;
        }
        let config = self.configs.entry(id.to_string()).or_default();
        for (k, v) in updates {
            config.insert(k, v);
        }
        Some(config.clone())
    }

    /// Fire an event to all enabled plugins that subscribe to it.
    pub fn fire_event(&self, event: &PluginEvent, payload: &serde_json::Value) -> Vec<String> {
        let mut errors = Vec::new();
        for plugin in &self.plugins {
            let id = plugin.id();
            let is_enabled = self
                .status
                .get(id)
                .is_some_and(|s| *s == PluginStatus::Enabled);
            if !is_enabled {
                continue;
            }
            if !plugin.subscribed_events().contains(event) {
                continue;
            }
            if let Err(e) = plugin.on_event(event, payload) {
                errors.push(format!("{id}: {e}"));
            }
        }
        errors
    }

    /// Get the number of registered plugins.
    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    /// Get the number of enabled plugins.
    pub fn enabled_count(&self) -> usize {
        self.status
            .values()
            .filter(|s| **s == PluginStatus::Enabled)
            .count()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// API REQUEST/RESPONSE TYPES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct UpdatePluginConfigRequest {
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct PluginListResponse {
    pub plugins: Vec<PluginInfo>,
    pub total: usize,
    pub enabled: usize,
}

#[derive(Debug, Serialize)]
pub struct PluginToggleResponse {
    pub id: String,
    pub status: PluginStatus,
}

// ═══════════════════════════════════════════════════════════════════════════════
// HANDLERS
// ═══════════════════════════════════════════════════════════════════════════════

/// `GET /api/v1/admin/plugins` — list all installed plugins with status.
pub async fn list_plugins(
    State(_state): State<SharedState>,
) -> (StatusCode, Json<ApiResponse<PluginListResponse>>) {
    let registry = PluginRegistry::new();
    let plugins = registry.list();
    let total = plugins.len();
    let enabled = plugins
        .iter()
        .filter(|p| p.status == PluginStatus::Enabled)
        .count();

    (
        StatusCode::OK,
        Json(ApiResponse::success(PluginListResponse {
            plugins,
            total,
            enabled,
        })),
    )
}

/// `PUT /api/v1/admin/plugins/{id}/toggle` — enable or disable a plugin.
pub async fn toggle_plugin(
    State(_state): State<SharedState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<PluginToggleResponse>>) {
    let mut registry = PluginRegistry::new();

    match registry.toggle(&id) {
        Some(new_status) => (
            StatusCode::OK,
            Json(ApiResponse::success(PluginToggleResponse {
                id,
                status: new_status,
            })),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Plugin not found")),
        ),
    }
}

/// `GET /api/v1/admin/plugins/{id}/config` — get plugin configuration.
pub async fn get_plugin_config(
    State(_state): State<SharedState>,
    Path(id): Path<String>,
) -> (
    StatusCode,
    Json<ApiResponse<HashMap<String, serde_json::Value>>>,
) {
    let registry = PluginRegistry::new();

    match registry.get_config(&id) {
        Some(config) => (StatusCode::OK, Json(ApiResponse::success(config))),
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Plugin not found")),
        ),
    }
}

/// `PUT /api/v1/admin/plugins/{id}/config` — update plugin configuration.
pub async fn update_plugin_config(
    State(_state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<UpdatePluginConfigRequest>,
) -> (
    StatusCode,
    Json<ApiResponse<HashMap<String, serde_json::Value>>>,
) {
    let mut registry = PluginRegistry::new();

    match registry.update_config(&id, body.config) {
        Some(config) => (StatusCode::OK, Json(ApiResponse::success(config))),
        None => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Plugin not found")),
        ),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_registry_new() {
        let registry = PluginRegistry::new();
        assert_eq!(registry.count(), 2);
        assert_eq!(registry.enabled_count(), 0);
    }

    #[test]
    fn test_plugin_registry_list() {
        let registry = PluginRegistry::new();
        let plugins = registry.list();
        assert_eq!(plugins.len(), 2);

        let slack = plugins.iter().find(|p| p.id == "slack-notifier").unwrap();
        assert_eq!(slack.name, "Slack Notifier");
        assert_eq!(slack.version, "1.0.0");
        assert_eq!(slack.status, PluginStatus::Disabled);
        assert_eq!(slack.subscribed_events.len(), 3);
        assert_eq!(slack.routes.len(), 1);

        let auto = plugins
            .iter()
            .find(|p| p.id == "auto-assign-preferred")
            .unwrap();
        assert_eq!(auto.name, "Auto-Assign Preferred Spot");
        assert_eq!(auto.subscribed_events.len(), 1);
        assert!(auto.routes.is_empty());
    }

    #[test]
    fn test_plugin_toggle() {
        let mut registry = PluginRegistry::new();

        // Initially disabled
        let info = registry.get("slack-notifier").unwrap();
        assert_eq!(info.status, PluginStatus::Disabled);

        // Toggle to enabled
        let status = registry.toggle("slack-notifier").unwrap();
        assert_eq!(status, PluginStatus::Enabled);
        assert_eq!(registry.enabled_count(), 1);

        // Toggle back to disabled
        let status = registry.toggle("slack-notifier").unwrap();
        assert_eq!(status, PluginStatus::Disabled);
        assert_eq!(registry.enabled_count(), 0);
    }

    #[test]
    fn test_plugin_toggle_nonexistent() {
        let mut registry = PluginRegistry::new();
        assert!(registry.toggle("nonexistent").is_none());
    }

    #[test]
    fn test_plugin_enable_disable() {
        let mut registry = PluginRegistry::new();

        assert!(registry.enable("slack-notifier"));
        assert_eq!(registry.enabled_count(), 1);

        assert!(registry.disable("slack-notifier"));
        assert_eq!(registry.enabled_count(), 0);

        assert!(!registry.enable("nonexistent"));
        assert!(!registry.disable("nonexistent"));
    }

    #[test]
    fn test_plugin_config_get() {
        let registry = PluginRegistry::new();

        let config = registry.get_config("slack-notifier").unwrap();
        assert!(config.contains_key("webhook_url"));
        assert!(config.contains_key("channel"));
        assert_eq!(config["channel"], serde_json::json!("#parking"));
    }

    #[test]
    fn test_plugin_config_update() {
        let mut registry = PluginRegistry::new();

        let mut updates = HashMap::new();
        updates.insert(
            "webhook_url".to_string(),
            serde_json::json!("https://hooks.slack.com/test"),
        );
        updates.insert("channel".to_string(), serde_json::json!("#parking-alerts"));

        let config = registry.update_config("slack-notifier", updates).unwrap();
        assert_eq!(
            config["webhook_url"],
            serde_json::json!("https://hooks.slack.com/test")
        );
        assert_eq!(config["channel"], serde_json::json!("#parking-alerts"));
    }

    #[test]
    fn test_plugin_config_update_nonexistent() {
        let mut registry = PluginRegistry::new();
        assert!(registry
            .update_config("nonexistent", HashMap::new())
            .is_none());
    }

    #[test]
    fn test_plugin_fire_event_disabled() {
        let registry = PluginRegistry::new();
        // All plugins disabled — no errors, no actions
        let errors = registry.fire_event(
            &PluginEvent::BookingCreated,
            &serde_json::json!({"booking_id": "test"}),
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_plugin_fire_event_enabled() {
        let mut registry = PluginRegistry::new();
        registry.enable("slack-notifier");

        let errors = registry.fire_event(
            &PluginEvent::BookingCreated,
            &serde_json::json!({"booking_id": "b1", "user_id": "u1"}),
        );
        assert!(errors.is_empty());
    }

    #[test]
    fn test_plugin_fire_event_wrong_event() {
        let mut registry = PluginRegistry::new();
        registry.enable("auto-assign-preferred");

        // auto-assign only subscribes to BookingCreated, not LotFull
        let errors =
            registry.fire_event(&PluginEvent::LotFull, &serde_json::json!({"lot_id": "l1"}));
        assert!(errors.is_empty());
    }

    #[test]
    fn test_plugin_event_display_names() {
        assert_eq!(
            PluginEvent::BookingCreated.display_name(),
            "Booking Created"
        );
        assert_eq!(
            PluginEvent::BookingCancelled.display_name(),
            "Booking Cancelled"
        );
        assert_eq!(
            PluginEvent::UserRegistered.display_name(),
            "User Registered"
        );
        assert_eq!(PluginEvent::LotFull.display_name(), "Lot Full");
    }

    #[test]
    fn test_plugin_event_all() {
        assert_eq!(PluginEvent::ALL.len(), 4);
    }

    #[test]
    fn test_plugin_event_serialize() {
        assert_eq!(
            serde_json::to_string(&PluginEvent::BookingCreated).unwrap(),
            "\"booking_created\""
        );
        assert_eq!(
            serde_json::to_string(&PluginEvent::LotFull).unwrap(),
            "\"lot_full\""
        );
    }

    #[test]
    fn test_plugin_event_deserialize() {
        let e: PluginEvent = serde_json::from_str("\"booking_created\"").unwrap();
        assert_eq!(e, PluginEvent::BookingCreated);
        let e: PluginEvent = serde_json::from_str("\"user_registered\"").unwrap();
        assert_eq!(e, PluginEvent::UserRegistered);
    }

    #[test]
    fn test_plugin_info_serialize() {
        let info = PluginInfo {
            id: "test".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A test plugin".to_string(),
            author: "Test".to_string(),
            status: PluginStatus::Enabled,
            subscribed_events: vec![PluginEvent::BookingCreated],
            routes: vec![],
            config: HashMap::new(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"id\":\"test\""));
        assert!(json.contains("\"status\":\"enabled\""));
    }

    #[test]
    fn test_plugin_status_serialize() {
        assert_eq!(
            serde_json::to_string(&PluginStatus::Enabled).unwrap(),
            "\"enabled\""
        );
        assert_eq!(
            serde_json::to_string(&PluginStatus::Disabled).unwrap(),
            "\"disabled\""
        );
    }

    #[test]
    fn test_plugin_list_response_serialize() {
        let resp = PluginListResponse {
            plugins: vec![],
            total: 0,
            enabled: 0,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"total\":0"));
        assert!(json.contains("\"enabled\":0"));
    }

    #[test]
    fn test_plugin_default_config_auto_assign() {
        let registry = PluginRegistry::new();
        let config = registry.get_config("auto-assign-preferred").unwrap();
        assert_eq!(config["fallback_to_any"], serde_json::json!(true));
        assert_eq!(config["respect_zone_preference"], serde_json::json!(true));
    }

    #[test]
    fn test_plugin_registry_default_trait() {
        let registry = PluginRegistry::default();
        assert_eq!(registry.count(), 2);
    }

    #[test]
    fn test_http_method_serialize() {
        assert_eq!(serde_json::to_string(&HttpMethod::Get).unwrap(), "\"GET\"");
        assert_eq!(
            serde_json::to_string(&HttpMethod::Post).unwrap(),
            "\"POST\""
        );
    }

    #[test]
    fn test_plugin_route_serialize() {
        let route = PluginRoute {
            path: "/test".to_string(),
            method: HttpMethod::Post,
            description: "Test route".to_string(),
        };
        let json = serde_json::to_string(&route).unwrap();
        assert!(json.contains("\"path\":\"/test\""));
        assert!(json.contains("\"method\":\"POST\""));
    }

    #[test]
    fn test_update_config_request_deserialize() {
        let json = r#"{"config":{"webhook_url":"https://example.com"}}"#;
        let req: UpdatePluginConfigRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.config.len(), 1);
        assert_eq!(
            req.config["webhook_url"],
            serde_json::json!("https://example.com")
        );
    }

    #[test]
    fn test_fire_event_multiple_plugins() {
        let mut registry = PluginRegistry::new();
        registry.enable("slack-notifier");
        registry.enable("auto-assign-preferred");
        assert_eq!(registry.enabled_count(), 2);

        // BookingCreated fires both plugins
        let errors = registry.fire_event(
            &PluginEvent::BookingCreated,
            &serde_json::json!({"booking_id": "b1", "user_id": "u1"}),
        );
        assert!(errors.is_empty());
    }
}
