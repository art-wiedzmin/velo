use std::sync::Arc;

pub mod autostart;
pub mod core_cmds;
pub mod elevation;
pub mod environment;
pub mod profiles;
pub mod routing;
pub mod settings;
pub mod subscriptions;
pub mod sysproxy;

pub struct AppStore(pub Arc<crate::store::Store>);

/// Serialization wrapper: surfaces store errors as strings to the frontend.
pub(crate) fn store_err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

/// Run a blocking closure against the store on a thread-pool worker.
/// SQLite calls are synchronous; isolating them here keeps the Tauri async
/// runtime free to serve the event loop.
pub(crate) async fn with_store<T, F>(store: Arc<crate::store::Store>, f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&crate::store::Store) -> crate::store::Result<T> + Send + 'static,
{
    tokio::task::spawn_blocking(move || f(&store))
        .await
        .map_err(|e| e.to_string())?
        .map_err(store_err)
}
