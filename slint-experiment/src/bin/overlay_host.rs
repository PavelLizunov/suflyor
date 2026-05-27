//! Phase 1 Day 2 — multi-window manager skeleton.
//!
//! Spawns the overlay bar window. Overlay's "+ Tile" callback creates
//! a new TileWindow and shows it. "⚙ Settings" opens a SettingsWindow
//! (toggles always-on-top + stealth via win32 helpers). All windows
//! share a single `Arc<Mutex<AppState>>`.
//!
//! Auto-applies transparent-overlay HWND wiring (Phase 1 Day 1 DWM
//! pattern) to overlay + tile windows. Settings is a normal window.
//!
//! Run: `cargo run --bin overlay-host` from `slint-experiment/`.

use slint::{ComponentHandle, SharedString, Timer};
use slint_replay::app_state::{new_shared_state, SharedState};
use slint_replay::win32::{grab_hwnd, make_transparent_overlay, set_always_on_top, set_stealth};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::pedantic,
    clippy::nursery,
    clippy::all
)]
mod ui {
    slint::include_modules!();
}

use ui::{OverlayBarWindow, SettingsWindow, TileWindow};

/// Holds weak refs to all child windows so the manager can apply
/// global commands (set_stealth-on-all, hide-all-for-quit) without
/// owning strong references that prevent natural cleanup.
type TileWindows = Rc<RefCell<Vec<TileWindow>>>;

fn main() -> Result<(), slint::PlatformError> {
    let state = new_shared_state();
    let tiles: TileWindows = Rc::new(RefCell::new(Vec::new()));
    let settings: Rc<RefCell<Option<SettingsWindow>>> = Rc::new(RefCell::new(None));

    let overlay = OverlayBarWindow::new()?;

    // Apply transparent-overlay HWND wiring 200 ms after show, once
    // winit has realized the native window.
    apply_overlay_hwnd(&overlay, state.clone());

    // ----- Overlay callbacks -----

    {
        let s = state.clone();
        let t = tiles.clone();
        let weak = overlay.as_weak();
        overlay.on_spawn_tile_clicked(move || {
            let Some(overlay) = weak.upgrade() else { return };
            let seq = {
                let mut st = match s.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                st.tiles_spawned += 1;
                st.tiles_spawned
            };
            overlay.set_tiles_spawned(seq as i32);

            // Spawn a new TileWindow. Apply transparent overlay flags.
            // Each tile is independent (own native window).
            match TileWindow::new() {
                Ok(tile) => {
                    tile.set_sequence(seq as i32);
                    tile.set_tile_title(SharedString::from(format!("Tile #{seq}")));
                    tile.set_tile_body(SharedString::from(
                        "Phase 4 will render markdown bodies via the pulldown-cmark adapter (spike 3). This stub just demonstrates the multi-window plumbing — spawn a new native window from a callback, share AppState via Arc<Mutex<...>>.",
                    ));

                    // Apply DWM transparent overlay wiring to this tile too.
                    apply_tile_hwnd(&tile);

                    // Close handler.
                    let weak_tile = tile.as_weak();
                    tile.on_close_clicked(move || {
                        if let Some(t) = weak_tile.upgrade() {
                            let _ = t.hide();
                        }
                    });

                    let _ = tile.show();
                    t.borrow_mut().push(tile);
                }
                Err(e) => eprintln!("[overlay-host] failed to create TileWindow: {e}"),
            }
        });
    }

    {
        let s = state.clone();
        let settings_ref = settings.clone();
        let tiles_ref = tiles.clone();
        overlay.on_open_settings_clicked(move || {
            let mut settings_slot = settings_ref.borrow_mut();
            if let Some(existing) = settings_slot.as_ref() {
                // Reuse the existing window, just bring it forward.
                let _ = existing.show();
                return;
            }
            match SettingsWindow::new() {
                Ok(win) => {
                    // Initialize toggles from current state.
                    {
                        let st = match s.lock() {
                            Ok(g) => g,
                            Err(p) => p.into_inner(),
                        };
                        win.set_always_on_top_toggle(st.always_on_top);
                        win.set_stealth_toggle(st.stealth);
                    }

                    // Wire callbacks. Each one updates AppState +
                    // applies the corresponding HWND change to the
                    // overlay AND all open tiles.
                    let s2 = s.clone();
                    let tiles_ref2 = tiles_ref.clone();
                    win.on_always_on_top_changed(move |on| {
                        {
                            let mut st = match s2.lock() {
                                Ok(g) => g,
                                Err(p) => p.into_inner(),
                            };
                            st.always_on_top = on;
                        }
                        // Apply to all open tiles (the overlay is a
                        // separate handle held by the outer scope; we
                        // could wire it via another weak ref but the
                        // overlay's `always-on-top: true` Slint property
                        // is the canonical source here).
                        for t in tiles_ref2.borrow().iter() {
                            if let Ok(hwnd) = grab_hwnd(t.window()) {
                                let _ = set_always_on_top(hwnd, on);
                            }
                        }
                    });

                    let s3 = s.clone();
                    let tiles_ref3 = tiles_ref.clone();
                    win.on_stealth_changed(move |on| {
                        {
                            let mut st = match s3.lock() {
                                Ok(g) => g,
                                Err(p) => p.into_inner(),
                            };
                            st.stealth = on;
                        }
                        for t in tiles_ref3.borrow().iter() {
                            if let Ok(hwnd) = grab_hwnd(t.window()) {
                                let _ = set_stealth(hwnd, on);
                            }
                        }
                    });

                    let weak_close = win.as_weak();
                    let settings_ref_close = settings_ref.clone();
                    win.on_close_clicked(move || {
                        if let Some(w) = weak_close.upgrade() {
                            let _ = w.hide();
                        }
                        // Drop our reference so a fresh open creates a new window.
                        *settings_ref_close.borrow_mut() = None;
                    });

                    let _ = win.show();
                    *settings_slot = Some(win);
                }
                Err(e) => eprintln!("[overlay-host] failed to create SettingsWindow: {e}"),
            }
        });
    }

    {
        overlay.on_quit_clicked(|| {
            eprintln!("[overlay-host] quit requested");
            let _ = slint::quit_event_loop();
        });
    }

    overlay.run()
}

/// Apply transparent-overlay HWND flags to the overlay bar after the
/// 200 ms paint settle.
fn apply_overlay_hwnd(overlay: &OverlayBarWindow, _state: SharedState) {
    let weak = overlay.as_weak();
    Timer::single_shot(Duration::from_millis(200), move || {
        let Some(o) = weak.upgrade() else { return };
        match grab_hwnd(o.window()) {
            Ok(hwnd) => match make_transparent_overlay(hwnd) {
                Ok(()) => eprintln!("[overlay-host] overlay transparency wired"),
                Err(e) => eprintln!("[overlay-host] overlay transparency failed: {e}"),
            },
            Err(e) => eprintln!("[overlay-host] overlay HWND grab failed: {e}"),
        }
    });
}

/// Apply transparent-overlay HWND flags to a freshly-spawned tile.
fn apply_tile_hwnd(tile: &TileWindow) {
    let weak = tile.as_weak();
    Timer::single_shot(Duration::from_millis(200), move || {
        let Some(t) = weak.upgrade() else { return };
        if let Ok(hwnd) = grab_hwnd(t.window()) {
            let _ = make_transparent_overlay(hwnd);
        }
    });
}
