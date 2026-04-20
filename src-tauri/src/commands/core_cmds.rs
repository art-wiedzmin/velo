use super::AppStore;
use crate::{config, core, parser, profile, routing, store, subscription};
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn parse_any(url: &str) -> Result<profile::Profile, String> {
    parser::parse_any(url).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fetch_subscription(url: String) -> Result<subscription::SubscriptionResult, String> {
    subscription::fetch(&url)
        .await
        .map(|f| f.result)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn build_singbox_config(profile: profile::Profile) -> Result<serde_json::Value, String> {
    Ok(config::singbox::build(&profile, &config::singbox::Options::default()))
}

#[tauri::command]
pub async fn core_start(
    state: State<'_, core::CoreState>,
    sys: State<'_, crate::sysproxy::SysProxyState>,
    store: State<'_, AppStore>,
    app: AppHandle,
    profile: profile::Profile,
    profile_id: Option<i64>,
) -> Result<(), String> {
    let mut guard = state.inner.lock().await;
    if guard.is_some() {
        return Err("core already running".into());
    }
    let opts = build_options_from_store(&store.0).await;
    // TUN inbound on Windows requires admin. Fail early with an actionable
    // message instead of propagating the opaque sing-box error buried in
    // stderr logs.
    if opts.mode == config::singbox::Mode::Tun && !routing::is_elevated() {
        return Err("Tunnel mode requires administrator privileges. Relaunch velo as admin.".into());
    }

    // Clear the decks before sing-box tries to bind:
    //   1. Evict any other proxy client squatting on our mixed port. Users
    //      routinely leave v2rayTun/Clash/etc. running and would otherwise
    //      see a cryptic bind error. Same behaviour v2rayTun itself does.
    //   2. Wipe a leftover `ProxyEnable=1, ProxyServer=127.0.0.1:<dead>`
    //      from a prior velo session that didn't clean up — otherwise
    //      WinINet apps will hang on our dead port until we re-bind it.
    #[cfg(windows)]
    if opts.mixed_port != 0 {
        if let Some(evicted) = core::port::evict_listener(opts.mixed_port) {
            eprintln!("evicted PID {evicted} from port {}", opts.mixed_port);
        }
        let _ = crate::sysproxy::clear_orphan_if_dead();
    }

    let cfg = config::singbox::build(&profile, &opts);
    // Dump the generated config next to the SQLite DB for debugging — when
    // the user says "no internet", they (or I) can open last-config.json and
    // see exactly what we handed to sing-box. Overwritten every start.
    let data_dir = crate::startup::resolve_data_dir(&app).ok();
    if let Some(dir) = data_dir.as_ref() {
        let _ = std::fs::write(
            dir.join("last-config.json"),
            serde_json::to_string_pretty(&cfg).unwrap_or_default(),
        );
    }
    let log_path = data_dir.clone().map(|d| d.join("sing-box.log"));
    let sink = core::tauri_sink_with_file(app.clone(), log_path);
    let runner = core::Runner::start(&cfg, sink).await.map_err(|e| e.to_string())?;
    *guard = Some(runner);

    // Sysproxy is meaningful only when traffic enters through the mixed
    // inbound (Sysproxy mode). In TUN mode the kernel-level capture handles
    // routing and layering sysproxy on top leaves WinINet apps pointing at
    // `127.0.0.1:<mixed_port>` — a dead port after Disconnect — which
    // silently breaks Edge/Chromium/Electron every time the user stops.
    if opts.mode == config::singbox::Mode::Sysproxy && opts.mixed_port != 0 {
        let mut sys_guard = sys.0.lock().await;
        if sys_guard.is_none() {
            match crate::sysproxy::enable(&opts.listen, opts.mixed_port) {
                Ok(snap) => {
                    if let Some(dir) = data_dir.as_ref() {
                        let _ = snap.save(dir);
                    }
                    *sys_guard = Some(snap);
                }
                Err(e) => {
                    // Rollback the runner: a connect that leaves the UI
                    // thinking it's "connected" but routes nothing is worse
                    // than a failed connect the user can retry.
                    drop(sys_guard);
                    if let Some(r) = guard.take() {
                        r.stop(core::tauri_sink(app)).await;
                    }
                    return Err(format!("enable system proxy: {e}"));
                }
            }
        }
    }

    // Recent-tracking: a successful start is the only signal we get for
    // "user actually connected to this profile". Failure here shouldn't
    // unwind the runner — a dropped UPDATE means Recent misses one row.
    if let Some(id) = profile_id {
        let s = store.0.clone();
        tokio::task::spawn_blocking(move || {
            let _ = s.touch_connected(id);
        });
    }
    Ok(())
}

#[tauri::command]
pub async fn core_stop(
    state: State<'_, core::CoreState>,
    sys: State<'_, crate::sysproxy::SysProxyState>,
    app: AppHandle,
) -> Result<(), String> {
    let mut guard = state.inner.lock().await;
    if let Some(runner) = guard.take() {
        runner.stop(core::tauri_sink(app.clone())).await;
    }
    // Stopping the core implies the local proxy is gone; leaving the system
    // proxy pointed at a dead port would silently break every WinINet app.
    // Surface the restore error to the UI so the user knows their registry
    // may be inconsistent — swallowing it here was the reason "Disconnect"
    // could leave ProxyEnable=1 pointing at a dead port.
    let mut sys_guard = sys.0.lock().await;
    let mut disable_result: Result<(), String> = Ok(());
    if let Some(snap) = sys_guard.take() {
        disable_result = crate::sysproxy::disable(&snap).map_err(|e| e.to_string());
        if let Ok(dir) = crate::startup::resolve_data_dir(&app) {
            crate::sysproxy::Snapshot::forget(&dir);
        }
    }
    drop(sys_guard);
    // Even when we had no snapshot (TUN mode, or startup before velo ran),
    // the registry might still hold `ProxyEnable=1, ProxyServer=127.0.0.1:N`
    // from a prior session or another client. Our runner just died, so our
    // port is now provably dead — safe to clear.
    #[cfg(windows)]
    {
        let _ = crate::sysproxy::clear_orphan_if_dead();
    }
    disable_result?;
    Ok(())
}

#[tauri::command]
pub async fn core_status(state: State<'_, core::CoreState>) -> Result<bool, String> {
    Ok(state.inner.lock().await.is_some())
}

/// Reads persisted mode/routing settings and assembles the sing-box config
/// options. Defaults (`mode=sysproxy`, no routing) apply when the keys are
/// absent — this is the state of a fresh install.
pub(super) async fn build_options_from_store(store: &Arc<store::Store>) -> config::singbox::Options {
    use config::singbox::{Mode, Options, RoutingMode};
    let store = store.clone();
    tokio::task::spawn_blocking(move || {
        let mode = match store.settings_get("core_mode").ok().flatten().as_deref() {
            Some("tun") => Mode::Tun,
            _ => Mode::Sysproxy,
        };
        let routing_mode = match store.settings_get("routing_mode").ok().flatten().as_deref() {
            Some("whitelist") => RoutingMode::Whitelist,
            Some("blacklist") => RoutingMode::Blacklist,
            _ => RoutingMode::None,
        };
        let routing_apps = if mode == Mode::Tun && routing_mode != RoutingMode::None {
            store
                .list_routing_rules()
                .unwrap_or_default()
                .into_iter()
                .filter(|r| r.enabled)
                .map(|r| r.app_path)
                .collect()
        } else {
            Vec::new()
        };
        Options {
            mode,
            routing_mode,
            routing_apps,
            ..Options::default()
        }
    })
    .await
    .unwrap_or_default()
}
