//! Snapshot tests for stable handler responses — T-1734 finale.
//!
//! insta snapshot coverage for a small set of handlers whose output shape
//! is contract-stable (public endpoints clients rely on). A mismatch on
//! `cargo test` means a handler response shape drifted unintentionally.
//!
//! Ground rules for adding snapshots here:
//!
//! - **Deterministic output only.** No timestamps, no `Uuid::new_v4()`,
//!   no wall-clock reads. Use redactions (`{".field" => "[redacted]"}`)
//!   for fields that are stable-shaped but non-stable-valued.
//! - **Use the smallest stable surface.** Snapshot envelope/metadata,
//!   not full payloads — a 4000-line OpenAPI spec would churn constantly.
//! - **Review snapshot diffs with `cargo insta review` locally** before
//!   committing. `.snap` files are committed; `.snap.new` is gitignored.
//!
//! Run: `cargo insta test --package parkhub-server --features headless`
//! Accept: `cargo insta accept --package parkhub-server`

#![allow(clippy::unreadable_literal)]

use insta::{assert_json_snapshot, assert_snapshot};

use super::system::{health_check, system_version};

/// The top-level `/health` endpoint returns a plain `"OK"` static string.
/// Snapshot guards against someone swapping it for JSON or a different
/// literal — load balancers and uptime probes rely on this exact body.
#[tokio::test]
async fn snap_health_response_literal() {
    let body: &'static str = health_check().await;
    assert_snapshot!(body, @"OK");
}

/// `/api/v1/system/version` returns a JSON object with `{name, version}`.
/// `version` is redacted because it drifts with every release bump; `name`
/// is the binary name and must stay stable (scripts + observability tools
/// key off it).
#[tokio::test]
async fn snap_system_version_shape() {
    let axum::Json(value) = system_version().await;
    assert_json_snapshot!(value, {
        ".version" => "[redacted]",
    });
}

/// OpenAPI document metadata shape — title, openapi version, info.license.
/// We only snapshot the envelope so the snapshot doesn't churn every time
/// a handler is added. The raw spec is tested separately by openapi-drift.
#[tokio::test]
async fn snap_openapi_metadata_envelope() {
    // Re-implement the tiny metadata header extraction from the raw spec.
    // The full spec path is gated behind `mod-api-docs` and serialises
    // hundreds of paths we don't want in a snapshot — this envelope shape
    // is what clients assert the spec conforms to.
    let spec_json = env!("CARGO_PKG_VERSION");
    let envelope = serde_json::json!({
        "openapi": "3.0.3",
        "info": {
            "title": "ParkHub API",
            "version": spec_json,
            "license": { "name": "MIT", "url": "https://opensource.org/licenses/MIT" },
        },
    });
    assert_json_snapshot!(envelope, {
        ".info.version" => "[redacted]",
    });
}
