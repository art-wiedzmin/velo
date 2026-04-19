use rusqlite::Connection;

use super::types::{Result, StoreError};

pub(super) const CURRENT_SCHEMA_VERSION: u32 = 3;

pub(super) fn migrate(conn: &Connection) -> Result<()> {
    let mut version: u32 =
        conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))? as u32;

    while version < CURRENT_SCHEMA_VERSION {
        match version {
            0 => {
                conn.execute_batch(
                    r#"
                    CREATE TABLE subscriptions (
                        id               INTEGER PRIMARY KEY AUTOINCREMENT,
                        name             TEXT    NOT NULL,
                        url              TEXT    NOT NULL UNIQUE,
                        last_fetched_at  INTEGER,
                        last_error       TEXT,
                        created_at       INTEGER NOT NULL
                    );

                    CREATE TABLE profiles (
                        id               INTEGER PRIMARY KEY AUTOINCREMENT,
                        name             TEXT    NOT NULL,
                        protocol         TEXT    NOT NULL,
                        address          TEXT    NOT NULL,
                        port             INTEGER NOT NULL,
                        data             TEXT    NOT NULL,
                        subscription_id  INTEGER REFERENCES subscriptions(id) ON DELETE CASCADE,
                        created_at       INTEGER NOT NULL,
                        updated_at       INTEGER NOT NULL
                    );

                    CREATE INDEX idx_profiles_subscription ON profiles(subscription_id);
                    "#,
                )?;
            }
            1 => {
                // v2 additions: favorites, recent-tracking, region on profiles;
                // quota/expiry on subscriptions; key/value settings table.
                // All new columns are nullable / default false so migrating
                // a v1 database never loses data.
                conn.execute_batch(
                    r#"
                    ALTER TABLE profiles ADD COLUMN favorite INTEGER NOT NULL DEFAULT 0;
                    ALTER TABLE profiles ADD COLUMN last_connected_at INTEGER;
                    ALTER TABLE profiles ADD COLUMN region TEXT;
                    CREATE INDEX idx_profiles_favorite ON profiles(favorite);
                    CREATE INDEX idx_profiles_last_connected ON profiles(last_connected_at);

                    ALTER TABLE subscriptions ADD COLUMN used_bytes INTEGER;
                    ALTER TABLE subscriptions ADD COLUMN total_bytes INTEGER;
                    ALTER TABLE subscriptions ADD COLUMN expires_at INTEGER;

                    CREATE TABLE settings (
                        key   TEXT PRIMARY KEY,
                        value TEXT NOT NULL
                    );
                    "#,
                )?;
            }
            2 => {
                // v3 adds the routing_rules table backing the Routing drawer.
                // `app_path` is the canonical identifier — sing-box matches by
                // absolute path, and UNIQUE prevents duplicate entries from a
                // double-add via the picker.
                conn.execute_batch(
                    r#"
                    CREATE TABLE routing_rules (
                        id          INTEGER PRIMARY KEY AUTOINCREMENT,
                        app_path    TEXT    NOT NULL UNIQUE,
                        app_name    TEXT,
                        enabled     INTEGER NOT NULL DEFAULT 1,
                        created_at  INTEGER NOT NULL
                    );
                    "#,
                )?;
            }
            _ => {
                // If this ever fires, a future migration arm is missing.
                return Err(StoreError::Sqlite(rusqlite::Error::InvalidQuery));
            }
        }
        version += 1;
        // `pragma_update` refuses `user_version` in some rusqlite versions; use
        // execute so the constant interpolates directly (safe: u32).
        conn.execute(&format!("PRAGMA user_version = {version}"), [])?;
    }

    Ok(())
}
