//! Windows system HTTP proxy toggle for WinINet-based apps.
//!
//! sing-box exposes a mixed HTTP+SOCKS5 inbound on a single port; Windows'
//! system proxy knob (WinINet) speaks HTTP, so we just point it at that port.
//! Apps that read their own proxy env vars (curl on POSIX, some browsers'
//! private setups) are out of scope here — this is the Windows global toggle.
//!
//! Design notes:
//!   * We snapshot the registry values we touch *before* writing, so disabling
//!     restores the user's prior state exactly, including the "was unset"
//!     case (delete vs. restore).
//!   * WinINet caches these values in every hosting process. After a write
//!     we must broadcast `INTERNET_OPTION_SETTINGS_CHANGED` + `REFRESH`, or
//!     Edge/IE/etc. keep using the old config until reboot.
//!   * The in-memory snapshot is also persisted to a JSON file in the data
//!     directory on enable. Hard-kill paths (installer force-close, crash,
//!     power loss) that skip [`disable`] leave the registry pointing at a
//!     dead port; the next velo launch reads the stale file and restores
//!     the user's prior state. See [`Snapshot::save`] / [`Snapshot::consume_stale`].

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;

#[cfg(windows)]
mod registry;

/// The "bypass proxy for these" list. Mirrors what v2rayN / Clash-for-Windows
/// write: localhost, loopback, RFC1918 private ranges, and `<local>` which
/// WinINet interprets as "hosts without a dot".
pub const DEFAULT_BYPASS: &str = "localhost;127.*;10.*;172.16.*;172.17.*;172.18.*;172.19.*;172.20.*;172.21.*;172.22.*;172.23.*;172.24.*;172.25.*;172.26.*;172.27.*;172.28.*;172.29.*;172.30.*;172.31.*;192.168.*;<local>";

#[derive(Debug, Error)]
pub enum Error {
    #[error("registry: {0}")]
    Registry(String),
    #[error("not supported on this platform")]
    Unsupported,
}

/// Snapshot of the three registry values we overwrite. `None` means the
/// value did not exist and should be deleted on restore.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Snapshot {
    pub enable: Option<u32>,
    pub server: Option<String>,
    pub override_list: Option<String>,
}

pub const SNAPSHOT_FILE: &str = "sysproxy-snapshot.json";

