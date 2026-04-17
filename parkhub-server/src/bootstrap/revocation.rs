//! JWT revocation store wiring for [`crate::AppState`].
//!
//! Picks a Redis-backed store when the `redis-revocation` feature is
//! compiled in and `PARKHUB_REDIS_URL` is set; falls back to the
//! in-memory default for single-replica deployments otherwise.

use std::sync::Arc;
use tracing::info;

use crate::jwt::TokenRevocationList;

/// Build the JWT revocation store used by [`crate::AppState`].
///
/// * With `--features redis-revocation` AND `PARKHUB_REDIS_URL` set → returns a
///   Redis-backed store shared across replicas.
/// * With `--features redis-revocation` AND `PARKHUB_REDIS_URL` **unset** →
///   panics per the Cargo.toml contract: a misconfigured production deploy
///   must fail loudly at startup rather than silently fall back to
///   process-local state that won't propagate logouts.
/// * Without the feature → returns the in-memory default (single-replica OK).
// `async` is unused when the `redis-revocation` feature is off, but we keep
// the signature stable so the call site in `main` stays uniform across
// feature matrices.
#[allow(clippy::unused_async)]
pub(crate) async fn build_revocation_store() -> Arc<TokenRevocationList> {
    #[cfg(feature = "redis-revocation")]
    {
        // `from_env` enforces the PARKHUB_REDIS_URL requirement by panicking
        // with a clear message when the feature is on but the env var is unset.
        let store = crate::jwt::RedisRevocationList::from_env().await;
        info!("JWT revocation store: Redis (shared across replicas)");
        TokenRevocationList::from_store(store)
    }
    #[cfg(not(feature = "redis-revocation"))]
    {
        info!("JWT revocation store: in-memory (single-replica)");
        TokenRevocationList::new()
    }
}
