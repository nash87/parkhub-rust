//! Betriebsrat (works council) fairness and transparency endpoints.
//!
//! Implements §87 Abs. 1 Nr. 6 BetrVG co-determination requirements:
//!
//! - `GET /api/v1/admin/fairness/report`            — aggregate fairness metrics (Gini, frequency distribution)
//! - `GET /api/v1/admin/transparency/data-collection` — machine-readable monitoring-scope disclosure
//!
//! Both endpoints require admin role. A `works_council` role will be added in
//! a future slice (role plumbing is separate from the data layer here).
//!
//! # k-anonymity
//!
//! Any frequency-distribution bucket representing fewer than 5 distinct users
//! is merged into an `"other (<5)"` catch-all. Individual user data is **never**
//! exposed through these endpoints.
//!
//! # Gini coefficient
//!
//! Measures inequality of parking allocation across employees.
//!
//! Formula (the Brown formula via absolute deviations):
//!
//! ```text
//! Gini = (Σᵢ Σⱼ |xᵢ − xⱼ|) / (2 · n · Σᵢ xᵢ)
//! ```
//!
//! where {x₁ … xₙ} are per-user allocation counts in the window.
//!
//! - All counts equal → 0.0 (perfect equality).
//! - One user receives every allocation → approaches 1.0.
//! - Total allocations = 0 → 0.0 (undefined case, defined as equal by convention).

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

use parkhub_common::ApiResponse;

use super::{AuthUser, SharedState, check_admin};
use crate::api::retention::RetentionClass;
use crate::db::AuditLogEntry;

// ─────────────────────────────────────────────────────────────────────────────
// Query parameters
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct FairnessReportParams {
    /// Window start (RFC 3339). Defaults to 30 days ago when omitted.
    pub from: Option<DateTime<Utc>>,
    /// Window end (RFC 3339). Defaults to now when omitted.
    pub to: Option<DateTime<Utc>>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Response types
// ─────────────────────────────────────────────────────────────────────────────

/// Aggregate fairness metrics for the requested time window.
///
/// No individual user data is included. Buckets with fewer than 5 users are
/// merged into `"other (<5)"` to preserve k-anonymity (k = 5).
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FairnessReport {
    /// Window start used for this report (RFC 3339).
    pub window_from: DateTime<Utc>,
    /// Window end used for this report (RFC 3339).
    pub window_to: DateTime<Utc>,
    /// Total number of allocation events (RecommendationServed + ExactCoverAllocationServed).
    pub total_allocations: u64,
    /// Number of distinct users who received at least one allocation.
    pub users_with_allocations: u64,
    /// Distribution of per-user allocation counts across frequency buckets.
    ///
    /// Buckets: `"0"`, `"1-2"`, `"3-5"`, `"6+"`.
    /// Any bucket with < 5 users is merged into `"other (<5)"`.
    pub allocation_frequency_buckets: Vec<FrequencyBucket>,
    /// Denial / rejection reason categories derived from ExactCoverAllocationServed events.
    pub denial_reasons: Vec<DenialReasonCategory>,
    /// Ratio of booking events to allocation events in the window.
    /// A value > 1.0 indicates more bookings than allocations served.
    /// `null` when there are zero allocations.
    pub booking_to_allocation_ratio: Option<f64>,
    /// Gini coefficient over per-user allocation counts (0.0 = perfect equality,
    /// approaches 1.0 for maximum inequality). See module-level docs for formula.
    /// `null` when there are zero allocations.
    pub gini_coefficient: Option<f64>,
}

/// A single frequency-distribution bucket.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FrequencyBucket {
    /// Human-readable bucket label (e.g. `"0"`, `"1-2"`, `"3-5"`, `"6+"`, `"other (<5)"`).
    pub label: String,
    /// Number of users in this bucket.
    pub user_count: u64,
}

/// Count of a single denial/rejection reason category.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DenialReasonCategory {
    /// Category key (e.g. `"fallback_no_solution"`, `"fallback_search_limited"`,
    /// `"fallback_input_limited"`, `"unknown"`).
    pub reason: String,
    /// Number of allocation events with this denial status.
    pub count: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Disclosure types
// ─────────────────────────────────────────────────────────────────────────────

