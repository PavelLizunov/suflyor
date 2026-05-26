//! Global hotkey registration. Reads keybinds from Config and
//! emits Tauri events that the frontend listens to.
//!
//! Each shortcut is registered independently — a single conflict (e.g.
//! F12 captured by another app) does NOT abort the whole batch. The list
//! of failures is returned + emitted as `hotkeys:warnings` so the UI
//! can show a warning indicator.

use crate::config::SharedConfig;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

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

    // v0.0.80: F2 — cycle through saved context profiles. Emits an
    // event with the next profile's name; frontend handles applying it
    // (so the toast / Settings update / etc. stay in JS-land). If no
    // profiles are configured, the event payload is null and the
    // frontend shows a hint toast.
    let app_h = app.clone();
    try_register(app, "F2 (profile cycle)", Code::F2, &mut warnings, move || {
        let app = app_h.clone();
        if let Some(cfg) = app.try_state::<crate::config::SharedConfig>() {
            let (next_name, next_context): (Option<String>, Option<String>) = {
                let c = cfg.read();
                if c.context_profiles.is_empty() {
                    (None, None)
                } else {
                    // Cycle: current → next (wrap). If active is None,
                    // pick the first profile.
                    let idx = c.active_profile.as_deref()
                        .and_then(|name| c.context_profiles.iter().position(|p| p.name == name));
                    let next_idx = idx.map(|i| (i + 1) % c.context_profiles.len()).unwrap_or(0);
                    let p = &c.context_profiles[next_idx];
                    (Some(p.name.clone()), Some(p.context.clone()))
                }
            };
            if let (Some(name), Some(context)) = (next_name.as_ref(), next_context.as_ref()) {
                // Persist the switch — same as Settings profile picker would do.
                {
                    let mut c = cfg.write();
                    c.active_profile = Some(name.clone());
                    c.meeting_context = context.clone();
                }
                let snap = cfg.read().clone();
                if let Err(e) = crate::config::save(&snap) {
                    log::warn!("F2 profile cycle save failed: {e:#}");
                }
            }
            let _ = app.emit_to(
                "overlay",
                "hotkey:profile-cycled",
                serde_json::json!({ "name": next_name }),
            );
        }
    });

    // v0.0.83: F7 — emit a toggle event for bulk-collapse tiles. The
    // overlay frontend mirrors the 📦 chip click handler, flipping
    // `allCollapsed` and emitting `tile:collapse-all` or
    // `tile:expand-all` to all windows.
    let app_h = app.clone();
    try_register(app, "F7 (collapse all tiles)", Code::F7, &mut warnings, move || {
        let _ = app_h.emit_to("overlay", "hotkey:collapse-all", ());
    });

    // v0.0.77: F1 — toggle the hotkey-help popover. Same UX as clicking
    // the ℹ button in the overlay bar. Useful when the user forgets
    // which keys are bound and wants the cheatsheet on top of their
    // current view. Emits an event so the frontend can toggle its
    // existing hotkeyHelpOpen state (which already handles window
    // resize + outside-click dismiss).
    let app_h = app.clone();
    try_register(app, "F1 (help)", Code::F1, &mut warnings, move || {
        let _ = app_h.emit_to("overlay", "hotkey:help", ());
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

    // v0.0.24: Ctrl+Alt+W — close every unpinned tile in one shot.
    // Useful when aggressive mode floods the screen with answers and the
    // user wants a clean slate without quitting the whole session.
    // F-key collisions: F1-F11 are already taken; F12 is reserved by
    // Windows. Chord modifier avoids both. W = "close all Windows".
    let app_h = app.clone();
    let result = app
        .global_shortcut()
        .on_shortcut(
            Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyW),
            move |_a, _s, event| {
                if event.state == ShortcutState::Pressed {
                    let app = app_h.clone();
                    tauri::async_runtime::spawn(async move {
                        let tiles = app.state::<crate::tile::SharedTiles>();
                        let n = crate::tile::close_all_unpinned(&app, tiles.inner());
                        log::info!("Ctrl+Alt+W: closed {n} tile(s)");
                    });
                }
            },
        );
    if let Err(e) = result {
        let msg = format!("Ctrl+Alt+W (close all tiles): {e}");
        log::warn!("hotkey skipped — {msg}");
        warnings.push(msg);
    }

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
