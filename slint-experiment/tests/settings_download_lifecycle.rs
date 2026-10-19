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
    for (path, guard) in [
        ("src/bin/overlay_host/window_lifecycle.rs", "if !w.window().is_visible()"),
        ("src/bin/overlay_host/tile_window.rs", "if !t.window().is_visible()"),
    ] {
        let source = std::fs::read_to_string(root.join(path)).unwrap();
        assert!(source.contains(guard), "deferred native setup needs a close guard: {path}");
    }
}

#[test]
fn hidden_settings_keep_download_and_shared_status_until_completion() {
    i_slint_backend_testing::init_no_event_loop();
    let win = ui::SettingsWindow::new().unwrap();
    win.show().unwrap();
    win.set_strata_status("Previous Strata result".into());
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
