//! System tray: menu + left-click toggle + close-to-tray.
//!
//! Menu actions that need the full connect/switch logic are forwarded to the
//! frontend as `tray://action` events — the frontend owns profile selection
//! and the sing-box config build path, so duplicating it in Rust would fork
//! the connect flow.

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, Emitter, Listener, Manager, WindowEvent,
};

const TRAY_ID: &str = "main";
const MAIN_WINDOW: &str = "main";
const EVENT_ACTION: &str = "tray://action";
const EVENT_CORE_STATE: &str = "core://state";

#[derive(serde::Deserialize)]
struct CoreStateEvent {
    running: bool,
}

pub fn install(app: &App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "tray:show", "Show velo", true, None::<&str>)?;
    // Single toggle item — label swaps with connection state. The frontend
    // owns the resolve (which profile to connect to) so the tray just emits
    // intent; Rust stays stateless w.r.t. the selected profile.
    let toggle = MenuItem::with_id(app, "tray:toggle", "Connect", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "tray:quit", "Quit", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[&show, &sep1, &toggle, &sep2, &quit],
    )?;

    let icon = app
        .default_window_icon()
        .cloned()
        .expect("default window icon missing from bundle");

    let _tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("velo — disconnected")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, ev| match ev.id.as_ref() {
            "tray:show" => show_main(app),
            "tray:toggle" => {
                let _ = app.emit(EVENT_ACTION, "toggle");
            }
            "tray:quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main(tray.app_handle());
            }
        })
        .build(app)?;

    // Intercept the main window's close: on a user click (visible window)
    // hide to tray; on any close request targeting an already-hidden window
    // let it proceed. The second path matters for installer upgrades and
    // OS shutdown — preventing close while hidden would force the caller
    // to TerminateProcess, which skips sysproxy restore and sing-box Drop.
    // (Sysproxy is also recovered from a persisted snapshot at next
    // launch, but letting clean shutdown run beats depending on recovery.)
    if let Some(win) = app.get_webview_window(MAIN_WINDOW) {
        let w = win.clone();
        win.on_window_event(move |ev| {
            if let WindowEvent::CloseRequested { api, .. } = ev {
                if w.is_visible().unwrap_or(true) {
                    api.prevent_close();
                    let _ = w.hide();
                }
            }
        });
    }

    // Reflect connection state in the tooltip *and* in the toggle item's
    // label. Single source of truth: the `core://state` event stream.
    let app_h = app.handle().clone();
    let toggle_item = toggle.clone();
    app.listen(EVENT_CORE_STATE, move |event| {
        let running = matches!(
            serde_json::from_str::<CoreStateEvent>(event.payload()),
            Ok(ev) if ev.running
        );
        let (tip, label) = if running {
            ("velo — connected", "Disconnect")
        } else {
            ("velo — disconnected", "Connect")
        };
        if let Some(tray) = app_h.tray_by_id(TRAY_ID) {
            let _ = tray.set_tooltip(Some(tip));
        }
        let _ = toggle_item.set_text(label);
    });

    Ok(())
}

fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN_WINDOW) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn toggle_main(app: &tauri::AppHandle) {
    let Some(w) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
    } else {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
