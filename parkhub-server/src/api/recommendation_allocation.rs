//! Allocation strategy primitives for recommendation workflows.
//!
//! `weighted_v1` stays the default for quick single-slot recommendations. This
//! module provides the small, deterministic `exact_cover_v1` core for later
//! batch/recurring allocation workflows where every required constraint must be
//! covered exactly once.

use axum::{Extension, Json, extract::State, http::StatusCode};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use uuid::Uuid;

use parkhub_common::ApiResponse;

use super::{
    AuthUser, SharedState, check_admin,
    recommendations::{RecommendationAllocationConfig, RecommendationEngineConfig},
};

const DEFAULT_MAX_OPTIONS: usize = 256;
const DEFAULT_MAX_SEARCH_NODES: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactCoverOption {
    pub id: String,
    pub covers: Vec<String>,
    pub weight: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactCoverLimits {
    pub max_options: usize,
    pub max_search_nodes: usize,
}

impl Default for ExactCoverLimits {
    fn default() -> Self {
        Self {
            max_options: DEFAULT_MAX_OPTIONS,
            max_search_nodes: DEFAULT_MAX_SEARCH_NODES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExactCoverResult {
    pub strategy: &'static str,
    pub status: ExactCoverStatus,
    pub selected_option_ids: Vec<String>,
    pub covered_constraints: Vec<String>,
    pub search_nodes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExactCoverStatus {
    Solved,
    FallbackNoSolution,
    FallbackInputLimited,
    FallbackSearchLimited,
}

#[derive(Debug, Deserialize)]
pub struct ExactCoverAllocationRequest {
    pub required_constraints: Vec<String>,
    pub options: Vec<ExactCoverOption>,
    pub limits: Option<ExactCoverLimits>,
}

#[derive(Debug, Serialize)]
pub struct ExactCoverAllocationResponse {
    pub allocation_trace_id: Uuid,
    pub result: ExactCoverResult,
    pub legal_boundary: ExactCoverLegalBoundary,
}

#[derive(Debug, Serialize)]
pub struct ExactCoverLegalBoundary {
    pub legal_review_required: bool,
    pub attorney_review_status: &'static str,
    pub execution_allowed: bool,
    pub disclaimer: &'static str,
}

#[derive(Debug, Clone)]
struct NormalizedOption {
    id: String,
    covers: BTreeSet<String>,
    weight: i64,
}

#[derive(Debug)]
struct SearchState {
    nodes: usize,
    max_nodes: usize,
    limited: bool,
}

/// Solve an exact-cover allocation with deterministic Algorithm X backtracking.
///
/// Non-required option constraints are ignored. Ties are stable: higher weight
/// wins first, then lower option id. Callers must still decide whether to fall
/// back to `weighted_v1`; this core reports only the allocation result.
pub fn solve_exact_cover_v1(
    required_constraints: &[String],
    options: &[ExactCoverOption],
    limits: ExactCoverLimits,
) -> ExactCoverResult {
    let required = normalize_constraints(required_constraints);
    if required.is_empty() {
        return ExactCoverResult {
            strategy: "exact_cover_v1",
            status: ExactCoverStatus::Solved,
            selected_option_ids: Vec::new(),
            covered_constraints: Vec::new(),
            search_nodes: 0,
        };
    }

    if options.len() > limits.max_options {
        return fallback(ExactCoverStatus::FallbackInputLimited, 0);
    }

    let normalized = normalize_options(options, &required);
    let mut state = SearchState {
        nodes: 0,
        max_nodes: limits.max_search_nodes,
        limited: false,
    };
    let mut selected = Vec::new();

    let solution = search_exact_cover(&required, &required, &normalized, &mut selected, &mut state);
    match solution {
        Some(indices) => {
            let mut selected_option_ids = indices
                .iter()
                .map(|idx| normalized[*idx].id.clone())
                .collect::<Vec<_>>();
            selected_option_ids.sort();
            ExactCoverResult {
                strategy: "exact_cover_v1",
                status: ExactCoverStatus::Solved,
                selected_option_ids,
                covered_constraints: required.into_iter().collect(),
                search_nodes: state.nodes,
            }
        }
        None if state.limited => fallback(ExactCoverStatus::FallbackSearchLimited, state.nodes),
        None => fallback(ExactCoverStatus::FallbackNoSolution, state.nodes),
    }
}

/// Admin-only exact-cover allocation utility for batch/recurring workflows.
///
/// This intentionally lives outside the quick-booking recommendation endpoint:
/// `weighted_v1` remains the default scorer for ordinary single-slot requests.
pub async fn solve_exact_cover_allocation(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(request): Json<ExactCoverAllocationRequest>,
) -> (StatusCode, Json<ApiResponse<ExactCoverAllocationResponse>>) {
    let engine = {
        let state_guard = state.read().await;
        if let Err((status, msg)) = check_admin(&state_guard, &auth_user).await {
            return (status, Json(ApiResponse::error("FORBIDDEN", msg)));
        }
        RecommendationEngineConfig::load(&state_guard.db).await
    };

    let limits = effective_limits(request.limits, &engine.allocation);
    let result = solve_exact_cover_v1(&request.required_constraints, &request.options, limits);
    let allocation_trace_id = Uuid::new_v4();

    let audit_result = {
        let state_guard = state.read().await;
        audit_exact_cover_allocation(
            &state_guard,
            allocation_trace_id,
            &auth_user,
            &request,
            limits,
            &result,
        )
        .await
    };
    if let Err(err) = audit_result {
        tracing::error!(
            %allocation_trace_id,
            error = ?err,
            "failed to persist exact-cover allocation audit trace"
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "AUDIT_TRACE_PERSIST_FAILED",
                "Failed to persist exact-cover allocation audit trace",
            )),
        );
    }

    (
        StatusCode::OK,
        Json(ApiResponse::success(ExactCoverAllocationResponse {
            allocation_trace_id,
            result,
            legal_boundary: ExactCoverLegalBoundary {
                legal_review_required: true,
                attorney_review_status: "required_before_customer_wording",
                execution_allowed: false,
                disclaimer: "exact_cover_v1 is operational scheduling support; attorney review, citation verification, client authorization, and final legal judgment remain required before customer-facing legal or profiling claims ship.",
            },
        })),
    )
}

fn effective_limits(
    request_limits: Option<ExactCoverLimits>,
    allocation: &RecommendationAllocationConfig,
) -> ExactCoverLimits {
    let configured = ExactCoverLimits {
        max_options: allocation.exact_cover_max_options,
        max_search_nodes: allocation.exact_cover_max_search_nodes,
    }
    .bounded(DEFAULT_MAX_OPTIONS, DEFAULT_MAX_SEARCH_NODES);

    request_limits
        .unwrap_or(configured)
        .bounded(configured.max_options, configured.max_search_nodes)
}

impl ExactCoverLimits {
    fn bounded(self, max_options: usize, max_search_nodes: usize) -> Self {
        Self {
            max_options: self.max_options.clamp(1, max_options),
            max_search_nodes: self.max_search_nodes.clamp(1, max_search_nodes),
        }
    }
}

async fn audit_exact_cover_allocation(
    app_state: &crate::AppState,
    trace_id: Uuid,
    auth_user: &AuthUser,
    request: &ExactCoverAllocationRequest,
    limits: ExactCoverLimits,
    result: &ExactCoverResult,
) -> anyhow::Result<()> {
    let tenant_id = super::resolve_tenant_id(app_state, auth_user.user_id).await;
    let selected = result
        .selected_option_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let rejected_candidate_ids = request
        .options
        .iter()
        .filter_map(|option| {
            let id = option.id.trim();
            (!id.is_empty() && !selected.contains(id)).then(|| id.to_string())
        })
        .collect::<Vec<_>>();

    let details = serde_json::json!({
        "request_id": trace_id,
        "solver_name": "exact_cover_v1",
        "solver_version": 1,
        "config_hash": exact_cover_config_hash(limits),
        "constraint_set_hash": exact_cover_constraint_hash(&request.required_constraints),
        "candidate_set_hash": exact_cover_candidate_hash(&request.options),
        "selected_option_ids": &result.selected_option_ids,
        "rejected_candidate_ids": rejected_candidate_ids,
        "covered_constraints": &result.covered_constraints,
        "search_nodes": result.search_nodes,
        "tie_break_inputs": {
            "candidate_order": "weight_desc_then_option_id_asc",
            "constraint_order": "fewest_candidates_then_constraint_asc",
            "max_options": limits.max_options,
            "max_search_nodes": limits.max_search_nodes,
        },
        "actor": {
            "user_id": auth_user.user_id,
            "api_key_id": auth_user.api_key_id,
        },
        "tenant_id": tenant_id,
        "fallback_status": status_name(result.status),
        "retention_deletion_class": "operational_evidence_personal_data_possible",
        "legal_boundary": {
            "legal_review_required": true,
            "attorney_review_status": "required_before_customer_wording",
            "execution_allowed": false
        }
    });

    let entry = crate::db::AuditLogEntry {
        id: trace_id,
        timestamp: Utc::now(),
        event_type: "ExactCoverAllocationServed".to_string(),
        user_id: Some(auth_user.user_id),
        username: None,
        details: Some(details.to_string()),
        target_type: Some("recommendation_allocation".to_string()),
        target_id: Some(trace_id.to_string()),
        ip_address: None,
    };

    app_state.db.save_audit_log(&entry).await
}

fn exact_cover_config_hash(limits: ExactCoverLimits) -> String {
    hash_json(&serde_json::json!({
        "strategy": "exact_cover_v1",
        "max_options": limits.max_options,
        "max_search_nodes": limits.max_search_nodes,
    }))
}

fn status_name(status: ExactCoverStatus) -> &'static str {
    match status {
        ExactCoverStatus::Solved => "solved",
        ExactCoverStatus::FallbackNoSolution => "fallback_no_solution",
        ExactCoverStatus::FallbackInputLimited => "fallback_input_limited",
        ExactCoverStatus::FallbackSearchLimited => "fallback_search_limited",
    }
}

fn exact_cover_constraint_hash(required_constraints: &[String]) -> String {
    hash_json(&serde_json::json!(
        normalize_constraints(required_constraints)
            .into_iter()
            .collect::<Vec<_>>()
    ))
}

fn exact_cover_candidate_hash(options: &[ExactCoverOption]) -> String {
    let mut normalized = options
        .iter()
        .filter_map(|option| {
            let id = option.id.trim();
            (!id.is_empty()).then(|| {
                serde_json::json!({
                    "id": id,
                    "covers": normalize_constraints(&option.covers).into_iter().collect::<Vec<_>>(),
                    "weight": option.weight,
                })
            })
        })
        .collect::<Vec<_>>();
    normalized.sort_by(|a, b| {
        let left = a
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let right = b
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        left.cmp(right)
    });
    hash_json(&serde_json::json!(normalized))
}

fn hash_json(value: &serde_json::Value) -> String {
    let payload = serde_json::to_vec(value).unwrap_or_default();
    let digest = Sha256::digest(&payload);
    digest.iter().fold(String::new(), |mut output, byte| {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
        output
    })
}

fn fallback(status: ExactCoverStatus, search_nodes: usize) -> ExactCoverResult {
    ExactCoverResult {
        strategy: "exact_cover_v1",
        status,
        selected_option_ids: Vec::new(),
        covered_constraints: Vec::new(),
        search_nodes,
    }
}

fn normalize_constraints(values: &[String]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn normalize_options(
    options: &[ExactCoverOption],
    required: &BTreeSet<String>,
) -> Vec<NormalizedOption> {
    let mut normalized = options
        .iter()
        .filter_map(|option| {
            let covers = normalize_constraints(&option.covers)
                .intersection(required)
                .cloned()
                .collect::<BTreeSet<_>>();
            (!option.id.trim().is_empty() && !covers.is_empty()).then(|| NormalizedOption {
                id: option.id.trim().to_string(),
                covers,
                weight: option.weight,
            })
        })
        .collect::<Vec<_>>();

    normalized.sort_by(|a, b| b.weight.cmp(&a.weight).then_with(|| a.id.cmp(&b.id)));
    normalized
}

fn search_exact_cover(
    required: &BTreeSet<String>,
    uncovered: &BTreeSet<String>,
    options: &[NormalizedOption],
    selected: &mut Vec<usize>,
    state: &mut SearchState,
) -> Option<Vec<usize>> {
    if state.nodes >= state.max_nodes {
        state.limited = true;
        return None;
    }
    state.nodes += 1;

    if uncovered.is_empty() {
        return Some(selected.clone());
    }

    let covered = required
        .difference(uncovered)
        .cloned()
        .collect::<BTreeSet<_>>();
    let (constraint, candidates) = choose_next_constraint(uncovered, &covered, options)?;

    if candidates.is_empty() {
        return None;
    }

    for option_idx in candidates {
        let option = &options[option_idx];
        if !option.covers.contains(constraint) || !option.covers.is_disjoint(&covered) {
            continue;
        }

        selected.push(option_idx);
        let next_uncovered = uncovered
            .difference(&option.covers)
            .cloned()
            .collect::<BTreeSet<_>>();
        if let Some(solution) =
            search_exact_cover(required, &next_uncovered, options, selected, state)
        {
            return Some(solution);
        }
        selected.pop();

        if state.limited {
            return None;
        }
    }

    None
}

fn choose_next_constraint<'a>(
    uncovered: &'a BTreeSet<String>,
    covered: &BTreeSet<String>,
    options: &[NormalizedOption],
) -> Option<(&'a String, Vec<usize>)> {
    uncovered
        .iter()
        .map(|constraint| {
            let candidates = options
                .iter()
                .enumerate()
                .filter(|(_, option)| {
                    option.covers.contains(constraint) && option.covers.is_disjoint(covered)
                })
                .map(|(idx, _)| idx)
                .collect::<Vec<_>>();
            (constraint, candidates)
        })
        .min_by(|(left_constraint, left), (right_constraint, right)| {
            left.len()
                .cmp(&right.len())
                .then_with(|| left_constraint.cmp(right_constraint))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct ExactCoverFixture {
        required_constraints: Vec<String>,
        options: Vec<ExactCoverFixtureOption>,
        expected: ExactCoverFixtureExpected,
    }

    #[derive(Debug, Deserialize)]
    struct ExactCoverFixtureOption {
        id: String,
        covers: Vec<String>,
        weight: i64,
    }

    #[derive(Debug, Deserialize)]
    struct ExactCoverFixtureExpected {
        status: String,
        selected_option_ids: Vec<String>,
        covered_constraints: Vec<String>,
    }

    fn option(id: &str, covers: &[&str], weight: i64) -> ExactCoverOption {
        ExactCoverOption {
            id: id.to_string(),
            covers: covers.iter().map(|value| (*value).to_string()).collect(),
            weight,
        }
    }

    fn required(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn exact_cover_v1_solves_batch_constraints() {
        let result = solve_exact_cover_v1(
            &required(&["tenant:alpha", "tenant:beta", "ev", "accessible"]),
            &[
                option("slot-a", &["tenant:alpha", "ev"], 90),
                option("slot-b", &["tenant:beta", "accessible"], 80),
                option("slot-c", &["tenant:beta"], 70),
            ],
            ExactCoverLimits::default(),
        );

        assert_eq!(result.status, ExactCoverStatus::Solved);
        assert_eq!(result.selected_option_ids, vec!["slot-a", "slot-b"]);
        assert_eq!(
            result.covered_constraints,
            vec!["accessible", "ev", "tenant:alpha", "tenant:beta"]
        );
    }

    #[test]
    fn exact_cover_v1_uses_deterministic_weight_and_id_tiebreaks() {
        let result = solve_exact_cover_v1(
            &required(&["tenant:alpha"]),
            &[
                option("slot-b", &["tenant:alpha"], 80),
                option("slot-a", &["tenant:alpha"], 80),
                option("slot-c", &["tenant:alpha"], 70),
            ],
            ExactCoverLimits::default(),
        );

        assert_eq!(result.status, ExactCoverStatus::Solved);
        assert_eq!(result.selected_option_ids, vec!["slot-a"]);
    }

    #[test]
    fn exact_cover_v1_reports_no_solution_for_maintenance_gap() {
        let result = solve_exact_cover_v1(
            &required(&["tenant:alpha", "maintenance:open"]),
            &[option("slot-a", &["tenant:alpha"], 90)],
            ExactCoverLimits::default(),
        );

        assert_eq!(result.status, ExactCoverStatus::FallbackNoSolution);
        assert!(result.selected_option_ids.is_empty());
    }

    #[test]
    fn exact_cover_v1_enforces_input_limits() {
        let result = solve_exact_cover_v1(
            &required(&["tenant:alpha"]),
            &[
                option("slot-a", &["tenant:alpha"], 90),
                option("slot-b", &["tenant:alpha"], 80),
            ],
            ExactCoverLimits {
                max_options: 1,
                max_search_nodes: 10,
            },
        );

        assert_eq!(result.status, ExactCoverStatus::FallbackInputLimited);
        assert_eq!(result.search_nodes, 0);
    }

    #[test]
    fn exact_cover_limits_respect_module_caps_and_request_overrides() {
        let allocation = RecommendationAllocationConfig {
            strategy: "exact_cover_v1".to_string(),
            exact_cover_max_options: 8,
            exact_cover_max_search_nodes: 500,
        };

        assert_eq!(
            effective_limits(None, &allocation),
            ExactCoverLimits {
                max_options: 8,
                max_search_nodes: 500,
            }
        );
        assert_eq!(
            effective_limits(
                Some(ExactCoverLimits {
                    max_options: 99,
                    max_search_nodes: 10_000,
                }),
                &allocation,
            ),
            ExactCoverLimits {
                max_options: 8,
                max_search_nodes: 500,
            }
        );
        assert_eq!(
            effective_limits(
                Some(ExactCoverLimits {
                    max_options: 3,
                    max_search_nodes: 50,
                }),
                &allocation,
            ),
            ExactCoverLimits {
                max_options: 3,
                max_search_nodes: 50,
            }
        );
    }

    #[test]
    fn exact_cover_v1_shared_fixtures_match_contract() {
        let fixtures = [
            include_str!(
                "../../../docs/recommendation-engine-fixtures/exact_cover_v1.batch_basic.json"
            ),
            include_str!("../../../docs/recommendation-engine-fixtures/exact_cover_v1.empty.json"),
            include_str!(
                "../../../docs/recommendation-engine-fixtures/exact_cover_v1.fairness_tiebreak.json"
            ),
            include_str!(
                "../../../docs/recommendation-engine-fixtures/exact_cover_v1.no_solution.json"
            ),
        ];

        for raw_fixture in fixtures {
            let fixture: ExactCoverFixture =
                serde_json::from_str(raw_fixture).expect("valid exact-cover fixture");
            let options = fixture
                .options
                .into_iter()
                .map(|option| ExactCoverOption {
                    id: option.id,
                    covers: option.covers,
                    weight: option.weight,
                })
                .collect::<Vec<_>>();
            let result = solve_exact_cover_v1(
                &fixture.required_constraints,
                &options,
                ExactCoverLimits::default(),
            );

            assert_eq!(super::status_name(result.status), fixture.expected.status);
            assert_eq!(
                result.selected_option_ids,
                fixture.expected.selected_option_ids
            );
            assert_eq!(
                result.covered_constraints,
                fixture.expected.covered_constraints
            );
        }
    }
}
