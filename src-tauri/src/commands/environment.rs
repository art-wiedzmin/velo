//! Read-only probes the frontend uses to adapt its UI to the launch context.

use crate::startup;

#[tauri::command]
pub fn is_portable() -> bool {
	startup::is_portable()
}

#[tauri::command]
pub fn is_autostart_launch() -> bool {
	startup::is_autostart_launch()
}
