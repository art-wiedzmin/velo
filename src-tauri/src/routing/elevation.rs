//! Windows privilege handling for TUN mode.
//!
//! sing-box's TUN inbound on Windows needs admin: it creates a virtual NIC via
//! WinTUN and sets system routes. Running velo unelevated and then flipping to
//! Tunnel mode produces a low-level "access denied" deep in sing-box — not
//! something a user can act on. So:
//!   1. `is_elevated()` tells the UI early whether Tunnel mode will work.
//!   2. `relaunch_as_admin()` fires ShellExecuteW with the "runas" verb, which
//!      triggers the standard UAC prompt and re-launches velo elevated.
//!
//! Both are Windows-only. On other platforms we return a stub `true` because
//! TUN-equivalent routing needs different plumbing there anyway.

#[cfg(windows)]
pub fn is_elevated() -> bool {
    use std::mem::MaybeUninit;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut info = MaybeUninit::<TOKEN_ELEVATION>::uninit();
        let mut ret_len: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            info.as_mut_ptr() as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        ) != 0;
        CloseHandle(token);
        ok && info.assume_init().TokenIsElevated != 0
    }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool { true }

#[cfg(windows)]
pub fn relaunch_as_admin() -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_wide: Vec<u16> = exe
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // Null-terminated UTF-16 "runas".
    let verb_wide: Vec<u16> = "runas".encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let hinst = ShellExecuteW(
            std::ptr::null_mut(),
            verb_wide.as_ptr(),
            exe_wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        );
        // ShellExecuteW returns an HINSTANCE; values <= 32 indicate failure.
        // SE_ERR_CANCELLED == 1223 but HINSTANCE mapping puts cancellation at
        // 0 so we lump it in with generic failure.
        let code = hinst as isize;
        if code <= 32 {
            return Err(format!("ShellExecuteW failed with code {code}"));
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn relaunch_as_admin() -> Result<(), String> {
    Err("elevation is only supported on Windows".into())
}
