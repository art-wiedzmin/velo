//! Launch-time environment: was this exe run as portable (marker file next
//! to it), and did the autostart plugin invoke us with its flag?
//!
//! Portable is a runtime property, not a compile-time one: the same bundled
//! `velo.exe` behaves as installed unless a sibling `velo.portable` marker is
//! present. This keeps CI simple (one binary) and lets users turn a regular
//! install into portable by copying the exe + dropping the marker.

pub const AUTOSTART_FLAG: &str = "--autostart";
pub const PORTABLE_MARKER: &str = "velo.portable";

pub fn is_autostart_launch() -> bool {
	std::env::args().any(|a| a == AUTOSTART_FLAG)
}

pub fn is_portable() -> bool {
	std::env::current_exe()
		.ok()
		.and_then(|p| p.parent().map(|d| d.join(PORTABLE_MARKER).is_file()))
		.unwrap_or(false)
}


/// Returns the directory velo owns for user data (DB, logs, dumped
/// config). Portable builds keep everything next to the exe under `data/`;
/// installed builds use the OS app-data dir. The directory is created if
/// missing.
pub fn resolve_data_dir<R: tauri::Runtime>(
	app: &tauri::AppHandle<R>,
) -> std::io::Result<std::path::PathBuf> {
	use tauri::Manager;
	let dir = if is_portable() {
		std::env::current_exe()?
			.parent()
			.ok_or_else(|| std::io::Error::other("executable has no parent directory"))?
			.join("data")
	} else {
		app.path()
			.app_data_dir()
			.map_err(|e| std::io::Error::other(e.to_string()))?
	};
	std::fs::create_dir_all(&dir)?;
	Ok(dir)
}