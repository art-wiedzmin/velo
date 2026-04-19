//! Elevated autostart via Windows Task Scheduler.
//!
//! The standard autostart path (HKCU\...\Run via tauri-plugin-autostart) runs
//! velo at the logged-in user's integrity level. That's fine for sysproxy
//! mode but breaks TUN auto-connect, which needs admin.
//!
//! Task Scheduler bypasses UAC at logon when the task is registered with
//! `/RL HIGHEST`: creating the task costs one UAC prompt up-front, but every
//! subsequent logon runs velo elevated silently. This mirrors the pattern
//! WireGuard, v2rayN, Clash Verge, and similar tools use.
//!
//! We shell out to `schtasks.exe` rather than driving the Task Scheduler COM
//! API — one extra process per rare setting toggle is cheaper than pulling
//! in a COM dependency for three call sites. Exit codes are stable across
//! Windows locales (output text is not, so we never parse stdout/stderr for
//! semantics — only surface stderr verbatim as an error message on failure).

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use std::process::{Command, Stdio};

const TASK_NAME: &str = "velo Autostart";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[tauri::command]
pub fn autostart_elevated_status() -> bool {
	#[cfg(windows)]
	{
		Command::new("schtasks")
			.args(["/Query", "/TN", TASK_NAME])
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.creation_flags(CREATE_NO_WINDOW)
			.status()
			.map(|s| s.success())
			.unwrap_or(false)
	}
	#[cfg(not(windows))]
	{
		false
	}
}

#[tauri::command]
pub fn autostart_elevated_enable() -> Result<(), String> {
	#[cfg(windows)]
	{
		// Registering a HIGHEST-integrity task itself requires admin. Fail
		// loudly here so the UI can offer a "Relaunch as admin" path instead
		// of burying the error in schtasks' localized output.
		if !crate::routing::is_elevated() {
			return Err(
				"Velo must be running as administrator to enable elevated autostart.".into(),
			);
		}
		let exe = std::env::current_exe().map_err(|e| e.to_string())?;
		// `/TR` takes a single command-line string; quoting the exe path
		// handles spaces ("C:\Program Files\..."). Windows Task Scheduler
		// parses the quoted prefix as the executable and the rest as args.
		let target = format!(r#""{}" {}"#, exe.display(), crate::startup::AUTOSTART_FLAG);
		let out = Command::new("schtasks")
			.args([
				"/Create",
				"/F",
				"/TN",
				TASK_NAME,
				"/SC",
				"ONLOGON",
				"/RL",
				"HIGHEST",
				"/IT",
				"/TR",
				&target,
			])
			.stdout(Stdio::null())
			.stderr(Stdio::piped())
			.creation_flags(CREATE_NO_WINDOW)
			.output()
			.map_err(|e| e.to_string())?;
		if !out.status.success() {
			let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
			return Err(if stderr.is_empty() {
				format!(
					"schtasks /Create failed (exit {})",
					out.status.code().unwrap_or(-1)
				)
			} else {
				stderr
			});
		}
		Ok(())
	}
	#[cfg(not(windows))]
	{
		Err("elevated autostart is Windows-only".into())
	}
}

#[tauri::command]
pub fn autostart_elevated_disable() -> Result<(), String> {
	#[cfg(windows)]
	{
		// Ignore the exit code: "task doesn't exist" and "task deleted" are
		// both the desired end state. Propagating a spawn error would be
		// needlessly noisy for a teardown call.
		let _ = Command::new("schtasks")
			.args(["/Delete", "/F", "/TN", TASK_NAME])
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.creation_flags(CREATE_NO_WINDOW)
			.status();
		Ok(())
	}
	#[cfg(not(windows))]
	{
		Ok(())
	}
}
