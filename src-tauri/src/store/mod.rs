//! Persistent storage for profiles and subscriptions.
//!
//! Schema design:
//!   * `Profile` is stored as a JSON blob alongside a few promoted columns
//!     (`name`, `protocol`, `address`, `port`). Promotion keeps the list view
//!     cheap without parsing JSON per row; the blob keeps imports lossless —
//!     any future field added to `Profile` round-trips without a schema bump.
//!   * Subscription ownership is modeled with `subscription_id` + cascade
//!     delete: removing a subscription drops its imported profiles atomically.
//!   * A "sync" is destructive for the imported set. We replace every profile
//!     linked to the subscription in one transaction rather than diff, because
//!     provider-supplied names are the only stable identifier we have and
//!     they're not unique or durable. The user's hand-added profiles
//!     (`subscription_id IS NULL`) are never touched by sync.
//!
//! All methods are synchronous; the Tauri command layer wraps calls in
//! `spawn_blocking` so the async runtime never stalls on SQLite.
//!
//! Migrations are tracked via `PRAGMA user_version`. Bumping the schema
//! means adding a numbered arm to `migrate`, never editing the existing ones.

use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

mod migrations;
mod profiles;
mod routing;
mod settings;
mod subscriptions;
mod types;

pub use types::*;

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::init(conn)
    }

    /// Opens an in-memory database — used by tests and can be useful for
    /// one-shot tooling. The DB is lost when the Store is dropped.
    pub fn open_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init(conn)
    }

    fn init(conn: Connection) -> Result<Self> {
        // Sensible defaults for a desktop app: WAL gives concurrent readers a
        // pass without blocking the writer; foreign keys are off by default in
        // SQLite — enabling them is required for ON DELETE CASCADE to work.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        migrations::migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

fn now_secs() -> i64 { SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_secs() as i64)
    .unwrap_or(0) }

#[cfg(test)]
mod tests {
    use super::migrations::CURRENT_SCHEMA_VERSION;
    use super::*;
    use crate::profile::{Profile, Protocol, Security, Transport, TransportParams};

    fn sample(name: &str, addr: &str, port: u16) -> Profile {
        Profile {
            name: name.into(),
            protocol: Protocol::Vless,
            address: addr.into(),
            port,
            credential: "00000000-0000-4000-8000-000000000000".into(),
            flow: None,
            packet_encoding: None,
            alter_id: None,
            cipher: None,
            ss_method: None,
            transport: Transport::Tcp,
            transport_params: TransportParams::default(),
            security: Security::None,
            tls: None,
        }
    }

