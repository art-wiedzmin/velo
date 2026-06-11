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
	// args_os: the Unicode-validating `args()` panics on malformed argv,
	// which third-party launchers can produce.
	std::env::args_os().any(|a| a == AUTOSTART_FLAG)
}

/// The autostart task fires within seconds of logon — before explorer and
/// the session services WebView2 depends on are up. Initializing the GUI
/// that early leaves a tray-only zombie: the webview environment silently
/// never materializes, so the frontend (which owns connect/autoconnect and
/// tray-action handling) never boots. Block until the shell's taskbar
/// window exists, then add a grace period for WebView2's own services.
#[cfg(windows)]
pub fn wait_for_session_ready(timeout: std::time::Duration) {
	let deadline = std::time::Instant::now() + timeout;
	while !shell_present() && std::time::Instant::now() < deadline {
		std::thread::sleep(std::time::Duration::from_millis(500));
	}
	std::thread::sleep(std::time::Duration::from_secs(5));
}

#[cfg(windows)]
fn shell_present() -> bool {
	use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;
	let class: Vec<u16> = "Shell_TrayWnd".encode_utf16().chain([0]).collect();
	!unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) }.is_null()
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

#[cfg(all(test, windows))]
mod tests {
	#[test]
	fn shell_present_in_interactive_session() {
		assert!(
			super::shell_present(),
			"explorer's taskbar must exist on an interactive dev machine"
		);
	}
}