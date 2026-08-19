//! Reachability guard for Settings in the canonical shared host.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

fn read(relative: &str) -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

#[test]
fn canonical_runtime_reaches_the_shared_settings_controller() {
    let host = read("src/bin/overlay_host_windows.rs");
    let settings = read("src/bin/overlay_host/settings_controller.rs");

    assert!(host.contains("#[path = \"overlay_host/settings_controller.rs\"]"));
    assert!(host.contains("open_settings("));
    assert!(settings.contains("pub(crate) fn open_settings("));
    assert!(settings.contains("SettingsWindow::new()"));
    assert!(settings.contains("present_window_stealth_aware(&win, |hwnd|"));
    assert!(settings.contains("win.on_close_clicked"));
}

#[test]
fn macos_reports_builtin_apple_vision_without_tesseract_install_actions() {
    let settings = read("src/bin/overlay_host/settings_controller.rs");
    let vision = read("src/bin/overlay_host/settings_vision.rs");
    let ui = read("ui/settings_panel.slint");

    assert!(settings.contains("c.detail = \"Apple Vision\".into()"));
    assert!(settings.contains("cfg!(windows) && c.kind == ComponentKind::Ocr"));
    assert!(settings
        .contains("cfg!(target_os = \"macos\") || overlay_backend::ocr_install::is_installed()"));
    assert!(vision.contains("#[cfg(windows)]\n    {\n        let weak = win.as_weak();\n        win.on_ocr_install_clicked"));
    assert!(ui.contains("if Platform.is-macos : Text"));
    assert!(ui.contains("Apple Vision is built into macOS; no download is required."));
    assert!(ui.contains("if !Platform.is-macos : VerticalLayout"));
}