    #[test]
    fn schema_is_at_current_version_after_open() {
        let store = Store::open_memory().unwrap();
        let conn = store.conn.lock().unwrap();
        let v: i64 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v as u32, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn profile_crud_roundtrip() {
        let s = Store::open_memory().unwrap();
        let inserted = s.insert_profile(&sample("one", "h.example", 443), None).unwrap();
        assert!(inserted.id > 0);
        assert_eq!(inserted.profile.name, "one");

        let got = s.get_profile(inserted.id).unwrap().unwrap();
        assert_eq!(got.profile, inserted.profile);

        s.update_profile(inserted.id, &sample("renamed", "h.example", 8443))
            .unwrap();
        let got = s.get_profile(inserted.id).unwrap().unwrap();
        assert_eq!(got.profile.name, "renamed");
        assert_eq!(got.profile.port, 8443);
        assert!(got.updated_at >= got.created_at);

        s.delete_profile(inserted.id).unwrap();
        assert!(s.get_profile(inserted.id).unwrap().is_none());
    }

    #[test]
    fn list_profiles_orders_by_created_then_id() {
        let s = Store::open_memory().unwrap();
        let a = s.insert_profile(&sample("a", "h", 1), None).unwrap();
        let b = s.insert_profile(&sample("b", "h", 2), None).unwrap();
        let c = s.insert_profile(&sample("c", "h", 3), None).unwrap();
        let ids: Vec<i64> = s.list_profiles().unwrap().into_iter().map(|p| p.id).collect();
        assert_eq!(ids, vec![a.id, b.id, c.id]);
    }

    #[test]
    fn update_and_delete_missing_id_returns_not_found() {
        let s = Store::open_memory().unwrap();
        assert!(matches!(
            s.update_profile(999, &sample("x", "h", 1)),
            Err(StoreError::NotFound)
        ));
        assert!(matches!(s.delete_profile(999), Err(StoreError::NotFound)));
    }

    #[test]
    fn subscription_unique_url_is_enforced() {
        let s = Store::open_memory().unwrap();
        s.insert_subscription("home", "http://x/sub").unwrap();
        let err = s.insert_subscription("dup", "http://x/sub");
        assert!(matches!(err, Err(StoreError::DuplicateSubscriptionUrl)));
    }

    #[test]
    fn apply_sync_replaces_only_subscription_profiles() {
        let s = Store::open_memory().unwrap();
        // hand-added: must survive sync
        let hand = s
            .insert_profile(&sample("hand", "h.example", 443), None)
            .unwrap();

        let sub = s.insert_subscription("home", "http://x/sub").unwrap();
        let first_batch = vec![sample("sub-a", "h", 1), sample("sub-b", "h", 2)];
        let applied = s.apply_sync_result(sub.id, &first_batch, None, None).unwrap();
        assert_eq!(applied.profiles_inserted, 2);

        // Second sync replaces.
        let second_batch = vec![sample("sub-c", "h", 3)];
        let applied = s.apply_sync_result(sub.id, &second_batch, None, None).unwrap();
        assert_eq!(applied.profiles_inserted, 1);

        let all = s.list_profiles().unwrap();
        let names: Vec<&str> = all.iter().map(|p| p.profile.name.as_str()).collect();
        assert!(names.contains(&"hand"));
        assert!(names.contains(&"sub-c"));
        assert!(!names.contains(&"sub-a"));
        assert!(!names.contains(&"sub-b"));

        // Sub metadata was updated.
        let got = s.get_subscription(sub.id).unwrap().unwrap();
        assert!(got.last_fetched_at.is_some());
        assert!(got.last_error.is_none());

        // Hand profile's id is unchanged.
        assert!(s.get_profile(hand.id).unwrap().is_some());
    }

    #[test]
    fn apply_sync_with_error_records_error_and_keeps_profiles() {
        let s = Store::open_memory().unwrap();
        let sub = s.insert_subscription("home", "http://x/sub").unwrap();
        s.apply_sync_result(sub.id, &[sample("a", "h", 1)], None, None)
            .unwrap();

        // Now a failed sync: should NOT wipe the previous profiles.
        s.apply_sync_result(sub.id, &[], Some("dns lookup failed"), None)
            .unwrap();

        let linked: Vec<_> = s
            .list_profiles()
            .unwrap()
            .into_iter()
            .filter(|p| p.subscription_id == Some(sub.id))
            .collect();
        assert_eq!(linked.len(), 1, "failed sync must not drop cached profiles");

        let got = s.get_subscription(sub.id).unwrap().unwrap();
        assert_eq!(got.last_error.as_deref(), Some("dns lookup failed"));
        assert!(got.last_fetched_at.is_some());
    }

    #[test]
    fn delete_subscription_cascades_to_profiles() {
        let s = Store::open_memory().unwrap();
        let sub = s.insert_subscription("home", "http://x/sub").unwrap();
        s.apply_sync_result(sub.id, &[sample("a", "h", 1), sample("b", "h", 2)], None, None)
        .unwrap();
        assert_eq!(s.list_profiles().unwrap().len(), 2);

        s.delete_subscription(sub.id).unwrap();
        assert!(s.list_profiles().unwrap().is_empty());
    }

    #[test]
    fn apply_sync_for_missing_subscription_is_error() {
        let s = Store::open_memory().unwrap();
        assert!(matches!(
            s.apply_sync_result(42, &[], None, None),
            Err(StoreError::NotFound)
        ));
    }

    #[test]
    fn data_blob_is_valid_json_for_full_profile() {
        let s = Store::open_memory().unwrap();
        let mut p = sample("full", "h.example", 443);
        p.flow = Some(crate::profile::Flow::XtlsRprxVision);
        p.transport = Transport::Ws;
        p.transport_params.path = Some("/ws".into());
        p.transport_params.host = Some("front".into());
        p.security = Security::Tls;
        p.tls = Some(crate::profile::TlsParams {
            sni: Some("sni.example".into()),
            alpn: vec!["h2".into()],
            fingerprint: Some(crate::profile::Fingerprint("chrome".into())),
            allow_insecure: false,
            reality: None,
        });

        let inserted = s.insert_profile(&p, None).unwrap();
        let got = s.get_profile(inserted.id).unwrap().unwrap();
        assert_eq!(got.profile, p);
    }

    #[test]
    fn routing_rules_crud() {
        let s = Store::open_memory().unwrap();
        let r = s.insert_routing_rule(r"C:\Apps\a.exe", Some("A")).unwrap();
        assert!(r.id > 0);
        assert!(r.enabled);

        // Second insert of the same path upserts, keeps id, refreshes name
        // when provided.
        let r2 = s.insert_routing_rule(r"C:\Apps\a.exe", Some("A2")).unwrap();
        assert_eq!(r.id, r2.id);
        assert_eq!(r2.app_name.as_deref(), Some("A2"));

        s.set_routing_rule_enabled(r.id, false).unwrap();
        let list = s.list_routing_rules().unwrap();
        assert_eq!(list.len(), 1);
        assert!(!list[0].enabled);

        s.delete_routing_rule(r.id).unwrap();
        assert!(s.list_routing_rules().unwrap().is_empty());
    }

    #[test]
    fn settings_get_set_roundtrip() {
        let s = Store::open_memory().unwrap();
        assert_eq!(s.settings_get("missing").unwrap(), None);
        s.settings_set("core_mode", "tun").unwrap();
        assert_eq!(s.settings_get("core_mode").unwrap().as_deref(), Some("tun"));
        // Upsert overwrites.
        s.settings_set("core_mode", "sysproxy").unwrap();
        assert_eq!(s.settings_get("core_mode").unwrap().as_deref(), Some("sysproxy"));
    }
}
