//! Nonblocking native pickers. Omarchy yields its layer surfaces while a
//! picker is open because a portal window cannot be parented to those layers.

use super::{ComponentHandle, SettingsWindow, SharedString};
use std::cell::Cell;
use std::path::PathBuf;

thread_local! {
    static DIALOG_OPEN: Cell<bool> = const { Cell::new(false) };
}

pub(crate) enum Kind {
    File,
    Folder,
    Save,
}

struct DialogGuard;

impl Drop for DialogGuard {
    fn drop(&mut self) {
        DIALOG_OPEN.set(false);
        #[cfg(all(feature = "remote-ui", target_os = "linux"))]
        slint_replay::remote_ui::send_global("Suflyor", "native-dialog-open", false.into());
    }
}

pub(crate) fn pick(
    dialog: rfd::AsyncFileDialog,
    kind: Kind,
    finished: impl FnOnce(Option<PathBuf>) + 'static,
) {
    if DIALOG_OPEN.replace(true) {
        return;
    }
    let guard = DialogGuard;
    #[cfg(all(feature = "remote-ui", target_os = "linux"))]
    slint_replay::remote_ui::send_global("Suflyor", "native-dialog-open", true.into());
    if let Err(error) = slint::spawn_local(async move {
        let picked = match kind {
            Kind::File => dialog.pick_file().await,
            Kind::Folder => dialog.pick_folder().await,
            Kind::Save => dialog.save_file().await,
        };
        drop(guard);
        finished(picked.map(|file| file.path().to_path_buf()));
    }) {
        eprintln!("[overlay-host] cannot open native picker: {error}");
    }
}

pub(crate) fn populate_storage(win: &SettingsWindow) {
    let saved = overlay_backend::paths::configured_model_storage_root();
    let active = overlay_backend::paths::model_storage_root();
    win.set_models_directory(SharedString::from(
        saved
            .as_ref()
            .map_or_else(String::new, |path| path.to_string_lossy().into_owned()),
    ));
    win.set_models_directory_status(SharedString::from(if saved != active {
        "Restart Suflyor to use the selected model folder. Existing files are not moved."
    } else {
        ""
    }));
}

fn save_storage(win: &SettingsWindow, value: &str) {
    let message = match overlay_backend::paths::save_model_storage_root(value) {
        Ok(()) => "Restart Suflyor to use the selected model folder. Existing files are not moved.",
        Err(_) => "Could not save the model folder. Choose an existing writable folder.",
    };
    win.set_models_directory_status(SharedString::from(message));
}

pub(crate) fn wire_storage(win: &SettingsWindow) {
    let weak = win.as_weak();
    win.on_models_directory_save(move |value| {
        if let Some(win) = weak.upgrade() {
            save_storage(&win, &value);
        }
    });
    let weak = win.as_weak();
    win.on_models_directory_browse(move || {
        let weak = weak.clone();
        let mut dialog = rfd::AsyncFileDialog::new();
        if let Some(path) = overlay_backend::paths::configured_model_storage_root() {
            dialog = dialog.set_directory(path);
        }
        pick(dialog, Kind::Folder, move |path| {
            let (Some(win), Some(path)) = (weak.upgrade(), path) else {
                return;
            };
            let value = path.to_string_lossy();
            win.set_models_directory(SharedString::from(value.as_ref()));
            save_storage(&win, &value);
        });
    });
}
