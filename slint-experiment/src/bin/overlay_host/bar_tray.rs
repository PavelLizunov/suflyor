use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[cfg(windows)]
use overlay_backend::config;
use slint::{ComponentHandle, SharedString, Timer};
use crate::ui::{OverlayBarWindow, TrayMenuWindow};

#[allow(unused_imports)]
use slint_replay::win32::{
    enum_monitors, focus_window, get_window_rect, grab_hwnd, make_transparent_overlay,
    move_window_pos_only, set_always_on_top, set_skip_taskbar, set_stealth, stealth_supported,
    work_area_for_point,
};

#[cfg(windows)]
use super::clamp_scheme;
use super::{
    apply_bar_stealth, global_stealth, realize_with_retries, set_global_stealth_effective,
    set_platform_window_position, surface_stealth_unavailable,
};

/// hidden-to-tray — the bar is hidden ONLY by an explicit action (bar chip
/// or tray menu) and restored from the tray; the flag is process-local and
/// NEVER persisted, so every startup is visible. While hidden, everything else
/// keeps running: recording, hotkeys, TTS, session tasks, tiles, F6/F9.
pub(super) static BAR_TRAY_HIDDEN: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "macos")]
pub(super) extern "C" fn sync_bar_status_visibility(visible: bool) {
    BAR_TRAY_HIDDEN.store(!visible, Ordering::Relaxed);
}

/// The hide chip is deliberately a no-op if Windows rejected the tray icon:
/// without an icon, hiding the only restore surface would strand the user.
pub(super) static TRAY_AVAILABLE: AtomicBool = AtomicBool::new(false);

/// `rt.mic_muted`), so any refresh while the mic is off must keep showing the
/// amber «mic muted» pill. Before this, a sys-chip click or the sys-probe
/// revert timer silently overwrote «mic muted» with «sys only»/«idle»,
/// hiding the privacy-relevant mute state from the user.
pub(super) fn refresh_status(overlay: &OverlayBarWindow, mic: bool, sys: bool) {
    let (text, color) = match (mic, sys) {
        (true, true) => ("recording", slint::Color::from_rgb_u8(0x34, 0xd3, 0x99)),
        (true, false) => ("mic only", slint::Color::from_rgb_u8(0x34, 0xd3, 0x99)),
        (false, _) => ("mic muted", slint::Color::from_rgb_u8(0xfb, 0xbf, 0x24)),
    };
    overlay.set_status_text(SharedString::from(text));
    overlay.set_status_color(color);
}

pub(super) fn get_mic_active(state: &slint_replay::app_state::SharedState) -> bool {
    state.lock().map(|s| s.mic_active).unwrap_or(false)
}

pub(super) fn get_sys_active(state: &slint_replay::app_state::SharedState) -> bool {
    state.lock().map(|s| s.sys_active).unwrap_or(false)
}

/// Apply transparent-overlay HWND flags to the overlay bar.
/// V0.8.0 (Поток B) — spawn a fresh copy of ourselves (with `--relaunch`) and
/// quit the current event loop so the post-`run()` teardown runs (kills the
/// possibly-hung local-AI servers; the child's `ensure_servers` then starts
/// fresh ones — this is what recovers a hung local model). The child blocks on
/// the singleton mutex until WE fully exit, so the two bars never overlap.
///
/// All persisted settings (incl. `stealth_enabled`) live in config.json, which
/// the child reloads — so the new instance comes up with the SAME stealth state
/// (and, thanks to Поток C, comes up flash-free under stealth). Returns true if
/// the child spawned (so the caller proceeds to quit); false if we couldn't
/// find/launch our own exe (then we must NOT quit — that would just close the
/// app with nothing to replace it).
pub(super) fn spawn_relaunch() -> bool {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[overlay-host] relaunch: current_exe failed: {e}; staying up");
            return false;
        }
    };
    // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP so the child is fully
    // independent of this (exiting) process and its console/group.
    #[cfg(windows)]
    let res = {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        std::process::Command::new(&exe)
            .arg("--relaunch")
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
            .spawn()
    };
    #[cfg(not(windows))]
    let res = std::process::Command::new(&exe).arg("--relaunch").spawn();
    match res {
        Ok(child) => {
            eprintln!(
                "[overlay-host] relaunch: spawned child pid={} from {:?}",
                child.id(),
                exe
            );
            true
        }
        Err(e) => {
            eprintln!("[overlay-host] relaunch: spawn failed: {e}; staying up");
            false
        }
    }
}