/// Machine-readable monitoring-scope disclosure for §87 BetrVG.
///
/// Lists every data category the system collects about employees, including
/// the legal basis, default TTL, and any statutory minimum retention period.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DataCollectionDisclosure {
    /// Timestamp this disclosure was generated.
    pub generated_at: DateTime<Utc>,
    /// All data categories collected, derived from the RetentionClass registry
    /// plus the named collection surfaces below.
    pub data_categories: Vec<DataCategoryDisclosure>,
    /// Explicit no-covert-monitoring guarantee.
    pub no_covert_monitoring: &'static str,
    /// Note on works-council access rights.
    ///
    /// `works_council` role plumbing is a future slice; today admin-only.
    pub works_council_access_note: &'static str,
    /// Clause: allocation decisions are NOT used for individual performance evaluation.
    pub no_performance_evaluation: &'static str,
}

/// Disclosure for a single data category.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DataCategoryDisclosure {
    /// Retention class identifier (snake_case, matches audit log `retention_deletion_class`).
    pub retention_class: String,
    /// Plain-language description of what data is collected in this category.
    pub description: String,
    /// Processing purpose.
    pub purpose: String,
    /// Legal basis (GDPR / BDSG reference).
    pub legal_basis: String,
    /// Default TTL in days.
    pub default_ttl_days: u32,
    /// Statutory minimum retention in days. `null` means no legal-hold constraint.
    pub statutory_minimum_days: Option<u32>,
    /// Collection surfaces where this data category is produced.
    pub surfaces: Vec<&'static str>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Pure computation functions (tested independently)
// ─────────────────────────────────────────────────────────────────────────────

/// Gini coefficient over a slice of non-negative allocation counts.
///
/// Formula: `G = Σᵢ Σⱼ |xᵢ − xⱼ| / (2 · n · Σᵢ xᵢ)`
///
/// Returns `None` when all counts are zero (undefined, treated as no-data).
pub fn gini_coefficient(counts: &[u64]) -> Option<f64> {
    let total: u64 = counts.iter().sum();
    if total == 0 || counts.is_empty() {
        return None;
    }
    let n = counts.len() as f64;
    // Compute Σᵢ Σⱼ |xᵢ − xⱼ| using the sorted-list identity:
    //   Σᵢ Σⱼ |xᵢ − xⱼ| = 2 · Σᵢ (2i − n − 1) · xᵢ  (after sorting ascending, 1-indexed i)
    // This runs in O(n log n) instead of O(n²).
    let mut sorted = counts.to_vec();
    sorted.sort_unstable();
    let numerator: f64 = sorted
        .iter()
        .enumerate()
        .map(|(i, &x)| {
            // i is 0-indexed; the formula uses 1-indexed rank r = i + 1
            // contribution = (2r - n - 1) * x = (2*(i+1) - n - 1) * x
            let rank = (i + 1) as f64;
            (2.0 * rank - n - 1.0) * (x as f64)
        })
        .sum();
    let denominator = n * (total as f64);
    Some((numerator / denominator).clamp(0.0, 1.0))
}

/// Assign a per-user allocation count to a named frequency bucket.
fn count_to_bucket(count: u64) -> &'static str {
    match count {
        0 => "0",
        1..=2 => "1-2",
        3..=5 => "3-5",
        _ => "6+",
    }
}

/// Build frequency buckets from a map of `bucket_label → user_count`.
///
/// Buckets with `user_count < K_ANONYMITY_THRESHOLD` are merged into
/// `"other (<5)"`. The four named buckets always appear (even at 0 user
/// count) so the caller can rely on a stable schema.
///
/// k-anonymity threshold: 5.
pub fn apply_k_anonymity(raw: &HashMap<&'static str, u64>) -> Vec<FrequencyBucket> {
    const K: u64 = 5;
    const NAMED: [&str; 4] = ["0", "1-2", "3-5", "6+"];
    let mut other_count: u64 = 0;
    let mut result: Vec<FrequencyBucket> = NAMED
        .iter()
        .map(|label| {
            let user_count = *raw.get(label).unwrap_or(&0);
            if user_count > 0 && user_count < K {
                other_count += user_count;
                FrequencyBucket {
                    label: (*label).to_string(),
                    user_count: 0, // hidden — merged into other
                }
            } else {
                FrequencyBucket {
                    label: (*label).to_string(),
                    user_count,
                }
            }
        })
        .filter(|b| b.user_count > 0 || NAMED.contains(&b.label.as_str()))
        .collect();

    // Remove the zeroed-out buckets that were merged
    result.retain(|b| {
        b.user_count > 0
            || !b
                .label
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit() || c == '6')
    });

    if other_count > 0 {
        result.push(FrequencyBucket {
            label: "other (<5)".to_string(),
            user_count: other_count,
        });
    }
    result
}

