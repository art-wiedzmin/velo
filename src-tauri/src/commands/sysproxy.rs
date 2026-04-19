use crate::sysproxy::{self, SysProxyState};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn sysproxy_enable(
	app: AppHandle,
	sys: State<'_, SysProxyState>,
	host: String,
	port: u16,
) -> Result<(), String> {
	let mut guard = sys.0.lock().await;
	if guard.is_some() {
		return Err("system proxy already set by velo".into());
	}
	let snap = sysproxy::enable(&host, port).map_err(|e| e.to_string())?;
	// Persist before the registry write is considered "committed" from the
	// frontend's perspective. If the process dies between enable() and this
	// save, the next launch reads the registry as-is and has no ground truth
	// to restore to — accepted cost; the window is microseconds.
	if let Ok(dir) = crate::startup::resolve_data_dir(&app) {
		let _ = snap.save(&dir);
	}
	*guard = Some(snap);
	Ok(())
}

#[tauri::command]
pub async fn sysproxy_disable(
	app: AppHandle,
	sys: State<'_, SysProxyState>,
) -> Result<(), String> {
	let mut guard = sys.0.lock().await;
	if let Some(snap) = guard.take() {
		sysproxy::disable(&snap).map_err(|e| e.to_string())?;
		if let Ok(dir) = crate::startup::resolve_data_dir(&app) {
			sysproxy::Snapshot::forget(&dir);
		}
	}
	Ok(())
}

#[tauri::command]
pub async fn sysproxy_status(sys: State<'_, SysProxyState>) -> Result<bool, String> {
	Ok(sys.0.lock().await.is_some())
}