/// Resize the bar window for compact "reader mode" (a small read-aloud pill)
/// vs the full bar, then re-center it on the primary monitor for the new width.
/// LogicalSize matches the `.slint` preferred sizes (Slint DPI-scales it). The
/// re-pin is deferred a frame because `set_size` is processed on the next event-
/// loop cycle — reading the window rect immediately would still see the old
/// width and mis-center.
pub(super) fn apply_bar_size(overlay: &OverlayBarWindow, compact: bool) {
    let (w, h) = if compact {
        // Narrow SESSION strip (Glacier redesign): status pill + спросить +
        // захватить + timer + expand in one row, over the live-status footer —
        // two 22px rows like the full bar, just fewer chips. 680×64 also fits
        // the explicit deep-lock label without crowding; kept in sync with
        // overlay_bar.slint's compact min-width (660) + preferred-height (64).
        (680.0_f32, 64.0_f32)
    } else {
        // 64 (was 86) — matches overlay_bar.slint preferred-height; trims the
        // empty vertical band around the 22px chip row (design review #1).
        (1280.0_f32, 64.0_f32)
    };
    overlay.window().set_size(slint::LogicalSize::new(w, h));
    recenter_when_sized(overlay.as_weak(), w, 0);
}

/// Re-center the bar on the primary monitor for its NEW width — but only once the
/// OS window has actually reached that size. `set_size` is applied on a later
/// event-loop cycle, and at STARTUP the HWND isn't realized for ~200 ms, so a
/// single fixed delay raced the resize/pin (it left a startup-compact bar
/// left-of-centre). Poll every 50 ms (≤ ~0.6 s) until the rect width matches the
/// requested size in native geometry units (DPI-scaled pixels on Windows,
/// logical points on macOS), then center by the ACTUAL width.
fn recenter_when_sized(weak: slint::Weak<OverlayBarWindow>, target_w_logical: f32, attempt: u32) {
    Timer::single_shot(Duration::from_millis(50), move || {
        let Some(o) = weak.upgrade() else { return };
        let Ok(hwnd) = grab_hwnd(o.window()) else {
            if attempt < 12 {
                recenter_when_sized(weak.clone(), target_w_logical, attempt + 1);
            }
            return;
        };
        #[cfg(windows)]
        let target_width = (target_w_logical * o.window().scale_factor()).round() as i32;
        #[cfg(target_os = "macos")]
        let target_width = target_w_logical.round() as i32;
        let cur_w = get_window_rect(hwnd).map(|(_, _, bw, _)| bw).unwrap_or(0);
        if (cur_w - target_width).abs() > 24 && attempt < 12 {
            recenter_when_sized(weak.clone(), target_w_logical, attempt + 1);
            return;
        }
        let primary = enum_monitors().into_iter().find(|m| m.is_primary);
        let (x, y) = match primary {
            Some(p) => (p.left + ((p.width() - cur_w) / 2).max(0), p.top + 24),
            None => (60, 24),
        };
        let _ = move_window_pos_only(hwnd, x, y);
        set_platform_window_position(o.window(), x, y);
    });
}

#[cfg(windows)]
pub(super) fn tray_menu_action(index: i32) -> Option<slint_replay::tray::TrayAction> {
    use slint_replay::tray::TrayAction;
    match index {
        0 => Some(TrayAction::ShowHide),
        1 => Some(TrayAction::PauseResume),
        2 => Some(TrayAction::Stop),
        3 => Some(TrayAction::Quit),
        _ => None,
    }
}

#[cfg(windows)]
pub(super) fn dismiss_tray_menu(menu: &TrayMenuWindow, focus_armed: &RefCell<bool>) {
    *focus_armed.borrow_mut() = false;
    let _ = menu.hide();
    slint_replay::tray::return_focus();
}

#[cfg(not(windows))]
pub(super) fn dismiss_tray_menu(menu: &TrayMenuWindow, focus_armed: &RefCell<bool>) {
    *focus_armed.borrow_mut() = false;
    let _ = menu.hide();
}

