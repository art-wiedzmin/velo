pub mod commands;
pub mod config;
pub mod core;
pub mod parser;
pub mod profile;
pub mod routing;
pub mod startup;
pub mod store;
pub mod subscription;
pub mod sysproxy;
mod tray;

use std::sync::Arc;
use tauri::{AppHandle, Manager};

use commands::AppStore;
use sysproxy::SysProxyState;


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(core::CoreState::default());
            app.manage(SysProxyState::default());
            let db_path = resolve_db_path(app.handle())?;
            // A persisted snapshot means the previous session ended before
            // sysproxy_disable could run (installer force-close, crash,
            // power loss). Restore the user's pre-velo WinINet state now,
            // before the window is shown — otherwise they'd see "no
            // internet" because the registry still points at a dead port.
            if let Ok(dir) = startup::resolve_data_dir(app.handle()) {
                if let Some(stale) = sysproxy::Snapshot::consume_stale(&dir) {
                    let _ = sysproxy::disable(&stale);
                }
            }
            let store = store::Store::open(&db_path)
                .map_err(|e| format!("open store at {}: {e}", db_path.display()))?;
            let store = Arc::new(store);
            app.manage(AppStore(store.clone()));
            tray::install(app)?;
            reveal_main_window(app.handle(), &store);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::core_cmds::parse_any,
            commands::core_cmds::fetch_subscription,
            commands::core_cmds::build_singbox_config,
            commands::core_cmds::core_start,
            commands::core_cmds::core_stop,
            commands::core_cmds::core_status,
            commands::sysproxy::sysproxy_status,
            commands::profiles::profiles_list,
            commands::profiles::profiles_add,
            commands::profiles::profiles_update,
            commands::profiles::profiles_delete,
            commands::profiles::profiles_set_favorite,
            commands::profiles::profiles_duplicate,
            commands::subscriptions::subscriptions_list,
            commands::subscriptions::subscriptions_add,
            commands::subscriptions::subscriptions_rename,
            commands::subscriptions::subscriptions_delete,
            commands::subscriptions::subscriptions_sync,
            commands::settings::settings_get,
            commands::settings::settings_set,
            commands::routing::routing_list,
            commands::routing::routing_add,
            commands::routing::routing_delete,
            commands::routing::routing_set_enabled,
            commands::routing::routing_processes_snapshot,
            commands::elevation::is_elevated,
            commands::elevation::relaunch_as_admin,
            commands::environment::is_portable,
            commands::environment::is_autostart_launch,
            commands::autostart::autostart_elevated_status,
            commands::autostart::autostart_elevated_enable,
            commands::autostart::autostart_elevated_disable,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Last chance before the process dies: restore the user's
            // pre-velo WinINet proxy settings. Covers tray Quit,
            // OS shutdown, and — in concert with tray.rs letting hidden-
            // window CloseRequested proceed — installer force-close.
            if matches!(event, tauri::RunEvent::Exit) {
                restore_sysproxy_on_exit(app);
            }
        });
}

/// Where the SQLite file lives. Installed builds use the OS app-data dir;
/// portable builds — identified by a sibling `velo.portable` marker — keep
/// it next to the exe under `data/`. Kept as a free function so tests can
/// point elsewhere without touching the real runtime.
fn resolve_db_path(app: &AppHandle) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    Ok(startup::resolve_data_dir(app)?.join("velo.db"))
}


/// Window is created hidden (`visible: false` in tauri.conf) so autostart
/// launches don't flash the UI before we know whether to land in the tray.
/// Policy: show the window unless this is an autostart launch AND the user
/// opted into `autostart.minimized` (default true). Tray icon remains
/// clickable in either case.
fn reveal_main_window(app: &AppHandle, store: &store::Store) {
    let is_autostart = startup::is_autostart_launch();
    let stay_hidden = is_autostart && autostart_minimized(store);
    if stay_hidden {
        return;
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
    }
}

/// Default `true` — once the user enables autostart we assume they want it
/// quiet. Explicit "false" flips to eager reveal.
fn autostart_minimized(store: &store::Store) -> bool {
    match store.settings_get("autostart.minimized") {
        Ok(Some(v)) => v != "false",
        _ => true,
    }
}

/// Best-effort sysproxy restore on any process-exit path. Uses `try_lock`
/// because the Tauri runtime is still technically alive here — if some
/// other task is mid-operation on the state, skipping is safer than
/// blocking; the persisted snapshot on disk ensures the next launch
/// recovers instead.
fn restore_sysproxy_on_exit(app: &AppHandle) {
    let Some(sys) = app.try_state::<SysProxyState>() else {
        return;
    };
    let Ok(mut guard) = sys.0.try_lock() else {
        return;
    };
    let Some(snap) = guard.take() else {
        return;
    };
    let _ = sysproxy::disable(&snap);
    if let Ok(dir) = startup::resolve_data_dir(app) {
        sysproxy::Snapshot::forget(&dir);
    }
    // Final safety net — covers the case where the snapshot restored us to
    // a prior `ProxyEnable=1` state that itself referenced our now-dead port.
    let _ = sysproxy::clear_orphan_if_dead();
}