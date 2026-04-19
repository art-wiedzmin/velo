use crate::routing;
use tauri::AppHandle;

#[tauri::command]
pub fn is_elevated() -> bool {
    routing::is_elevated()
}

#[tauri::command]
pub async fn relaunch_as_admin(app: AppHandle) -> Result<(), String> {
    // Spawn the elevated process, then exit ourselves so we don't race on
    // the SQLite lock.
    routing::relaunch_as_admin()?;
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        app.exit(0);
    });
    Ok(())
}
