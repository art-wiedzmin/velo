use super::{with_store, AppStore};
use crate::{profile, store};
use tauri::State;

#[tauri::command]
pub async fn profiles_list(store: State<'_, AppStore>) -> Result<Vec<store::StoredProfile>, String> {
    with_store(store.0.clone(), |s| s.list_profiles()).await
}

#[tauri::command]
pub async fn profiles_add(
    store: State<'_, AppStore>,
    profile: profile::Profile,
) -> Result<store::StoredProfile, String> {
    with_store(store.0.clone(), move |s| s.insert_profile(&profile, None)).await
}

#[tauri::command]
pub async fn profiles_update(
    store: State<'_, AppStore>,
    id: i64,
    profile: profile::Profile,
) -> Result<(), String> {
    with_store(store.0.clone(), move |s| s.update_profile(id, &profile)).await
}

#[tauri::command]
pub async fn profiles_delete(store: State<'_, AppStore>, id: i64) -> Result<(), String> {
    with_store(store.0.clone(), move |s| s.delete_profile(id)).await
}

#[tauri::command]
pub async fn profiles_set_favorite(
    store: State<'_, AppStore>,
    id: i64,
    favorite: bool,
) -> Result<(), String> {
    with_store(store.0.clone(), move |s| s.set_favorite(id, favorite)).await
}

#[tauri::command]
pub async fn profiles_duplicate(
    store: State<'_, AppStore>,
    id: i64,
) -> Result<store::StoredProfile, String> {
    with_store(store.0.clone(), move |s| s.duplicate_profile(id)).await
}