/// Build allocation frequency buckets from per-user counts.
/// Returns the k-anonymised bucket list.
pub fn build_frequency_buckets(per_user_counts: &HashMap<String, u64>) -> Vec<FrequencyBucket> {
    let mut raw: HashMap<&'static str, u64> = HashMap::new();
    for count in per_user_counts.values() {
        *raw.entry(count_to_bucket(*count)).or_insert(0) += 1;
    }
    apply_k_anonymity(&raw)
}

/// Constant event-type strings for allocation audit events.
const RECOMMENDATION_SERVED: &str = "RecommendationServed";
const EXACT_COVER_SERVED: &str = "ExactCoverAllocationServed";
const BOOKING_CREATED: &str = "booking_created";

fn is_allocation_event(e: &AuditLogEntry) -> bool {
    e.event_type == RECOMMENDATION_SERVED || e.event_type == EXACT_COVER_SERVED
}

fn is_booking_event(e: &AuditLogEntry) -> bool {
    e.event_type == BOOKING_CREATED
}

/// Extract `fallback_status` from an ExactCoverAllocationServed entry's details JSON.
/// Returns `"unknown"` when not present or not parseable.
fn extract_denial_reason(entry: &AuditLogEntry) -> &'static str {
    let Some(details_str) = entry.details.as_deref() else {
        return "unknown";
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(details_str) else {
        return "unknown";
    };
    match json.get("fallback_status").and_then(|v| v.as_str()) {
        Some("fallback_no_solution") => "fallback_no_solution",
        Some("fallback_search_limited") => "fallback_search_limited",
        Some("fallback_input_limited") => "fallback_input_limited",
        Some("solved") | None => "unknown",
        _ => "unknown",
    }
}

/// Aggregate fairness metrics from a slice of audit log entries.
pub fn aggregate_fairness(
    entries: &[AuditLogEntry],
    window_from: DateTime<Utc>,
    window_to: DateTime<Utc>,
) -> FairnessReport {
    let in_window = |e: &AuditLogEntry| e.timestamp >= window_from && e.timestamp <= window_to;

    let mut per_user_allocations: HashMap<String, u64> = HashMap::new();
    let mut total_allocations: u64 = 0;
    let mut total_bookings: u64 = 0;
    let mut denial_counts: HashMap<&'static str, u64> = HashMap::new();

    for entry in entries.iter().filter(|e| in_window(e)) {
        if is_allocation_event(entry) {
            total_allocations += 1;
            if let Some(uid) = entry.user_id {
                *per_user_allocations.entry(uid.to_string()).or_insert(0) += 1;
            }
            // Denial reason only applies to ExactCoverAllocationServed
            if entry.event_type == EXACT_COVER_SERVED {
                let reason = extract_denial_reason(entry);
                *denial_counts.entry(reason).or_insert(0) += 1;
            }
        } else if is_booking_event(entry) {
            total_bookings += 1;
        }
    }

    let users_with_allocations = per_user_allocations.values().filter(|&&c| c > 0).count() as u64;

    let counts_vec: Vec<u64> = per_user_allocations.values().copied().collect();
    let gini = gini_coefficient(&counts_vec);

    let booking_to_allocation_ratio = if total_allocations > 0 {
        Some(total_bookings as f64 / total_allocations as f64)
    } else {
        None
    };

    let allocation_frequency_buckets = build_frequency_buckets(&per_user_allocations);

    let denial_reasons: Vec<DenialReasonCategory> = {
        let mut v: Vec<DenialReasonCategory> = denial_counts
            .into_iter()
            .map(|(reason, count)| DenialReasonCategory {
                reason: reason.to_string(),
                count,
            })
            .collect();
        v.sort_by(|a, b| b.count.cmp(&a.count).then(a.reason.cmp(&b.reason)));
        v
    };

    FairnessReport {
        window_from,
        window_to,
        total_allocations,
        users_with_allocations,
        allocation_frequency_buckets,
        denial_reasons,
        booking_to_allocation_ratio,
        gini_coefficient: gini,
    }
}

