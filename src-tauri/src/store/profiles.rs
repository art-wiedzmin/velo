use rusqlite::{params, OptionalExtension, Transaction};

use crate::profile::Profile;

use super::now_secs;
use super::types::{Result, StoreError, StoredProfile};
use super::Store;

const PROFILE_COLS_SELECT: &str = concat!(
    "SELECT id, data, subscription_id, favorite, last_connected_at, region, created_at, updated_at ",
    "FROM profiles WHERE id = ?1"
);
const PROFILE_LIST_SQL: &str = concat!(
    "SELECT id, data, subscription_id, favorite, last_connected_at, region, created_at, updated_at ",
    "FROM profiles ORDER BY created_at ASC, id ASC"
);

impl Store {
    pub fn insert_profile(
        &self,
        profile: &Profile,
        subscription_id: Option<i64>,
    ) -> Result<StoredProfile> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let now = now_secs();
        let data = serde_json::to_string(profile)?;
        conn.execute(
            "INSERT INTO profiles (name, protocol, address, port, data, subscription_id, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![
                profile.name,
                protocol_str(profile),
                profile.address,
                profile.port,
                data,
                subscription_id,
                now,
            ],
        )?;
        let id = conn.last_insert_rowid();
        Ok(StoredProfile {
            id,
            profile: profile.clone(),
            subscription_id,
            favorite: false,
            last_connected_at: None,
            region: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn update_profile(&self, id: i64, profile: &Profile) -> Result<()> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let now = now_secs();
        let data = serde_json::to_string(profile)?;
        let n = conn.execute(
            "UPDATE profiles SET name = ?1, protocol = ?2, address = ?3, port = ?4, data = ?5, updated_at = ?6 \
             WHERE id = ?7",
            params![
                profile.name,
                protocol_str(profile),
                profile.address,
                profile.port,
                data,
                now,
                id,
            ],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    pub fn delete_profile(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let n = conn.execute("DELETE FROM profiles WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    pub fn get_profile(&self, id: i64) -> Result<Option<StoredProfile>> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        conn.query_row(
            PROFILE_COLS_SELECT,
            params![id],
            row_to_stored_profile,
        )
        .optional()
        .map_err(StoreError::from)
        .and_then(|opt| opt.transpose().map_err(Into::into))
    }

    pub fn list_profiles(&self) -> Result<Vec<StoredProfile>> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare(PROFILE_LIST_SQL)?;
        let rows = stmt.query_map([], row_to_stored_profile)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r??);
        }
        Ok(out)
    }

    pub fn set_favorite(&self, id: i64, favorite: bool) -> Result<()> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let n = conn.execute(
            "UPDATE profiles SET favorite = ?1 WHERE id = ?2",
            params![favorite as i64, id],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    /// Records a successful connect for Recent ordering. Called by the
    /// runner on `core_start` success — not by the user directly.
    pub fn touch_connected(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let now = now_secs();
        let n = conn.execute(
            "UPDATE profiles SET last_connected_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        if n == 0 {
            return Err(StoreError::NotFound);
        }
        Ok(())
    }

    /// Deep-copy a profile (hand-added only — duplicates of subscription
    /// rows would be overwritten on the next sync, which is confusing).
    /// The copy's name gets a " copy" suffix and `subscription_id` is cleared.
    pub fn duplicate_profile(&self, id: i64) -> Result<StoredProfile> {
        let src = self.get_profile(id)?.ok_or(StoreError::NotFound)?;
        let mut p = src.profile.clone();
        p.name = format!("{} copy", p.name);
        self.insert_profile(&p, None)
    }
}

pub(super) fn insert_profile_tx(
    tx: &Transaction<'_>,
    profile: &Profile,
    subscription_id: Option<i64>,
    now: i64,
) -> Result<()> {
    let data = serde_json::to_string(profile)?;
    tx.execute(
        "INSERT INTO profiles (name, protocol, address, port, data, subscription_id, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![
            profile.name,
            protocol_str(profile),
            profile.address,
            profile.port,
            data,
            subscription_id,
            now,
        ],
    )?;
    Ok(())
}

fn protocol_str(p: &Profile) -> &'static str {
    use crate::profile::Protocol::*;
    match p.protocol {
        Vless => "vless",
        Vmess => "vmess",
        Trojan => "trojan",
        Shadowsocks => "shadowsocks",
        Hysteria2 => "hysteria2",
        Tuic => "tuic",
    }
}

fn row_to_stored_profile(row: &rusqlite::Row<'_>) -> rusqlite::Result<Result<StoredProfile>> {
    let id: i64 = row.get(0)?;
    let data: String = row.get(1)?;
    let subscription_id: Option<i64> = row.get(2)?;
    let favorite: i64 = row.get(3)?;
    let last_connected_at: Option<i64> = row.get(4)?;
    let region: Option<String> = row.get(5)?;
    let created_at: i64 = row.get(6)?;
    let updated_at: i64 = row.get(7)?;
    Ok((|| {
        let profile: Profile = serde_json::from_str(&data)?;
        Ok(StoredProfile {
            id,
            profile,
            subscription_id,
            favorite: favorite != 0,
            last_connected_at,
            region,
            created_at,
            updated_at,
        })
    })())
}
