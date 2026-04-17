//! User CRUD, username/email secondary indexes, and GDPR anonymization.

use anyhow::Result;
use redb::{ReadableDatabase, ReadableTable, ReadableTableMetadata};
use tracing::{debug, info};
use uuid::Uuid;

use parkhub_common::models::User;

use super::{Database, USERS, USERS_BY_EMAIL, USERS_BY_USERNAME, pagination_offset};

impl Database {
    /// Save a user to the database
    pub async fn save_user(&self, user: &User) -> Result<()> {
        let id = user.id.to_string();
        let data = self.serialize(user)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(USERS)?;
            table.insert(id.as_str(), data.as_slice())?;

            // Update username index
            let mut idx = write_txn.open_table(USERS_BY_USERNAME)?;
            idx.insert(user.username.as_str(), id.as_str())?;

            // Update email index
            let mut email_idx = write_txn.open_table(USERS_BY_EMAIL)?;
            email_idx.insert(user.email.as_str(), id.as_str())?;
        }
        write_txn.commit()?;
        debug!("Saved user: {} ({})", user.username, user.id);
        Ok(())
    }

    /// Get a user by ID (string)
    pub async fn get_user(&self, id: &str) -> Result<Option<User>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(USERS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// Get a user by username
    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);

        // Look up user ID from username index
        let idx = read_txn.open_table(USERS_BY_USERNAME)?;
        let user_id = match idx.get(username)? {
            Some(id) => id.value().to_string(),
            None => return Ok(None),
        };

        // Get user data
        let table = read_txn.open_table(USERS)?;
        match table.get(user_id.as_str())? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// Get a user by email
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);

        // Look up user ID from email index
        let idx = read_txn.open_table(USERS_BY_EMAIL)?;
        let user_id = match idx.get(email)? {
            Some(id) => id.value().to_string(),
            None => return Ok(None),
        };

        // Get user data
        let table = read_txn.open_table(USERS)?;
        match table.get(user_id.as_str())? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// List all users
    pub async fn list_users(&self) -> Result<Vec<User>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(USERS)?;

        let mut users = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            users.push(self.deserialize(value.value())?);
        }
        Ok(users)
    }

    /// List users with pagination. Returns (page_items, total_count).
    pub async fn list_users_paginated(
        &self,
        page: i32,
        per_page: i32,
    ) -> Result<(Vec<User>, usize)> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(USERS)?;

        let total = table.len()? as usize;
        let (skip, per_page) = pagination_offset(page, per_page);

        let mut users = Vec::with_capacity(per_page.min(total.saturating_sub(skip)));
        for entry in table.iter()?.skip(skip).take(per_page) {
            let (_, value) = entry?;
            users.push(self.deserialize(value.value())?);
        }
        Ok((users, total))
    }

    /// Delete a user
    pub async fn delete_user(&self, id: &str) -> Result<bool> {
        // First get the user to find the username/email
        let Some(user) = self.get_user(id).await? else {
            return Ok(false);
        };

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(USERS)?;
            table.remove(id)?;

            let mut idx = write_txn.open_table(USERS_BY_USERNAME)?;
            idx.remove(user.username.as_str())?;

            let mut email_idx = write_txn.open_table(USERS_BY_EMAIL)?;
            email_idx.remove(user.email.as_str())?;
        }
        write_txn.commit()?;
        debug!("Deleted user: {}", id);
        Ok(true)
    }

    /// GDPR Art. 17 — Anonymize a user: scrub PII while keeping booking records.
    /// Atomically replaces user's name/email/username/password with placeholder values,
    /// removes old index entries, and deletes all linked vehicle records.
    pub async fn anonymize_user(&self, user_id: &str) -> Result<bool> {
        let Some(user) = self.get_user(user_id).await? else {
            return Ok(false);
        };

        let old_username = user.username.clone();
        let old_email = user.email.clone();
        let anon_id = format!("deleted-{}", Uuid::new_v4());
        let anon_email = format!("{anon_id}@deleted.invalid");
        let anon_password = format!("DELETED_{}", Uuid::new_v4());

        // Anonymize user record + clean indexes atomically
        let mut anon_user = user;
        anon_user.name = "[Deleted User]".to_string();
        anon_user.email = anon_email.clone();
        anon_user.username = anon_id.clone();
        anon_user.password_hash = anon_password;

        let user_data = self.serialize(&anon_user)?;
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            // Overwrite user record
            let mut table = write_txn.open_table(USERS)?;
            table.insert(user_id, user_data.as_slice())?;

            // Remove stale index entries and add anonymized ones
            let mut idx = write_txn.open_table(USERS_BY_USERNAME)?;
            let _ = idx.remove(old_username.as_str());
            idx.insert(anon_id.as_str(), user_id)?;

            let mut email_idx = write_txn.open_table(USERS_BY_EMAIL)?;
            let _ = email_idx.remove(old_email.as_str());
            email_idx.insert(anon_email.as_str(), user_id)?;
        }
        write_txn.commit()?;

        // Delete all vehicles (personal data — can be deleted per GDPR Art. 17)
        let vehicles = self
            .list_vehicles_by_user(user_id)
            .await
            .unwrap_or_default();
        for vehicle in vehicles {
            if let Err(e) = self.delete_vehicle(&vehicle.id.to_string()).await {
                tracing::warn!("GDPR: failed to delete vehicle {}: {e}", vehicle.id);
            }
        }

        // Scrub license plate from bookings (keep records for accounting, strip PII)
        let bookings = self
            .list_bookings_by_user(user_id)
            .await
            .unwrap_or_default();
        for mut booking in bookings {
            booking.vehicle.license_plate = "[DELETED]".to_string();
            if let Err(e) = self.save_booking(&booking).await {
                tracing::warn!("GDPR: failed to scrub booking {}: {e}", booking.id);
            }
        }

        info!(
            "GDPR anonymization completed for user: {} → {}",
            user_id, anon_id
        );
        Ok(true)
    }
}
