//! Sequential invoice number counters (fortlaufende Rechnungsnummern).
//!
//! Implements German § 14 UStG compliance: invoice numbers must be
//! fortlaufend — strictly ascending with no gaps, per tax subject. Per-year
//! reset (YYYY-0000001, YYYY-0000002, …) is an accepted pattern because the
//! numbers within each series remain monotonic and unambiguous.
//!
//! The counter lives in the shared `SETTINGS` table under
//! `invoice_counter_{year}`. Read + increment + write happen inside a single
//! redb write transaction, which redb serialises globally — no two concurrent
//! callers can observe the same counter value, so gap-free monotonic allocation
//! is guaranteed even under parallel invoice generation.

use anyhow::{Context, Result};
use redb::ReadableTable;

use super::{Database, SETTINGS};

/// Format an invoice number from a year and sequence. Public so tests and
/// callers can round-trip the format without hard-coding the string.
#[must_use]
pub fn format_invoice_number(year: i32, seq: u64) -> String {
    format!("{year}-{seq:07}")
}

/// Settings-table key for the per-year monotonic counter.
fn counter_key(year: i32) -> String {
    format!("invoice_counter_{year}")
}

/// Settings-table key for a booking's assigned invoice number.
fn assigned_key(booking_id: &str) -> String {
    format!("invoice_number_{booking_id}")
}

impl Database {
    /// Atomically allocate the next sequential invoice number for `year`.
    ///
    /// Opens a single redb write transaction: reads the current counter,
    /// increments it, writes the new value, commits. redb serialises write
    /// transactions, so concurrent callers see strictly monotonic values with
    /// no gaps — required for § 14 UStG compliance.
    ///
    /// Counter is stored in `SETTINGS` under key `invoice_counter_{year}`.
    /// The first invoice of a year returns `{year}-0000001`.
    pub async fn next_invoice_number(&self, year: i32) -> Result<String> {
        let key = counter_key(year);

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);

        let next: u64 = {
            let mut table = write_txn.open_table(SETTINGS)?;
            let current: u64 = match table.get(key.as_str())? {
                Some(v) => v
                    .value()
                    .parse()
                    .context("stored invoice counter is not a valid u64")?,
                None => 0,
            };
            let next = current
                .checked_add(1)
                .context("invoice counter overflow (u64)")?;
            table.insert(key.as_str(), next.to_string().as_str())?;
            next
        };
        write_txn.commit()?;

        Ok(format_invoice_number(year, next))
    }

    /// Return the invoice number assigned to `booking_id`, allocating a new
    /// one from the `year` counter on first call. Subsequent calls return the
    /// same stored number — re-downloading a PDF must not burn a new counter
    /// value (that would leave gaps in the fortlaufende series).
    ///
    /// Both the presence check and the counter increment happen inside a
    /// single redb write transaction, so concurrent first-time requests for
    /// the same booking allocate exactly once.
    pub async fn get_or_assign_invoice_number(
        &self,
        booking_id: &str,
        year: i32,
    ) -> Result<String> {
        let assigned = assigned_key(booking_id);
        let counter = counter_key(year);

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);

        let number = {
            let mut table = write_txn.open_table(SETTINGS)?;

            // Already assigned? Return stored value.
            if let Some(existing) = table.get(assigned.as_str())? {
                existing.value().to_string()
            } else {
                // Allocate next counter value and persist the assignment
                // together in the same transaction.
                let current: u64 = match table.get(counter.as_str())? {
                    Some(v) => v
                        .value()
                        .parse()
                        .context("stored invoice counter is not a valid u64")?,
                    None => 0,
                };
                let next = current
                    .checked_add(1)
                    .context("invoice counter overflow (u64)")?;
                table.insert(counter.as_str(), next.to_string().as_str())?;
                let number = format_invoice_number(year, next);
                table.insert(assigned.as_str(), number.as_str())?;
                number
            }
        };
        write_txn.commit()?;

        Ok(number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Database, DatabaseConfig};
    use std::sync::Arc;
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

    #[test]
    fn test_invoice_number_format() {
        assert_eq!(format_invoice_number(2026, 1), "2026-0000001");
        assert_eq!(format_invoice_number(2026, 42), "2026-0000042");
        assert_eq!(format_invoice_number(2026, 9_999_999), "2026-9999999");
        assert_eq!(format_invoice_number(2026, 10_000_000), "2026-10000000");
    }

    #[tokio::test]
    async fn test_next_invoice_number_is_sequential() {
        let (_dir, db) = test_db();
        let mut numbers = Vec::new();
        for _ in 0..5 {
            numbers.push(db.next_invoice_number(2026).await.unwrap());
        }
        assert_eq!(
            numbers,
            vec![
                "2026-0000001".to_string(),
                "2026-0000002".to_string(),
                "2026-0000003".to_string(),
                "2026-0000004".to_string(),
                "2026-0000005".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn test_next_invoice_number_year_reset() {
        let (_dir, db) = test_db();
        assert_eq!(db.next_invoice_number(2025).await.unwrap(), "2025-0000001");
        assert_eq!(db.next_invoice_number(2025).await.unwrap(), "2025-0000002");
        // Crossing into a new year resets the counter — each year forms an
        // independent fortlaufende series.
        assert_eq!(db.next_invoice_number(2026).await.unwrap(), "2026-0000001");
        assert_eq!(db.next_invoice_number(2025).await.unwrap(), "2025-0000003");
    }

    #[tokio::test]
    async fn test_get_or_assign_is_stable_per_booking() {
        let (_dir, db) = test_db();
        let first = db
            .get_or_assign_invoice_number("booking-abc", 2026)
            .await
            .unwrap();
        assert_eq!(first, "2026-0000001");

        // Same booking id -> same number (re-download must not burn a new one).
        let second = db
            .get_or_assign_invoice_number("booking-abc", 2026)
            .await
            .unwrap();
        assert_eq!(second, first);

        // Different booking id -> next in sequence, no gap.
        let other = db
            .get_or_assign_invoice_number("booking-xyz", 2026)
            .await
            .unwrap();
        assert_eq!(other, "2026-0000002");
    }

    #[tokio::test]
    async fn test_invoice_number_concurrent_writes_no_gap() {
        let (_dir, db) = test_db();
        let db = Arc::new(db);

        // Spawn 10 tasks that each allocate one number concurrently.
        let mut handles = Vec::new();
        for _ in 0..10 {
            let db = db.clone();
            handles.push(tokio::spawn(async move {
                db.next_invoice_number(2026).await.unwrap()
            }));
        }

        let mut results: Vec<String> = Vec::new();
        for h in handles {
            results.push(h.await.unwrap());
        }

        // Extract sequence numbers.
        let mut seqs: Vec<u64> = results
            .iter()
            .map(|s| {
                s.strip_prefix("2026-")
                    .expect("year prefix")
                    .parse()
                    .expect("numeric seq")
            })
            .collect();
        seqs.sort_unstable();

        // Exactly 10 unique, gap-free, starting at 1.
        assert_eq!(seqs, (1u64..=10).collect::<Vec<_>>());
    }
}
