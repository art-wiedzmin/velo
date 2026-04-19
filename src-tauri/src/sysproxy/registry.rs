//! Low-level Windows registry FFI helpers used by the sysproxy module.
//!
//! These functions know nothing about the Internet Settings subkey or the
//! sysproxy snapshot semantics — they just read and write typed registry
//! values under `HKEY_CURRENT_USER`.

#![cfg(windows)]

use super::Error;
use std::iter::once;
use std::ptr;
use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ,
};

pub(super) fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(once(0)).collect()
}

pub(super) fn with_key<F, T>(path: &str, f: F) -> Result<T, Error>
where
    F: FnOnce(HKEY) -> Result<T, Error>,
{
    let wide_path = wide(path);
    let mut h: HKEY = ptr::null_mut();
    // Safety: HKEY_CURRENT_USER is a constant pseudo-handle; the wide
    // path and HKEY out-pointer are valid and live for the call.
    let rc = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            wide_path.as_ptr(),
            0,
            ptr::null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_READ | KEY_WRITE,
            ptr::null_mut(),
            &mut h,
            ptr::null_mut(),
        )
    };
    if rc != ERROR_SUCCESS {
        return Err(Error::Registry(format!("RegCreateKeyExW({path}): {rc}")));
    }
    let result = f(h);
    // Safety: `h` was populated by a successful RegCreateKeyExW above.
    unsafe {
        RegCloseKey(h);
    }
    result
}

pub(super) fn read_dword(h: HKEY, name: &str) -> Result<Option<u32>, Error> {
    let wname = wide(name);
    let mut kind: u32 = 0;
    let mut val: u32 = 0;
    let mut size: u32 = std::mem::size_of::<u32>() as u32;
    // Safety: all pointers live for the call; `val` is u32-sized as
    // declared in `size`.
    let rc = unsafe {
        RegQueryValueExW(
            h,
            wname.as_ptr(),
            ptr::null_mut(),
            &mut kind,
            &mut val as *mut u32 as *mut u8,
            &mut size,
        )
    };
    match rc {
        ERROR_SUCCESS if kind == REG_DWORD => Ok(Some(val)),
        ERROR_SUCCESS => Err(Error::Registry(format!(
            "ProxyEnable wrong type: {kind}"
        ))),
        x if x == ERROR_FILE_NOT_FOUND => Ok(None),
        other => Err(Error::Registry(format!(
            "RegQueryValueExW({name}): {other}"
        ))),
    }
}

pub(super) fn read_sz(h: HKEY, name: &str) -> Result<Option<String>, Error> {
    let wname = wide(name);
    let mut kind: u32 = 0;
    let mut size: u32 = 0;
    // Probe required buffer size.
    let rc = unsafe {
        RegQueryValueExW(
            h,
            wname.as_ptr(),
            ptr::null_mut(),
            &mut kind,
            ptr::null_mut(),
            &mut size,
        )
    };
    if rc == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    if rc != ERROR_SUCCESS {
        return Err(Error::Registry(format!("RegQueryValueExW({name}): {rc}")));
    }
    if kind != REG_SZ {
        return Err(Error::Registry(format!("{name} wrong type: {kind}")));
    }
    if size == 0 {
        return Ok(Some(String::new()));
    }
    // `size` is in bytes including any NUL terminator(s).
    let len_u16 = (size as usize).div_ceil(2);
    let mut buf = vec![0u16; len_u16];
    let mut size_again = size;
    // Safety: buffer is sized per the probe; `size_again` bounds the write.
    let rc2 = unsafe {
        RegQueryValueExW(
            h,
            wname.as_ptr(),
            ptr::null_mut(),
            &mut kind,
            buf.as_mut_ptr() as *mut u8,
            &mut size_again,
        )
    };
    if rc2 != ERROR_SUCCESS {
        return Err(Error::Registry(format!(
            "RegQueryValueExW({name}) read: {rc2}"
        )));
    }
    // Trim trailing NULs.
    while buf.last().copied() == Some(0) {
        buf.pop();
    }
    Ok(Some(String::from_utf16_lossy(&buf)))
}

pub(super) fn write_dword(h: HKEY, name: &str, v: u32) -> Result<(), Error> {
    let wname = wide(name);
    let bytes = v.to_ne_bytes();
    // Safety: `bytes` lives for the call; length matches REG_DWORD.
    let rc = unsafe {
        RegSetValueExW(
            h,
            wname.as_ptr(),
            0,
            REG_DWORD,
            bytes.as_ptr(),
            bytes.len() as u32,
        )
    };
    if rc == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(Error::Registry(format!("RegSetValueExW({name}): {rc}")))
    }
}

pub(super) fn write_sz(h: HKEY, name: &str, v: &str) -> Result<(), Error> {
    let wname = wide(name);
    let wval: Vec<u16> = v.encode_utf16().chain(once(0)).collect();
    let bytes = wval.len() * std::mem::size_of::<u16>();
    // Safety: `wval` lives for the call and is NUL-terminated per REG_SZ contract.
    let rc = unsafe {
        RegSetValueExW(
            h,
            wname.as_ptr(),
            0,
            REG_SZ,
            wval.as_ptr() as *const u8,
            bytes as u32,
        )
    };
    if rc == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(Error::Registry(format!("RegSetValueExW({name}): {rc}")))
    }
}

pub(super) fn delete_value(h: HKEY, name: &str) -> Result<(), Error> {
    let wname = wide(name);
    // Safety: wname is valid UTF-16 NUL-terminated buffer.
    let rc = unsafe { RegDeleteValueW(h, wname.as_ptr()) };
    if rc == ERROR_SUCCESS || rc == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(Error::Registry(format!("RegDeleteValueW({name}): {rc}")))
    }
}

/// Test-only entry point: run an arbitrary closure with an open HKCU subkey.
#[cfg(test)]
pub(super) fn with_subkey<F, T>(path: &str, f: F) -> Result<T, Error>
where
    F: FnOnce(HKEY) -> Result<T, Error>,
{
    with_key(path, f)
}

#[cfg(test)]
pub(super) fn put_dword(h: HKEY, name: &str, v: u32) -> Result<(), Error> {
    write_dword(h, name, v)
}

#[cfg(test)]
pub(super) fn put_sz(h: HKEY, name: &str, v: &str) -> Result<(), Error> {
    write_sz(h, name, v)
}

#[cfg(test)]
pub(super) fn del(h: HKEY, name: &str) -> Result<(), Error> {
    delete_value(h, name)
}

#[cfg(test)]
pub(super) fn delete_subkey_tree(path: &str) -> Result<(), Error> {
    use windows_sys::Win32::System::Registry::RegDeleteTreeW;
    let wide = wide(path);
    // Safety: HKEY_CURRENT_USER is a well-known pseudo-handle; `wide` is
    // a valid UTF-16 NUL-terminated buffer owned by this frame.
    let rc = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, wide.as_ptr()) };
    if rc == ERROR_SUCCESS || rc == ERROR_FILE_NOT_FOUND {
        Ok(())
    } else {
        Err(Error::Registry(format!("RegDeleteTreeW: {rc}")))
    }
}
