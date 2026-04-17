//! Community-translation workflow: proposals, votes, and approved overrides.

use anyhow::Result;
use redb::{ReadableDatabase, ReadableTable};
use tracing::debug;
use uuid::Uuid;

use parkhub_common::models::{
    ProposalStatus, TranslationOverride, TranslationProposal, TranslationVote,
};

use super::{Database, TRANSLATION_OVERRIDES, TRANSLATION_PROPOSALS, TRANSLATION_VOTES};

impl Database {
    /// Save a translation proposal
    pub async fn save_translation_proposal(&self, proposal: &TranslationProposal) -> Result<()> {
        let id = proposal.id.to_string();
        let data = self.serialize(proposal)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(TRANSLATION_PROPOSALS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved translation proposal: {}", proposal.id);
        Ok(())
    }

    /// List translation proposals, optionally filtered by status
    pub async fn list_translation_proposals(
        &self,
        status_filter: Option<&ProposalStatus>,
    ) -> Result<Vec<TranslationProposal>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(TRANSLATION_PROPOSALS)?;

        let mut proposals = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let p: TranslationProposal = self.deserialize(value.value())?;
            if let Some(filter) = status_filter {
                if &p.status == filter {
                    proposals.push(p);
                }
            } else {
                proposals.push(p);
            }
        }
        proposals.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(proposals)
    }

    /// Get a single translation proposal by ID
    pub async fn get_translation_proposal(&self, id: &str) -> Result<Option<TranslationProposal>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(TRANSLATION_PROPOSALS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// Delete a translation proposal
    pub async fn delete_translation_proposal(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(TRANSLATION_PROPOSALS)?;
            let result = table.remove(id)?;
            result.is_some()
        };
        write_txn.commit()?;
        Ok(existed)
    }

    /// Save a translation vote
    pub async fn save_translation_vote(&self, vote: &TranslationVote) -> Result<()> {
        let id = vote.id.to_string();
        let data = self.serialize(vote)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(TRANSLATION_VOTES)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// List votes for a specific proposal
    pub async fn list_votes_for_proposal(&self, proposal_id: Uuid) -> Result<Vec<TranslationVote>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(TRANSLATION_VOTES)?;

        let mut votes = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let v: TranslationVote = self.deserialize(value.value())?;
            if v.proposal_id == proposal_id {
                votes.push(v);
            }
        }
        Ok(votes)
    }

    /// Get a user's vote on a specific proposal
    pub async fn get_user_vote(
        &self,
        proposal_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<TranslationVote>> {
        let votes = self.list_votes_for_proposal(proposal_id).await?;
        Ok(votes.into_iter().find(|v| v.user_id == user_id))
    }

    /// Delete a vote by ID
    pub async fn delete_translation_vote(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(TRANSLATION_VOTES)?;
            let result = table.remove(id)?;
            result.is_some()
        };
        write_txn.commit()?;
        Ok(existed)
    }

    /// Save a translation override (approved translation)
    pub async fn save_translation_override(&self, ovr: &TranslationOverride) -> Result<()> {
        // Key format: "language:key" for uniqueness
        let composite_key = format!("{}:{}", ovr.language, ovr.key);
        let data = self.serialize(ovr)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(TRANSLATION_OVERRIDES)?;
            table.insert(composite_key.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved translation override: {}:{}", ovr.language, ovr.key);
        Ok(())
    }

    /// List all translation overrides
    pub async fn list_translation_overrides(&self) -> Result<Vec<TranslationOverride>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(TRANSLATION_OVERRIDES)?;

        let mut overrides = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            overrides.push(self.deserialize(value.value())?);
        }
        Ok(overrides)
    }
}
