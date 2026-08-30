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

    assert!(ai.contains("cfg!(target_os = \"macos\") && idx == 4"));
    assert!(ai.contains("4 => \"codex\""));
    assert!(ai.contains("provider == \"local\" && !cfg!(target_os = \"macos\")"));
    assert!(vision.contains("\"codex\" if !cfg!(target_os = \"macos\") => 6"));
    assert!(vision.contains("\"codex\" => -1"));
    assert!(stt.contains("\"gigaam\" => 1"));
    assert!(vision.contains("\"mlx\" if cfg!(target_os = \"macos\") => 6"));
    assert!(settings.contains("(\"codex\", true) => -1"));
    assert!(vision.contains("cfg!(target_os = \"macos\") && idx == 6"));
    assert!(vision.contains("6 => \"codex\""));
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

    for (source, callback, branch, label) in [
        (
            ai.as_str(),
            "on_ai_provider_changed",
            "if cfg!(target_os = \"macos\") && idx == 4",
            "text",
        ),
        (
            vision.as_str(),
            "on_vision_provider_changed",
            "if cfg!(target_os = \"macos\") && idx == 6",
            "Vision",
        ),
    ] {
        let callback_body = source
            .split_once(callback)
            .unwrap_or_else(|| panic!("missing {label} provider callback"))
            .1;
        let preview_body = callback_body
            .split_once(branch)
            .unwrap_or_else(|| panic!("missing macOS {label} MLX preview branch"))
            .1
            .split_once('}')
            .expect("MLX preview branch must close")
            .0;
        assert_eq!(
            preview_body.trim().trim_start_matches('{').trim(),
            "return;",
            "macOS {label} MLX preview must not mutate or persist provider state"
        );
    }
    assert!(ui.contains(
        "root.ai-provider-index = self.current-index;\n                                root.ai-provider-changed(self.current-index);"
    ));
    assert!(ui.contains(
        "root.vision-provider-index = self.current-index;\n                                root.vision-provider-changed(self.current-index);"
    ));

    assert_eq!(
        ui.matches("@tr(\"Version: {} (suflyor / Slint)\", root.app-version)")
            .count(),
        1,
        "ui/settings_panel.slint must contain exactly one Version text line"
    );
    let backup = ui
        .split_once("title: @tr(\"Backup / transfer settings\");")
        .expect("Backup / transfer settings card title must exist in UI")
        .1
        .split_once("// Phase E6 v28")
        .expect("Backup card content marker must follow its title")
        .0;
    assert!(
        backup.contains("@tr(\"Version: {} (suflyor / Slint)\", root.app-version)"),
        "Version text must be inside the platform-neutral Backup card"
    );
}

#[test]
fn model_dropdowns_keep_selected_indices_across_provider_switches() {
    let ui = read("ui/settings_panel.slint");

    assert!(ui.contains(
        "root.ai-model-index = self.current-index;\n                                        root.ai-model-selected(self.current-value);"
    ));
    assert!(ui.contains(
        "root.ai-local-model-index = self.current-index;\n                                        root.ai-local-model-selected(self.current-value);"
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

    let settings = read("src/bin/overlay_host/settings_stt.rs");
    let controller = read("src/bin/overlay_host/settings_controller.rs");
    let ui = read("ui/settings_panel.slint");
    assert!(settings.contains("skip_llama: true,"));
    assert!(settings.contains("skip_whisper: true,"));
    assert!(settings.contains("win.on_stt_gigaam_install("));
    assert!(settings.contains("win.on_stt_gigaam_install_cancel("));
    assert!(settings.contains("installed_gigaam_dir(&c.stt_gigaam_dir)"));
    assert!(controller.contains("set_stt_gigaam_installed(installed.is_some())"));
    assert!(ui.contains("if Platform.is-macos && root.stt-provider-index == 1 : VerticalLayout"));
    assert!(ui.contains("root.stt-gigaam-install-progress"));
}

#[test]
fn macos_mlx_models_are_opt_in_downloads_with_honest_state() {
    let settings = read("src/bin/overlay_host/settings_controller.rs");
    let mlx = read("src/bin/overlay_host/settings_mlx.rs");
    let ui = read("ui/settings_panel.slint");

    assert!(settings.contains("include!(\"settings_mlx.rs\")"));
    assert!(settings.contains("settings_mlx::wire(&win, cfg);"));
    assert!(settings.contains("settings_mlx::populate(win);"));
    assert!(mlx.contains("overlay_backend::mlx_install::install("));
    assert!(mlx.contains("super::super::activate_mlx_model(role.model())"));
    assert!(mlx.contains("cancel.store(true, Ordering::Release)"));
    assert!(ui.contains("@tr(\"Managed MLX text model\")"));
    assert!(ui.contains("@tr(\"Managed MLX Vision model\")"));
    assert!(ui.contains("@tr(\"Download / Resume\")"));
    assert!(ui.contains("@tr(\"Enable for text\")"));
    assert!(ui.contains("@tr(\"Enable for Vision\")"));
    assert!(ui.contains("if Platform.is-macos && root.ai-provider-index == 4 : SettingsCard"));
    assert!(ui.contains("if Platform.is-macos && root.vision-provider-index == 6 : SettingsCard"));
    assert!(ui.contains("Download size: 4.52 GiB."));
    assert!(ui.contains("Download size: 1.63 GiB."));
    assert!(ui.contains("@tr(\"{} / {} MB downloaded\""));
    assert_eq!(ui.matches("x: 0;\n                                    y: 0;\n                                    height: parent.height;\n                                    width: parent.width * root.mlx").count(), 2);
    assert!(mlx.contains("format_mebibytes(4_851_993_338)"));
    assert!(mlx.contains("format_mebibytes(1_749_079_691)"));
    assert!(ui.contains("Russian response quality is not guaranteed"));
    assert!(ui.contains("One managed MLX model stays active at a time"));
    assert!(!mlx.contains("install(role.model(), &cancel, &|_, _| {})"));
}
