use crate::profile::Profile;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("not found")]
    NotFound,
    /// Tried to insert a subscription whose URL is already stored. Kept
    /// separate from the generic Sqlite error so the UI can surface a
    /// human message ("already imported") instead of a UNIQUE-constraint
    /// leak.
    #[error("subscription with this URL already exists")]
    DuplicateSubscriptionUrl,
}

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredProfile {
    pub id: i64,
    pub profile: Profile,
    pub subscription_id: Option<i64>,
    pub favorite: bool,
    pub last_connected_at: Option<i64>,
    pub region: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSubscription {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub last_fetched_at: Option<i64>,
    pub last_error: Option<String>,
    pub used_bytes: Option<i64>,
    pub total_bytes: Option<i64>,
    pub expires_at: Option<i64>,
    pub created_at: i64,
}

/// Routing rule: a single application whose traffic deviates from the
/// global outbound per the active mode (whitelist/blacklist).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub id: i64,
    pub app_path: String,
    pub app_name: Option<String>,
    pub enabled: bool,
    pub created_at: i64,
}

/// Result of one subscription sync applied to the store.
#[derive(Debug, Clone, Serialize)]
pub struct SyncApplied {
    pub subscription_id: i64,
    pub profiles_inserted: usize,
}
