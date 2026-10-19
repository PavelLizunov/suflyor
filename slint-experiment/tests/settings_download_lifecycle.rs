//! Settings must keep an in-flight job's text, progress and cancellation when hidden.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use slint::ComponentHandle;

mod ui {
    slint::include_modules!();
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
    assert!(!win.get_local_ai_installing());
    assert_eq!(win.get_local_choice_status(), "Cancelled");
}
