//! macOS HelpWindow slice (F1 / 🆘 help chip).

use std::cell::RefCell;

use slint::ComponentHandle;

use crate::ui;

pub(super) struct MacHelpSlice {
    win: RefCell<Option<ui::HelpWindow>>,
}

impl MacHelpSlice {
    pub(super) fn new() -> Self {
        Self {
            win: RefCell::new(None),
        }
    }

    pub(super) fn toggle_help(&self) {
        let mut slot = self.win.borrow_mut();
        if let Some(win) = slot.as_ref() {
            if win.window().is_visible() {
                let _ = win.hide();
                return;
            }
            let _ = win.show();
            let _ = slint_replay::native::window::raise_key_front(win.window());
            return;
        }

        let win = match ui::HelpWindow::new() {
            Ok(w) => w,
            Err(e) => {
                slint_replay::logging::line(&format!("[macos] HelpWindow::new failed: {e}"));
                return;
            }
        };
        win.global::<ui::Platform>().set_is_macos(true);

        let weak = win.as_weak();
        win.on_cancelled(move || {
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
        });

        let _ = win.show();
        if let Err(e) = slint_replay::native::window::configure_floating(win.window()) {
            slint_replay::logging::line(&format!("[macos] help configure_floating failed: {e}"));
        }
        let _ = slint_replay::native::window::raise_key_front(win.window());

        *slot = Some(win);
    }
}
