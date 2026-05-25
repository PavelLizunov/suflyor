//! Global hotkey registration. Reads keybinds from Config and
//! emits Tauri events that the frontend listens to.
//!
//! Each shortcut is registered independently — a single conflict (e.g.
//! F12 captured by another app) does NOT abort the whole batch. The list
//! of failures is returned + emitted as `hotkeys:warnings` so the UI
//! can show a warning indicator.

use crate::config::SharedConfig;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};

/// Register all hotkeys. Returns a vector of human-readable warnings —
/// empty when everything registered cleanly.
pub fn register_all(app: &AppHandle, _cfg: SharedConfig) -> Vec<String> {
    let mut warnings = Vec::new();

    // F9 — ask
    let app_h = app.clone();
    try_register(app, "F9 (ask AI)", Code::F9, &mut warnings, move || {
        let _ = app_h.emit_to("overlay", "hotkey:ask", ());
    });

    // F10 — screenshot
    let app_h = app.clone();
    try_register(app, "F10 (screenshot)", Code::F10, &mut warnings, move || {
        let _ = app_h.emit_to("overlay", "hotkey:screenshot", ());
    });

    // F11 — PANIC HIDE: toggle visibility of overlay AND every active tile.
    // #1 adoption blocker per brainstorm — instant "hide everything" if a
    // screenshare starts unexpectedly. Single tap = invisible to viewer.
    // Second tap = restore exactly what was visible before.
    let app_h = app.clone();
    try_register(app, "F11 (panic-hide)", Code::F11, &mut warnings, move || {
        let app = app_h.clone();
        let overlay = app.get_webview_window("overlay");
        let overlay_visible = overlay
            .as_ref()
            .map(|w| w.is_visible().unwrap_or(true))
            .unwrap_or(true);

        // Determine target state: hide everything if overlay is currently visible,
        // otherwise show everything.
        if overlay_visible {
            // HIDE PASS
            if let Some(w) = overlay {
                let _ = w.hide();
            }
            // Iterate all webview windows; tile windows have label prefix "tile-".
            for (label, w) in app.webview_windows() {
                if label.starts_with("tile-") {
                    let _ = w.hide();
                }
            }
        } else {
            // SHOW PASS
            if let Some(w) = overlay {
                let _ = w.show();
            }
            for (label, w) in app.webview_windows() {
                if label.starts_with("tile-") {
                    let _ = w.show();
                }
            }
        }
    });

    // F8 — pause audio (F12 collides with Windows-wide handlers — DON'T use F12)
    let app_h = app.clone();
    try_register(app, "F8 (pause)", Code::F8, &mut warnings, move || {
        let _ = app_h.emit_to("overlay", "hotkey:pause_audio", ());
    });

    // F6 — manual spawn tile from last transcript line (bypass detector).
    let app_h = app.clone();
    try_register(app, "F6 (manual tile)", Code::F6, &mut warnings, move || {
        let app = app_h.clone();
        tauri::async_runtime::spawn(async move {
            let cfg = app.state::<crate::config::SharedConfig>();
            let rt = app.state::<crate::runtime::SharedRuntime>();
            let tiles = app.state::<crate::tile::SharedTiles>();
            crate::runtime::manual_spawn_tile(
                app.clone(),
                cfg.inner().clone(),
                rt.inner().clone(),
                tiles.inner().clone(),
            )
            .await;
        });
    });

    // F4 — KB palette: opens an inline search overlay over the bar.
    // Just emits the event — UI handles modal lifecycle.
    let app_h = app.clone();
    try_register(app, "F4 (kb palette)", Code::F4, &mut warnings, move || {
        let _ = app_h.emit_to("overlay", "hotkey:kb-palette", ());
    });

    // F3 — Reask: re-ask the last question with LATEST transcript +
    // previous answer as context. Useful when conversation moved on
    // and the AI's first take is stale.
    let app_h = app.clone();
    try_register(app, "F3 (reask)", Code::F3, &mut warnings, move || {
        let app = app_h.clone();
        tauri::async_runtime::spawn(async move {
            let cfg = app.state::<crate::config::SharedConfig>();
            let rt = app.state::<crate::runtime::SharedRuntime>();
            let tiles = app.state::<crate::tile::SharedTiles>();
            crate::runtime::reask_last(
                app.clone(),
                cfg.inner().clone(),
                rt.inner().clone(),
                tiles.inner().clone(),
            )
            .await;
        });
    });

    // F7 — DEBUG: spawn a hardcoded test tile
    let app_h = app.clone();
    try_register(app, "F7 (test tile)", Code::F7, &mut warnings, move || {
        let app = app_h.clone();
        tauri::async_runtime::spawn(async move {
            let tiles = app.state::<crate::tile::SharedTiles>();
            let cfg = app.state::<crate::config::SharedConfig>();
            let preferred = cfg.read().tile_monitor_name.clone();
            match crate::tile::spawn_tile(
                &app,
                tiles.inner(),
                "DEBUG: Что такое etcd?".into(),
                "etcd — distributed key-value store на основе Raft. Используется Kubernetes \
                 как source of truth для cluster state. Strongly consistent, durable, watch-based."
                    .into(),
                preferred,
            ) {
                Ok(label) => log::info!("F7 test tile spawned: {label}"),
                Err(e) => {
                    log::warn!("F7 test tile failed: {e:#}");
                    let _ = app.emit_to(
                        "overlay",
                        "hotkey:error",
                        serde_json::json!({ "hotkey": "F7", "error": format!("{e:#}") }),
                    );
                }
            }
        });
    });

    if !warnings.is_empty() {
        log::warn!("hotkey registration warnings: {warnings:?}");
        let _ = app.emit_to("overlay", "hotkeys:warnings", &warnings);
    }

    warnings
}

/// Helper: register one shortcut, push a warning instead of bailing on conflict.
fn try_register<F>(
    app: &AppHandle,
    label: &str,
    code: Code,
    warnings: &mut Vec<String>,
    handler: F,
) where
    F: Fn() + Send + Sync + 'static,
{
    let result = app
        .global_shortcut()
        .on_shortcut(Shortcut::new(None, code), move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                handler();
            }
        });
    if let Err(e) = result {
        let msg = format!("{label}: {e}");
        log::warn!("hotkey skipped — {msg}");
        warnings.push(msg);
    }
}
