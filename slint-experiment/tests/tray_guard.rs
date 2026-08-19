//! hidden-to-tray guards (bar action + tray menu routing + contracts).
//!
//! Pure file parsing, no UI build — same style as i18n_guard / rc3 guards.
//! The runtime state/menu/label unit tests live next to the code they guard
//! (`src/tray.rs`); this file pins the CROSS-FILE contracts that a partial
//! edit could silently break:
//!
//! - the bar action exists in BOTH layouts (wide + compact),
//! - tray Pause/Resume/Stop route through the EXISTING bar callbacks instead
//!   of re-implementing session lifecycle,
//! - tray Quit uses the existing clean `quit_event_loop` path,
//! - the hidden flag starts false (visible) and is only set inside the hide
//!   helper (no stray hide path at startup),
//! - hidden state is never persisted (no config write in `tray.rs`),
//! - the icon asset exists (icon_guard pins its grid/stroke).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::fs;
use std::path::Path;

fn read(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

#[test]
fn hide_action_present_in_both_bar_layouts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let bar = read(root, "ui/overlay_bar.slint");
    assert!(
        bar.contains("callback hide-to-tray-clicked();"),
        "the bar must declare the hide-to-tray callback"
    );
    assert!(
        bar.contains("in property <bool> tray-available: false;"),
        "the hide action must stay unavailable until tray installation succeeds"
    );
    let invokes = bar.matches("root.hide-to-tray-clicked();").count();
    assert_eq!(
        invokes, 2,
        "hide-to-tray must be wired in BOTH the wide and compact layouts"
    );
    let icons = bar.matches("icons/tray.svg").count();
    assert_eq!(icons, 2, "both layouts use the shared tray icon");
}

#[test]
fn tray_actions_route_through_existing_bar_callbacks() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let host = read(root, "src/bin/overlay_host_windows.rs");

    // Session actions must reuse the bar's own callbacks (single lifecycle
    // implementation — the tray must never start/stop sessions itself).
    assert!(
        host.contains("o.invoke_pause_toggle_clicked();"),
        "tray Pause/Resume must invoke the bar's pause callback"
    );
    assert!(
        host.contains("o.invoke_timer_toggle_clicked();"),
        "tray Stop must invoke the bar's timer-toggle callback"
    );

    // Stop must be guarded by the live session flag (the toggle callback
    // would otherwise START a session from the tray).
    let dispatch_start = host
        .find("fn tray_action_dispatch(")
        .expect("tray dispatch helper");
    let dispatch_end = host
        .find("fn apply_overlay_hwnd(")
        .expect("helper after dispatch");
    let dispatch = &host[dispatch_start..dispatch_end];
    assert!(
        dispatch.matches("s.timer_active").count() >= 2,
        "Pause/Resume and Stop both check timer_active before invoking"
    );
    assert!(
        dispatch.contains("slint::quit_event_loop()"),
        "tray Quit must use the existing clean event-loop shutdown path"
    );
    assert!(
        !dispatch.contains("stop_session"),
        "the tray path must not call stop_session directly (teardown owns it)"
    );
}

#[test]
fn startup_is_always_visible_and_hide_is_explicit_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let host = read(root, "src/bin/overlay_host_windows.rs");
    assert!(
        host.contains("static BAR_TRAY_HIDDEN: AtomicBool = AtomicBool::new(false);"),
        "the hidden flag must start false — every startup is visible"
    );
    assert!(
        host.contains("if TRAY_AVAILABLE.load(Ordering::Relaxed)"),
        "the bar must not hide when Windows rejected the only restore surface"
    );
    assert!(
        host.contains("o.set_tray_available(available);"),
        "the tray callback must keep the hide-chip availability current"
    );
    assert!(
        host.contains("if !available && BAR_TRAY_HIDDEN.load(Ordering::Relaxed)"),
        "losing the tray icon while hidden must restore the bar"
    );
    assert!(
        host.contains("restore_bar_from_tray(&weak_for_availability);"),
        "tray-loss recovery must reuse the normal restore path"
    );
    assert_eq!(
        host.matches("BAR_TRAY_HIDDEN.store(true").count(),
        1,
        "exactly ONE path hides the bar (hide_bar_to_tray)"
    );
    assert_eq!(
        host.matches("fn hide_bar_to_tray(").count(),
        1,
        "single hide helper"
    );
    // Chip handler + tray dispatch are the only hide callers.
    assert_eq!(
        host.matches("hide_bar_to_tray(").count(),
        3, // definition + bar chip handler + tray ShowHide dispatch
        "hide is reachable only from the explicit chip and the tray"
    );
}

#[test]
fn restore_keeps_compact_mode_and_icon_lifecycle_is_clean() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let host = read(root, "src/bin/overlay_host_windows.rs");

    // Restore must not resize / re-compact the bar.
    let restore_start = host
        .find("fn restore_bar_from_tray(")
        .expect("restore helper");
    let restore_end = host
        .find("fn tray_action_dispatch(")
        .expect("helper after restore");
    let restore = &host[restore_start..restore_end];
    assert!(
        !restore.contains("apply_bar_size"),
        "restore must not touch the compact sizing"
    );
    assert!(
        !restore.contains("config::save"),
        "restore must not persist anything"
    );
    assert!(
        restore.contains("slint_replay::tray::hide_icon();"),
        "restoring the bar must remove its temporary notification icon"
    );

    // Tray restore surface lifecycle: installed before the event loop, dropped
    // right after it returns so any temporary icon is removed on shutdown.
    let install_pos = host
        .find("slint_replay::tray::install(")
        .expect("tray install");
    let show_pos = host.find("overlay.show()?").expect("initial bar show");
    let run_pos = host
        .find("slint::run_event_loop_until_quit()")
        .expect("tray-safe event loop run");
    let drop_pos = host.find("drop(_tray_handle)").expect("tray drop");
    assert!(
        install_pos < show_pos,
        "the tray installs before the bar shows"
    );
    assert!(
        show_pos < run_pos,
        "the bar is shown before entering the tray-safe event loop"
    );
    assert!(
        drop_pos > run_pos,
        "the tray restore surface is removed after the loop exits"
    );
    assert!(
        !host.contains("let result = overlay.run();"),
        "ordinary ComponentHandle::run exits when hide-to-tray hides the last window"
    );
}

