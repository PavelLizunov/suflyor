//! Phase 1 Day 2 + Phase 3 — multi-window manager with real overlay bar.
//!
//! Spawns the overlay bar with a full chip set (status pill, mic/sys
//! capture chips, session timer, AI model selector, cost, tips,
//! bookmark, stealth, +Tile, ⚙ Settings, ✕ Quit).
//!
//! All callbacks update the shared AppState. Stealth toggle applies
//! WDA_EXCLUDEFROMCAPTURE to overlay + all open tiles via win32 helpers.
//! Tile spawn uses pick_monitor + move_window for proper multi-monitor
//! placement (respects user's portrait-secondary setup).
//!
//! Run: `cargo run --bin overlay-host` from `slint-experiment/`.

use slint::{ComponentHandle, SharedString, Timer, TimerMode};
use slint_replay::app_state::{format_timer, new_shared_state, next_model};
use slint_replay::win32::{
    enum_monitors, grab_hwnd, make_transparent_overlay, move_window, pick_monitor,
    set_always_on_top, set_stealth,
};
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

type TileWindows = Rc<RefCell<Vec<TileWindow>>>;

fn main() -> Result<(), slint::PlatformError> {
    let state = new_shared_state();
    let tiles: TileWindows = Rc::new(RefCell::new(Vec::new()));
    let settings: Rc<RefCell<Option<SettingsWindow>>> = Rc::new(RefCell::new(None));

    let overlay = OverlayBarWindow::new()?;
    overlay.set_status_text(SharedString::from("idle"));
    overlay.set_status_color(slint::Color::from_rgb_u8(0x88, 0x88, 0x8c));
    overlay.set_ai_model(SharedString::from("sonnet"));
    overlay.set_cost_label(SharedString::from("$0.000"));
    overlay.set_timer_label(SharedString::from("00:00"));

    apply_overlay_hwnd(&overlay);

    // ===== Mic chip =====
    {
        let s = state.clone();
        let weak = overlay.as_weak();
        overlay.on_mic_toggle_clicked(move || {
            let new_active = {
                let mut st = match s.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                st.mic_active = !st.mic_active;
                st.mic_active
            };
            if let Some(o) = weak.upgrade() {
                o.set_mic_active(new_active);
                refresh_status(&o, new_active, get_sys_active(&s));
            }
        });
    }

    // ===== System chip =====
    {
        let s = state.clone();
        let weak = overlay.as_weak();
        overlay.on_sys_toggle_clicked(move || {
            let new_active = {
                let mut st = match s.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                st.sys_active = !st.sys_active;
                st.sys_active
            };
            if let Some(o) = weak.upgrade() {
                o.set_sys_active(new_active);
                refresh_status(&o, get_mic_active(&s), new_active);
            }
        });
    }

    // ===== Session timer =====
    {
        let s = state.clone();
        let weak = overlay.as_weak();
        overlay.on_timer_toggle_clicked(move || {
            let new_active = {
                let mut st = match s.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                st.timer_active = !st.timer_active;
                if !st.timer_active {
                    st.session_secs = 0;
                }
                st.timer_active
            };
            if let Some(o) = weak.upgrade() {
                o.set_timer_active(new_active);
                if !new_active {
                    o.set_timer_label(SharedString::from("00:00"));
                }
            }
        });
    }

    // Periodic timer (every 1 s) — updates the session-timer label
    // when active. Slint Timer::default() with `start(Repeated, ...)`
    // pattern.
    let tick_state = state.clone();
    let tick_weak = overlay.as_weak();
    let tick_timer = Timer::default();
    tick_timer.start(TimerMode::Repeated, Duration::from_secs(1), move || {
        let (active, secs) = {
            let mut st = match tick_state.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            if st.timer_active {
                st.session_secs += 1;
            }
            (st.timer_active, st.session_secs)
        };
        if active {
            if let Some(o) = tick_weak.upgrade() {
                o.set_timer_label(SharedString::from(format_timer(secs)));
            }
        }
    });

    // ===== AI model cycle =====
    {
        let s = state.clone();
        let weak = overlay.as_weak();
        overlay.on_ai_model_cycle_clicked(move || {
            let new_model = {
                let mut st = match s.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                st.ai_model = next_model(&st.ai_model).to_string();
                st.ai_model.clone()
            };
            if let Some(o) = weak.upgrade() {
                o.set_ai_model(SharedString::from(new_model));
            }
        });
    }

    // ===== Bookmark / Tips (stubs) =====
    overlay.on_bookmark_clicked(|| eprintln!("[overlay-host] bookmark clicked (stub)"));
    overlay.on_tips_clicked(|| eprintln!("[overlay-host] tips clicked (stub)"));

    // ===== Stealth toggle on overlay bar =====
    {
        let s = state.clone();
        let tiles_ref = tiles.clone();
        let weak = overlay.as_weak();
        overlay.on_stealth_toggle_clicked(move || {
            let new_stealth = {
                let mut st = match s.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                st.stealth = !st.stealth;
                st.stealth
            };
            eprintln!("[overlay-host] stealth -> {new_stealth}");
            // Apply to overlay
            if let Some(o) = weak.upgrade() {
                if let Ok(hwnd) = grab_hwnd(o.window()) {
                    let _ = set_stealth(hwnd, new_stealth);
                }
            }
            // Apply to all tiles
            for t in tiles_ref.borrow().iter() {
                if let Ok(hwnd) = grab_hwnd(t.window()) {
                    let _ = set_stealth(hwnd, new_stealth);
                }
            }
        });
    }

    // ===== Spawn tile =====
    {
        let s = state.clone();
        let t = tiles.clone();
        let weak = overlay.as_weak();
        overlay.on_spawn_tile_clicked(move || {
            let Some(overlay) = weak.upgrade() else {
                return;
            };
            let seq = {
                let mut st = match s.lock() {
                    Ok(g) => g,
                    Err(p) => p.into_inner(),
                };
                st.tiles_spawned += 1;
                st.tiles_spawned
            };
            overlay.set_tiles_spawned(seq as i32);

            match TileWindow::new() {
                Ok(tile) => {
                    tile.set_sequence(seq as i32);
                    tile.set_tile_title(SharedString::from(format!("Tile #{seq}")));
                    tile.set_tile_body(SharedString::from(
                        "Phase 4 will render markdown via the pulldown-cmark adapter. \
                         This tile demonstrates the multi-window plumbing — each new \
                         tile is its own native window with HWND-managed transparency \
                         + click-through, sharing AppState via Arc<Mutex<...>>.",
                    ));

                    let weak_tile = tile.as_weak();
                    tile.on_close_clicked(move || {
                        if let Some(t) = weak_tile.upgrade() {
                            let _ = t.hide();
                        }
                    });

                    let _ = tile.show();
                    apply_tile_hwnd_with_monitor(&tile);
                    t.borrow_mut().push(tile);
                }
                Err(e) => eprintln!("[overlay-host] TileWindow::new failed: {e}"),
            }
        });
    }

    // ===== Settings =====
    {
        let s = state.clone();
        let settings_ref = settings.clone();
        let tiles_ref = tiles.clone();
        overlay.on_open_settings_clicked(move || {
            open_settings(&s, &settings_ref, &tiles_ref);
        });
    }

    // ===== Quit =====
    overlay.on_quit_clicked(|| {
        eprintln!("[overlay-host] quit requested");
        let _ = slint::quit_event_loop();
    });

    overlay.run()
}

