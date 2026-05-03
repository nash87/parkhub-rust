//! Booking CRUD with user secondary index, plus guest bookings, swap requests,
//! recurring bookings, and waitlist persistence.

use anyhow::Result;
use chrono::NaiveDate;
use redb::{ReadableDatabase, ReadableTable, ReadableTableMetadata};
use tracing::debug;

use parkhub_common::models::{
    Booking, BookingStatus, GuestBooking, RecurringBooking, SwapRequest, WaitlistEntry,
};

use super::{
    BOOKINGS, BOOKINGS_BY_USER, Database, GUEST_BOOKINGS, RECURRING_BOOKINGS, SWAP_REQUESTS,
    WAITLIST, pagination_offset,
};

impl Database {
    // ── Booking CRUD ──

    /// Save a booking
    pub async fn save_booking(&self, booking: &Booking) -> Result<()> {
        let id = booking.id.to_string();
        let user_id = booking.user_id.to_string();
        let data = self.serialize(booking)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(BOOKINGS)?;
            table.insert(id.as_str(), data.as_slice())?;

            // Maintain user → booking secondary index
            let mut idx = write_txn.open_table(BOOKINGS_BY_USER)?;
            let idx_key = format!("{user_id}:{id}");
            idx.insert(idx_key.as_str(), id.as_str())?;
        }
        write_txn.commit()?;
        debug!("Saved booking: {}", booking.id);
        Ok(())
    }

    /// Get a booking by ID (string)
    pub async fn get_booking(&self, id: &str) -> Result<Option<Booking>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(BOOKINGS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// List all bookings
    pub async fn list_bookings(&self) -> Result<Vec<Booking>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(BOOKINGS)?;

        let mut bookings = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            bookings.push(self.deserialize(value.value())?);
        }
        Ok(bookings)
    }

    /// List bookings with pagination. Returns (`page_items`, `total_count`).
    pub async fn list_bookings_paginated(
        &self,
        page: i32,
        per_page: i32,
    ) -> Result<(Vec<Booking>, usize)> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(BOOKINGS)?;

        let total = table.len()? as usize;
        let (skip, per_page) = pagination_offset(page, per_page);

        let mut bookings = Vec::with_capacity(per_page.min(total.saturating_sub(skip)));
        for entry in table.iter()?.skip(skip).take(per_page) {
            let (_, value) = entry?;
            bookings.push(self.deserialize(value.value())?);
        }
        Ok((bookings, total))
    }

    /// Get bookings for a user using the `BOOKINGS_BY_USER` secondary index.
    ///
    /// O(k) where k = number of bookings for this user, instead of O(n) over
    /// all bookings.
    pub async fn list_bookings_by_user(&self, user_id: &str) -> Result<Vec<Booking>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);

        let idx = read_txn.open_table(BOOKINGS_BY_USER)?;
        let bookings_table = read_txn.open_table(BOOKINGS)?;

        let prefix = format!("{user_id}:");
        let mut bookings = Vec::new();

        for entry in idx.iter()? {
            let (key, booking_id_val) = entry?;
            if !key.value().starts_with(&prefix) {
                continue;
            }
            let booking_id = booking_id_val.value();
            if let Some(data) = bookings_table.get(booking_id)? {
                bookings.push(self.deserialize(data.value())?);
            }
        }
        Ok(bookings)
    }

    /// Count non-cancelled bookings for a user on a specific calendar day.
    /// Uses the canonical BOOKINGS table so policy enforcement does not rely on
    /// secondary-index freshness.
    pub async fn count_bookings_for_user_on_day(
        &self,
        user_id: &str,
        booking_date: NaiveDate,
    ) -> Result<usize> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);

        let table = read_txn.open_table(BOOKINGS)?;
        let mut count = 0usize;

        for entry in table.iter()? {
            let (_key, value) = entry?;
            let booking: Booking = self.deserialize(value.value())?;
            if booking.user_id.to_string() == user_id
                && booking.start_time.date_naive() == booking_date
                && booking.status != BookingStatus::Cancelled
            {
                count += 1;
            }
        }

        Ok(count)
    }

    /// Delete a booking
    pub async fn delete_booking(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;

        // Read pass: find the user_id to remove the secondary-index entry
        let user_id_opt: Option<String> = {
            let read_txn = db.begin_read()?;
            let table = read_txn.open_table(BOOKINGS)?;
            match table.get(id)? {
                Some(value) => {
                    let booking: Booking = self.deserialize(value.value())?;
                    Some(booking.user_id.to_string())
                }
                None => None,
            }
        };

        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(BOOKINGS)?;
            let result = table.remove(id)?;
            // Remove secondary index entry if booking was found
            if result.is_some()
                && let Some(ref uid) = user_id_opt
            {
                let mut idx = write_txn.open_table(BOOKINGS_BY_USER)?;
                let idx_key = format!("{uid}:{id}");
                idx.remove(idx_key.as_str())?;
            }
            result.is_some()
        };
        write_txn.commit()?;
        if existed {
            debug!("Deleted booking: {}", id);
        }
        Ok(existed)
    }

    // ── Waitlist CRUD ──

    /// Save a waitlist entry
    pub async fn save_waitlist_entry(&self, entry: &WaitlistEntry) -> Result<()> {
        let id = entry.id.to_string();
        let data = self.serialize(entry)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(WAITLIST)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved waitlist entry: {}", entry.id);
        Ok(())
    }

    /// Get a waitlist entry by ID
    pub async fn get_waitlist_entry(&self, id: &str) -> Result<Option<WaitlistEntry>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(WAITLIST)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// List waitlist entries for a user
    pub async fn list_waitlist_by_user(&self, user_id: &str) -> Result<Vec<WaitlistEntry>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(WAITLIST)?;

        let mut entries = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let waitlist_entry: WaitlistEntry = self.deserialize(value.value())?;
            if waitlist_entry.user_id.to_string() == user_id {
                entries.push(waitlist_entry);
            }
        }
        Ok(entries)
    }

    /// List all waitlist entries for a specific parking lot, ordered by creation time.
    pub async fn list_waitlist_by_lot(&self, lot_id: &str) -> Result<Vec<WaitlistEntry>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(WAITLIST)?;

        let mut entries = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let waitlist_entry: WaitlistEntry = self.deserialize(value.value())?;
            if waitlist_entry.lot_id.to_string() == lot_id {
                entries.push(waitlist_entry);
            }
        }
        // Sort by created_at so earlier waitlist entries are notified first
        entries.sort_by_key(|e| e.created_at);
        Ok(entries)
    }

    /// Delete a waitlist entry
    pub async fn delete_waitlist_entry(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(WAITLIST)?;
            let result = table.remove(id)?;
            result.is_some()
        };
        write_txn.commit()?;
        if existed {
            debug!("Deleted waitlist entry: {}", id);
        }
        Ok(existed)
    }

    // ── Guest Booking CRUD ──

    /// Save a guest booking
    pub async fn save_guest_booking(&self, booking: &GuestBooking) -> Result<()> {
        let id = booking.id.to_string();
        let data = self.serialize(booking)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(GUEST_BOOKINGS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved guest booking: {}", booking.id);
        Ok(())
    }

    /// Get a guest booking by ID
    pub async fn get_guest_booking(&self, id: &str) -> Result<Option<GuestBooking>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(GUEST_BOOKINGS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// List all guest bookings
    pub async fn list_guest_bookings(&self) -> Result<Vec<GuestBooking>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(GUEST_BOOKINGS)?;

        let mut bookings = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            bookings.push(self.deserialize(value.value())?);
        }
        Ok(bookings)
    }

    // ── Swap Request CRUD ──

    /// Save a swap request
    pub async fn save_swap_request(&self, req: &SwapRequest) -> Result<()> {
        let id = req.id.to_string();
        let data = self.serialize(req)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(SWAP_REQUESTS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved swap request: {}", req.id);
        Ok(())
    }

    /// Get a swap request by ID
    pub async fn get_swap_request(&self, id: &str) -> Result<Option<SwapRequest>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(SWAP_REQUESTS)?;

        match table.get(id)? {
            Some(value) => Ok(Some(self.deserialize(value.value())?)),
            None => Ok(None),
        }
    }

    /// List swap requests involving a user (as requester or target)
    pub async fn list_swap_requests_by_user(&self, user_id: &str) -> Result<Vec<SwapRequest>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(SWAP_REQUESTS)?;

        let mut requests = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let req: SwapRequest = self.deserialize(value.value())?;
            if req.requester_id.to_string() == user_id || req.target_id.to_string() == user_id {
                requests.push(req);
            }
        }
        Ok(requests)
    }

    // ── Recurring Booking CRUD ──

    /// Save a recurring booking
    pub async fn save_recurring_booking(&self, booking: &RecurringBooking) -> Result<()> {
        let id = booking.id.to_string();
        let data = self.serialize(booking)?;

        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        {
            let mut table = write_txn.open_table(RECURRING_BOOKINGS)?;
            table.insert(id.as_str(), data.as_slice())?;
        }
        write_txn.commit()?;
        debug!("Saved recurring booking: {}", booking.id);
        Ok(())
    }

    /// List recurring bookings for a user
    pub async fn list_recurring_bookings_by_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<RecurringBooking>> {
        let db = self.inner.read().await;
        let read_txn = db.begin_read()?;
        drop(db);
        let table = read_txn.open_table(RECURRING_BOOKINGS)?;

        let mut bookings = Vec::new();
        for entry in table.iter()? {
            let (_, value) = entry?;
            let booking: RecurringBooking = self.deserialize(value.value())?;
            if booking.user_id.to_string() == user_id {
                bookings.push(booking);
            }
        }
        Ok(bookings)
    }

    /// Delete a recurring booking
    pub async fn delete_recurring_booking(&self, id: &str) -> Result<bool> {
        let db = self.inner.write().await;
        let write_txn = db.begin_write()?;
        drop(db);
        let existed = {
            let mut table = write_txn.open_table(RECURRING_BOOKINGS)?;
            let result = table.remove(id)?;
            result.is_some()
        };
        write_txn.commit()?;
        if existed {
            debug!("Deleted recurring booking: {}", id);
        }
        Ok(existed)
    }
}
