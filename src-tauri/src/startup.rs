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
/// window exists, then retry a real WebView2 environment bring-up until it
/// succeeds. Past the deadline we proceed anyway — a degraded launch beats
/// never launching.
#[cfg(windows)]
pub fn wait_for_session_ready(timeout: std::time::Duration) {
	let deadline = std::time::Instant::now() + timeout;
	while !shell_present() && std::time::Instant::now() < deadline {
		std::thread::sleep(std::time::Duration::from_millis(500));
	}
	while !webview2_env_ready(std::time::Duration::from_secs(10))
		&& std::time::Instant::now() < deadline
	{
		std::thread::sleep(std::time::Duration::from_secs(1));
	}
}

#[cfg(windows)]
fn shell_present() -> bool {
	use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;
	let class: Vec<u16> = "Shell_TrayWnd".encode_utf16().chain([0]).collect();
	!unsafe { FindWindowW(class.as_ptr(), std::ptr::null()) }.is_null()
}

/// One real `CreateCoreWebView2EnvironmentWithOptions` attempt — the exact
/// operation that silently dies on early-logon launches, so its success is
/// the readiness signal. Runs on a throwaway thread: a hung attempt must be
/// abandonable without blocking startup.
#[cfg(windows)]
fn webview2_env_ready(attempt_timeout: std::time::Duration) -> bool {
	let (tx, rx) = std::sync::mpsc::channel();
	std::thread::spawn(move || {
		let _ = tx.send(probe_webview2_env(attempt_timeout));
	});
	rx.recv_timeout(attempt_timeout + std::time::Duration::from_secs(2))
		.unwrap_or(false)
}

/// Probes against a scratch user data folder, never the real `EBWebView`
/// profile: WebView2 requires every client of one folder to pass identical
/// environment options, and the probe's defaults differ from wry's — sharing
/// the real profile would fail the actual webview creation for as long as
/// the probe's browser process lingers.
#[cfg(windows)]
fn webview2_probe_dir() -> std::path::PathBuf {
	std::env::var_os("LOCALAPPDATA")
		.map(|base| {
			std::path::Path::new(&base)
				.join("com.velo.app")
				.join("webview2-probe")
		})
		.unwrap_or_else(|| std::env::temp_dir().join("velo-webview2-probe"))
}

/// One real WebView2 bring-up: environment plus a controller on a hidden
/// host window. The controller step is mandatory — it is what spawns the
/// browser process; environment creation alone is runtime validation and
/// succeeds instantly even in sessions where the webview can't start.
#[cfg(windows)]
fn probe_webview2_env(timeout: std::time::Duration) -> bool {
	use webview2_com::Microsoft::Web::WebView2::Win32::{
		CreateCoreWebView2EnvironmentWithOptions, ICoreWebView2Environment,
		ICoreWebView2EnvironmentOptions,
	};
	use webview2_com::{
		CreateCoreWebView2ControllerCompletedHandler,
		CreateCoreWebView2EnvironmentCompletedHandler,
	};
	use windows_core::{HSTRING, PCWSTR};
	use windows_sys::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
	use windows_sys::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, WS_POPUP};

	unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };

	let dir = webview2_probe_dir();
	let _ = std::fs::create_dir_all(&dir);
	let data_dir = HSTRING::from(dir.as_os_str());
	let deadline = std::time::Instant::now() + timeout;

	let (env_tx, env_rx) = std::sync::mpsc::channel();
	let env_handler = CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
		move |error_code: windows_core::Result<()>, environment| {
			let _ = env_tx.send(error_code.is_ok().then_some(environment).flatten());
			Ok(())
		},
	));
	let started = unsafe {
		CreateCoreWebView2EnvironmentWithOptions(
			PCWSTR::null(),
			&data_dir,
			None::<&ICoreWebView2EnvironmentOptions>,
			&env_handler,
		)
	};
	if started.is_err() {
		return false;
	}
	let Some(env): Option<ICoreWebView2Environment> = pump_until(deadline, &env_rx).flatten()
	else {
		return false;
	};

	let class: Vec<u16> = "STATIC".encode_utf16().chain([0]).collect();
	let hwnd = unsafe {
		CreateWindowExW(
			0,
			class.as_ptr(),
			std::ptr::null(),
			WS_POPUP,
			0,
			0,
			1,
			1,
			std::ptr::null_mut(),
			std::ptr::null_mut(),
			std::ptr::null_mut(),
			std::ptr::null(),
		)
	};
	if hwnd.is_null() {
		return false;
	}

	let (ctl_tx, ctl_rx) = std::sync::mpsc::channel();
	let ctl_handler = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
		move |error_code: windows_core::Result<()>, controller| {
			let _ = ctl_tx.send(error_code.is_ok().then_some(controller).flatten());
			Ok(())
		},
	));
	let ok = match unsafe {
		env.CreateCoreWebView2Controller(windows::Win32::Foundation::HWND(hwnd as _), &ctl_handler)
	} {
		Err(_) => false,
		Ok(()) => match pump_until(deadline, &ctl_rx).flatten() {
			Some(controller) => {
				let _ = unsafe { controller.Close() };
				true
			}
			None => false,
		},
	};
	unsafe { DestroyWindow(hwnd) };
	ok
}

/// Deadline-bounded message pump: completion callbacks need this thread to
/// dispatch messages, and `webview2_com::wait_with_pump` would block in
/// GetMessage forever in exactly the broken-session case probed for.
#[cfg(windows)]
fn pump_until<T>(deadline: std::time::Instant, rx: &std::sync::mpsc::Receiver<T>) -> Option<T> {
	use windows_sys::Win32::UI::WindowsAndMessaging::{
		DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
	};
	loop {
		if let Ok(v) = rx.try_recv() {
			return Some(v);
		}
		if std::time::Instant::now() >= deadline {
			return None;
		}
		unsafe {
			let mut msg: MSG = std::mem::zeroed();
			while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
				TranslateMessage(&msg);
				DispatchMessageW(&msg);
			}
		}
		std::thread::sleep(std::time::Duration::from_millis(30));
	}
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

	#[test]
	fn webview2_env_probe_succeeds_on_dev_machine() {
		assert!(
			super::webview2_env_ready(std::time::Duration::from_secs(30)),
			"WebView2 runtime must be installed and startable on a dev machine"
		);
	}
}