//! Session storage: access-token keyed session records with refresh-token
//! lookup, per-user deletion, and expiry semantics.

use anyhow::Result;
use chrono::{DateTime, Utc};
use redb::{ReadableDatabase, ReadableTable};
use serde::{Deserialize, Serialize};
use tracing::debug;
use uuid::Uuid;

use super::{Database, SESSIONS};

/// User session for authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub user_id: Uuid,
    pub username: String,
    pub role: String,
    pub refresh_token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl Session {
    /// Create a new session with the given duration in hours.
    ///
    /// `username` and `role` are stored for audit/logging purposes.
    pub fn new(user_id: Uuid, duration_hours: i64, username: &str, role: &str) -> Self {
        let now = Utc::now();
        // Use cryptographically random refresh token (not a UUID — UUIDs have
        // a fixed structure that reduces effective entropy).
        let mut rng_bytes = [0u8; 32];
        rand::Rng::fill_bytes(&mut rand::rng(), &mut rng_bytes);
        let refresh_token = format!("rt_{}", hex::encode(rng_bytes));
        Self {
            user_id,
            username: username.to_string(),
            role: role.to_string(),
            refresh_token,
            created_at: now,
            expires_at: now + chrono::Duration::hours(duration_hours),
        }
    }

    /// Check if the session has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }
}

impl Database {
    /// Save a session (access token -> session data)
    pub async fn save_session(&self, token: &str, session: &Session) -> Result<()> {
        let data = self.serialize(session)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(SESSIONS)?;
            table.insert(token, data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved session for user: {}", session.username);
        Ok(())
    }

    /// Get a session by token
    pub async fn get_session(&self, token: &str) -> Result<Option<Session>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(SESSIONS)?;

        match table.get(token)? {
            Some(value) => {
                let session: Session = self.deserialize(value.value())?;
                // Check if expired
                if session.expires_at < Utc::now() {
                    Ok(None)
                } else {
                    Ok(Some(session))
                }
            }
            None => Ok(None),
        }
    }

    /// Find a session by its refresh token (scans all sessions)
    ///
    /// Returns a tuple of (`access_token`, session) if found and not expired.
    pub async fn get_session_by_refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<Option<(String, Session)>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(SESSIONS)?;

        for entry in table.iter()? {
            let (key, value) = entry?;
            let access_token = key.value().to_string();
            let session: Session = self.deserialize(value.value())?;
            if session.refresh_token == refresh_token {
                if session.is_expired() {
                    return Ok(None);
                }
                return Ok(Some((access_token, session)));
            }
        }
        Ok(None)
    }

    /// Delete all sessions belonging to a specific user.
    ///
    /// Scans every session, deserializes it, and removes entries whose
    /// `user_id` matches the given ID. Returns the number of deleted sessions.
    pub async fn delete_sessions_by_user(&self, user_id: Uuid) -> Result<u64> {
        let db = self.inner.write().await;
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(SESSIONS)?;

        // Collect tokens to delete (cannot mutate while iterating)
        let mut tokens_to_delete = Vec::new();
        for entry in table.iter()? {
            let (key, value) = entry?;
            let session: Session = self.deserialize(value.value())?;
            if session.user_id == user_id {
                tokens_to_delete.push(key.value().to_string());
            }
        }
        drop(table);
        drop(read_txn);

        let count = tokens_to_delete.len() as u64;
        if count > 0 {
            let write_txn = db.begin_write()?;
            drop(db);
            {
                let mut table = write_txn.open_table(SESSIONS)?;
                for token in &tokens_to_delete {
                    table.remove(token.as_str())?;
                }
            }
            write_txn.commit()?;
            debug!("Deleted {} session(s) for user {}", count, user_id);
        }
        Ok(count)
    }

    /// List all active (non-expired) sessions for a user.
    /// Returns `(access_token, Session)` pairs.
    pub async fn list_sessions_by_user(&self, user_id: Uuid) -> Result<Vec<(String, Session)>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(SESSIONS)?;
        let now = Utc::now();

        let mut sessions = Vec::new();
        for entry in table.iter()? {
            let (key, value) = entry?;
            let session: Session = self.deserialize(value.value())?;
            if session.user_id == user_id && session.expires_at > now {
                sessions.push((key.value().to_string(), session));
            }
        }
        Ok(sessions)
    }

    /// Delete a session
    pub async fn delete_session(&self, token: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(SESSIONS)?;
            let result = table.remove(token)?;
            result.is_some()
        };
        write_txn.commit()?;
        Ok(existed)
    }
}
