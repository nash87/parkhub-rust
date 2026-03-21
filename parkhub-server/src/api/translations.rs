//! Translation management handlers: proposals, voting, review, and overrides.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use parkhub_common::models::{
    ProposalStatus, TranslationOverride, TranslationProposal, TranslationVote,
};
use parkhub_common::ApiResponse;

use super::{check_admin, AuthUser, SharedState};

// ─────────────────────────────────────────────────────────────────────────────
// Request / Query DTOs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateProposalRequest {
    pub language: String,
    pub key: String,
    pub proposed_value: String,
    pub context: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct VoteRequest {
    pub vote: String, // "up" or "down"
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ReviewRequest {
    pub status: String, // "approved" or "rejected"
    pub comment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProposalQuery {
    pub status: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Validation helpers
// ─────────────────────────────────────────────────────────────────────────────

fn validate_proposal_input(req: &CreateProposalRequest) -> Result<(), &'static str> {
    if req.language.is_empty() || req.language.len() > 10 {
        return Err("Language must be 1-10 characters");
    }
    if req.key.is_empty() || req.key.len() > 255 {
        return Err("Key must be 1-255 characters");
    }
    if req.proposed_value.is_empty() || req.proposed_value.len() > 5000 {
        return Err("Proposed value must be 1-5000 characters");
    }
    if let Some(ref ctx) = req.context {
        if ctx.len() > 1000 {
            return Err("Context must be at most 1000 characters");
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

/// `GET /api/v1/translations/overrides` — list all approved translation overrides
#[utoipa::path(
    get,
    path = "/api/v1/translations/overrides",
    tag = "Translations",
    summary = "List translation overrides",
    description = "Returns all approved translation overrides for runtime i18n patching.",
    responses((status = 200, description = "List of overrides"))
)]
pub(crate) async fn list_overrides(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
) -> Json<ApiResponse<Vec<TranslationOverride>>> {
    let state = state.read().await;
    match state.db.list_translation_overrides().await {
        Ok(overrides) => Json(ApiResponse::success(overrides)),
        Err(e) => {
            tracing::error!("Failed to list translation overrides: {}", e);
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list overrides",
            ))
        }
    }
}

/// `GET /api/v1/translations/proposals` — list proposals (optionally filtered)
#[utoipa::path(
    get,
    path = "/api/v1/translations/proposals",
    tag = "Translations",
    summary = "List translation proposals",
    params(("status" = Option<String>, Query, description = "Filter: pending, approved, rejected")),
    responses((status = 200, description = "List of proposals"))
)]
pub(crate) async fn list_proposals(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<ProposalQuery>,
) -> Json<ApiResponse<Vec<serde_json::Value>>> {
    let state = state.read().await;

    let filter = query.status.as_deref().and_then(|s| match s {
        "pending" => Some(ProposalStatus::Pending),
        "approved" => Some(ProposalStatus::Approved),
        "rejected" => Some(ProposalStatus::Rejected),
        _ => None,
    });

    match state.db.list_translation_proposals(filter.as_ref()).await {
        Ok(proposals) => {
            // Build enriched response with user_vote (single pass, no N+1)
            let mut enriched = Vec::with_capacity(proposals.len());
            for p in &proposals {
                let user_vote = state
                    .db
                    .get_user_vote(p.id, auth_user.user_id)
                    .await
                    .ok()
                    .flatten()
                    .map(|v| v.vote);

                let mut val = serde_json::to_value(p).unwrap_or_default();
                if let Some(obj) = val.as_object_mut() {
                    obj.insert(
                        "user_vote".into(),
                        user_vote
                            .map_or(serde_json::Value::Null, serde_json::Value::String),
                    );
                }
                enriched.push(val);
            }

            Json(ApiResponse::success(enriched))
        }
        Err(e) => {
            tracing::error!("Failed to list translation proposals: {}", e);
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to list proposals",
            ))
        }
    }
}

/// `GET /api/v1/translations/proposals/{id}` — get a single proposal
#[utoipa::path(
    get,
    path = "/api/v1/translations/proposals/{id}",
    tag = "Translations",
    summary = "Get translation proposal",
    params(("id" = String, Path, description = "Proposal ID")),
    responses(
        (status = 200, description = "Proposal details"),
        (status = 404, description = "Not found"),
    )
)]
pub(crate) async fn get_proposal(
    State(state): State<SharedState>,
    Extension(_auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> (StatusCode, Json<ApiResponse<TranslationProposal>>) {
    let state = state.read().await;
    match state.db.get_translation_proposal(&id).await {
        Ok(Some(p)) => (StatusCode::OK, Json(ApiResponse::success(p))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::error("NOT_FOUND", "Proposal not found")),
        ),
        Err(e) => {
            tracing::error!("Failed to get proposal: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to get proposal")),
            )
        }
    }
}