#[test]
fn tray_module_never_persists_state() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tray = read(root, "src/tray.rs");
    assert!(
        !tray.contains("config::save"),
        "tray.rs must not write config — hidden state is explicit-only"
    );
    assert!(
        !tray.contains("compact_bar"),
        "tray.rs must not touch compact mode"
    );
    assert!(
        tray.contains("Shell_NotifyIconW(NIM_DELETE"),
        "the drop path must remove the icon (no lingering tray entries)"
    );
    assert!(
        tray.contains("Shell_NotifyIconW(NIM_SETVERSION"),
        "the tray must opt into the accessible current notification protocol"
    );
    assert!(
        tray.contains("TaskbarCreated"),
        "the restore icon must survive an Explorer/taskbar restart"
    );
    assert!(
        tray.contains("TRAY_ICON_VISIBLE: AtomicBool = AtomicBool::new(false)"),
        "the icon must start absent while the bar is visible"
    );
    assert!(
        tray.contains("pub fn show_icon()") && tray.contains("pub fn hide_icon()"),
        "hide/restore must own the temporary icon lifecycle"
    );
    assert!(
        tray.contains("claim_install_slot"),
        "single-icon guard prevents duplicate icons within one process"
    );
    assert!(
        tray.contains("WM_RBUTTONUP | WM_CONTEXTMENU => request_tray_menu()"),
        "legacy and v4 Explorer context events must share one menu path"
    );
    assert!(
        tray.contains("arm_menu_request(&TRAY_MENU_REQUEST_PENDING)"),
        "duplicate legacy + v4 callbacks must arm only one async menu"
    );
    assert!(
        tray.contains("GetCursorPos(&mut point)"),
        "the menu must use a valid cursor position instead of decoding an undefined anchor"
    );
    assert!(
        tray.contains("TrayAction::OpenMenu"),
        "right click must route to the host's themed Slint menu"
    );
    assert!(
        !tray.contains("TrackPopupMenu"),
        "the unstyled native popup must not return"
    );
    assert!(
        tray.contains("NIN_SELECT | NIN_KEYSELECT => dispatch_from_ctx(TrayAction::ShowHide)"),
        "notify-icon v4 activation must restore the bar exactly once"
    );
    assert!(
        !tray.contains("WM_LBUTTONUP | NIN_SELECT"),
        "handling both mouse-up and NIN_SELECT can toggle restore twice"
    );
    assert!(
        !tray.contains("point_from_wparam"),
        "WM_CONTEXTMENU wparam is undefined under notify-icon v4"
    );

    let request_start = tray
        .find("fn request_tray_menu()")
        .expect("menu request helper");
    let request_end = tray[request_start..]
        .find("#[cfg(test)]")
        .map(|offset| request_start + offset)
        .expect("tests after request helper");
    assert!(
        !tray[request_start..request_end].contains("NIM_SETFOCUS"),
        "opening the custom window must not immediately steal its focus back"
    );
    assert!(
        tray.contains("pub fn return_focus()"),
        "Shell focus bookkeeping returns only after the menu operation completes"
    );
}

#[test]
fn every_non_open_tray_action_closes_the_styled_menu_first() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let host = read(root, "src/bin/overlay_host_windows.rs");
    let start = host
        .find("fn tray_action_dispatch(")
        .expect("tray dispatch helper");
    let end = host[start..]
        .find("fn apply_overlay_hwnd(")
        .map(|offset| start + offset)
        .expect("helper after dispatch");
    let dispatch = &host[start..end];
    assert!(dispatch.contains("if !matches!(action, TrayAction::OpenMenu { .. })"));
    assert!(dispatch.contains("dismiss_tray_menu(menu.as_ref(), focus_armed.as_ref());"));
    assert!(
        host.contains("slint_replay::tray::return_focus();"),
        "dismissal must complete the notification-area focus lifecycle"
    );
}

#[test]
fn themed_menu_stays_above_and_clear_of_the_taskbar() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let host = read(root, "src/bin/overlay_host_windows.rs");
    let menu = read(root, "ui/tray_menu.slint");
    assert!(menu.contains("export component TrayMenuWindow inherits Window"));
    assert!(menu.contains("always-on-top: true;"));
    assert!(menu.lines().any(|line| line.trim() == "width: 196px;"));
    assert!(menu.lines().any(|line| line.trim() == "height: 136px;"));
    assert!(host.contains("work_area_for_point(anchor_x, anchor_y)"));
    assert!(host.contains("set_always_on_top(hwnd, true)"));
    assert!(host.contains("set_skip_taskbar(hwnd, true)"));
}

#[test]
fn tray_icon_asset_exists() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(
        root.join("assets/icons/tray.svg").exists(),
        "the hide-to-tray chip icon must exist (icon_guard pins its grid)"
    );
}
