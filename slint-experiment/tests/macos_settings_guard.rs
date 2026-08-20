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

#[test]
fn macos_settings_hide_windows_only_setup_and_managed_local_actions() {
    let host = read("src/bin/overlay_host_windows.rs");
    let settings = read("src/bin/overlay_host/settings_controller.rs");
    let ui = read("ui/settings_panel.slint");

    assert!(host.contains("#[cfg(windows)]\n    if first_run {"));
    assert!(host.contains("#[cfg(windows)]\n#[path = \"overlay_host/wizard.rs\"]"));
    assert!(settings.contains("#[cfg(windows)]\n    {\n        // The wizard slot"));
    assert!(
        settings.contains("#[cfg(windows)]\n    wire_local_ai(&win, cfg, state, overlay_weak);")
    );
    assert!(ui.contains(
        "if !Platform.is-macos : SettingsCard {\n                            title: @tr(\"Setup\")"
    ));
    assert!(ui.contains("if !Platform.is-macos && root.ai-provider-index == 1 : SettingsCard {\n                            title: @tr(\"Engine (llama.cpp)\")"));
    assert!(ui.contains(
        "if !Platform.is-macos : VerticalLayout {\n                            spacing: 6px;"
    ));
}

#[test]
fn macos_provider_catalogs_and_components_keep_platform_indices_honest() {
    let ai = read("src/bin/overlay_host/settings_ai.rs");
    let settings = read("src/bin/overlay_host/settings_controller.rs");
    let stt = read("src/bin/overlay_host/settings_stt.rs");
    let vision = read("src/bin/overlay_host/settings_vision.rs");
    let ui = read("ui/settings_panel.slint");

    assert!(ai.contains("4 if !cfg!(target_os = \"macos\") => \"codex\""));
    assert!(ai.contains("provider == \"local\" && !cfg!(target_os = \"macos\")"));
    assert!(vision.contains("\"codex\" if !cfg!(target_os = \"macos\") => 6"));
    assert!(vision.contains("\"codex\" => -1"));
    assert!(stt.contains("\"gigaam\" => 1"));
    assert!(settings.contains("(\"codex\", true) => -1"));
    assert!(vision.contains("6 if !cfg!(target_os = \"macos\") => \"codex\""));
    assert!(settings.contains("ComponentKind::Engine | ComponentKind::LocalModel"));
    assert!(settings.contains("if cfg!(target_os = \"macos\") { 1 } else { 3 }"));
    assert!(settings.contains("c.detail = \"Apple Vision\".into()"));
    assert!(settings.contains("\"whisper\" => !snap.stt_whisper_url.trim().is_empty()"));
    assert!(ui.contains("if !Platform.is-macos && root.ai-provider-index == 4 : VerticalLayout"));
    assert!(
        ui.contains("if !Platform.is-macos && root.vision-provider-index == 6 : VerticalLayout")
    );
    assert!(ui.contains("if root.stt-provider-index == 0 : VerticalLayout"));
    assert!(ui.contains("if Platform.is-macos && root.ai-provider-index < 0 : Text"));
    assert!(ui.contains("if Platform.is-macos && root.vision-provider-index < 0 : Text"));
    assert!(ui.contains("@tr(\"Use Apple Core ML — faster on Apple Silicon\")"));
    assert!(ui.contains("if root.stt-provider-index == 1 : HorizontalLayout"));
    assert!(ui.contains("entry[0] == \"Ctrl+F8\" ? @tr(\"Control+F8\")"));
    assert!(ui.contains("entry[0] == \"Shift+Alt+2\" ? @tr(\"Shift+Option+2\")"));
    assert!(ui.contains(
        "Microphone device selection. The list comes from the audio devices available to macOS."
    ));
    assert!(ui.contains(
        "Stored in a user-only local credentials file and excluded from configuration exports."
    ));
    assert!(ui.contains(
        "Personal memory is stored locally. Approved items are added to new AI requests"
    ));
}

#[test]
fn macos_gigaam_is_coreml_backed_and_primary_on_fresh_config() {
    let cargo = read("../overlay-backend/Cargo.toml");
    let stt = read("../overlay-backend/src/stt.rs");
    let config = read("../overlay-backend/src/config.rs");

    assert!(cargo.contains("features = [\"onnx\", \"ort-coreml\"]"));
    assert!(stt.contains("#[cfg(any(windows, target_os = \"macos\"))]"));
    assert!(stt.contains("transcribe_rs::OrtAccelerator::CoreMl"));
    assert!(config.contains("if cfg!(target_os = \"macos\") {\n        \"gigaam\".into()"));
    assert!(config.contains("gigaam_default_dir(&crate::local_ai::default_root())"));
}
