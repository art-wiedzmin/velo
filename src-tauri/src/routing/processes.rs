//! Running-process enumeration for the per-app routing UI.
//! The `From running` picker in the Routing drawer needs a list of the
//! currently running user-space processes with their executable paths so
//! the user can tick the ones whose traffic they want routed differently.
//! We deduplicate by canonical .exe path — a single browser with 20 tabs
//! should appear once, not twenty times.
//!
//! System services (`svchost.exe`, kernel code, etc.) are filtered out
//! best-effort by skipping processes whose exe path lives under
//! `%SystemRoot%` — they're rarely the intended routing target and the
//! list quickly becomes unwieldy otherwise.

use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub name: String,
    pub path: String,
}

/// Snapshot of user-space processes with unique .exe paths, sorted by name.
pub fn processes_snapshot() -> Vec<ProcessInfo> {
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()),
    );
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::everything(),
    );

    let system_root = system_root_lowercase();
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<ProcessInfo> = Vec::new();

    for proc in sys.processes().values() {
        let Some(exe) = proc.exe() else { continue };
        let path: PathBuf = exe.to_path_buf();
        let path_str = path.to_string_lossy().to_string();
        if path_str.is_empty() {
            continue;
        }
        // Skip Windows system processes best-effort — a typical install has
        // hundreds and they dilute the picker. Users can still type paths by
        // hand via the browse-file flow.
        if let Some(root) = &system_root {
            if path_str.to_ascii_lowercase().starts_with(root) {
                continue;
            }
        }
        if !seen.insert(path_str.clone()) {
            continue;
        }
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| proc.name().to_string_lossy().to_string());
        out.push(ProcessInfo {
            name,
            path: path_str,
        });
    }

    out.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    out
}

fn system_root_lowercase() -> Option<String> {
    #[cfg(windows)]
    {
        std::env::var("SystemRoot")
            .ok()
            .map(|s| s.to_ascii_lowercase())
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_returns_unique_paths() {
        let snap = processes_snapshot();
        // Sanity: the test harness itself is running, so the list shouldn't
        // be empty. We don't assert specific entries — the runtime surface is
        // machine-dependent.
        assert!(!snap.is_empty());

        let mut paths: Vec<&str> = snap.iter().map(|p| p.path.as_str()).collect();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), snap.len(), "snapshot must deduplicate by path");
    }
}
