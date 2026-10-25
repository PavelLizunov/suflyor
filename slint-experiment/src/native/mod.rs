//! Compile-time selected native UI and capture adapters.
//!
//! Both supported platforms expose empty-filtered `read_text`, result-aware
//! `set_text`, and best-effort `write_text` / `clear` clipboard operations.

#[cfg(windows)]
#[path = "windows/clipboard.rs"]
pub mod clipboard;

#[cfg(target_os = "macos")]
#[path = "macos/clipboard.rs"]
pub mod clipboard;

#[cfg(target_os = "linux")]
#[path = "linux/clipboard.rs"]
pub mod clipboard;

#[cfg(target_os = "linux")]
#[path = "linux/desktop.rs"]
pub mod desktop;
#[cfg(target_os = "linux")]
#[path = "linux/displays.rs"]
mod displays;
#[cfg(target_os = "linux")]
#[path = "linux/screen.rs"]
pub mod screen;
#[cfg(all(target_os = "linux", not(feature = "remote-ui")))]
#[path = "linux/window.rs"]
pub mod window;
// The Omarchy plugin build: windows are shell layers, placed through suflyord's
// remote windows instead of Hyprland clients.
#[cfg(all(target_os = "linux", feature = "remote-ui"))]
#[path = "remote/window.rs"]
pub mod window;

#[cfg(windows)]
#[path = "windows/lifecycle.rs"]
pub mod lifecycle;

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "macos/lifecycle.rs"]
pub mod lifecycle;

#[cfg(target_os = "linux")]
#[path = "linux/status.rs"]
pub mod status;

#[cfg(target_os = "macos")]
#[path = "macos/status.rs"]
pub mod status;

#[cfg(target_os = "macos")]
#[path = "macos/window.rs"]
pub mod window;

#[cfg(windows)]
#[path = "windows/screen.rs"]
pub mod screen;

#[cfg(target_os = "macos")]
#[path = "macos/screen.rs"]
pub mod screen;

// The remote-ui build compiles only ui/remote_host.slint, so these real-window tests do not apply.
#[cfg(all(test, not(feature = "remote-ui")))]
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
            "Suflyor для macOS. Окна Суфлёра готовы; функции продукта подключаются.",
            "Выйти",
        );
    }
}
