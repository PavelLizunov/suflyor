//! Wiring guard for the macOS SettingsWindow slice.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

fn read(root: &Path, relative: &str) -> String {
    fs::read_to_string(root.join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn macos_overlay_host_wires_full_settings_slice() {
    let host = read(root(), "src/bin/overlay_host.rs");
    let settings = read(root(), "src/bin/overlay_host/macos_settings.rs");

    assert!(host.contains("mod macos_settings;"));
    assert!(host.contains("MacSettingsSlice::new"));
    assert!(host.contains("settings_slice.open_settings"));

    assert!(settings.contains("ui::SettingsWindow::new()"));
    assert!(settings.contains("slint_replay::native::window::configure_floating"));
    assert!(settings.contains("slint_replay::native::window::raise_key_front"));
    assert!(settings.contains("win.set_ai_provider_index"));
    assert!(settings.contains("win.set_stt_provider_index"));
    assert!(settings.contains("win.on_close_clicked"));
    assert!(settings.contains("win.set_codex_auth_busy"));
    assert!(settings.contains("win.on_trigger_keywords_save"));
}
