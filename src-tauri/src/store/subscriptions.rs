use rusqlite::{params, OptionalExtension};

use super::now_secs;
use super::types::{Result, StoreError, StoredSubscription, SyncApplied};
use super::Store;
use crate::subscription::SubscriptionQuota;
use crate::profile::Profile;

const SUBSCRIPTION_SELECT_COLS: &str = concat!(
    "id, name, url, last_fetched_at, last_error, used_bytes, total_bytes, expires_at, created_at"
);

impl Store {
    pub fn insert_subscription(&self, name: &str, url: &str) -> Result<StoredSubscription> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let now = now_secs();
        // Catch the UNIQUE(url) violation and convert to a typed error so
        // subscriptions_add can surface a human message in the UI.
        match conn.execute(
            "INSERT INTO subscriptions (name, url, created_at) VALUES (?1, ?2, ?3)",
            params![name, url, now],
        ) {
            Ok(_) => {}
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                return Err(StoreError::DuplicateSubscriptionUrl);
            }
            Err(e) => return Err(StoreError::from(e)),
        }
        Ok(StoredSubscription {
            id: conn.last_insert_rowid(),
            name: name.to_owned(),
            url: url.to_owned(),
            last_fetched_at: None,
            last_error: None,
            used_bytes: None,
            total_bytes: None,
            expires_at: None,
            created_at: now,
        })
    }

    pub fn rename_subscription(&self, id: i64, name: &str) -> Result<()> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let n = conn.execute(
            "UPDATE subscriptions SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    pub fn delete_subscription(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let n = conn.execute("DELETE FROM subscriptions WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    pub fn get_subscription(&self, id: i64) -> Result<Option<StoredSubscription>> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        conn.query_row(
            &format!("SELECT {SUBSCRIPTION_SELECT_COLS} FROM subscriptions WHERE id = ?1"),
            params![id],
            row_to_stored_subscription,
        )
        .optional()
        .map_err(StoreError::from)
    }

    pub fn list_subscriptions(&self) -> Result<Vec<StoredSubscription>> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let sql = format!(
            "SELECT {SUBSCRIPTION_SELECT_COLS} FROM subscriptions ORDER BY created_at ASC, id ASC",
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], row_to_stored_subscription)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Apply a fetch result: replace all profiles linked to this subscription
    /// with the fresh list, and record fetch metadata. All in one transaction
    /// so a crash mid-sync never leaves the subscription in a half-updated
    /// state. Hand-added profiles (subscription_id IS NULL) are untouched.
    pub fn apply_sync_result(
        &self,
        subscription_id: i64,
        profiles: &[Profile],
        fetch_error: Option<&str>,
        quota: Option<&SubscriptionQuota>,
    ) -> Result<SyncApplied> {
        let mut conn = self.conn.lock().expect("store mutex poisoned");
        let tx = conn.transaction()?;

        // Subscription must exist; otherwise the cascade invariant is meaningless.
        let exists: i64 = tx
            .query_row(
                "SELECT 1 FROM subscriptions WHERE id = ?1",
                params![subscription_id],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(0);
        if exists == 0 {
            return Err(StoreError::NotFound);
        }

        let now = now_secs();

        if fetch_error.is_none() {
            tx.execute(
                "DELETE FROM profiles WHERE subscription_id = ?1",
                params![subscription_id],
            )?;
            for p in profiles {
                super::profiles::insert_profile_tx(&tx, p, Some(subscription_id), now)?;
            }
        }

        let quota_used = quota.and_then(|q| q.used_bytes);
        let quota_total = quota.and_then(|q| q.total_bytes);
        let quota_expires = quota.and_then(|q| q.expires_at);
        tx.execute(
            "UPDATE subscriptions \
             SET last_fetched_at = ?1, last_error = ?2, \
                 used_bytes = COALESCE(?3, used_bytes), \
                 total_bytes = COALESCE(?4, total_bytes), \
                 expires_at  = COALESCE(?5, expires_at) \
             WHERE id = ?6",
            params![now, fetch_error, quota_used, quota_total, quota_expires, subscription_id],
        )?;

        tx.commit()?;

        Ok(SyncApplied {
            subscription_id,
            profiles_inserted: if fetch_error.is_some() { 0 } else { profiles.len() },
        })
    }
}

fn row_to_stored_subscription(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredSubscription> {
    Ok(StoredSubscription {
        id: row.get(0)?,
        name: row.get(1)?,
        url: row.get(2)?,
        last_fetched_at: row.get(3)?,
        last_error: row.get(4)?,
        used_bytes: row.get(5)?,
        total_bytes: row.get(6)?,
        expires_at: row.get(7)?,
        created_at: row.get(8)?,
    })
}
