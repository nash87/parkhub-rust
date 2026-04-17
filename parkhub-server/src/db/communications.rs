//! Cross-cutting user communications: announcements, notifications, push
//! subscriptions, webhooks, and credit transactions.

use anyhow::Result;
use redb::{ReadableDatabase, ReadableTable};
use tracing::debug;
use uuid::Uuid;

use parkhub_common::models::{Announcement, Notification};

use super::{
    ANNOUNCEMENTS, CREDIT_TRANSACTIONS, Database, NOTIFICATIONS, PUSH_SUBSCRIPTIONS,
    PushSubscription, WEBHOOKS, Webhook,
};

impl Database {
    // ── Announcements ──

    /// Save an announcement
    pub async fn save_announcement(&self, ann: &Announcement) -> Result<()> {
        let id = ann.id.to_string();
        let data = self.serialize(ann)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(ANNOUNCEMENTS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved announcement: {}", ann.id);
        Ok(())
    }

    /// List all announcements
    pub async fn list_announcements(&self) -> Result<Vec<Announcement>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(ANNOUNCEMENTS)?;

        let mut announcements = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            announcements.push(self.deserialize(value.value())?);
        }
        Ok(announcements)
    }

    /// Delete an announcement
    pub async fn delete_announcement(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(ANNOUNCEMENTS)?;
            let result = table.remove(id)?;
            result.is_some()
        };
        write_txn.commit()?;
        if existed {
            debug!("Deleted announcement: {}", id);
        }
        Ok(existed)
    }

    // ── Notifications ──

    /// Save a notification
    pub async fn save_notification(&self, notification: &Notification) -> Result<()> {
        let id = notification.id.to_string();
        let data = self.serialize(notification)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(NOTIFICATIONS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved notification: {}", notification.id);
        Ok(())
    }

    /// List notifications for a user
    pub async fn list_notifications_by_user(&self, user_id: &str) -> Result<Vec<Notification>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(NOTIFICATIONS)?;

        let mut notifications = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let notification: Notification = self.deserialize(value.value())?;
            if notification.user_id.to_string() == user_id {
                notifications.push(notification);
            }
        }
        Ok(notifications)
    }

    /// Mark a notification as read
    pub async fn mark_notification_read(&self, id: &str) -> Result<bool> {
        let Some(mut notification) = self.get_notification(id).await? else {
            return Ok(false);
        };

        notification.read = true;
        self.save_notification(&notification).await?;
        Ok(true)
    }

    /// Get a notification by ID (helper for `mark_notification_read`)
    async fn get_notification(&self, id: &str) -> Result<Option<Notification>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(NOTIFICATIONS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// Delete a notification by ID
    // Write lock intentionally spans the mutating write txn — critical section.
    #[allow(clippy::significant_drop_tightening)]
    pub async fn delete_notification(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(NOTIFICATIONS)?;
            if table.get(id)?.is_none() {
                return Ok(false);
            }
            table.remove(id)?;
        }
        write_txn.commit()?;
        Ok(true)
    }

    // ── Credit Transactions ──

    pub async fn save_credit_transaction(
        &self,
        tx: &parkhub_common::models::CreditTransaction,
    ) -> Result<()> {
        let data = self.serialize(tx)?;
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(CREDIT_TRANSACTIONS)?;
            table.insert(tx.id.to_string().as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub async fn list_credit_transactions_for_user(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<Vec<parkhub_common::models::CreditTransaction>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(CREDIT_TRANSACTIONS)?;
        let mut transactions = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let tx: parkhub_common::models::CreditTransaction = self.deserialize(value.value())?;
            if tx.user_id == user_id {
                transactions.push(tx);
            }
        }
        transactions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(transactions)
    }

    /// List all credit transactions across all users, with optional filters.
    pub async fn list_all_credit_transactions(
        &self,
        user_id_filter: Option<uuid::Uuid>,
        type_filter: Option<parkhub_common::models::CreditTransactionType>,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<parkhub_common::models::CreditTransaction>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(CREDIT_TRANSACTIONS)?;
        let mut transactions = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let tx: parkhub_common::models::CreditTransaction = self.deserialize(value.value())?;
            if let Some(uid) = user_id_filter
                && tx.user_id != uid
            {
                continue;
            }
            if let Some(ref t) = type_filter
                && &tx.transaction_type != t
            {
                continue;
            }
            if let Some(f) = from
                && tx.created_at < f
            {
                continue;
            }
            if let Some(t) = to
                && tx.created_at > t
            {
                continue;
            }
            transactions.push(tx);
        }
        transactions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(transactions)
    }

    // ── Webhooks ──

    /// Save a webhook (insert or update)
    pub async fn save_webhook(&self, webhook: &Webhook) -> Result<()> {
        let id = webhook.id.to_string();
        let data = self.serialize(webhook)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(WEBHOOKS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved webhook: {}", webhook.id);
        Ok(())
    }

    /// Get a webhook by ID
    pub async fn get_webhook(&self, id: &str) -> Result<Option<Webhook>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(WEBHOOKS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// List all webhooks
    pub async fn list_webhooks(&self) -> Result<Vec<Webhook>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(WEBHOOKS)?;

        let mut webhooks = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            webhooks.push(self.deserialize(value.value())?);
        }
        Ok(webhooks)
    }

    /// Delete a webhook by ID
    pub async fn delete_webhook(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(WEBHOOKS)?;
            let result = table.remove(id)?;
            result.is_some()
        };
        write_txn.commit()?;
        if existed {
            debug!("Deleted webhook: {}", id);
        }
        Ok(existed)
    }

    // ── Push Subscriptions ──

    /// Save a push subscription (upsert by id)
    pub async fn save_push_subscription(&self, sub: &PushSubscription) -> Result<()> {
        let id = sub.id.to_string();
        let data = self.serialize(sub)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(PUSH_SUBSCRIPTIONS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!(
            "Saved push subscription {} for user {}",
            sub.id, sub.user_id
        );
        Ok(())
    }

    /// Get all push subscriptions for a given user
    pub async fn get_push_subscriptions_by_user(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<PushSubscription>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(PUSH_SUBSCRIPTIONS)?;

        let mut subs = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let sub: PushSubscription = self.deserialize(value.value())?;
            if sub.user_id == *user_id {
                subs.push(sub);
            }
        }
        Ok(subs)
    }

    /// Delete all push subscriptions for a given user
    pub async fn delete_push_subscriptions_by_user(&self, user_id: &Uuid) -> Result<u64> {
        // First, collect IDs to delete
        let ids: Vec<String> = self
            .get_push_subscriptions_by_user(user_id)
            .await?
            .iter()
            .map(|s| s.id.to_string())
            .collect();

        let count = ids.len() as u64;
        if count == 0 {
            return Ok(0);
        }

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(PUSH_SUBSCRIPTIONS)?;
            for id in &ids {
                table.remove(id.as_str())?;
            }
        }
        write_txn.commit()?;
        debug!(
            "Deleted {} push subscription(s) for user {}",
            count, user_id
        );
        Ok(count)
    }

    /// List all push subscriptions (admin use / delivery fan-out)
    pub async fn list_all_push_subscriptions(&self) -> Result<Vec<PushSubscription>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(PUSH_SUBSCRIPTIONS)?;

        let mut subs = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            subs.push(self.deserialize(value.value())?);
        }
        Ok(subs)
    }
}
