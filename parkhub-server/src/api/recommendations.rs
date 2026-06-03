//! Smart parking slot recommendations based on user behavior and availability.
//!
//! Scoring algorithm:
//! - frequency_score (40%): how often the user booked this slot/lot
//! - availability_score (30%): slot is currently available
//! - price_score (20%): cheaper slots score higher
//! - distance_score (10%): proximity to entrance / accessibility match

// AppState read/write guards are held across handler duration by design —
// db access goes through its own inner RwLock. See workspace lint config.
#![allow(clippy::significant_drop_tightening)]

use axum::{
    Extension, Json,
    extract::{Query, State},
    http::StatusCode,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write as _,
    sync::OnceLock,
    time::Duration,
};
use uuid::Uuid;

use parkhub_common::ApiResponse;
use parkhub_common::models::{BookingStatus, SlotFeature, SlotStatus};

use super::modules::config_setting_key;
use super::{AuthUser, SharedState, check_admin};

const RECOMMENDATION_MODULE: &str = "recommendations";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RecommendationWeights {
    pub frequency: f64,
    pub preferred_lot: f64,
    pub availability: f64,
    pub price: f64,
    pub distance: f64,
    pub accessibility_bonus: f64,
    pub feature_bonus: f64,
}

impl Default for RecommendationWeights {
    fn default() -> Self {
        Self {
            frequency: 40.0,
            preferred_lot: 20.0,
            availability: 30.0,
            price: 20.0,
            distance: 10.0,
            accessibility_bonus: 0.0,
            feature_bonus: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RecommendationEngineConfig {
    pub algorithm: String,
    pub weights: RecommendationWeights,
    pub max_results: usize,
    pub explain: bool,
    pub profile_safe_mode: bool,
    pub pipeline: RecommendationPipelineConfig,
    pub allocation: RecommendationAllocationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RecommendationPipelineConfig {
    pub endpoint: Option<String>,
    pub pipeline_name: String,
    pub timeout_ms: u64,
    pub fallback_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct RecommendationAllocationConfig {
    pub strategy: String,
    pub exact_cover_max_options: usize,
    pub exact_cover_max_search_nodes: usize,
}

impl Default for RecommendationPipelineConfig {
    fn default() -> Self {
        Self {
            endpoint: None,
            pipeline_name: "parkhub-recommendations".to_string(),
            timeout_ms: 750,
            fallback_enabled: true,
        }
    }
}

impl Default for RecommendationAllocationConfig {
    fn default() -> Self {
        Self {
            strategy: "weighted_v1".to_string(),
            exact_cover_max_options: 256,
            exact_cover_max_search_nodes: 10_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct RecommendationAdapterStatus {
    pub requested_algorithm: String,
    pub effective_algorithm: String,
    pub attempted: bool,
    pub status: String,
    pub pipeline_name: Option<String>,
    pub endpoint_configured: bool,
    pub fallback_enabled: bool,
    pub error: Option<String>,
}

impl Default for RecommendationEngineConfig {
    fn default() -> Self {
        Self {
            algorithm: "weighted_v1".to_string(),
            weights: RecommendationWeights::default(),
            max_results: 5,
            explain: true,
            profile_safe_mode: true,
            pipeline: RecommendationPipelineConfig::default(),
            allocation: RecommendationAllocationConfig::default(),
        }
    }
}

impl RecommendationEngineConfig {
    pub(crate) async fn load(db: &crate::db::Database) -> Self {
        let mut cfg = Self::default();
        cfg.weights.frequency =
            read_module_f64(db, "weight_frequency", cfg.weights.frequency, 0.0, 100.0).await;
        cfg.weights.preferred_lot = read_module_f64(
            db,
            "weight_preferred_lot",
            cfg.weights.preferred_lot,
            0.0,
            100.0,
        )
        .await;
        cfg.weights.availability = read_module_f64(
            db,
            "weight_availability",
            cfg.weights.availability,
            0.0,
            100.0,
        )
        .await;
        cfg.weights.price =
            read_module_f64(db, "weight_price", cfg.weights.price, 0.0, 100.0).await;
        cfg.weights.distance =
            read_module_f64(db, "weight_distance", cfg.weights.distance, 0.0, 100.0).await;
        cfg.weights.accessibility_bonus = read_module_f64(
            db,
            "weight_accessibility_bonus",
            cfg.weights.accessibility_bonus,
            0.0,
            25.0,
        )
        .await;
        cfg.weights.feature_bonus = read_module_f64(
            db,
            "weight_feature_bonus",
            cfg.weights.feature_bonus,
            0.0,
            25.0,
        )
        .await;
        cfg.max_results = read_module_usize(db, "max_results", cfg.max_results, 1, 25).await;
        if !read_module_bool(db, "explain", true).await {
            tracing::warn!(
                "recommendation explain=false ignored; explanations are required until legal/privacy review approves disabling them"
            );
        }
        if !read_module_bool(db, "profile_safe_mode", true).await {
            tracing::warn!(
                "recommendation profile_safe_mode=false ignored; privacy guardrail is fail-closed until legal/privacy review approves disabling it"
            );
        }
        cfg.explain = true;
        cfg.profile_safe_mode = true;
        cfg.pipeline.endpoint =
            validate_pipeline_endpoint(read_module_optional_string(db, "pipeline_endpoint").await);
        cfg.pipeline.pipeline_name = read_module_string(
            db,
            "pipeline_name",
            &RecommendationPipelineConfig::default().pipeline_name,
        )
        .await;
        if cfg.pipeline.pipeline_name.trim().is_empty() {
            cfg.pipeline.pipeline_name = RecommendationPipelineConfig::default().pipeline_name;
        }
        cfg.pipeline.timeout_ms = read_module_u64(db, "pipeline_timeout_ms", 750, 100, 5_000).await;
        if !read_module_bool(db, "pipeline_fallback_enabled", true).await {
            tracing::warn!(
                "recommendation pipeline_fallback_enabled=false ignored; weighted_v1 fallback is required until fop_pipeline_v1 is production-certified"
            );
        }
        cfg.pipeline.fallback_enabled = true;
        cfg.algorithm = read_module_string(db, "algorithm", &cfg.algorithm).await;
        if !matches!(cfg.algorithm.as_str(), "weighted_v1" | "fop_pipeline_v1") {
            tracing::warn!(
                algorithm = %cfg.algorithm,
                "unknown recommendation algorithm requested; falling back to weighted_v1"
            );
            cfg.algorithm = "weighted_v1".to_string();
        }
        let requested_allocation_strategy =
            read_module_string(db, "allocation_strategy", "weighted_v1").await;
        if !is_supported_allocation_strategy(&requested_allocation_strategy) {
            tracing::warn!(
                allocation_strategy = %requested_allocation_strategy,
                "unknown allocation strategy requested; falling back to weighted_v1"
            );
        }
        cfg.allocation.strategy = normalize_allocation_strategy(&requested_allocation_strategy);
        cfg.allocation.exact_cover_max_options =
            read_module_usize(db, "exact_cover_max_options", 256, 1, 256).await;
        cfg.allocation.exact_cover_max_search_nodes =
            read_module_usize(db, "exact_cover_max_search_nodes", 10_000, 1, 10_000).await;
        cfg
    }
}

async fn read_module_optional_string(db: &crate::db::Database, field: &str) -> Option<String> {
    let value = read_module_string(db, field, "").await;
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn validate_pipeline_endpoint(endpoint: Option<String>) -> Option<String> {
    let endpoint = endpoint?;
    match reqwest::Url::parse(&endpoint) {
        Ok(url) if matches!(url.scheme(), "http" | "https") => {
            let allowed_host = url.host_str().is_some_and(|host| {
                let host = host.to_ascii_lowercase();
                is_loopback_or_localhost(&host)
                    || is_local_dev_test_host(&host)
                    || is_kubernetes_service_host(&host)
            });
            if allowed_host {
                Some(endpoint)
            } else {
                tracing::warn!(
                    endpoint = %url,
                    "recommendation pipeline_endpoint rejected by local/cluster allowlist"
                );
                None
            }
        }
        Ok(url) => {
            tracing::warn!(endpoint = %url, "recommendation pipeline_endpoint rejected by scheme");
            None
        }
        Err(err) => {
            tracing::warn!(endpoint = %endpoint, error = %err, "recommendation pipeline_endpoint rejected as invalid URL");
            None
        }
    }
}

fn is_loopback_or_localhost(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

fn is_local_dev_test_host(host: &str) -> bool {
    let labels = host.split('.').collect::<Vec<_>>();
    labels.len() >= 2
        && labels.last().is_some_and(|suffix| *suffix == "test")
        && labels.iter().all(|label| !label.is_empty())
}

fn normalize_allocation_strategy(strategy: &str) -> String {
    let strategy = strategy.trim();
    if is_supported_allocation_strategy(strategy) {
        strategy.to_string()
    } else {
        "weighted_v1".to_string()
    }
}

fn is_supported_allocation_strategy(strategy: &str) -> bool {
    matches!(strategy.trim(), "weighted_v1" | "exact_cover_v1")
}

fn is_kubernetes_service_host(host: &str) -> bool {
    let labels = host.split('.').collect::<Vec<_>>();
    let is_short_service = labels.len() == 3 && labels[2] == "svc";
    let is_cluster_service =
        labels.len() == 5 && labels[2] == "svc" && labels[3] == "cluster" && labels[4] == "local";
    (is_short_service || is_cluster_service)
        && labels[0..2]
            .iter()
            .all(|label| !label.is_empty() && label.chars().all(is_dns_label_char))
}

fn is_dns_label_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '-'
}

async fn read_module_string(db: &crate::db::Database, field: &str, default: &str) -> String {
    let key = config_setting_key(RECOMMENDATION_MODULE, field);
    db.get_setting(&key)
        .await
        .ok()
        .flatten()
        .map(|raw| serde_json::from_str::<String>(&raw).unwrap_or(raw))
        .unwrap_or_else(|| default.to_string())
}

async fn read_module_bool(db: &crate::db::Database, field: &str, default: bool) -> bool {
    let key = config_setting_key(RECOMMENDATION_MODULE, field);
    db.get_setting(&key)
        .await
        .ok()
        .flatten()
        .and_then(|raw| {
            serde_json::from_str::<bool>(&raw)
                .ok()
                .or_else(|| raw.parse().ok())
        })
        .unwrap_or(default)
}

async fn read_module_f64(
    db: &crate::db::Database,
    field: &str,
    default: f64,
    min: f64,
    max: f64,
) -> f64 {
    let key = config_setting_key(RECOMMENDATION_MODULE, field);
    db.get_setting(&key)
        .await
        .ok()
        .flatten()
        .and_then(|raw| {
            serde_json::from_str::<f64>(&raw)
                .ok()
                .or_else(|| raw.parse().ok())
        })
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

async fn read_module_usize(
    db: &crate::db::Database,
    field: &str,
    default: usize,
    min: usize,
    max: usize,
) -> usize {
    let key = config_setting_key(RECOMMENDATION_MODULE, field);
    db.get_setting(&key)
        .await
        .ok()
        .flatten()
        .and_then(|raw| {
            serde_json::from_str::<usize>(&raw)
                .ok()
                .or_else(|| raw.parse().ok())
        })
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

async fn read_module_u64(
    db: &crate::db::Database,
    field: &str,
    default: u64,
    min: u64,
    max: u64,
) -> u64 {
    let key = config_setting_key(RECOMMENDATION_MODULE, field);
    db.get_setting(&key)
        .await
        .ok()
        .flatten()
        .and_then(|raw| {
            serde_json::from_str::<u64>(&raw)
                .ok()
                .or_else(|| raw.parse().ok())
        })
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RecommendationQuery {
    pub lot_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct SlotRecommendation {
    pub recommendation_id: Uuid,
    pub slot_id: Uuid,
    pub slot_number: i32,
    pub lot_id: Uuid,
    pub lot_name: String,
    pub floor_name: String,
    pub score: f64,
    pub reasons: Vec<String>,
    pub reason_badges: Vec<RecommendationBadge>,
}

/// Recommendation reason badge for the frontend
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationBadge {
    YourUsualSpot,
    BestPrice,
    ClosestEntrance,
    AvailableNow,
    PreferredLot,
    Accessible,
}

struct RecommendationScoreInput<'a> {
    slot_usage: i32,
    lot_usage: i32,
    lot_rate: Option<f64>,
    max_price: f64,
    slot_number: i32,
    is_accessible: bool,
    feature_names: &'a [String],
}

fn weighted_v1_candidate_score(
    weights: &RecommendationWeights,
    input: &RecommendationScoreInput<'_>,
) -> (f64, Vec<String>, Vec<RecommendationBadge>) {
    let mut score = 0.0;
    let mut reasons = Vec::new();
    let mut badges = Vec::new();

    if input.slot_usage > 0 {
        let freq_score = (f64::from(input.slot_usage).min(10.0) / 10.0) * weights.frequency;
        score += freq_score;
        reasons.push(format!("Used {} times before", input.slot_usage));
        badges.push(RecommendationBadge::YourUsualSpot);
    } else if input.lot_usage > 0 {
        let lot_score = (f64::from(input.lot_usage).min(10.0) / 10.0) * weights.preferred_lot;
        score += lot_score;
        reasons.push(format!(
            "In your preferred lot (used {} times)",
            input.lot_usage
        ));
        badges.push(RecommendationBadge::PreferredLot);
    }

    score += weights.availability;
    badges.push(RecommendationBadge::AvailableNow);
    if reasons.is_empty() {
        reasons.push("Available now".to_string());
    }

    if let Some(lot_rate) = input
        .lot_rate
        .filter(|rate| rate.is_finite() && *rate > 0.0)
    {
        let price_score = (1.0 - (lot_rate / input.max_price.max(1.0))).max(0.0) * weights.price;
        score += price_score;
        if price_score >= weights.price * 0.75 {
            badges.push(RecommendationBadge::BestPrice);
            reasons.push("Great price".to_string());
        }
    }

    let distance_score = weights.distance / f64::from(input.slot_number.max(1));
    score += distance_score;
    if distance_score >= weights.distance * 0.5 {
        badges.push(RecommendationBadge::ClosestEntrance);
        reasons.push("Near entrance".to_string());
    }

    if input.is_accessible {
        score += weights.accessibility_bonus;
        badges.push(RecommendationBadge::Accessible);
        reasons.push("Accessible".to_string());
    }

    if !input.feature_names.is_empty() {
        score += weights.feature_bonus;
        reasons.push(format!("Features: {}", input.feature_names.join(", ")));
    }

    (score, reasons, badges)
}

fn slot_feature_label(feature: &SlotFeature) -> &'static str {
    match feature {
        SlotFeature::NearExit => "Near exit",
        SlotFeature::NearElevator => "Near elevator",
        SlotFeature::NearStairs => "Near stairs",
        SlotFeature::Covered => "Covered",
        SlotFeature::SecurityCamera => "Security camera",
        SlotFeature::WellLit => "Well lit",
        SlotFeature::WideLane => "Wide lane",
        SlotFeature::ChargingStation => "Charging station",
    }
}

#[derive(Debug, Serialize)]
struct FopPipelineRecommendationRequest<'a> {
    schema_version: &'static str,
    batch_id: Uuid,
    algorithm: &'static str,
    fallback_algorithm: &'static str,
    weights: RecommendationWeights,
    max_results: usize,
    explain: bool,
    profile_safe_mode: bool,
    candidates: &'a [SlotRecommendation],
}

#[derive(Debug, Deserialize)]
struct FopPipelineRunResponse {
    ok: bool,
    data: Option<FopPipelineRecommendationData>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FopPipelineRecommendationData {
    ranked: Vec<FopPipelineRankedRecommendation>,
}

#[derive(Debug, Deserialize)]
struct FopPipelineRankedRecommendation {
    slot_id: Option<Uuid>,
    id: Option<String>,
    score: Option<f64>,
    reasons: Option<Vec<String>>,
    reason_badges: Option<Vec<RecommendationBadge>>,
}

fn pipeline_run_url(endpoint: &str, pipeline_name: &str) -> String {
    format!(
        "{}/pipeline/{}/run",
        endpoint.trim_end_matches('/'),
        pipeline_name.trim_matches('/')
    )
}

fn fop_pipeline_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

fn adapter_status_for_weighted_v1(
    engine: &RecommendationEngineConfig,
) -> RecommendationAdapterStatus {
    RecommendationAdapterStatus {
        requested_algorithm: engine.algorithm.clone(),
        effective_algorithm: "weighted_v1".to_string(),
        attempted: false,
        status: "weighted_v1".to_string(),
        pipeline_name: None,
        endpoint_configured: engine.pipeline.endpoint.is_some(),
        fallback_enabled: engine.pipeline.fallback_enabled,
        error: None,
    }
}

fn adapter_status_for_fallback(
    engine: &RecommendationEngineConfig,
    attempted: bool,
    status: &str,
    error: Option<String>,
) -> RecommendationAdapterStatus {
    RecommendationAdapterStatus {
        requested_algorithm: engine.algorithm.clone(),
        effective_algorithm: "weighted_v1".to_string(),
        attempted,
        status: status.to_string(),
        pipeline_name: Some(engine.pipeline.pipeline_name.clone()),
        endpoint_configured: engine.pipeline.endpoint.is_some(),
        fallback_enabled: engine.pipeline.fallback_enabled,
        error,
    }
}

async fn try_fop_pipeline_recommendations(
    engine: &RecommendationEngineConfig,
    batch_id: Uuid,
    candidates: &[SlotRecommendation],
) -> Result<Vec<SlotRecommendation>, String> {
    let endpoint = engine
        .pipeline
        .endpoint
        .as_deref()
        .ok_or_else(|| "fop_pipeline_v1 endpoint is not configured".to_string())?;
    let request = FopPipelineRecommendationRequest {
        schema_version: "parkhub.recommendation.pipeline.v1",
        batch_id,
        algorithm: "fop_pipeline_v1",
        fallback_algorithm: "weighted_v1",
        weights: engine.weights,
        max_results: engine.max_results,
        explain: engine.explain,
        profile_safe_mode: engine.profile_safe_mode,
        candidates,
    };
    let response = fop_pipeline_client()
        .post(pipeline_run_url(endpoint, &engine.pipeline.pipeline_name))
        .timeout(Duration::from_millis(engine.pipeline.timeout_ms))
        .json(&request)
        .send()
        .await
        .map_err(|err| format!("fop-pipeline request failed: {err}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("fop-pipeline returned HTTP {status}"));
    }
    let body = response
        .json::<FopPipelineRunResponse>()
        .await
        .map_err(|err| format!("fop-pipeline response was not valid JSON: {err}"))?;
    if !body.ok {
        return Err(body
            .error
            .unwrap_or_else(|| "fop-pipeline returned ok=false".to_string()));
    }
    apply_fop_pipeline_response(candidates, body.data, engine.max_results)
}

fn apply_fop_pipeline_response(
    candidates: &[SlotRecommendation],
    data: Option<FopPipelineRecommendationData>,
    max_results: usize,
) -> Result<Vec<SlotRecommendation>, String> {
    let data = data.ok_or_else(|| "fop-pipeline response did not include data".to_string())?;
    let mut by_id: HashMap<Uuid, SlotRecommendation> = candidates
        .iter()
        .cloned()
        .map(|candidate| (candidate.slot_id, candidate))
        .collect();
    let mut ranked = Vec::new();
    for item in data.ranked.into_iter().take(max_results) {
        let slot_id = item
            .slot_id
            .or_else(|| item.id.as_deref().and_then(|id| Uuid::parse_str(id).ok()));
        let Some(slot_id) = slot_id else {
            continue;
        };
        let Some(mut candidate) = by_id.remove(&slot_id) else {
            continue;
        };
        if let Some(score) = item.score {
            candidate.score = score;
        }
        if let Some(reasons) = item.reasons {
            candidate.reasons = reasons;
        }
        if let Some(badges) = item.reason_badges {
            candidate.reason_badges = badges;
        }
        ranked.push(candidate);
    }
    if ranked.is_empty() {
        Err("fop-pipeline response did not rank any known slots".to_string())
    } else {
        Ok(ranked)
    }
}

/// Admin stats derived from RecommendationServed audit events.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RecommendationStats {
    pub total_recommendations: i32,
    pub total_recommendations_served: i32,
    pub accepted_recommendations: Option<i32>,
    pub acceptance_rate: Option<f64>,
    pub acceptance_metric_source: String,
    pub unique_users: i32,
    pub avg_score: Option<f64>,
    pub metrics_source: String,
    pub algorithm: String,
    pub algorithm_weights: RecommendationWeights,
    pub allocation: RecommendationAllocationConfig,
    pub algorithm_adapter: RecommendationAdapterStatus,
    pub legal_boundary: RecommendationLegalBoundary,
    pub top_recommended_lots: Vec<LotRecommendationCount>,
}

#[derive(Default)]
struct RecommendationAuditStats {
    total_batches: i32,
    total_candidates_served: i32,
    unique_users: i32,
    avg_score: Option<f64>,
    lot_counts: HashMap<Uuid, i32>,
}

fn recommendation_audit_stats(entries: &[crate::db::AuditLogEntry]) -> RecommendationAuditStats {
    let mut stats = RecommendationAuditStats::default();
    let mut unique_users = HashSet::new();
    let mut score_total = 0.0;
    let mut score_count = 0_i32;

    for entry in entries
        .iter()
        .filter(|entry| entry.event_type == "RecommendationServed")
    {
        stats.total_batches += 1;
        if let Some(user_id) = entry.user_id {
            unique_users.insert(user_id);
        }
        let Some(details) = entry.details.as_deref() else {
            continue;
        };
        let Ok(details) = serde_json::from_str::<serde_json::Value>(details) else {
            continue;
        };
        let Some(candidates) = details.get("candidates").and_then(|value| value.as_array()) else {
            continue;
        };
        stats.total_candidates_served += candidates.len() as i32;
        for candidate in candidates {
            if let Some(score) = candidate.get("score").and_then(serde_json::Value::as_f64) {
                score_total += score;
                score_count += 1;
            }
            if let Some(lot_id) = candidate
                .get("lot_id")
                .and_then(|value| value.as_str())
                .and_then(|raw| Uuid::parse_str(raw).ok())
            {
                *stats.lot_counts.entry(lot_id).or_default() += 1;
            }
        }
    }

    stats.unique_users = unique_users.len() as i32;
    stats.avg_score =
        (score_count > 0).then(|| (score_total / f64::from(score_count) * 10.0).round() / 10.0);
    stats
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RecommendationLegalBoundary {
    pub legal_review_required: bool,
    pub attorney_review_status: String,
    pub execution_allowed: bool,
    pub disclaimer: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct LotRecommendationCount {
    pub lot_name: String,
    pub count: i32,
}

/// `GET /api/v1/bookings/recommendations` — suggest optimal parking slots
#[utoipa::path(
    get,
    path = "/api/v1/bookings/recommendations",
    tag = "Bookings",
    summary = "Get smart parking recommendations",
    description = "Returns top slot recommendations based on user history, favorites, and availability.",
    params(("lot_id" = Option<String>, Query, description = "Filter by lot")),
    responses(
        (status = 200, description = "Slot recommendations"),
    )
)]
pub async fn get_recommendations(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<RecommendationQuery>,
) -> Json<ApiResponse<Vec<SlotRecommendation>>> {
    let state = state.read().await;

    // 1. Get user's booking history
    let bookings = match state
        .db
        .list_bookings_by_user(&auth_user.user_id.to_string())
        .await
    {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to load bookings for recommendations: {}", e);
            return Json(ApiResponse::success(vec![]));
        }
    };

    // 2. Count slot usage frequency from intent + fulfilled lifecycle states.
    let mut slot_frequency: HashMap<Uuid, i32> = HashMap::new();
    let mut lot_frequency: HashMap<Uuid, i32> = HashMap::new();
    for b in &bookings {
        if booking_status_counts_for_recommendation_history(&b.status) {
            *slot_frequency.entry(b.slot_id).or_default() += 1;
            *lot_frequency.entry(b.lot_id).or_default() += 1;
        }
    }

    // 3. Get all lots and available slots
    let Ok(lots) = state.db.list_parking_lots().await else {
        return Json(ApiResponse::success(vec![]));
    };

    let engine = RecommendationEngineConfig::load(&state.db).await;
    let weights = engine.weights;
    let batch_id = Uuid::new_v4();
    let max_price = lots
        .iter()
        .filter(|lot| {
            query
                .lot_id
                .as_ref()
                .is_none_or(|filter_lot| lot.id.to_string() == *filter_lot)
        })
        .filter_map(|lot| lot.pricing.rates.first().map(|rate| rate.price))
        .filter(|price| price.is_finite() && *price > 0.0)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let mut candidates: Vec<SlotRecommendation> = Vec::new();

    for lot in &lots {
        // Filter by lot_id if specified
        if let Some(ref filter_lot) = query.lot_id
            && lot.id.to_string() != *filter_lot
        {
            continue;
        }

        let Ok(slots) = state.db.list_slots_by_lot(&lot.id.to_string()).await else {
            continue;
        };

        for slot in &slots {
            // Only recommend available slots
            if slot.status != SlotStatus::Available {
                continue;
            }
            if slot.current_booking.is_some() {
                continue;
            }

            let freq = slot_frequency.get(&slot.id).copied().unwrap_or(0);
            let lot_freq = lot_frequency.get(&lot.id).copied().unwrap_or(0);
            let base_rate = lot
                .pricing
                .rates
                .first()
                .map(|r| r.price)
                .filter(|price| price.is_finite() && *price > 0.0);
            let feature_names = slot
                .features
                .iter()
                .map(|feature| slot_feature_label(feature).to_string())
                .collect::<Vec<_>>();
            let (score, reasons, badges) = weighted_v1_candidate_score(
                &weights,
                &RecommendationScoreInput {
                    slot_usage: freq,
                    lot_usage: lot_freq,
                    lot_rate: base_rate,
                    max_price,
                    slot_number: slot.slot_number,
                    is_accessible: slot.is_accessible,
                    feature_names: &feature_names,
                },
            );

            let floor_name = lot
                .floors
                .first()
                .map_or_else(|| "Ground".to_string(), |f| f.name.clone());

            candidates.push(SlotRecommendation {
                recommendation_id: Uuid::new_v4(),
                slot_id: slot.id,
                slot_number: slot.slot_number,
                lot_id: lot.id,
                lot_name: lot.name.clone(),
                floor_name,
                score,
                reasons,
                reason_badges: badges,
            });
        }
    }

    // Sort fallback candidates by score; max_results is applied after the
    // optional fop_pipeline_v1 ranking so the pipeline sees the full set.
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let adapter_status = if engine.algorithm == "fop_pipeline_v1" {
        if engine.pipeline.endpoint.is_none() {
            adapter_status_for_fallback(
                &engine,
                false,
                "fallback_not_configured",
                Some("fop_pipeline_v1 endpoint is not configured".to_string()),
            )
        } else {
            match try_fop_pipeline_recommendations(&engine, batch_id, &candidates).await {
                Ok(ranked) => {
                    candidates = ranked;
                    RecommendationAdapterStatus {
                        requested_algorithm: engine.algorithm.clone(),
                        effective_algorithm: "fop_pipeline_v1".to_string(),
                        attempted: true,
                        status: "succeeded".to_string(),
                        pipeline_name: Some(engine.pipeline.pipeline_name.clone()),
                        endpoint_configured: true,
                        fallback_enabled: engine.pipeline.fallback_enabled,
                        error: None,
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        %batch_id,
                        error = %err,
                        "fop_pipeline_v1 recommendation attempt failed; falling back to weighted_v1"
                    );
                    adapter_status_for_fallback(&engine, true, "fallback_error", Some(err))
                }
            }
        }
    } else {
        adapter_status_for_weighted_v1(&engine)
    };
    candidates.truncate(engine.max_results);
    persist_recommendation_served_audit(
        &state.db,
        &auth_user,
        batch_id,
        &engine,
        &adapter_status,
        &candidates,
    )
    .await;
    drop(state);

    Json(ApiResponse::success(candidates))
}

fn booking_status_counts_for_recommendation_history(status: &BookingStatus) -> bool {
    matches!(
        status,
        BookingStatus::Pending
            | BookingStatus::Confirmed
            | BookingStatus::Active
            | BookingStatus::Completed
    )
}

async fn persist_recommendation_served_audit(
    db: &crate::db::Database,
    auth_user: &AuthUser,
    batch_id: Uuid,
    engine: &RecommendationEngineConfig,
    adapter_status: &RecommendationAdapterStatus,
    recommendations: &[SlotRecommendation],
) {
    let config_hash = recommendation_config_hash(engine);
    let weights_hash = recommendation_weights_hash(&engine.weights);
    let candidates: Vec<_> = recommendations
        .iter()
        .map(|rec| {
            serde_json::json!({
                "recommendation_id": rec.recommendation_id,
                "slot_id": rec.slot_id,
                "lot_id": rec.lot_id,
                "score": rec.score,
                "reason_badges": &rec.reason_badges,
                "reasons": &rec.reasons,
            })
        })
        .collect();

    let details = serde_json::json!({
        "batch_id": batch_id,
        "algorithm": &engine.algorithm,
        "config_hash": config_hash,
        "weights_hash": weights_hash,
        "adapter": adapter_status,
        "profile_safe_mode": engine.profile_safe_mode,
        "explain": engine.explain,
        "recommendation_ids": recommendations.iter().map(|rec| rec.recommendation_id).collect::<Vec<_>>(),
        "candidate_ids": recommendations.iter().map(|rec| rec.slot_id).collect::<Vec<_>>(),
        "candidates": candidates,
        "legal_boundary": {
            "legal_review_required": true,
            "attorney_review_status": "required_before_customer_wording",
            "execution_allowed": false
        }
    });

    let entry = crate::db::AuditLogEntry {
        id: batch_id,
        timestamp: Utc::now(),
        event_type: "RecommendationServed".to_string(),
        user_id: Some(auth_user.user_id),
        username: None,
        details: Some(details.to_string()),
        target_type: Some("recommendation".to_string()),
        target_id: Some(batch_id.to_string()),
        ip_address: None,
    };

    if let Err(err) = db.save_audit_log(&entry).await {
        tracing::warn!(%batch_id, error = ?err, "failed to persist recommendation audit event");
    }
}

fn recommendation_config_hash(engine: &RecommendationEngineConfig) -> String {
    let payload = serde_json::json!({
        "algorithm": &engine.algorithm,
        "weights": engine.weights,
        "max_results": engine.max_results,
        "explain": engine.explain,
        "profile_safe_mode": engine.profile_safe_mode,
        "pipeline": &engine.pipeline,
        "allocation": &engine.allocation,
    });
    sha256_hex(
        serde_json::to_string(&payload)
            .unwrap_or_default()
            .as_bytes(),
    )
}

fn recommendation_weights_hash(weights: &RecommendationWeights) -> String {
    sha256_hex(
        serde_json::to_string(weights)
            .unwrap_or_default()
            .as_bytes(),
    )
}

fn sha256_hex(input: &[u8]) -> String {
    let digest = Sha256::digest(input);
    digest.iter().fold(String::new(), |mut output, byte| {
        let _ = write!(&mut output, "{byte:02x}");
        output
    })
}

/// `GET /api/v1/recommendations/stats` — admin: recommendation statistics
#[utoipa::path(
    get,
    path = "/api/v1/recommendations/stats",
    tag = "Admin",
    summary = "Recommendation acceptance stats",
    description = "Admin-only: view recommendation service statistics.",
    security(("bearer_auth" = []))
)]
pub async fn get_recommendation_stats(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<RecommendationStats>>) {
    let state_guard = state.read().await;
    if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let lots = state_guard.db.list_parking_lots().await.unwrap_or_default();
    let audit_entries = state_guard
        .db
        .list_all_audit_log()
        .await
        .unwrap_or_default();
    let audit_stats = recommendation_audit_stats(&audit_entries);

    let top_lots: Vec<LotRecommendationCount> = {
        let mut entries: Vec<_> = audit_stats.lot_counts.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));
        entries
            .into_iter()
            .take(5)
            .map(|(lot_id, count)| {
                let name = lots
                    .iter()
                    .find(|l| l.id == *lot_id)
                    .map(|l| l.name.clone())
                    .unwrap_or_else(|| lot_id.to_string());
                LotRecommendationCount {
                    lot_name: name,
                    count: *count,
                }
            })
            .collect()
    };

    let engine = RecommendationEngineConfig::load(&state_guard.db).await;
    let algorithm_adapter = adapter_status_for_weighted_v1(&engine);

    let stats = RecommendationStats {
        total_recommendations: audit_stats.total_batches,
        total_recommendations_served: audit_stats.total_candidates_served,
        accepted_recommendations: None,
        acceptance_rate: None,
        acceptance_metric_source: "not_tracked".to_string(),
        unique_users: audit_stats.unique_users,
        avg_score: audit_stats.avg_score,
        metrics_source: "audit_log.RecommendationServed".to_string(),
        algorithm: engine.algorithm.clone(),
        algorithm_weights: engine.weights,
        allocation: engine.allocation,
        algorithm_adapter,
        legal_boundary: RecommendationLegalBoundary {
            legal_review_required: true,
            attorney_review_status: "required_before_customer_wording".to_string(),
            execution_allowed: false,
            disclaimer: "fop legal output is reference-only drafting support; attorney review, citation verification, client authorization, and final legal judgment remain required before customer-facing profiling or legal wording ships.".to_string(),
        },
        top_recommended_lots: top_lots,
    };

    (StatusCode::OK, Json(ApiResponse::success(stats)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, serde::Deserialize)]
    struct WeightedV1Fixture {
        algorithm: String,
        weights: RecommendationWeights,
        max_results: usize,
        price_normalization: FixturePriceNormalization,
        history: FixtureHistory,
        candidate_lots: Vec<FixtureLot>,
        expected_ranked_slots: Vec<ExpectedFixtureSlot>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct FixturePriceNormalization {
        max_candidate_hourly_rate: f64,
    }

    #[derive(Debug, serde::Deserialize)]
    struct FixtureHistory {
        slot_usage: std::collections::HashMap<String, i32>,
        lot_usage: std::collections::HashMap<String, i32>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct FixtureLot {
        id: String,
        hourly_rate: f64,
        slots: Vec<FixtureSlot>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct FixtureSlot {
        id: String,
        slot_number: i32,
        status: String,
        is_accessible: bool,
        features: Vec<String>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct ExpectedFixtureSlot {
        slot_id: String,
        score: f64,
        badges: Vec<String>,
        reasons: Vec<String>,
    }

    #[test]
    fn test_recommendation_query_default() {
        let q: RecommendationQuery = serde_json::from_str("{}").unwrap();
        assert!(q.lot_id.is_none());
    }

    #[test]
    fn test_recommendation_query_with_lot() {
        let q: RecommendationQuery = serde_json::from_str(r#"{"lot_id":"abc-123"}"#).unwrap();
        assert_eq!(q.lot_id.as_deref(), Some("abc-123"));
    }

    #[test]
    fn test_recommendation_badge_serialization() {
        assert_eq!(
            serde_json::to_string(&RecommendationBadge::YourUsualSpot).unwrap(),
            "\"your_usual_spot\""
        );
        assert_eq!(
            serde_json::to_string(&RecommendationBadge::BestPrice).unwrap(),
            "\"best_price\""
        );
        assert_eq!(
            serde_json::to_string(&RecommendationBadge::ClosestEntrance).unwrap(),
            "\"closest_entrance\""
        );
        assert_eq!(
            serde_json::to_string(&RecommendationBadge::AvailableNow).unwrap(),
            "\"available_now\""
        );
        assert_eq!(
            serde_json::to_string(&RecommendationBadge::Accessible).unwrap(),
            "\"accessible\""
        );
    }

    #[test]
    fn test_slot_recommendation_serialize() {
        let recommendation_id = Uuid::new_v4();
        let rec = SlotRecommendation {
            recommendation_id,
            slot_id: Uuid::new_v4(),
            slot_number: 42,
            lot_id: Uuid::new_v4(),
            lot_name: "Main Lot".to_string(),
            floor_name: "Level 1".to_string(),
            score: 85.5,
            reasons: vec!["Available now".to_string(), "Near entrance".to_string()],
            reason_badges: vec![
                RecommendationBadge::AvailableNow,
                RecommendationBadge::ClosestEntrance,
            ],
        };
        let json = serde_json::to_string(&rec).unwrap();
        assert!(json.contains("\"slot_number\":42"));
        assert!(json.contains("\"score\":85.5"));
        assert!(json.contains(&recommendation_id.to_string()));
        assert!(json.contains("available_now"));
        assert!(json.contains("closest_entrance"));
    }

    #[test]
    fn test_recommendation_stats_serialize() {
        let stats = RecommendationStats {
            total_recommendations: 100,
            total_recommendations_served: 300,
            accepted_recommendations: None,
            acceptance_rate: None,
            acceptance_metric_source: "not_tracked".to_string(),
            unique_users: 50,
            avg_score: None,
            metrics_source: "audit_log.RecommendationServed".to_string(),
            algorithm: "weighted_v1".to_string(),
            algorithm_weights: RecommendationWeights::default(),
            allocation: RecommendationAllocationConfig::default(),
            algorithm_adapter: adapter_status_for_weighted_v1(
                &RecommendationEngineConfig::default(),
            ),
            legal_boundary: RecommendationLegalBoundary {
                legal_review_required: true,
                attorney_review_status: "required_before_customer_wording".to_string(),
                execution_allowed: false,
                disclaimer: "fop legal output is reference-only drafting support.".to_string(),
            },
            top_recommended_lots: vec![LotRecommendationCount {
                lot_name: "Main Lot".to_string(),
                count: 120,
            }],
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total_recommendations_served\":300"));
        assert!(json.contains("\"accepted_recommendations\":null"));
        assert!(json.contains("\"acceptance_metric_source\":\"not_tracked\""));
        assert!(json.contains("\"unique_users\":50"));
        assert!(json.contains("\"legal_review_required\":true"));
    }

    #[test]
    fn test_recommendation_hashes_are_sha256_hex() {
        let cfg = RecommendationEngineConfig::default();
        let config_hash = recommendation_config_hash(&cfg);
        let weights_hash = recommendation_weights_hash(&cfg.weights);

        assert_eq!(config_hash.len(), 64);
        assert_eq!(weights_hash.len(), 64);
        assert!(config_hash.chars().all(|ch| ch.is_ascii_hexdigit()));
        assert!(weights_hash.chars().all(|ch| ch.is_ascii_hexdigit()));
    }

    #[test]
    fn test_scoring_algorithm_weights() {
        // frequency: 40%, availability: 30%, price: 20%, distance: 10%
        // Max possible: 40 + 30 + 20 + 10 = 100
        // An available slot with no history should get ~30 (availability) + some price + some distance
        let weights = RecommendationWeights::default();
        let availability_score = weights.availability;
        let max_price_score = weights.price;
        let max_distance_score = weights.distance;
        let max_frequency_score = weights.frequency;
        let total_max: f64 =
            availability_score + max_price_score + max_distance_score + max_frequency_score;
        assert!((total_max - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_missing_or_zero_price_gets_no_price_bonus() {
        let weights = RecommendationWeights::default();
        let base = RecommendationScoreInput {
            slot_usage: 0,
            lot_usage: 0,
            lot_rate: Some(8.0),
            max_price: 8.0,
            slot_number: 2,
            is_accessible: false,
            feature_names: &[],
        };
        let (priced_score, priced_reasons, priced_badges) =
            weighted_v1_candidate_score(&weights, &base);
        assert!(!priced_badges.contains(&RecommendationBadge::BestPrice));
        assert!(!priced_reasons.contains(&"Great price".to_string()));

        let missing_price = RecommendationScoreInput {
            lot_rate: None,
            ..base
        };
        let (missing_score, missing_reasons, missing_badges) =
            weighted_v1_candidate_score(&weights, &missing_price);
        assert!((missing_score - priced_score).abs() < f64::EPSILON);
        assert!(!missing_badges.contains(&RecommendationBadge::BestPrice));
        assert!(!missing_reasons.contains(&"Great price".to_string()));

        let zero_price = RecommendationScoreInput {
            lot_rate: Some(0.0),
            ..base
        };
        let (zero_score, zero_reasons, zero_badges) =
            weighted_v1_candidate_score(&weights, &zero_price);
        assert!((zero_score - priced_score).abs() < f64::EPSILON);
        assert!(!zero_badges.contains(&RecommendationBadge::BestPrice));
        assert!(!zero_reasons.contains(&"Great price".to_string()));
    }

    #[test]
    fn test_booking_history_statuses_include_pending_and_confirmed() {
        assert!(booking_status_counts_for_recommendation_history(
            &BookingStatus::Pending
        ));
        assert!(booking_status_counts_for_recommendation_history(
            &BookingStatus::Confirmed
        ));
        assert!(booking_status_counts_for_recommendation_history(
            &BookingStatus::Active
        ));
        assert!(booking_status_counts_for_recommendation_history(
            &BookingStatus::Completed
        ));
        assert!(!booking_status_counts_for_recommendation_history(
            &BookingStatus::Cancelled
        ));
        assert!(!booking_status_counts_for_recommendation_history(
            &BookingStatus::Expired
        ));
        assert!(!booking_status_counts_for_recommendation_history(
            &BookingStatus::NoShow
        ));
    }

    #[test]
    fn test_slot_feature_labels_are_user_visible() {
        assert_eq!(slot_feature_label(&SlotFeature::NearExit), "Near exit");
        assert_eq!(
            slot_feature_label(&SlotFeature::NearElevator),
            "Near elevator"
        );
        assert_eq!(slot_feature_label(&SlotFeature::NearStairs), "Near stairs");
        assert_eq!(slot_feature_label(&SlotFeature::Covered), "Covered");
        assert_eq!(
            slot_feature_label(&SlotFeature::SecurityCamera),
            "Security camera"
        );
        assert_eq!(slot_feature_label(&SlotFeature::WellLit), "Well lit");
        assert_eq!(slot_feature_label(&SlotFeature::WideLane), "Wide lane");
        assert_eq!(
            slot_feature_label(&SlotFeature::ChargingStation),
            "Charging station"
        );
    }

    #[test]
    fn test_recommendation_engine_config_defaults_are_legacy_safe() {
        let cfg = RecommendationEngineConfig::default();
        assert_eq!(cfg.algorithm, "weighted_v1");
        assert_eq!(cfg.max_results, 5);
        assert!(cfg.explain);
        assert!(cfg.profile_safe_mode);
        assert_eq!(cfg.pipeline.pipeline_name, "parkhub-recommendations");
        assert_eq!(cfg.pipeline.timeout_ms, 750);
        assert!(cfg.pipeline.fallback_enabled);
        assert!(cfg.pipeline.endpoint.is_none());
        assert!((cfg.weights.preferred_lot - 20.0).abs() < f64::EPSILON);
        assert!((cfg.weights.feature_bonus - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_pipeline_endpoint_allowlist() {
        assert_eq!(
            validate_pipeline_endpoint(Some("http://fop-pipeline.fop-agents.svc:9310".to_string())),
            Some("http://fop-pipeline.fop-agents.svc:9310".to_string())
        );
        assert_eq!(
            validate_pipeline_endpoint(Some(
                "http://fop-pipeline.fop-agents.svc.cluster.local:9310".to_string()
            )),
            Some("http://fop-pipeline.fop-agents.svc.cluster.local:9310".to_string())
        );
        assert_eq!(
            validate_pipeline_endpoint(Some("http://localhost:9310".to_string())),
            Some("http://localhost:9310".to_string())
        );
        assert_eq!(
            validate_pipeline_endpoint(Some("http://fop-pipeline.test:9310".to_string())),
            Some("http://fop-pipeline.test:9310".to_string())
        );
        assert!(validate_pipeline_endpoint(Some("http://fop-pipeline".to_string())).is_none());
        assert!(
            validate_pipeline_endpoint(Some("http://fop-pipeline.svc:9310".to_string())).is_none()
        );
        assert!(
            validate_pipeline_endpoint(Some("http://svc.cluster.local:9310".to_string())).is_none()
        );
        assert!(validate_pipeline_endpoint(Some("https://example.com".to_string())).is_none());
        assert!(validate_pipeline_endpoint(Some("file:///tmp/pipeline".to_string())).is_none());
    }

    #[test]
    fn test_allocation_strategy_falls_back_to_weighted_v1() {
        assert_eq!(
            normalize_allocation_strategy("exact_cover_v1"),
            "exact_cover_v1"
        );
        assert_eq!(
            normalize_allocation_strategy(" weighted_v1 "),
            "weighted_v1"
        );
        assert_eq!(
            normalize_allocation_strategy("unknown_strategy"),
            "weighted_v1"
        );
    }

    #[test]
    fn test_pipeline_run_url_trims_edges() {
        assert_eq!(
            pipeline_run_url(
                "http://fop-pipeline.test:9310/",
                "/parkhub-recommendations/"
            ),
            "http://fop-pipeline.test:9310/pipeline/parkhub-recommendations/run"
        );
    }

    #[test]
    fn test_apply_fop_pipeline_response_maps_known_slots_only() {
        let recommendation_id = Uuid::new_v4();
        let slot_a = Uuid::new_v4();
        let slot_b = Uuid::new_v4();
        let lot_id = Uuid::new_v4();
        let candidates = vec![
            SlotRecommendation {
                recommendation_id,
                slot_id: slot_a,
                slot_number: 1,
                lot_id,
                lot_name: "Lot".to_string(),
                floor_name: "Ground".to_string(),
                score: 10.0,
                reasons: vec!["Available now".to_string()],
                reason_badges: vec![RecommendationBadge::AvailableNow],
            },
            SlotRecommendation {
                recommendation_id,
                slot_id: slot_b,
                slot_number: 2,
                lot_id,
                lot_name: "Lot".to_string(),
                floor_name: "Ground".to_string(),
                score: 20.0,
                reasons: vec!["Available now".to_string()],
                reason_badges: vec![RecommendationBadge::AvailableNow],
            },
        ];
        let ranked = apply_fop_pipeline_response(
            &candidates,
            Some(FopPipelineRecommendationData {
                ranked: vec![FopPipelineRankedRecommendation {
                    slot_id: Some(slot_b),
                    id: None,
                    score: Some(99.0),
                    reasons: Some(vec!["Pipeline selected".to_string()]),
                    reason_badges: Some(vec![RecommendationBadge::BestPrice]),
                }],
            }),
            5,
        )
        .unwrap();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].slot_id, slot_b);
        assert!((ranked[0].score - 99.0).abs() < f64::EPSILON);
        assert_eq!(ranked[0].reasons, vec!["Pipeline selected"]);
        assert_eq!(
            ranked[0].reason_badges,
            vec![RecommendationBadge::BestPrice]
        );
    }

    #[test]
    fn test_apply_fop_pipeline_response_applies_max_results_after_ranking() {
        let recommendation_id = Uuid::new_v4();
        let slot_a = Uuid::new_v4();
        let slot_b = Uuid::new_v4();
        let lot_id = Uuid::new_v4();
        let candidates = [slot_a, slot_b]
            .into_iter()
            .enumerate()
            .map(|(idx, slot_id)| SlotRecommendation {
                recommendation_id,
                slot_id,
                slot_number: idx as i32 + 1,
                lot_id,
                lot_name: "Lot".to_string(),
                floor_name: "Ground".to_string(),
                score: idx as f64,
                reasons: vec!["Available now".to_string()],
                reason_badges: vec![RecommendationBadge::AvailableNow],
            })
            .collect::<Vec<_>>();
        let ranked = apply_fop_pipeline_response(
            &candidates,
            Some(FopPipelineRecommendationData {
                ranked: vec![
                    FopPipelineRankedRecommendation {
                        slot_id: Some(slot_b),
                        id: None,
                        score: Some(50.0),
                        reasons: None,
                        reason_badges: None,
                    },
                    FopPipelineRankedRecommendation {
                        slot_id: Some(slot_a),
                        id: None,
                        score: Some(49.0),
                        reasons: None,
                        reason_badges: None,
                    },
                ],
            }),
            1,
        )
        .unwrap();

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].slot_id, slot_b);
    }

    #[test]
    fn test_recommendation_audit_stats_are_derived_from_served_events() {
        let user_id = Uuid::new_v4();
        let lot_id = Uuid::new_v4();
        let details = serde_json::json!({
            "batch_id": Uuid::new_v4(),
            "candidates": [
                {
                    "recommendation_id": Uuid::new_v4(),
                    "slot_id": Uuid::new_v4(),
                    "lot_id": lot_id,
                    "score": 40.0,
                    "reason_badges": ["available_now"],
                    "reasons": ["Available now"]
                },
                {
                    "recommendation_id": Uuid::new_v4(),
                    "slot_id": Uuid::new_v4(),
                    "lot_id": lot_id,
                    "score": 60.0,
                    "reason_badges": ["best_price"],
                    "reasons": ["Great price"]
                }
            ]
        });
        let entries = vec![
            crate::db::AuditLogEntry {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                event_type: "RecommendationServed".to_string(),
                user_id: Some(user_id),
                username: None,
                details: Some(details.to_string()),
                target_type: Some("recommendation".to_string()),
                target_id: None,
                ip_address: None,
            },
            crate::db::AuditLogEntry {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                event_type: "booking.created".to_string(),
                user_id: Some(Uuid::new_v4()),
                username: None,
                details: None,
                target_type: Some("booking".to_string()),
                target_id: None,
                ip_address: None,
            },
        ];

        let stats = recommendation_audit_stats(&entries);
        assert_eq!(stats.total_batches, 1);
        assert_eq!(stats.total_candidates_served, 2);
        assert_eq!(stats.unique_users, 1);
        assert_eq!(stats.avg_score, Some(50.0));
        assert_eq!(stats.lot_counts.get(&lot_id), Some(&2));
    }

    #[test]
    fn test_weighted_v1_fixture_matches_contract() {
        let fixture: WeightedV1Fixture = serde_json::from_str(include_str!(
            "../../../docs/recommendation-engine-fixtures/weighted_v1.basic.json"
        ))
        .unwrap();
        assert_eq!(fixture.algorithm, "weighted_v1");

        let max_price = fixture
            .candidate_lots
            .iter()
            .map(|lot| lot.hourly_rate)
            .filter(|price| price.is_finite() && *price > 0.0)
            .fold(0.0_f64, f64::max)
            .max(1.0);
        assert!((max_price - fixture.price_normalization.max_candidate_hourly_rate).abs() < 0.01);

        let mut actual = Vec::new();
        for lot in &fixture.candidate_lots {
            for slot in &lot.slots {
                if slot.status != "available" {
                    continue;
                }
                let features = slot
                    .features
                    .iter()
                    .map(|feature| match feature.as_str() {
                        "covered" => "Covered".to_string(),
                        other => other.to_string(),
                    })
                    .collect::<Vec<_>>();
                let (score, reasons, badges) = weighted_v1_candidate_score(
                    &fixture.weights,
                    &RecommendationScoreInput {
                        slot_usage: fixture
                            .history
                            .slot_usage
                            .get(&slot.id)
                            .copied()
                            .unwrap_or(0),
                        lot_usage: fixture.history.lot_usage.get(&lot.id).copied().unwrap_or(0),
                        lot_rate: Some(lot.hourly_rate)
                            .filter(|price| price.is_finite() && *price > 0.0),
                        max_price,
                        slot_number: slot.slot_number,
                        is_accessible: slot.is_accessible,
                        feature_names: &features,
                    },
                );
                let badges = badges
                    .into_iter()
                    .map(|badge| {
                        serde_json::to_value(badge)
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .to_string()
                    })
                    .collect::<Vec<_>>();
                actual.push(ExpectedFixtureSlot {
                    slot_id: slot.id.clone(),
                    score: (score * 100.0).round() / 100.0,
                    badges,
                    reasons,
                });
            }
        }
        actual.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        actual.truncate(fixture.max_results);

        assert_eq!(actual.len(), fixture.expected_ranked_slots.len());
        for (actual, expected) in actual.iter().zip(&fixture.expected_ranked_slots) {
            assert_eq!(actual.slot_id, expected.slot_id);
            assert!((actual.score - expected.score).abs() < 0.01);
            assert_eq!(actual.badges, expected.badges);
            assert_eq!(actual.reasons, expected.reasons);
        }
    }

    #[test]
    fn test_lot_recommendation_count_serialize() {
        let c = LotRecommendationCount {
            lot_name: "Test Lot".to_string(),
            count: 42,
        };
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("\"lot_name\":\"Test Lot\""));
        assert!(json.contains("\"count\":42"));
    }

    #[test]
    fn test_recommendation_badge_deserialization() {
        assert_eq!(
            serde_json::from_str::<RecommendationBadge>("\"your_usual_spot\"").unwrap(),
            RecommendationBadge::YourUsualSpot
        );
        assert_eq!(
            serde_json::from_str::<RecommendationBadge>("\"best_price\"").unwrap(),
            RecommendationBadge::BestPrice
        );
    }
}