#[cfg(windows)]
fn open_tray_menu(
    menu: &Rc<TrayMenuWindow>,
    anchor_x: i32,
    anchor_y: i32,
    state: &slint_replay::app_state::SharedState,
    cfg: &config::SharedConfig,
    focus_armed: &Rc<RefCell<bool>>,
) {
    let (paused, running) = state
        .lock()
        .map(|s| (s.paused, s.timer_active))
        .unwrap_or((false, false));
    let snapshot = slint_replay::tray::TraySnapshot {
        bar_visible: !BAR_TRAY_HIDDEN.load(Ordering::Relaxed),
        paused,
        session_running: running,
    };
    let config = cfg.read();
    let entries = slint_replay::tray::menu_entries(&snapshot, config.ui_language == "ru");
    menu.set_show_hide_label(entries[0].label.into());
    menu.set_pause_resume_label(entries[1].label.into());
    menu.set_stop_label(entries[2].label.into());
    menu.set_quit_label(entries[3].label.into());
    menu.set_session_running(running);
    menu.global::<crate::ui::Theme>()
        .set_scheme(clamp_scheme(config.color_scheme));
    drop(config);

    let Some(work) = work_area_for_point(anchor_x, anchor_y) else {
        diag!("tray menu open failed: no monitor work area");
        slint_replay::tray::return_focus();
        return;
    };
    let scale = menu.window().scale_factor().max(0.1);
    let width = (196.0 * scale).round() as i32;
    let height = (136.0 * scale).round() as i32;
    let max_x = (work.right - width).max(work.left);
    let max_y = (work.bottom - height).max(work.top);
    let x = (anchor_x - width).clamp(work.left, max_x);
    let y = (anchor_y - height - (8.0 * scale).round() as i32).clamp(work.top, max_y);

    *focus_armed.borrow_mut() = false;
    if menu.window().is_visible() {
        let _ = menu.hide();
    }
    menu.window()
        .set_position(slint::PhysicalPosition::new(-32000, -32000));
    if let Err(e) = menu.show() {
        diag!("tray menu show failed: {e}");
        slint_replay::tray::return_focus();
        return;
    }

    let reveal = Rc::new(move |window: &TrayMenuWindow| {
        let Ok(hwnd) = grab_hwnd(window.window()) else {
            return false;
        };
        if set_skip_taskbar(hwnd, true).is_err() || set_always_on_top(hwnd, true).is_err() {
            return false;
        }
        if global_stealth() && set_stealth(hwnd, true).is_err() {
            return false;
        }
        if move_window_pos_only(hwnd, x, y).is_err() {
            return false;
        }
        focus_window(hwnd);
        diag!("tray menu shown x={x} y={y}");
        true
    });
    let armed_for_fallback = focus_armed.clone();
    let fallback = Rc::new(move |window: &TrayMenuWindow| {
        *armed_for_fallback.borrow_mut() = false;
        let _ = window.hide();
        slint_replay::tray::return_focus();
        diag!("tray menu show failed: native window unavailable");
    });
    realize_with_retries(menu.as_ref(), reveal, fallback);
}

/// — hide ONLY the bar window (hide-to-tray). Same hide recipe as the
/// tile close path (`hide()` + Win32 `force_hide`, which also clears
/// secondary-monitor pixels). Nothing else stops: recording, hotkeys, TTS,
/// session tasks and tiles keep running. Never touches compact state.
pub(super) fn hide_bar_to_tray(weak: &slint::Weak<OverlayBarWindow>) {
    let Some(o) = weak.upgrade() else { return };
    #[cfg(windows)]
    if let Err(e) = slint_replay::tray::show_icon() {
        diag!("hide-to-tray ignored: notification icon failed: {e}");
        return;
    }
    let _ = o.hide();
    slint_replay::win32::force_hide(o.window());
    BAR_TRAY_HIDDEN.store(true, Ordering::Relaxed);
    diag!("bar hidden to tray (recording/hotkeys/TTS keep running)");
}