impl Snapshot {
    /// Persist the snapshot JSON-encoded next to the DB. Only writes if the
    /// file doesn't already exist — a stale file there represents an earlier
    /// instance's true pre-velo state, and clobbering it would lose that
    /// ground truth if the current session is itself an unclean recovery.
    pub fn save(&self, data_dir: &std::path::Path) -> std::io::Result<()> {
        let path = data_dir.join(SNAPSHOT_FILE);
        if path.exists() {
            return Ok(());
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    /// Delete the persisted snapshot. Called on clean disable, and after
    /// successful recovery at startup.
    pub fn forget(data_dir: &std::path::Path) {
        let _ = std::fs::remove_file(data_dir.join(SNAPSHOT_FILE));
    }

    /// If a persisted snapshot exists, load it and delete the file. Returns
    /// `None` on missing file; returns `None` and deletes the file on parse
    /// errors (stale garbage can't help us, keeping it around just blocks
    /// future recoveries).
    pub fn consume_stale(data_dir: &std::path::Path) -> Option<Self> {
        let path = data_dir.join(SNAPSHOT_FILE);
        let raw = std::fs::read_to_string(&path).ok()?;
        let snap = serde_json::from_str::<Self>(&raw).ok();
        let _ = std::fs::remove_file(&path);
        snap
    }
}


/// Tauri-managed state: the currently-active sysproxy snapshot, if any.
#[derive(Default)]
pub struct SysProxyState(pub Mutex<Option<Snapshot>>);

/// Write proxy config and return the prior state. Caller must retain the
/// snapshot and pass it back to [`disable`] to restore.
pub fn enable(host: &str, port: u16) -> Result<Snapshot, Error> {
    #[cfg(windows)]
    {
        let server = format!("{host}:{port}");
        apply(Some((1, server.as_str(), DEFAULT_BYPASS)))
    }
    #[cfg(not(windows))]
    {
        let _ = (host, port);
        Err(Error::Unsupported)
    }
}

/// Restore the snapshot captured by [`enable`].
pub fn disable(snapshot: &Snapshot) -> Result<(), Error> {
    #[cfg(windows)]
    {
        restore(snapshot)
    }
    #[cfg(not(windows))]
    {
        let _ = snapshot;
        Err(Error::Unsupported)
    }
}

#[cfg(windows)]
const SETTINGS_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

#[cfg(windows)]
fn apply(values: Option<(u32, &str, &str)>) -> Result<Snapshot, Error> {
    registry::with_key(SETTINGS_PATH, |h| {
        let snap = snapshot(h)?;
        if let Some((enable, server, override_list)) = values {
            registry::write_dword(h, "ProxyEnable", enable)?;
            registry::write_sz(h, "ProxyServer", server)?;
            registry::write_sz(h, "ProxyOverride", override_list)?;
        }
        Ok(snap)
    })
    .and_then(|snap| {
        notify_wininet();
        Ok(snap)
    })
}

#[cfg(windows)]
fn restore(snap: &Snapshot) -> Result<(), Error> {
    registry::with_key(SETTINGS_PATH, |h| {
        match snap.enable {
            Some(v) => registry::write_dword(h, "ProxyEnable", v)?,
            None => registry::delete_value(h, "ProxyEnable")?,
        }
        match &snap.server {
            Some(v) => registry::write_sz(h, "ProxyServer", v)?,
            None => registry::delete_value(h, "ProxyServer")?,
        }
        match &snap.override_list {
            Some(v) => registry::write_sz(h, "ProxyOverride", v)?,
            None => registry::delete_value(h, "ProxyOverride")?,
        }
        Ok(())
    })?;
    notify_wininet();
    Ok(())
}

/// Clears `ProxyEnable` when the registry points at a local port nobody is
/// listening on. Covers the specific failure where a prior velo session
/// (or crashed sibling process) left `ProxyEnable=1, ProxyServer=127.0.0.1:N`
/// behind — WinINet apps then hang on connection refused until something
/// binds that port again. Port-agnostic: any localhost-like host, any port.
/// No-op when the port is actually alive (another proxy app owns it) or
/// when the proxy points at a non-local host (corporate proxy, etc.).
///
/// Returns `true` when the registry was modified.
#[cfg(windows)]
pub fn clear_orphan_if_dead() -> Result<bool, Error> {
    let cleared = registry::with_key(SETTINGS_PATH, |h| {
        let Some(enable) = registry::read_dword(h, "ProxyEnable")? else {
            return Ok(false);
        };
        if enable != 1 {
            return Ok(false);
        }
        let Some(server) = registry::read_sz(h, "ProxyServer")? else {
            return Ok(false);
        };
        let Some(port) = parse_local_port(&server) else {
            return Ok(false);
        };
        if is_port_listening(port) {
            return Ok(false);
        }
        registry::write_dword(h, "ProxyEnable", 0)?;
        Ok(true)
    })?;
    if cleared {
        notify_wininet();
    }
    Ok(cleared)
}

#[cfg(not(windows))]
pub fn clear_orphan_if_dead() -> Result<bool, Error> {
    Err(Error::Unsupported)
}

/// Extracts the port from a `ProxyServer` string when it points at a local
/// host. Accepts both the simple `host:port` form velo writes and the
/// protocol-keyed form `http=host:port;https=host:port;...` that some other
/// clients write. Returns `None` for remote hosts or unparseable input.
fn parse_local_port(server: &str) -> Option<u16> {
    for part in server.split(';') {
        let part = part.trim();
        let addr = part.split_once('=').map(|(_, v)| v).unwrap_or(part);
        let Some((host, port)) = addr.rsplit_once(':') else { continue };
        let host = host.trim_start_matches('[').trim_end_matches(']');
        if matches!(host, "127.0.0.1" | "localhost" | "::1") {
            if let Ok(p) = port.parse::<u16>() {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(windows)]
fn is_port_listening(port: u16) -> bool {
    use std::net::{SocketAddr, TcpStream};
    use std::time::Duration;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok()
}

#[cfg(windows)]
fn snapshot(h: windows_sys::Win32::System::Registry::HKEY) -> Result<Snapshot, Error> {
    Ok(Snapshot {
        enable: registry::read_dword(h, "ProxyEnable")?,
        server: registry::read_sz(h, "ProxyServer")?,
        override_list: registry::read_sz(h, "ProxyOverride")?,
    })
}

/// Tell every WinINet host (Edge, IE, apps using WinHTTP over WinINet) to
/// re-read proxy config. Without this the keys in the registry are
/// correct but apps keep using the cached old settings until reboot.
#[cfg(windows)]
fn notify_wininet() {
    use std::ffi::c_void;
    use std::ptr;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Networking::WinInet::{
        InternetSetOptionW, INTERNET_OPTION_REFRESH, INTERNET_OPTION_SETTINGS_CHANGED,
    };
    // Both options with NULL handle broadcast to every WinINet session.
    // Errors here are non-fatal: the registry is already correct, we just
    // couldn't nudge listeners. Log at the call site if we ever care.
    unsafe {
        InternetSetOptionW(
            ptr::null_mut::<c_void>() as *const _ as HANDLE,
            INTERNET_OPTION_SETTINGS_CHANGED,
            ptr::null(),
            0,
        );
        InternetSetOptionW(
            ptr::null_mut::<c_void>() as *const _ as HANDLE,
            INTERNET_OPTION_REFRESH,
            ptr::null(),
            0,
        );
    }
}

#[cfg(test)]
mod persist_tests {
    use super::*;

    fn tmp_dir() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!(
            "velo-sysproxy-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn save_then_consume_roundtrips() {
        let dir = tmp_dir();
        let s = Snapshot {
            enable: Some(0),
            server: None,
            override_list: Some("<local>".into()),
        };
        s.save(&dir).unwrap();
        assert!(dir.join(SNAPSHOT_FILE).is_file());

        let recovered = Snapshot::consume_stale(&dir).expect("present");
        assert_eq!(recovered.enable, Some(0));
        assert_eq!(recovered.server, None);
        assert_eq!(recovered.override_list.as_deref(), Some("<local>"));
        assert!(!dir.join(SNAPSHOT_FILE).is_file(), "file removed after consume");
    }

    #[test]
    fn save_does_not_clobber_existing_file() {
        // If a snapshot file already exists, it represents an earlier
        // instance's true pre-velo state. A later save() from a recovery
        // chain must not overwrite it — otherwise the user's ground truth
        // gets lost on double-crash.
        let dir = tmp_dir();
        let original = Snapshot {
            enable: Some(1),
            server: Some("corp.proxy:8080".into()),
            override_list: None,
        };
        original.save(&dir).unwrap();

        let replacement = Snapshot::default();
        replacement.save(&dir).unwrap();

        let recovered = Snapshot::consume_stale(&dir).expect("present");
        assert_eq!(recovered.enable, Some(1));
        assert_eq!(recovered.server.as_deref(), Some("corp.proxy:8080"));
    }

    #[test]
    fn consume_stale_missing_is_none() {
        let dir = tmp_dir();
        assert!(Snapshot::consume_stale(&dir).is_none());
    }

    #[test]
    fn consume_stale_deletes_corrupt_file() {
        let dir = tmp_dir();
        std::fs::write(dir.join(SNAPSHOT_FILE), b"not valid json{{{").unwrap();
        assert!(Snapshot::consume_stale(&dir).is_none());
        assert!(!dir.join(SNAPSHOT_FILE).is_file(), "garbage cleared");
    }

    #[test]
    fn forget_is_idempotent_on_missing_file() {
        let dir = tmp_dir();
        Snapshot::forget(&dir); // no file — no panic
        Snapshot::forget(&dir); // still no file — still no panic
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::registry;
    use super::*;

    const SCRATCH: &str = r"Software\velo-sysproxy-test";

    /// Exercises the same snapshot/restore machinery against a scratch HKCU
    /// subkey rather than the real Internet Settings key. If this passes,
    /// the only thing production adds is pointing at a different subkey.
    #[test]
    fn snapshot_and_restore_roundtrip() {
        // Start clean.
        let _ = registry::delete_subkey_tree(SCRATCH);

        // Pre-populate "prior user state": ProxyEnable=0, ProxyServer missing,
        // ProxyOverride="<local>".
        registry::with_subkey(SCRATCH, |h| {
            registry::put_dword(h, "ProxyEnable", 0)?;
            registry::del(h, "ProxyServer")?;
            registry::put_sz(h, "ProxyOverride", "<local>")?;
            Ok(())
        })
        .unwrap();

        // Snapshot + overwrite (simulating enable).
        let snap = registry::with_subkey(SCRATCH, |h| {
            let s = snapshot(h)?;
            registry::put_dword(h, "ProxyEnable", 1)?;
            registry::put_sz(h, "ProxyServer", "127.0.0.1:10808")?;
            registry::put_sz(h, "ProxyOverride", DEFAULT_BYPASS)?;
            Ok(s)
        })
        .unwrap();

        assert_eq!(snap.enable, Some(0));
        assert_eq!(snap.server, None);
        assert_eq!(snap.override_list.as_deref(), Some("<local>"));

        // Verify writes landed.
        let after_enable = registry::with_subkey(SCRATCH, |h| snapshot(h)).unwrap();
        assert_eq!(after_enable.enable, Some(1));
        assert_eq!(after_enable.server.as_deref(), Some("127.0.0.1:10808"));
        assert_eq!(after_enable.override_list.as_deref(), Some(DEFAULT_BYPASS));

        // Restore against the scratch key (mirrors `disable`).
        registry::with_subkey(SCRATCH, |h| {
            match snap.enable {
                Some(v) => registry::put_dword(h, "ProxyEnable", v)?,
                None => registry::del(h, "ProxyEnable")?,
            }
            match &snap.server {
                Some(v) => registry::put_sz(h, "ProxyServer", v)?,
                None => registry::del(h, "ProxyServer")?,
            }
            match &snap.override_list {
                Some(v) => registry::put_sz(h, "ProxyOverride", v)?,
                None => registry::del(h, "ProxyOverride")?,
            }
            Ok(())
        })
        .unwrap();

        let restored = registry::with_subkey(SCRATCH, |h| snapshot(h)).unwrap();
        assert_eq!(restored.enable, Some(0));
        assert_eq!(restored.server, None);
        assert_eq!(restored.override_list.as_deref(), Some("<local>"));

        // Cleanup.
        registry::delete_subkey_tree(SCRATCH).unwrap();
    }

    #[test]
    fn default_bypass_contains_rfc1918_and_local() {
        assert!(DEFAULT_BYPASS.contains("10.*"));
        assert!(DEFAULT_BYPASS.contains("192.168.*"));
        assert!(DEFAULT_BYPASS.contains("<local>"));
        // sanity: semicolon-separated, no stray whitespace
        assert!(!DEFAULT_BYPASS.contains(' '));
    }
}


#[cfg(test)]
mod parse_port_tests {
    use super::parse_local_port;

    #[test]
    fn simple_loopback_forms() {
        assert_eq!(parse_local_port("127.0.0.1:10808"), Some(10808));
        assert_eq!(parse_local_port("localhost:1080"), Some(1080));
        assert_eq!(parse_local_port("[::1]:7890"), Some(7890));
    }

    #[test]
    fn protocol_keyed_form() {
        // Some clients write `http=host:port;https=host:port;ftp=...`.
        // Pick the first localhost entry we find.
        let s = "http=127.0.0.1:10808;https=127.0.0.1:10808";
        assert_eq!(parse_local_port(s), Some(10808));
    }

    #[test]
    fn rejects_remote_host() {
        assert_eq!(parse_local_port("corp.proxy:8080"), None);
        assert_eq!(parse_local_port("10.0.0.1:3128"), None);
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_local_port(""), None);
        assert_eq!(parse_local_port("not-an-address"), None);
        assert_eq!(parse_local_port("127.0.0.1:notaport"), None);
    }
}