/// Recompute status pill based on capture flags.
fn refresh_status(overlay: &OverlayBarWindow, mic: bool, sys: bool) {
    let (text, color) = match (mic, sys) {
        (true, true) => ("recording 🎤🗣", slint::Color::from_rgb_u8(0x34, 0xd3, 0x99)),
        (true, false) => ("mic only 🎤", slint::Color::from_rgb_u8(0x34, 0xd3, 0x99)),
        (false, true) => ("sys only 🗣", slint::Color::from_rgb_u8(0x6c, 0xcf, 0xff)),
        (false, false) => ("idle", slint::Color::from_rgb_u8(0x88, 0x88, 0x8c)),
    };
    overlay.set_status_text(SharedString::from(text));
    overlay.set_status_color(color);
}

fn get_mic_active(state: &slint_replay::app_state::SharedState) -> bool {
    state.lock().map(|s| s.mic_active).unwrap_or(false)
}

fn get_sys_active(state: &slint_replay::app_state::SharedState) -> bool {
    state.lock().map(|s| s.sys_active).unwrap_or(false)
}

/// Apply transparent-overlay HWND flags to the overlay bar.
fn apply_overlay_hwnd(overlay: &OverlayBarWindow) {
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

/// Apply transparency + position tile on the appropriate monitor.
/// Uses pick_monitor to respect the user's portrait-secondary setup
/// (default to primary unless non-primary is landscape + at-least-as-wide).
fn apply_tile_hwnd_with_monitor(tile: &TileWindow) {
    let weak = tile.as_weak();
    Timer::single_shot(Duration::from_millis(200), move || {
        let Some(t) = weak.upgrade() else { return };
        let Ok(hwnd) = grab_hwnd(t.window()) else {
            return;
        };

        let _ = make_transparent_overlay(hwnd);

        // Position on the user's chosen monitor. For Day 2 stub: just
        // center on the primary monitor. Phases 3+ tile grid logic
        // would compute (x,y) from monitor + grid slot.
        let monitors = enum_monitors();
        if let Some(mon) = pick_monitor(&monitors) {
            let tile_w = 440;
            let tile_h = 260;
            let x = mon.left + (mon.width() - tile_w) / 2;
            let y = mon.top + (mon.height() - tile_h) / 2;
            let _ = move_window(hwnd, x, y, tile_w, tile_h);
        }
    });
}

/// Open the settings window. Reuses existing instance if open.
fn open_settings(
    state: &slint_replay::app_state::SharedState,
    settings_ref: &Rc<RefCell<Option<SettingsWindow>>>,
    tiles_ref: &TileWindows,
) {
    let mut settings_slot = settings_ref.borrow_mut();
    if let Some(existing) = settings_slot.as_ref() {
        let _ = existing.show();
        return;
    }
    let win = match SettingsWindow::new() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[overlay-host] SettingsWindow::new failed: {e}");
            return;
        }
    };
    {
        let st = state.lock().ok();
        if let Some(st) = st {
            win.set_always_on_top_toggle(st.always_on_top);
            win.set_stealth_toggle(st.stealth);
        }
    }

    let s2 = state.clone();
    let tiles_ref2 = tiles_ref.clone();
    win.on_always_on_top_changed(move |on| {
        if let Ok(mut st) = s2.lock() {
            st.always_on_top = on;
        }
        for t in tiles_ref2.borrow().iter() {
            if let Ok(hwnd) = grab_hwnd(t.window()) {
                let _ = set_always_on_top(hwnd, on);
            }
        }
    });

    let s3 = state.clone();
    let tiles_ref3 = tiles_ref.clone();
    win.on_stealth_changed(move |on| {
        if let Ok(mut st) = s3.lock() {
            st.stealth = on;
        }
        for t in tiles_ref3.borrow().iter() {
            if let Ok(hwnd) = grab_hwnd(t.window()) {
                let _ = set_stealth(hwnd, on);
            }
        }
    });

    let weak_close = win.as_weak();
    let settings_close = settings_ref.clone();
    win.on_close_clicked(move || {
        if let Some(w) = weak_close.upgrade() {
            let _ = w.hide();
        }
        *settings_close.borrow_mut() = None;
    });

    let _ = win.show();
    *settings_slot = Some(win);
}