/// `POST /api/v1/translations/proposals` — create a new translation proposal
#[utoipa::path(
    post,
    path = "/api/v1/translations/proposals",
    tag = "Translations",
    summary = "Create translation proposal",
    request_body = CreateProposalRequest,
    responses(
        (status = 201, description = "Proposal created"),
    )
)]
pub(crate) async fn create_proposal(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateProposalRequest>,
) -> (StatusCode, Json<ApiResponse<TranslationProposal>>) {
    // Validate input lengths (matching PHP: language max:10, key max:255, value max:5000, context max:1000)
    if let Err(msg) = validate_proposal_input(&req) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("VALIDATION", msg)),
        );
    }

    let state = state.read().await;

    // Get proposer's name
    let proposer_name = match state.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(u)) => u.name,
        _ => "Unknown".to_string(),
    };

    let proposal = TranslationProposal {
        id: Uuid::new_v4(),
        language: req.language,
        key: req.key.clone(),
        current_value: req.key, // Frontend sends the key; actual current value is client-side
        proposed_value: req.proposed_value,
        context: req.context,
        proposed_by: auth_user.user_id,
        proposed_by_name: proposer_name,
        status: ProposalStatus::Pending,
        votes_for: 0,
        votes_against: 0,
        reviewer_id: None,
        reviewer_name: None,
        review_comment: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    if let Err(e) = state.db.save_translation_proposal(&proposal).await {
        tracing::error!("Failed to save proposal: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to create proposal")),
        );
    }

    tracing::info!(
        "User {} created translation proposal {} for key {}",
        auth_user.user_id,
        proposal.id,
        proposal.key
    );

    (StatusCode::CREATED, Json(ApiResponse::success(proposal)))
}

/// `POST /api/v1/translations/proposals/{id}/vote` — vote on a proposal
#[utoipa::path(
    post,
    path = "/api/v1/translations/proposals/{id}/vote",
    tag = "Translations",
    summary = "Vote on translation proposal",
    request_body = VoteRequest,
    params(("id" = String, Path, description = "Proposal ID")),
    responses(
        (status = 200, description = "Vote recorded"),
        (status = 400, description = "Invalid vote or own proposal"),
        (status = 404, description = "Proposal not found"),
    )
)]
pub(crate) async fn vote_on_proposal(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<VoteRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    if req.vote != "up" && req.vote != "down" {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error("INVALID", "Vote must be 'up' or 'down'")),
        );
    }

    let state = state.read().await;

    let mut proposal = match state.db.get_translation_proposal(&id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Proposal not found")),
            )
        }
        Err(e) => {
            tracing::error!("DB error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Database error")),
            );
        }
    };

    if proposal.status != ProposalStatus::Pending {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "NOT_PENDING",
                "Can only vote on pending proposals",
            )),
        );
    }

    if proposal.proposed_by == auth_user.user_id {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "OWN_PROPOSAL",
                "Cannot vote on your own proposal",
            )),
        );
    }

    // Check for existing vote
    let existing = state
        .db
        .get_user_vote(proposal.id, auth_user.user_id)
        .await
        .unwrap_or(None);

    if let Some(existing_vote) = existing {
        if existing_vote.vote == req.vote {
            // Toggle off — remove vote
            if let Err(e) = state
                .db
                .delete_translation_vote(&existing_vote.id.to_string())
                .await
            {
                tracing::error!("Failed to delete vote: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("SERVER_ERROR", "Failed to update vote")),
                );
            }
            if req.vote == "up" {
                proposal.votes_for = (proposal.votes_for - 1).max(0);
            } else {
                proposal.votes_against = (proposal.votes_against - 1).max(0);
            }
        } else {
            // Switch vote: delete old, create new
            if let Err(e) = state
                .db
                .delete_translation_vote(&existing_vote.id.to_string())
                .await
            {
                tracing::error!("Failed to delete old vote: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("SERVER_ERROR", "Failed to update vote")),
                );
            }
            let new_vote = TranslationVote {
                id: Uuid::new_v4(),
                proposal_id: proposal.id,
                user_id: auth_user.user_id,
                vote: req.vote.clone(),
                created_at: Utc::now(),
            };
            if let Err(e) = state.db.save_translation_vote(&new_vote).await {
                tracing::error!("Failed to save vote: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error("SERVER_ERROR", "Failed to save vote")),
                );
            }
            if req.vote == "up" {
                proposal.votes_for += 1;
                proposal.votes_against = (proposal.votes_against - 1).max(0);
            } else {
                proposal.votes_for = (proposal.votes_for - 1).max(0);
                proposal.votes_against += 1;
            }
        }
    } else {
        // New vote
        let new_vote = TranslationVote {
            id: Uuid::new_v4(),
            proposal_id: proposal.id,
            user_id: auth_user.user_id,
            vote: req.vote.clone(),
            created_at: Utc::now(),
        };
        if let Err(e) = state.db.save_translation_vote(&new_vote).await {
            tracing::error!("Failed to save vote: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Failed to save vote")),
            );
        }
        if req.vote == "up" {
            proposal.votes_for += 1;
        } else {
            proposal.votes_against += 1;
        }
    }

    proposal.updated_at = Utc::now();
    if let Err(e) = state.db.save_translation_proposal(&proposal).await {
        tracing::error!("Failed to save proposal after vote: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error("SERVER_ERROR", "Failed to update proposal")),
        );
    }

    // Determine current user's vote after the operation
    let user_vote = state
        .db
        .get_user_vote(proposal.id, auth_user.user_id)
        .await
        .ok()
        .flatten()
        .map(|v| v.vote);

    let mut val = serde_json::to_value(&proposal).unwrap_or_default();
    if let Some(obj) = val.as_object_mut() {
        obj.insert(
            "user_vote".into(),
            user_vote
                .map_or(serde_json::Value::Null, serde_json::Value::String),
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(val)))
}

