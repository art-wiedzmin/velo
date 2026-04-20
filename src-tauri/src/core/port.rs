//! Detects and resolves port conflicts on velo's local mixed inbound.
//!
//! Motivation: users routinely leave another proxy client (v2rayTun, Clash,
//! Shadowsocks-Windows, …) running or configured to autostart, and that
//! client binds the same `127.0.0.1:10808` velo wants. sing-box then fails
//! to bind and the user sees a cryptic "Connection refused"-flavoured error
//! from stderr. Rather than surface that, we proactively detect who owns
//! the port and evict them — same behaviour v2rayTun itself implements.

#![cfg(windows)]

use std::ffi::c_void;
use std::ptr;
use std::time::{Duration, Instant};

use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
};
use windows_sys::Win32::Networking::WinSock::AF_INET;

const NO_ERROR: u32 = 0;
const MIB_TCP_STATE_LISTEN: u32 = 2;

/// Returns the PID listening on `127.0.0.1:<port>` (or `0.0.0.0:<port>`),
/// or `None` if nothing is bound. Port-in-use on any address counts — sing-box
/// binds `127.0.0.1` so a `0.0.0.0` listener on the same port still blocks us.
pub fn listener_pid(port: u16) -> Option<u32> {
    let want_port_net = (port.to_be() as u32) & 0xFFFF;
    let localhost_net = u32::from_ne_bytes([127, 0, 0, 1]);

    let mut size: u32 = 0;
    // Probe the required buffer size. We deliberately ignore the return
    // value here: the "insufficient buffer" result is expected and any
    // other error will be surfaced by the second call with a populated
    // buffer — or leave `size` at 0 and we'll short-circuit.
    unsafe {
        GetExtendedTcpTable(
            ptr::null_mut(),
            &mut size,
            0,
            AF_INET as u32,
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        );
    }
    if size == 0 {
        return None;
    }

    let mut buf = vec![0u8; size as usize];
    let rc = unsafe {
        GetExtendedTcpTable(
            buf.as_mut_ptr() as *mut c_void,
            &mut size,
            0,
            AF_INET as u32,
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        )
    };
    if rc != NO_ERROR {
        return None;
    }

    // SAFETY: GetExtendedTcpTable populated `buf` with a MIB_TCPTABLE_OWNER_PID
    // header followed by `dwNumEntries` MIB_TCPROW_OWNER_PID entries. The
    // layout is stable per the Win32 API and `size` bounds our access.
    let table = unsafe { &*(buf.as_ptr() as *const MIB_TCPTABLE_OWNER_PID) };
    let rows = unsafe {
        std::slice::from_raw_parts(
            &table.table as *const MIB_TCPROW_OWNER_PID,
            table.dwNumEntries as usize,
        )
    };
    for row in rows {
        if row.dwState != MIB_TCP_STATE_LISTEN {
            continue;
        }
        // dwLocalPort: low 16 bits hold the port in network byte order.
        if (row.dwLocalPort & 0xFFFF) != want_port_net {
            continue;
        }
        let addr_any_or_local = row.dwLocalAddr == 0 || row.dwLocalAddr == localhost_net;
        if !addr_any_or_local {
            continue;
        }
        return Some(row.dwOwningPid);
    }
    None
}

/// Kill whatever process is listening on `127.0.0.1:<port>`. Best-effort and
/// idempotent: no listener → `Ok(None)`; listener found → terminate, wait
/// for the socket to be released, return the evicted PID.
///
/// Skips our own PID to guard against pathological cases where velo somehow
/// queries this against its own already-running sing-box (e.g. a restart
/// race). The Windows Job Object attached to sing-box handles that case by
/// killing it when the parent detaches, so we never need to self-kill here.
pub fn evict_listener(port: u16) -> Option<u32> {
    let pid = listener_pid(port)?;
    if pid == 0 || pid == std::process::id() {
        return None;
    }
    if !terminate_pid(pid) {
        return None;
    }
    // Wait briefly for the socket to release. Windows doesn't free the
    // port synchronously with process death — we'd race sing-box's bind
    // otherwise. 1s is enough in practice; longer means the process is
    // stuck in a kernel wait and we can't help it anyway.
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        if listener_pid(port).is_none() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Some(pid)
}

fn terminate_pid(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    // SAFETY: OpenProcess accepts raw PID; we check the returned handle
    // against NULL before using it, and close it unconditionally.
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if handle.is_null() {
            return false;
        }
        let ok = TerminateProcess(handle, 1) != 0;
        CloseHandle(handle);
        ok
    }
}
