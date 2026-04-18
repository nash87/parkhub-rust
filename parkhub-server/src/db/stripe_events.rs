//! Stripe webhook event idempotency log.
//!
//! Stripe retries webhook deliveries on any non-2xx response — and occasionally
//! even on 2xx responses during network blips. Processing the same event twice
//! would double-credit the user. This module records each processed event id
//! so retries short-circuit before mutating state.
//!
//! The write (insert) happens inside a single redb write transaction: either
//! the event was already present (returns `false`, caller short-circuits) or
//! it is newly inserted (returns `true`, caller grants credits). redb
//! serialises write transactions globally, so two concurrent webhooks for the
//! same event id will see exactly one `true` return.

use anyhow::Result;
use redb::ReadableTable;

use super::{Database, STRIPE_EVENTS};

impl Database {
    /// Atomically record a Stripe webhook event by its Stripe event id.
    ///
    /// Returns `Ok(true)` if the id was not previously recorded — caller may
    /// proceed with the credit-grant mutation. Returns `Ok(false)` if the id
    /// was already in the table — caller must short-circuit (retry / duplicate
    /// delivery).
    ///
    /// The check + insert happens in one redb write transaction, so a
    /// duplicate delivery racing with the original processing cannot both
    /// observe "new" and both grant credits.
    pub async fn record_stripe_event(&self, event_id: &str, event_type: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);

        let inserted = {
            let mut table = write_txn.open_table(STRIPE_EVENTS)?;
            if table.get(event_id)?.is_some() {
                false
            } else {
                table.insert(event_id, event_type)?;
                true
            }
        };
        write_txn.commit()?;

        Ok(inserted)
    }

    /// Test helper: check whether an event id has been recorded.
    #[cfg(test)]
    pub async fn stripe_event_recorded(&self, event_id: &str) -> Result<bool> {
        use redb::ReadableDatabase;

        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(STRIPE_EVENTS)?;
        Ok(table.get(event_id)?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use crate::db::{Database, DatabaseConfig};
    use tempfile::tempdir;

    fn test_db() -> (tempfile::TempDir, Database) {
        let dir = tempdir().expect("tempdir");
        let config = DatabaseConfig {
            path: dir.path().to_path_buf(),
            encryption_enabled: false,
            passphrase: None,
            create_if_missing: true,
        };
        let db = Database::open(&config).expect("open db");
        (dir, db)
    }

    #[tokio::test]
    async fn test_record_stripe_event_returns_true_for_new() {
        let (_dir, db) = test_db();
        let first = db
            .record_stripe_event("evt_new_1", "checkout.session.completed")
            .await
            .unwrap();
        assert!(first, "first insert must return true");
        assert!(db.stripe_event_recorded("evt_new_1").await.unwrap());
    }

    #[tokio::test]
    async fn test_record_stripe_event_returns_false_for_duplicate() {
        let (_dir, db) = test_db();

        let first = db
            .record_stripe_event("evt_dup_1", "checkout.session.completed")
            .await
            .unwrap();
        assert!(first);

        let second = db
            .record_stripe_event("evt_dup_1", "checkout.session.completed")
            .await
            .unwrap();
        assert!(!second, "duplicate insert must return false");

        // Still present exactly once.
        assert!(db.stripe_event_recorded("evt_dup_1").await.unwrap());
    }

    #[tokio::test]
    async fn test_record_stripe_event_different_ids_all_new() {
        let (_dir, db) = test_db();
        for i in 0..5 {
            let id = format!("evt_multi_{i}");
            let inserted = db
                .record_stripe_event(&id, "checkout.session.completed")
                .await
                .unwrap();
            assert!(inserted, "id {id} must be new");
        }
    }
}