/// `PUT /api/v1/translations/proposals/{id}/review` — admin approve/reject
#[utoipa::path(
    put,
    path = "/api/v1/translations/proposals/{id}/review",
    tag = "Translations",
    summary = "Review translation proposal (admin)",
    request_body = ReviewRequest,
    params(("id" = String, Path, description = "Proposal ID")),
    responses(
        (status = 200, description = "Proposal reviewed"),
        (status = 400, description = "Already reviewed"),
        (status = 403, description = "Admin only"),
        (status = 404, description = "Not found"),
    )
)]
pub(crate) async fn review_proposal(
    State(state): State<SharedState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<String>,
    Json(req): Json<ReviewRequest>,
) -> (StatusCode, Json<ApiResponse<TranslationProposal>>) {
    let state = state.read().await;

    // Admin check
    if let Err((status, msg)) = check_admin(&state, &auth_user).await {
        return (
            status,
            Json(ApiResponse::error("FORBIDDEN", msg)),
        );
    }

    let new_status = match req.status.as_str() {
        "approved" => ProposalStatus::Approved,
        "rejected" => ProposalStatus::Rejected,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "INVALID",
                    "Status must be 'approved' or 'rejected'",
                )),
            )
        }
    };

    let mut proposal = match state.db.get_translation_proposal(&id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error("NOT_FOUND", "Proposal not found")),
            )
        }
        Err(e) => {
            tracing::error!("DB error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("SERVER_ERROR", "Database error")),
            );
        }
    };

    if proposal.status != ProposalStatus::Pending {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "NOT_PENDING",
                "Proposal already reviewed",
            )),
        );
    }

    // Get reviewer name
    let reviewer_name = match state.db.get_user(&auth_user.user_id.to_string()).await {
        Ok(Some(u)) => Some(u.name),
        _ => None,
    };

    proposal.status = new_status;
    proposal.reviewer_id = Some(auth_user.user_id);
    proposal.reviewer_name = reviewer_name;
    proposal.review_comment = req.comment;
    proposal.updated_at = Utc::now();

    if let Err(e) = state.db.save_translation_proposal(&proposal).await {
        tracing::error!("Failed to save reviewed proposal: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(
                "SERVER_ERROR",
                "Failed to save review",
            )),
        );
    }

    // If approved, create/update override
    if new_status == ProposalStatus::Approved {
        let ovr = TranslationOverride {
            language: proposal.language.clone(),
            key: proposal.key.clone(),
            value: proposal.proposed_value.clone(),
            updated_at: Utc::now(),
        };
        if let Err(e) = state.db.save_translation_override(&ovr).await {
            tracing::error!("Failed to save translation override: {}", e);
            // Don't fail the whole request — the proposal is already updated
        }
    }

    tracing::info!(
        "Admin {} reviewed proposal {} as {:?}",
        auth_user.user_id,
        proposal.id,
        proposal.status
    );

    (StatusCode::OK, Json(ApiResponse::success(proposal)))
}