/// — restore the bar from the tray EXACTLY as it was: no resize, no
/// compact-mode change, no config write. `show_windows` re-shows without
/// stealing focus AND re-asserts the always-on-top band that hide/show drops
/// (the bar is always topmost — `always-on-top: true` in overlay_bar.slint).
#[allow(dead_code)]
pub(super) fn restore_bar_from_tray(weak: &slint::Weak<OverlayBarWindow>) {
    let Some(o) = weak.upgrade() else { return };
    BAR_TRAY_HIDDEN.store(false, Ordering::Relaxed);
    #[cfg(windows)]
    slint_replay::tray::hide_icon();
    // Slint-side show first; the bar was SW_HIDE'd out from under Slint, so
    // the Win32 re-show below is what actually makes it visible again (same
    // stale-state reasoning as `win32::reveal_window`).
    let _ = o.show();
    if let Ok(hwnd) = grab_hwnd(o.window()) {
        #[cfg(windows)]
        let raw_h = hwnd.0 as isize;
        #[cfg(not(windows))]
        let raw_h = hwnd.0;
        slint_replay::win32::show_windows(&[(raw_h, true)]);
    }
    diag!("bar restored from tray (compact mode unchanged)");
}

/// — tray menu / icon actions, all routed through the EXISTING session
/// machinery: Pause/Resume invokes the bar's pause callback, Stop invokes the
/// timer-toggle callback (guarded to STOP-only — it must never START a
/// session), Quit uses the same clean `quit_event_loop` path as the bar's ✕.
#[cfg(windows)]
pub(super) fn tray_action_dispatch(
    action: slint_replay::tray::TrayAction,
    weak: &slint::Weak<OverlayBarWindow>,
    state: &slint_replay::app_state::SharedState,
    cfg: &config::SharedConfig,
    menu: &Rc<TrayMenuWindow>,
    focus_armed: &Rc<RefCell<bool>>,
) {
    use slint_replay::tray::TrayAction;
    if !matches!(action, TrayAction::OpenMenu { .. }) {
        // Left-click activation and menu rows share one close path. This
        // prevents an open, stale "Restore" menu from surviving a left-click
        // restore and toggling the now-visible bar back to hidden.
        dismiss_tray_menu(menu.as_ref(), focus_armed.as_ref());
    }
    match action {
        TrayAction::OpenMenu { x, y } => open_tray_menu(menu, x, y, state, cfg, focus_armed),
        TrayAction::ShowHide => {
            if BAR_TRAY_HIDDEN.load(Ordering::Relaxed) {
                restore_bar_from_tray(weak);
            } else {
                hide_bar_to_tray(weak);
            }
        }
        TrayAction::PauseResume => {
            let running = state.lock().map(|s| s.timer_active).unwrap_or(false);
            if running {
                if let Some(o) = weak.upgrade() {
                    diag!("session pause/resume requested from tray");
                    o.invoke_pause_toggle_clicked();
                }
            }
        }
        TrayAction::Stop => {
            let running = state.lock().map(|s| s.timer_active).unwrap_or(false);
            if running {
                if let Some(o) = weak.upgrade() {
                    diag!("session stop requested from tray");
                    o.invoke_timer_toggle_clicked();
                }
            }
        }
        TrayAction::Quit => {
            diag!("quit confirmed (tray)");
            let _ = slint::quit_event_loop();
        }
    }
}

