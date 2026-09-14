//! Windows audio selectors must stay wired on both Settings open paths.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

#[test]
fn audio_settings_refresh_on_reopen_and_keep_macos_separate() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let controller = fs::read_to_string(root.join("src/bin/overlay_host/settings_controller.rs")).unwrap();
    let reused = controller.split("if let Some(existing) = settings_slot.as_ref()").nth(1).unwrap().split("let win = match SettingsWindow::new()").next().unwrap();
    assert!(reused.contains("settings_audio::refresh(existing, cfg)"));
    assert!(controller.contains("settings_audio::wire(&win, cfg)"));
    assert!(controller.contains("settings_audio::refresh(&w, &cfg_lang)"));
    assert!(controller.contains("win.set_audio_save_state(0)"));
    assert!(controller.contains("#[cfg(windows)]\n#[path = \"settings_audio.rs\"]"));
    let ui = fs::read_to_string(root.join("ui/settings_panel.slint")).unwrap();
    assert!(ui.contains("root.system-device-selected(root.system-device-index)"));
    assert!(ui.contains("root.mic-device-index-selected(root.mic-device-index)"));
    assert!(ui.contains("root.audio-devices-refresh()"));
    assert!(ui.contains("if !Platform.is-macos : SettingsCard {\n                            title: @tr(\"System audio - headphones / speakers\")"));
}

#[test]
fn windows_enumeration_does_not_disguise_failure_as_an_empty_list() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let audio = fs::read_to_string(root.join("../overlay-backend/src/audio.rs")).unwrap();
    let list = audio.split("pub fn list_devices()").nth(1).unwrap().split("fn enumerate(").next().unwrap();
    assert!(!list.contains("unwrap_or_default"));
    assert!(list.contains("enumerate(&Direction::Render).context"));
    assert!(list.contains("enumerate(&Direction::Capture).context"));
}
