//! macOS ArchiveWindow slice (F7 / 🗄 archive chip).

use std::cell::RefCell;

use slint::ComponentHandle;

use crate::ui;

#[allow(dead_code)]
pub(super) struct MacArchiveSlice {
    win: RefCell<Option<ui::ArchiveWindow>>,
    cfg: overlay_backend::config::SharedConfig,
}

impl MacArchiveSlice {
    pub(super) fn new(cfg: overlay_backend::config::SharedConfig) -> Self {
        Self {
            win: RefCell::new(None),
            cfg,
        }
    }

    pub(super) fn toggle_archive(&self) {
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

        let win = match ui::ArchiveWindow::new() {
            Ok(w) => w,
            Err(e) => {
                slint_replay::logging::line(&format!("[macos] ArchiveWindow::new failed: {e}"));
                return;
            }
        };

        let weak = win.as_weak();
        win.on_close_requested(move || {
            if let Some(w) = weak.upgrade() {
                let _ = w.hide();
            }
        });

        let _ = win.show();
        if let Err(e) = slint_replay::native::window::configure_floating(win.window()) {
            slint_replay::logging::line(&format!("[macos] archive configure_floating failed: {e}"));
        }
        let _ = slint_replay::native::window::raise_key_front(win.window());

        *slot = Some(win);
    }
}
