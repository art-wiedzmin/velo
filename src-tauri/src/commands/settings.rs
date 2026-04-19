use super::{with_store, AppStore};
use tauri::State;

#[tauri::command]
pub async fn settings_get(
    store: State<'_, AppStore>,
    key: String,
) -> Result<Option<String>, String> {
    with_store(store.0.clone(), move |s| s.settings_get(&key)).await
}

#[tauri::command]
pub async fn settings_set(
    store: State<'_, AppStore>,
    key: String,
    value: String,
) -> Result<(), String> {
    with_store(store.0.clone(), move |s| s.settings_set(&key, &value)).await
}
