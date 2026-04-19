use super::{with_store, AppStore};
use crate::{store, subscription};
use tauri::State;

#[tauri::command]
pub async fn subscriptions_list(
    store: State<'_, AppStore>,
) -> Result<Vec<store::StoredSubscription>, String> {
    with_store(store.0.clone(), |s| s.list_subscriptions()).await
}

#[tauri::command]
pub async fn subscriptions_add(
    store: State<'_, AppStore>,
    name: String,
    url: String,
) -> Result<store::StoredSubscription, String> {
    with_store(store.0.clone(), move |s| s.insert_subscription(&name, &url)).await
}

#[tauri::command]
pub async fn subscriptions_rename(
    store: State<'_, AppStore>,
    id: i64,
    name: String,
) -> Result<(), String> {
    with_store(store.0.clone(), move |s| s.rename_subscription(id, &name)).await
}

#[tauri::command]
pub async fn subscriptions_delete(store: State<'_, AppStore>, id: i64) -> Result<(), String> {
    with_store(store.0.clone(), move |s| s.delete_subscription(id)).await
}

/// Fetch a subscription and replace its linked profiles atomically.
/// A fetch failure is still persisted (as `last_error`) so the UI can render
/// the state without asking "did it run?" separately.
#[tauri::command]
pub async fn subscriptions_sync(
    store: State<'_, AppStore>,
    id: i64,
) -> Result<store::SyncApplied, String> {
    let store = store.0.clone();
    let sub = with_store(store.clone(), move |s| {
        s.get_subscription(id)?.ok_or(store::StoreError::NotFound)
    })
    .await?;

    let fetched = subscription::fetch(&sub.url).await;
    match fetched {
        Ok(fetched) => {
            let profiles = fetched.result.profiles;
            let quota = fetched.quota;
            with_store(store, move |s| {
                s.apply_sync_result(id, &profiles, None, Some(&quota))
            })
            .await
        }
        Err(e) => {
            let msg = e.to_string();
            // Record the failure; apply_sync_result with error keeps cached profiles.
            with_store(store, move |s| {
                s.apply_sync_result(id, &[], Some(&msg), None)
            })
            .await?;
            Err(e.to_string())
        }
    }
}
