//! Locates the `sing-box` binary at runtime.
//!
//! Resolution order:
//!   1. `VELO_SINGBOX` env var (explicit override — wins always)
//!   2. Alongside the current executable: `./sing-box.exe` (what the
//!      portable build script names the bundled copy)
//!   3. Alongside the current executable: `./sing-box-<target-triple>.exe`
//!      (what Tauri's `externalBin` drops into installed bundles)
//!   4. Repo-local `tools/sing-box.exe` (dev runs from source)
//!
//! Returning a path, not launching — the runner owns process lifecycle.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("sing-box binary not found (tried VELO_SINGBOX, exe dir, tools/)")]
    NotFound,
}

const EXE_NAME: &str = if cfg!(windows) { "sing-box.exe" } else { "sing-box" };

pub fn resolve() -> Result<PathBuf, ResolveError> {
    if let Ok(p) = std::env::var("VELO_SINGBOX") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Ok(pb);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let cand = dir.join(EXE_NAME);
            if cand.is_file() {
                return Ok(cand);
            }
            let triple = option_env!("VELO_TARGET_TRIPLE").unwrap_or("");
            if !triple.is_empty() {
                let triple_cand = dir.join(format!(
                    "sing-box-{triple}{}",
                    if cfg!(windows) { ".exe" } else { "" },
                ));
                if triple_cand.is_file() {
                    return Ok(triple_cand);
                }
            }
        }
    }

    // Dev fallback: repo-local tools/ (two levels up from `src-tauri/target/debug`).
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tools")
        .join(EXE_NAME);
    if dev.is_file() {
        return Ok(dev);
    }

    Err(ResolveError::NotFound)
}
