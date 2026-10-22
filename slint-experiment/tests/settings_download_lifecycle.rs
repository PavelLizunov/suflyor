//! Settings must keep an in-flight job's text, progress and cancellation when hidden.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use slint::ComponentHandle;

mod ui {
    slint::include_modules!();
}

#[test]
fn retained_windows_cannot_be_revealed_after_close_by_native_setup() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let controller =
        std::fs::read_to_string(root.join("src/bin/overlay_host/settings_controller.rs")).unwrap();
    let close = controller
        .split_once("win.on_close_clicked(move || {")
        .unwrap()
        .1
        .split_once("// Keep the instance alive")
        .unwrap()
        .0;
    assert!(close.contains("force_hide(w.window())"));
    assert!(controller.contains("position.x <= -30000 || position.y <= -30000"));
    assert!(controller.contains("present_window_stealth_aware(existing,"));
    for (path, guard) in [
        (
            "src/bin/overlay_host/window_lifecycle.rs",
            "if !w.window().is_visible()",
        ),
        (
            "src/bin/overlay_host/tile_window.rs",
            "if !t.window().is_visible()",
        ),
    ] {
        let source = std::fs::read_to_string(root.join(path)).unwrap();
        assert!(
            source.contains(guard),
            "deferred native setup needs a close guard: {path}"
        );
    }
}

#[test]
fn closing_a_vision_tile_aborts_its_registered_request() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source =
        std::fs::read_to_string(root.join("src/bin/overlay_host/vision_capture.rs")).unwrap();
    assert!(source.contains("tile.set_tile_id(tile_id)"));
    assert!(source.contains("super::register_tile_stream(tile_id, task.abort_handle())"));
    let close = source
        .split_once("tile.on_close_clicked(move || {")
        .unwrap()
        .1;
    assert!(close
        .trim_start()
        .starts_with("super::abort_tile_stream(tile_id);"));
}

#[test]
fn hidden_settings_keep_download_and_shared_status_until_completion() {
    i_slint_backend_testing::init_no_event_loop();
    let win = ui::SettingsWindow::new().unwrap();
    win.show().unwrap();
    win.set_strata_status("Previous Strata result".into());
    win.set_strata_vision_job(1);
    win.set_strata_vision_status("Strata Vision: 300 / 907 MB".into());
    win.set_strata_vision_progress(0.33);
    win.set_local_ai_installing(true);
    win.set_local_ai_progress(0.42);
    win.set_local_ai_status("Gemma: 2688 / 6405 MB".into());
    slint::platform::update_timers_and_animations();
    let cancel_called = std::rc::Rc::new(std::cell::Cell::new(false));
    let cancelled = cancel_called.clone();
    win.on_cancel_local_ai_clicked(move || cancelled.set(true));
    win.hide().unwrap();
    // A retained window and its live job fields survive without another event.
    win.show().unwrap();
    assert!(win.get_local_ai_installing());
    assert_eq!(win.get_strata_vision_job(), 1);
    assert_eq!(
        win.get_strata_vision_status(),
        "Strata Vision: 300 / 907 MB"
    );
    assert_eq!(win.get_strata_vision_progress(), 0.33);
    assert_eq!(win.get_local_ai_progress(), 0.42);
    assert_eq!(win.get_local_ai_status(), "Gemma: 2688 / 6405 MB");
    assert_eq!(win.get_local_choice_status(), "Gemma: 2688 / 6405 MB");
    win.invoke_cancel_local_ai_clicked();
    assert!(cancel_called.get());
    win.set_local_ai_installing(false);
    win.set_local_ai_status("Cancelled".into());
    // Slint's changed handlers are dispatched on the next event-loop turn.
    slint::platform::update_timers_and_animations();
    assert!(!win.get_local_ai_installing());
    assert_eq!(win.get_local_choice_status(), "Cancelled");
}