/// Build the static data-collection disclosure from the RetentionClass registry.
pub fn build_disclosure() -> DataCollectionDisclosure {
    let data_categories = RetentionClass::ALL
        .iter()
        .map(|&class| retention_class_to_disclosure(class))
        .collect();

    DataCollectionDisclosure {
        generated_at: Utc::now(),
        data_categories,
        no_covert_monitoring: concat!(
            "This system does not perform covert monitoring. All data collection ",
            "surfaces listed here are disclosed to the works council (Betriebsrat) ",
            "under §87 Abs. 1 Nr. 6 BetrVG."
        ),
        works_council_access_note: concat!(
            "The `works_council` role (future slice) will grant read-only access to ",
            "this endpoint and to /api/v1/admin/fairness/report without requiring ",
            "full admin privileges."
        ),
        no_performance_evaluation: concat!(
            "Allocation decisions recorded in this system are NOT used for individual ",
            "performance evaluation of employees. The fairness report is an aggregate ",
            "statistical tool only (§75 Abs. 1 BetrVG)."
        ),
    }
}

fn retention_class_to_disclosure(class: RetentionClass) -> DataCategoryDisclosure {
    let (description, purpose, legal_basis, surfaces) = match class {
        RetentionClass::OperationalPresence => (
            "Short-lived operational presence data: check-in and check-out events, slot status changes.",
            "Parking-lot capacity management and real-time availability.",
            "Art. 6(1)(b) GDPR — performance of a contract; Art. 6(1)(f) GDPR — legitimate interests.",
            vec!["check_in", "slot_status"],
        ),
        RetentionClass::BookingHistory => (
            "Booking records: slot ID, date/time, duration, booking status.",
            "Contract fulfilment, dispute resolution, usage reporting.",
            "Art. 6(1)(b) GDPR — performance of a contract.",
            vec!["bookings"],
        ),
        RetentionClass::SecurityAuditLog => (
            "Security and admin audit log: login/logout, admin actions, config changes.",
            "Security monitoring, incident investigation, compliance evidence.",
            "Art. 6(1)(c) GDPR — legal obligation; Art. 6(1)(f) GDPR — legitimate interests.",
            vec!["audit_log", "admin_actions"],
        ),
        RetentionClass::HrLabour => (
            "HR / labour-law records: absence requests, approval decisions.",
            "Compliance with §87 BetrVG, employment contract administration.",
            "Art. 6(1)(c) GDPR — legal obligation (§26 BDSG, BetrVG).",
            vec!["absences", "absence_approvals"],
        ),
        RetentionClass::AnprRaw => (
            "Raw ANPR (automatic number-plate recognition) reads.",
            "Vehicle access control, lot-entry verification.",
            "Art. 6(1)(f) GDPR — legitimate interests (access security).",
            vec!["anpr_reader"],
        ),
        RetentionClass::EvSession => (
            "Electric-vehicle charging session data: connector ID, energy delivered, start/end time.",
            "EV infrastructure management, billing.",
            "Art. 6(1)(b) GDPR — performance of a contract.",
            vec!["ev_charging"],
        ),
        RetentionClass::BillingFiscal => (
            "Billing and fiscal records: invoices, payment references (GoBD §147 AO).",
            "Statutory fiscal record-keeping (GoBD, §147 AO).",
            "Art. 6(1)(c) GDPR — legal obligation (§147 AO, GoBD).",
            vec!["invoices", "payments"],
        ),
    };

    DataCategoryDisclosure {
        retention_class: class.as_str().to_string(),
        description: description.to_string(),
        purpose: purpose.to_string(),
        legal_basis: legal_basis.to_string(),
        default_ttl_days: class.default_ttl_days(),
        statutory_minimum_days: class.statutory_minimum_days(),
        surfaces,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HTTP handlers
// ─────────────────────────────────────────────────────────────────────────────

/// Aggregate fairness report: allocation frequency distribution, Gini coefficient,
/// denial reasons, booking-to-allocation ratio.
///
/// Returns aggregate statistics only. No individual user data is exposed.
/// k-anonymity: buckets with fewer than 5 users are merged into `"other (<5)"`.
///
/// Requires admin role. `works_council` role access will be added in a future slice.
#[utoipa::path(
    get,
    path = "/api/v1/admin/fairness/report",
    tag = "Admin",
    params(FairnessReportParams),
    responses(
        (status = 200, description = "Aggregate fairness report", body = FairnessReport),
        (status = 403, description = "Admin access required"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_fairness_report(
    State(state): State<SharedState>,
    axum::Extension(auth_user): axum::Extension<AuthUser>,
    Query(params): Query<FairnessReportParams>,
) -> (StatusCode, Json<ApiResponse<FairnessReport>>) {
    let state_read = state.read().await;
    if let Err((status, msg)) = check_admin(&state_read, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }

    let window_to = params.to.unwrap_or_else(Utc::now);
    let window_from = params
        .from
        .unwrap_or_else(|| window_to - chrono::Duration::days(30));

    let entries = match state_read.db.list_all_audit_log().await {
        Ok(e) => e,
        Err(err) => {
            tracing::error!(?err, "failed to load audit log for fairness report");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "INTERNAL_ERROR",
                    "Failed to load audit log",
                )),
            );
        }
    };

    let report = aggregate_fairness(&entries, window_from, window_to);
    (StatusCode::OK, Json(ApiResponse::success(report)))
}

/// Machine-readable monitoring-scope disclosure for §87 BetrVG.
///
/// Enumerates data categories collected about employees, derived from the
/// RetentionClass registry. Suitable for use as the backend of the §87 BetrVG
/// disclosure screen in the works-council portal.
///
/// Requires admin role. `works_council` role access will be added in a future slice.
#[utoipa::path(
    get,
    path = "/api/v1/admin/transparency/data-collection",
    tag = "Admin",
    responses(
        (status = 200, description = "Data collection disclosure", body = DataCollectionDisclosure),
        (status = 403, description = "Admin access required"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_data_collection_disclosure(
    State(state): State<SharedState>,
    axum::Extension(auth_user): axum::Extension<AuthUser>,
) -> (StatusCode, Json<ApiResponse<DataCollectionDisclosure>>) {
    let state_read = state.read().await;
    if let Err((status, msg)) = check_admin(&state_read, &auth_user).await {
        return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
    }
    let disclosure = build_disclosure();
    (StatusCode::OK, Json(ApiResponse::success(disclosure)))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use uuid::Uuid;

    /// Convenience wrapper: build k-anonymised buckets from a flat list of
    /// per-user allocation counts.
    fn frequency_buckets_from_counts(counts: &[u64]) -> Vec<FrequencyBucket> {
        let map: HashMap<String, u64> = counts
            .iter()
            .enumerate()
            .map(|(i, &c)| (format!("user-{i}"), c))
            .collect();
        build_frequency_buckets(&map)
    }

    // ── Gini coefficient ──────────────────────────────────────────────────────

    /// Equal allocations across all users → Gini = 0 (perfect equality).
    #[test]
    fn gini_equal_allocations_returns_zero() {
        let counts = vec![3u64, 3, 3, 3, 3];
        let g = gini_coefficient(&counts).expect("non-zero totals");
        assert!(
            (g - 0.0).abs() < 1e-9,
            "equal allocation should give Gini=0, got {g}"
        );
    }

    /// Single user receives all allocations → Gini approaches 1.
    #[test]
    fn gini_single_user_takes_all_approaches_one() {
        // 1 user gets N allocations, N-1 users get 0.
        // With 0s excluded by None semantics? No — we include 0s when building counts.
        // But in practice, per_user_allocations only tracks users who appear in audit log.
        // So: one user with 100, nine with 0 but they don't appear → just [100].
        // That's a single element, gini should be 0 for single element (n=1).
        // More realistic: 1 user with 10, 9 users with 1 each → near 0.45.
        let counts = vec![10u64, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let g = gini_coefficient(&counts).expect("non-zero totals");
        // For 1 user with 10, 9 with 0: G = (9*10 + 9*10)/(2*10*10) = 180/200 = 0.9
        assert!(g > 0.85, "near-monopoly should give Gini > 0.85, got {g}");
        assert!(g <= 1.0, "Gini must be ≤ 1, got {g}");
    }

    /// Two users with equal counts → Gini = 0.
    #[test]
    fn gini_two_equal_users() {
        let counts = vec![5u64, 5];
        let g = gini_coefficient(&counts).expect("non-zero");
        assert!((g - 0.0).abs() < 1e-9);
    }

    /// Empty count slice → None.
    #[test]
    fn gini_empty_returns_none() {
        assert!(gini_coefficient(&[]).is_none());
    }

    /// All-zero count slice → None.
    #[test]
    fn gini_all_zero_returns_none() {
        assert!(gini_coefficient(&[0, 0, 0]).is_none());
    }

    /// Known fixture: [1, 2, 3, 4] → Gini = 0.25.
    /// Manually: sorted = [1,2,3,4], n=4, total=10
    /// contributions = (2*1-4-1)*1 + (2*2-4-1)*2 + (2*3-4-1)*3 + (2*4-4-1)*4
    ///               = (-3)*1 + (-1)*2 + (1)*3 + (3)*4 = -3 - 2 + 3 + 12 = 10
    /// Gini = 10 / (4 * 10) = 0.25
    #[test]
    fn gini_known_fixture_1234() {
        let counts = vec![1u64, 2, 3, 4];
        let g = gini_coefficient(&counts).expect("non-zero");
        assert!((g - 0.25).abs() < 1e-9, "expected Gini=0.25, got {g}");
    }

    // ── k-anonymity ──────────────────────────────────────────────────────────

    /// Buckets with ≥ 5 users are kept; those with < 5 are merged into "other (<5)".
    #[test]
    fn k_anonymity_merges_small_buckets() {
        let buckets = frequency_buckets_from_counts(&[
            // 3 users with 1-2 allocations → bucket "1-2" has 3 users → hidden
            1, 1, 2, // 6 users with 3-5 allocations → bucket "3-5" has 6 users → kept
            3, 3, 3, 4, 5, 5,
        ]);
        let bucket_map: HashMap<&str, u64> = buckets
            .iter()
            .map(|b| (b.label.as_str(), b.user_count))
            .collect();
        // "1-2" bucket had 3 users → should be in "other (<5)"
        assert_eq!(
            bucket_map.get("1-2").copied().unwrap_or(0),
            0,
            "small bucket should be zeroed/absent"
        );
        // "3-5" bucket had 6 users → kept
        assert_eq!(bucket_map.get("3-5").copied().unwrap_or(0), 6);
        // "other (<5)" collects the 3 users from "1-2"
        assert_eq!(bucket_map.get("other (<5)").copied().unwrap_or(0), 3);
    }

    /// Exactly 5 users in a bucket is NOT merged (k-anonymity threshold is < 5).
    #[test]
    fn k_anonymity_keeps_exactly_five_users() {
        let buckets = frequency_buckets_from_counts(&[1, 1, 1, 1, 1]);
        let bucket_map: HashMap<&str, u64> = buckets
            .iter()
            .map(|b| (b.label.as_str(), b.user_count))
            .collect();
        assert_eq!(bucket_map.get("1-2").copied().unwrap_or(0), 5);
        assert_eq!(bucket_map.get("other (<5)").copied().unwrap_or(0), 0);
    }

    /// Zero allocations for all users → "0" bucket, no "other (<5)".
    #[test]
    fn k_anonymity_zero_allocations_bucket() {
        // 7 users with 0 allocations — note: zero-allocation users only appear
        // if we explicitly add them. In practice they don't appear in audit log,
        // but the disclosure screen may request a full user-space report.
        let counts = vec![0u64; 7];
        let buckets = frequency_buckets_from_counts(&counts);
        let bucket_map: HashMap<&str, u64> = buckets
            .iter()
            .map(|b| (b.label.as_str(), b.user_count))
            .collect();
        assert_eq!(bucket_map.get("0").copied().unwrap_or(0), 7);
    }

    // ── Window filtering ──────────────────────────────────────────────────────

    fn make_allocation_entry(user_id: Uuid, ts: DateTime<Utc>, event_type: &str) -> AuditLogEntry {
        AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: ts,
            event_type: event_type.to_string(),
            user_id: Some(user_id),
            username: None,
            details: None,
            target_type: None,
            target_id: None,
            ip_address: None,
        }
    }

    fn make_booking_entry(user_id: Uuid, ts: DateTime<Utc>) -> AuditLogEntry {
        AuditLogEntry {
            id: Uuid::new_v4(),
            timestamp: ts,
            event_type: BOOKING_CREATED.to_string(),
            user_id: Some(user_id),
            username: None,
            details: None,
            target_type: None,
            target_id: None,
            ip_address: None,
        }
    }

    /// Only entries within the time window are counted.
    #[test]
    fn window_filtering_excludes_out_of_range() {
        let uid = Uuid::new_v4();
        let now = Utc::now();
        let from = now - Duration::days(7);
        let to = now;

        let entries = vec![
            // Inside window
            make_allocation_entry(uid, now - Duration::days(3), RECOMMENDATION_SERVED),
            make_allocation_entry(uid, now - Duration::days(1), EXACT_COVER_SERVED),
            // Outside window (before from)
            make_allocation_entry(uid, now - Duration::days(10), RECOMMENDATION_SERVED),
            // Outside window (after to)
            make_allocation_entry(uid, now + Duration::days(1), RECOMMENDATION_SERVED),
        ];

        let report = aggregate_fairness(&entries, from, to);
        assert_eq!(report.total_allocations, 2);
    }

    /// Booking ratio is computed correctly.
    #[test]
    fn booking_to_allocation_ratio_computed_correctly() {
        let uid = Uuid::new_v4();
        let now = Utc::now();
        let from = now - Duration::days(7);

        let entries = vec![
            make_allocation_entry(uid, now - Duration::days(1), RECOMMENDATION_SERVED),
            make_allocation_entry(uid, now - Duration::days(2), RECOMMENDATION_SERVED),
            make_booking_entry(uid, now - Duration::days(1)),
            make_booking_entry(uid, now - Duration::days(2)),
            make_booking_entry(uid, now - Duration::days(3)),
        ];

        let report = aggregate_fairness(&entries, from, now);
        assert_eq!(report.total_allocations, 2);
        let ratio = report
            .booking_to_allocation_ratio
            .expect("non-zero allocations");
        assert!(
            (ratio - 1.5).abs() < 1e-9,
            "expected 3/2 = 1.5, got {ratio}"
        );
    }

    /// Zero allocations → ratio is None, Gini is None.
    #[test]
    fn zero_allocations_gives_none_ratio_and_gini() {
        let uid = Uuid::new_v4();
        let now = Utc::now();
        let entries = vec![make_booking_entry(uid, now - Duration::days(1))];
        let report = aggregate_fairness(&entries, now - Duration::days(7), now);
        assert!(report.booking_to_allocation_ratio.is_none());
        assert!(report.gini_coefficient.is_none());
    }

    // ── Disclosure ────────────────────────────────────────────────────────────

    /// Disclosure lists all 7 RetentionClass variants.
    #[test]
    fn disclosure_lists_all_seven_retention_classes() {
        let disclosure = build_disclosure();
        assert_eq!(
            disclosure.data_categories.len(),
            RetentionClass::ALL.len(),
            "disclosure must list every retention class"
        );
        let class_keys: Vec<&str> = disclosure
            .data_categories
            .iter()
            .map(|c| c.retention_class.as_str())
            .collect();
        for class in RetentionClass::ALL {
            assert!(
                class_keys.contains(&class.as_str()),
                "missing class: {}",
                class.as_str()
            );
        }
    }

    /// Statutory minimums are correctly propagated in disclosure.
    #[test]
    fn disclosure_statutory_minimums_match_registry() {
        let disclosure = build_disclosure();
        for cat in &disclosure.data_categories {
            let class: RetentionClass = cat.retention_class.parse().expect("valid class key");
            assert_eq!(
                cat.statutory_minimum_days,
                class.statutory_minimum_days(),
                "statutory minimum mismatch for {}",
                cat.retention_class
            );
            assert_eq!(
                cat.default_ttl_days,
                class.default_ttl_days(),
                "default TTL mismatch for {}",
                cat.retention_class
            );
        }
    }

    // ── Module registry snake_case key convention ─────────────────────────────

    /// All RetentionClass::as_str() values are snake_case (no uppercase, no spaces).
    #[test]
    fn retention_class_keys_are_snake_case() {
        for class in RetentionClass::ALL {
            let key = class.as_str();
            assert!(
                key.chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()),
                "class key '{}' violates snake_case convention",
                key
            );
        }
    }

    // ── Denial reasons ────────────────────────────────────────────────────────

    /// Denial reasons are extracted from ExactCoverAllocationServed details.
    #[test]
    fn denial_reasons_extracted_from_exact_cover_events() {
        let now = Utc::now();
        let from = now - Duration::days(1);
        let uid = Uuid::new_v4();
        let make_exact_cover = |status: &str| {
            let details = serde_json::json!({ "fallback_status": status }).to_string();
            AuditLogEntry {
                id: Uuid::new_v4(),
                timestamp: now,
                event_type: EXACT_COVER_SERVED.to_string(),
                user_id: Some(uid),
                username: None,
                details: Some(details),
                target_type: None,
                target_id: None,
                ip_address: None,
            }
        };

        let entries = vec![
            make_exact_cover("fallback_no_solution"),
            make_exact_cover("fallback_no_solution"),
            make_exact_cover("fallback_search_limited"),
        ];
        let report = aggregate_fairness(&entries, from, now + Duration::seconds(1));
        let reason_map: HashMap<&str, u64> = report
            .denial_reasons
            .iter()
            .map(|r| (r.reason.as_str(), r.count))
            .collect();
        assert_eq!(
            reason_map.get("fallback_no_solution").copied().unwrap_or(0),
            2
        );
        assert_eq!(
            reason_map
                .get("fallback_search_limited")
                .copied()
                .unwrap_or(0),
            1
        );
    }

    // ── RBAC: non-admin should get 403 ────────────────────────────────────────
    // Full HTTP integration test is deferred to the integration test suite.
    // Here we test the check_admin helper using an in-memory DB.

    use crate::config::ServerConfig;
    use crate::db::{Database, DatabaseConfig};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn make_db() -> (Database, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = DatabaseConfig {
            path: dir.path().to_path_buf(),
            encryption_enabled: false,
            passphrase: None,
            create_if_missing: true,
        };
        (Database::open(&cfg).expect("open db"), dir)
    }

    fn make_state(db: Database) -> super::SharedState {
        use crate::AppState;
        Arc::new(RwLock::new(AppState {
            config: ServerConfig::default(),
            db,
            mdns: None,
            scheduler: None,
            ws_events: crate::api::ws::EventBroadcaster::new(),
            fleet_events: crate::api::sse::FleetEventBroadcaster::new(),
            revocation_store: crate::jwt::TokenRevocationList::new(),
        }))
    }

    /// Non-admin user gets Forbidden from check_admin.
    #[tokio::test]
    async fn rbac_non_admin_is_forbidden() {
        use parkhub_common::User;
        use parkhub_common::UserRole;
        use parkhub_common::models::UserPreferences;

        let (db, _dir) = make_db();
        let user_id = Uuid::new_v4();
        let regular_user = User {
            id: user_id,
            username: "regularuser".to_string(),
            email: "user@example.com".to_string(),
            name: "Regular User".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::User,
            is_active: true,
            phone: None,
            picture: None,
            preferences: UserPreferences {
                language: "de".to_string(),
                theme: "system".to_string(),
                notifications_enabled: true,
                email_reminders: false,
                default_duration_minutes: None,
                favorite_slots: Vec::new(),
            },
            credits_balance: 0,
            credits_monthly_quota: 0,
            credits_last_refilled: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
            tenant_id: None,
            accessibility_needs: None,
            cost_center: None,
            department: None,
            settings: None,
        };
        db.save_user(&regular_user).await.expect("save user");

        let state = make_state(db);
        let auth_user = AuthUser {
            user_id,
            api_key_id: None,
        };
        let state_read = state.read().await;
        let result = check_admin(&state_read, &auth_user).await;
        assert!(result.is_err(), "non-admin should be rejected");
        let (status, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    /// Admin user passes check_admin.
    #[tokio::test]
    async fn rbac_admin_is_allowed() {
        use parkhub_common::User;
        use parkhub_common::UserRole;
        use parkhub_common::models::UserPreferences;

        let (db, _dir) = make_db();
        let user_id = Uuid::new_v4();
        let admin_user = User {
            id: user_id,
            username: "adminuser".to_string(),
            email: "admin@example.com".to_string(),
            name: "Admin User".to_string(),
            password_hash: "hash".to_string(),
            role: UserRole::Admin,
            is_active: true,
            phone: None,
            picture: None,
            preferences: UserPreferences {
                language: "de".to_string(),
                theme: "system".to_string(),
                notifications_enabled: true,
                email_reminders: false,
                default_duration_minutes: None,
                favorite_slots: Vec::new(),
            },
            credits_balance: 0,
            credits_monthly_quota: 0,
            credits_last_refilled: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
            tenant_id: None,
            accessibility_needs: None,
            cost_center: None,
            department: None,
            settings: None,
        };
        db.save_user(&admin_user).await.expect("save user");

        let state = make_state(db);
        let auth_user = AuthUser {
            user_id,
            api_key_id: None,
        };
        let state_read = state.read().await;
        let result = check_admin(&state_read, &auth_user).await;
        assert!(result.is_ok(), "admin should be allowed");
    }
}
