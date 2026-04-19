use super::{with_store, AppStore};
use crate::{routing, store};
use tauri::State;

#[tauri::command]
pub async fn routing_list(
    store: State<'_, AppStore>,
) -> Result<Vec<store::RoutingRule>, String> {
    with_store(store.0.clone(), |s| s.list_routing_rules()).await
}

#[tauri::command]
pub async fn routing_add(
    store: State<'_, AppStore>,
    app_path: String,
    app_name: Option<String>,
) -> Result<store::RoutingRule, String> {
    with_store(store.0.clone(), move |s| s.insert_routing_rule(&app_path, app_name.as_deref())).await
}

#[tauri::command]
pub async fn routing_delete(store: State<'_, AppStore>, id: i64) -> Result<(), String> {
    with_store(store.0.clone(), move |s| s.delete_routing_rule(id)).await
}

#[tauri::command]
pub async fn routing_set_enabled(
    store: State<'_, AppStore>,
    id: i64,
    enabled: bool,
) -> Result<(), String> {
    with_store(store.0.clone(), move |s| s.set_routing_rule_enabled(id, enabled)).await
}

#[tauri::command]
pub async fn routing_processes_snapshot() -> Result<Vec<routing::ProcessInfo>, String> {
    // sysinfo's full refresh is non-trivial (~50ms on a busy box); run on
    // a blocking thread so we don't stall the async runtime.
    tokio::task::spawn_blocking(routing::processes_snapshot)
        .await
        .map_err(|e| e.to_string())
}
