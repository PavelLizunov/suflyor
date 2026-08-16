//! Compile-time selected native UI and capture adapters.
//!
//! Clipboard implementations preserve four operations: empty-filtered
//! `read_text`, result-aware `set_text`, and best-effort `write_text` / `clear`.

#[cfg(windows)]
#[path = "windows/clipboard.rs"]
pub mod clipboard;

#[cfg(windows)]
#[path = "windows/lifecycle.rs"]
pub mod lifecycle;

#[cfg(target_os = "macos")]
#[path = "macos/lifecycle.rs"]
pub mod lifecycle;

#[cfg(target_os = "macos")]
#[path = "macos/window.rs"]
pub mod window;

#[cfg(windows)]
#[path = "windows/screen.rs"]
pub mod screen;

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

    use i_slint_backend_testing::ElementHandle;
    use slint::{ComponentHandle, LogicalSize};
    use std::cell::Cell;
    use std::rc::Rc;

    mod ui {
        slint::include_modules!();
    }

    fn assert_bootstrap(bar: &ui::OverlayBarWindow, message: &str, quit_label: &str) {
        bar.set_bootstrap_mode(true);
        bar.window().set_size(LogicalSize::new(560.0, 64.0));

        let message = ElementHandle::find_by_accessible_label(bar, message)
            .next()
            .expect("find bootstrap status");
        let quit = ElementHandle::find_by_accessible_label(bar, quit_label)
            .next()
            .expect("find bootstrap Quit");
        for (name, item) in [("message", &message), ("quit", &quit)] {
            let pos = item.absolute_position();
            let size = item.size();
            assert!(
                pos.x >= 0.0 && pos.x + size.width <= 560.0,
                "{name} escaped"
            );
            assert!(
                pos.y >= 0.0 && pos.y + size.height <= 64.0,
                "{name} escaped"
            );
        }
        assert!(
            ElementHandle::find_by_accessible_label(bar, "System audio capture")
                .next()
                .is_none(),
            "bootstrap mode must hide unconnected product controls"
        );

        let quit_called = Rc::new(Cell::new(false));
        let quit_called_in_callback = quit_called.clone();
        bar.on_quit_confirm(move || quit_called_in_callback.set(true));
        quit.invoke_accessible_default_action();
        assert!(
            quit_called.get(),
            "bootstrap Quit must reach the host callback"
        );
    }

    #[test]
    fn macos_bootstrap_is_honest_bounded_and_localized() {
        i_slint_backend_testing::init_no_event_loop();

        let english = ui::OverlayBarWindow::new().expect("create English bar");
        assert_bootstrap(
            &english,
            "Suflyor for macOS. Native overlay ready; product features are being connected.",
            "Quit",
        );

        slint::select_bundled_translation("ru").expect("select Russian translation");
        let russian = ui::OverlayBarWindow::new().expect("create Russian bar");
        assert_bootstrap(
            &russian,
            "Suflyor для macOS. Нативный оверлей готов; функции продукта подключаются.",
            "Выйти",
        );
    }
}