pub(super) fn apply_overlay_hwnd(overlay: &OverlayBarWindow, state: &slint_replay::app_state::SharedState) {
    // Поток C (stealth bar-flash fix) + I3: park the bar OFF the virtual desktop
    // synchronously NOW (this fn runs before overlay.run(), which composites the
    // window), ALWAYS — mirrors present_window_stealth_aware. Without parking the
    // bar was shown at winit's default position and only decorated/stealthed later,
    // so a screen-share saw a flash of the bar on every cold start — and would on
    // every emergency restart (Поток B). The reveal attempt wires transparency +
    // verified WDA *before* the pin moves the bar on-screen, so the first on-screen
    // frame is already complete + capture-excluded. Parking unconditionally also
    // closes the bare-outline flash for stealth-off starts (same reasoning as the
    // aux-window helper).
    set_platform_window_position(overlay.window(), -32000, -32000);
    let st = state.clone();
    let attempt: Rc<dyn Fn(&OverlayBarWindow) -> bool> = Rc::new(move |o: &OverlayBarWindow| {
        let Ok(hwnd) = grab_hwnd(o.window()) else {
            return false;
        };
        match make_transparent_overlay(hwnd) {
            Ok(()) => eprintln!("[overlay-host] overlay transparency wired"),
            Err(e) => eprintln!("[overlay-host] overlay transparency failed: {e}"),
        }
        // Surface WHY transparency may look broken: per-pixel alpha needs
        // DWM composition. If it's off (RDP / a VM without a GPU / very old
        // driver) the overlay renders OPAQUE no matter the wiring. This is
        // NOT the Windows "Transparency effects" toggle. Logged so a
        // tester's "transparency doesn't work" report is diagnosable.
        if slint_replay::win32::composition_enabled() {
            eprintln!("[overlay-host] DWM composition: ON (overlay transparency available)");
        } else {
            eprintln!("[overlay-host] DWM composition: OFF — overlay renders OPAQUE (no per-pixel alpha). Cause is the environment (RDP/remote, a VM without a GPU, or an outdated GPU driver), not the app. NB: NOT the Windows 'Transparency effects' toggle.");
        }
        // #E10.2 + I1 — apply persisted stealth to the bar with a readback;
        // the 🎯 chip + effective global follow the VERIFIED outcome (a failed
        // exclusion leaves the chip dark + surfaces "stealth unavailable").
        apply_bar_stealth(o, &st, global_stealth());
        // #127 — pin the bar to the PRIMARY monitor. The bar has no
        // position logic of its own; Slint/winit's default placement
        // can drop it onto the user's PORTRAIT secondary (at negative
        // X) or straddle two displays. Centre it near the top of
        // primary. One-shot at launch — the user can still drag it
        // afterward (the logo is a drag handle).
        // Поток C — the pin MUST always land the bar on-screen: we parked it
        // at (-32000) above, so any path that skips the move would strand the
        // bar off the desktop (the bar is the whole control surface — the user
        // would be locked out). Compute the target with safe fallbacks
        // (primary monitor → its origin → (60, 24)) and ALWAYS move.
        let primary = enum_monitors().into_iter().find(|m| m.is_primary);
        let bar_w = get_window_rect(hwnd).map(|(_, _, w, _)| w).unwrap_or(0);
        let (x, y) = match primary {
            Some(p) => (p.left + ((p.width() - bar_w) / 2).max(0), p.top + 24),
            None => (60, 24),
        };
        set_platform_window_position(o.window(), x, y);
        // I3 — success means the bar actually landed on-screen. If BOTH the
        // computed pin and the hard (60,24) retry fail, report the attempt as
        // FAILED so realize_with_retries keeps retrying and eventually runs
        // the Slint fallback below — a parked bar must never stay invisible
        // at (-32000) behind a claimed success.
        match move_window_pos_only(hwnd, x, y) {
            Ok(()) => {
                eprintln!("[overlay-host] bar pinned at ({x}, {y})");
                true
            }
            Err(e) => {
                // Last resort: even the pin failed — try a hard (60,24) so
                // a parked bar can't stay invisible at (-32000).
                eprintln!("[overlay-host] bar pin failed: {e}; retry at (60,24)");
                set_platform_window_position(o.window(), 60, 24);
                match move_window_pos_only(hwnd, 60, 24) {
                    Ok(()) => {
                        eprintln!("[overlay-host] bar pinned at hard fallback (60, 24)");
                        true
                    }
                    Err(e2) => {
                        eprintln!(
                            "[overlay-host] bar hard pin failed too: {e2}; reveal will retry"
                        );
                        false
                    }
                }
            }
        }
    });
    // I3 — last-ditch reveal when the HWND NEVER becomes grabbable: the bar is
    // the whole control surface, so (unlike aux windows) it is brought on-screen
    // via Slint's own positioning EVEN UNDER stealth — and stealth is reported
    // UNAVAILABLE, because without an HWND the exclusion could not be applied or
    // verified (I1: never present a false success). The user keeps control; the
    // log + pill surface the failure.
    let fallback: Rc<dyn Fn(&OverlayBarWindow)> = Rc::new(move |o: &OverlayBarWindow| {
        set_global_stealth_effective(false);
        o.set_stealth_active(false);
        if stealth_supported() && global_stealth() {
            surface_stealth_unavailable(o);
        }
        diag!(
            "[overlay-host] bar HWND never realized after retries; revealing via Slint fallback \
             (stealth NOT verified — reported unavailable)"
        );
        let primary = enum_monitors().into_iter().find(|m| m.is_primary);
        let (x, y) = match primary {
            Some(p) => (p.left + 60, p.top + 24),
            None => (60, 24),
        };
        set_platform_window_position(o.window(), x, y);
    });
    // The SAME retry/fallback schedule as the aux windows (I3) — fast attempt,
    // two conservative retries, then the fallback above. No private timer loop.
    realize_with_retries(overlay, attempt, fallback);
}
