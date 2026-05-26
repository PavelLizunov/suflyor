//! System tray icon with right-click menu.
//! Without this the overlay window has skipTaskbar=true → no way to
//! reach the app if the hotkeys collide with something.

use anyhow::Result;
use tauri::{
    image::Image,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::config::SharedConfig;
use crate::runtime::SharedRuntime;
use crate::tile::SharedTiles;

pub fn setup(app: &AppHandle) -> Result<()> {
    let show = MenuItem::with_id(app, "show", "Show overlay", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, "hide", "Hide overlay", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let close_tiles = MenuItem::with_id(
        app, "close_all_tiles", "Close all tiles (Ctrl+Alt+W)", true, None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show, &hide, &settings, &close_tiles, &sep, &quit])?;

    // Use embedded PNG (bundled by tauri-build from icons/).
    let icon = app
        .default_window_icon()
        .cloned()
        .unwrap_or_else(|| Image::new_owned(vec![0u8; 4], 1, 1));

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .icon_as_template(false)
        .tooltip("Overlay — F9 ask · F11 hide")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu)
        .on_tray_icon_event(handle_icon)
        .build(app)?;

    Ok(())
}

fn handle_menu(app: &AppHandle, event: MenuEvent) {
    match event.id.as_ref() {
        "show" => show_overlay(app, false),
        "hide" => hide_overlay(app),
        "settings" => open_settings(app),
        "close_all_tiles" => {
            // v0.0.24: nuke every unpinned tile so the user can recover
            // from an aggressive-mode flood without quitting.
            let tiles = app.state::<SharedTiles>().inner().clone();
            let n = crate::tile::close_all_unpinned(app, &tiles);
            log::info!("tray Close-all-tiles: {n} closed");
        }
        "quit" => {
            // Stop the active session before exit so the JSONL journal
            // closes cleanly with SessionStop + SessionSummary (P0-1).
            // Tray menu fires from a non-overlay context, so we pull
            // managed state directly off the app handle.
            log::info!("tray Quit clicked — closing session first");
            let cfg = app.state::<SharedConfig>().inner().clone();
            let rt = app.state::<SharedRuntime>().inner().clone();
            let tiles = app.state::<SharedTiles>().inner().clone();
            crate::runtime::stop_session(app.clone(), cfg, rt, tiles);
            app.exit(0);
        }
        _ => {}
    }
}

fn handle_icon(tray: &tauri::tray::TrayIcon, event: TrayIconEvent) {
    // Left-click on tray icon → toggle overlay visibility.
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        let app = tray.app_handle();
        if let Some(w) = app.get_webview_window("overlay") {
            if w.is_visible().unwrap_or(false) {
                let _ = w.hide();
            } else {
                show_overlay(app, true);
            }
        }
    }
}

fn show_overlay(app: &AppHandle, focus: bool) {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.show();
        if focus {
            let _ = w.set_focus();
        }
    }
}

fn hide_overlay(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.hide();
    }
}

fn open_settings(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.show();
        let _ = w.set_focus();
        let _ = w.eval("window.location.search = '?settings=1'");
    }
}
