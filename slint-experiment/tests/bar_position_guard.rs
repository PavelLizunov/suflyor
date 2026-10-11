//! Position memory must never replace the bar's unconditional reveal fallbacks.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

fn read(relative: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

#[test]
fn startup_restore_keeps_parking_primary_pin_hard_retry_and_slint_fallback() {
    let source = read("src/bin/overlay_host/bar_tray.rs");
    let apply = source.split_once("pub(super) fn apply_overlay_hwnd(")
        .expect("startup helper").1;
    for required in [
        "set_platform_window_position(overlay.window(), -32000, -32000);",
        "let Ok(hwnd) = grab_hwnd(o.window()) else",
        "return false;",
        "let (x, y) = primary_bar_work_area().map_or((60, 24), |work|",
        "bar_position(",
        "Some(p) => (p.left + ((p.width() - bar_w) / 2).max(0), p.top + 24)",
        "set_platform_window_position(o.window(), x, y);",
        "match move_window_pos_only(hwnd, x, y)",
        "match move_window_pos_only(hwnd, 60, 24)",
        "set_platform_window_position(o.window(), 60, 24);",
        "set_global_stealth_effective(false);",
        "surface_stealth_unavailable(o);",
        "realize_with_retries(overlay, attempt, fallback);",
    ] {
        assert!(apply.contains(required), "startup reveal lost {required}");
    }
    let restore = apply.split_once("// Restore is optional:").expect("restore path").1
        .split_once("#[cfg(not(windows))]").expect("restore boundary").0;
    assert!(restore.contains("find(|m| m.is_primary && !o.get_bootstrap_mode())"));
    assert!(restore.contains("monitor_work_area(primary)"));
    assert!(restore.contains("saved_bar_position("));
    assert!(restore.contains("width.max(current_width), height.max(current_height)"));
    assert!(restore.contains("move_window_pos_only(hwnd, rx, ry).is_ok()"));
    assert!(apply.find("make_transparent_overlay(hwnd)").unwrap()
        < apply.find("// Restore is optional:").unwrap());
    assert!(apply.find("apply_bar_stealth(o, &st, global_stealth())").unwrap()
        < apply.find("// Restore is optional:").unwrap());
}

#[test]
fn saving_is_sampled_hidden_and_drag_guarded_and_recovery_is_not_a_preference() {
    let source = read("src/bin/overlay_host/bar_tray.rs");
    let check = source.split_once("pub(super) fn reevaluate_bar_monitor(").unwrap().1
        .split_once("fn recenter_when_sized(").unwrap().0;
    for required in [
        "BAR_TRAY_HIDDEN.load(Ordering::Relaxed)",
        "!overlay.window().is_visible()",
        "bar_drag_in_progress()",
        "tracker.borrow_mut().observe(rect, false, false)",
        "if should_save",
        "saved == &position",
        "c.bar_positions.retain(",
        "c.bar_positions.len().saturating_sub(8)",
        "if config::save(&c).is_ok()",
        "c.bar_positions = previous;",
        "clamped_bar_position(work, rect)",
    ] {
        assert!(check.contains(required), "position sampling lost {required}");
    }
    let monitor_change = check.split_once("if changed {").expect("monitor change branch").1
        .split_once("let works:").expect("sampling boundary").0;
    for required in [
        "let key = monitor_key(*monitor);",
        "find(|saved| saved.monitor == key)",
        "bar_position_after_monitor_change(saved_offset, *work, form_rect)",
        "(logical_width * scale).round() as i32",
        "(spec.get_height() * scale).round() as i32",
        "BAR_PENDING_POSITION.with(|position| position.set(saved_offset.map(Some)))",
        "forget_bar_position_sample();",
        "return;",
    ] {
        assert!(monitor_change.contains(required), "monitor restore lost {required}");
    }
    assert!(check.contains("last.replace(Some(bounds)).is_some_and(|previous| previous != bounds)"));
    assert!(!monitor_change.contains("config::save"));
    assert!(!monitor_change.contains("user_drag()"));
    let recovery = check.split_once("// Recovery is never a user preference").unwrap().1;
    assert!(recovery.contains("forget_bar_position_sample();"));
    assert!(!recovery.contains("config::save"));
    let host = read("src/bin/overlay_host_windows.rs");
    let drag = host.split_once("overlay.on_drag_start_requested(move || {").unwrap().1
        .split_once("// ===== Quit").unwrap().0;
    let (start, moved) = drag.split_once("overlay.on_drag_moved(move || {")
        .expect("bar drag move callback");
    assert!(start.contains("begin_bar_position_drag();"));
    assert!(moved.contains("begin_bar_position_drag();"));
    let tracker = read("src/bar_form.rs");
    let observe = tracker.split_once("pub fn observe(").expect("position tracker").1
        .split_once("pub fn saved(").expect("save acknowledgement").0;
    assert!(observe.contains("should_save_bar_position(self.dragged_since_save, stable, dragging_now, hidden)"));
    assert!(!observe.contains("self.user_drag()"));
    assert!(tracker.contains("dragged_since_save && stable && !dragging_now && !hidden"));
    assert!(tracker.contains("self.dragged_since_save = false;"));
    assert!(!drag.contains("config::save"));
    assert!(source.contains("GetAsyncKeyState(i32::from(VK_LBUTTON.0)) < 0"));
    let resize = source.split_once("fn recenter_when_sized(").unwrap().1
        .split_once("pub(super) fn tray_menu_action(").unwrap().0;
    assert!(resize.contains("resize_work.or_else(|| bar_work_area(&o))"));
    assert!(resize.contains("None => clamped_bar_position(work, rect)"));
    assert!(resize.contains("Some(Some(offset)) => saved_bar_position(offset, work, width, height)"));
    assert!(resize.contains("if selected_width != target_w_logical"));
    assert!(resize.contains("work_area_bounds(requested) != work_area_bounds(current)"));
    assert!(resize.contains("match BAR_PENDING_POSITION.with(|position| position.take())"));
    assert!(!resize.contains("user_drag()"));
}
