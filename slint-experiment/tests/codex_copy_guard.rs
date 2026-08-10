//! Guard for the explicit, local-only device-code copy and safe-model UI.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::Path;

fn between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let tail = source.split_once(start).expect("start marker").1;
    tail.split_once(end).expect("end marker").0
}

#[test]
fn copy_button_is_code_gated_accessible_and_routes_exact_displayed_code() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ui = fs::read_to_string(root.join("ui/settings_panel.slint")).expect("read UI");
    assert!(ui.contains("if root.codex-login-url != \"\" && root.codex-user-code != \"\""));
    assert!(ui.contains("text: @tr(\"Copy code\")"));
    assert!(ui.contains("accessible-label: @tr(\"Copy one-time code\")"));
    assert!(ui.contains("enabled: root.codex-user-code != \"\""));
    assert!(ui.contains("root.codex-copy-code-clicked(root.codex-user-code)"));

    let rust = fs::read_to_string(root.join("src/bin/overlay_host/settings_ai.rs"))
        .expect("read settings_ai.rs");
    let copy = between(
        &rust,
        "win.on_codex_copy_code_clicked",
        "win.on_ai_local_base_url_save",
    );
    assert!(copy.contains("code != window.get_codex_user_code() || code.is_empty()"));
    assert!(copy.contains("clipboard_win::set_clipboard_string(value)"));
    assert!(!copy.contains("explorer.exe"));

    let connect = between(
        &rust,
        "win.on_codex_connect_clicked",
        "win.on_codex_disconnect_clicked",
    );
    assert!(!connect.contains("explorer.exe"));
}

#[test]
fn transient_code_and_feedback_are_cleared_on_all_terminal_routes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rust = fs::read_to_string(root.join("src/bin/overlay_host/settings_ai.rs"))
        .expect("read settings_ai.rs");
    assert!(
        rust.matches("set_codex_copy_status(SharedString::default())")
            .count()
            >= 6
    );
    assert!(rust.contains("invalidate_codex_login_ui()"));
    assert!(rust.contains("cancel_pending_login()"));

    let controller = fs::read_to_string(root.join("src/bin/overlay_host/settings_controller.rs"))
        .expect("read settings_controller.rs");
    assert!(
        controller
            .matches("set_codex_user_code(SharedString::default())")
            .count()
            >= 1
    );
    assert!(
        controller
            .matches("set_codex_copy_status(SharedString::default())")
            .count()
            >= 1
    );
    assert!(controller.matches("invalidate_codex_snapshot_ui()").count() >= 2);
    assert!(!controller.contains("invalidate_codex_login_ui()"));
}

#[test]
fn account_catalog_picker_saves_exact_hidden_model_id() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ui = fs::read_to_string(root.join("ui/settings_panel.slint")).expect("read UI");
    assert!(ui.contains("model: root.codex-model-labels"));
    assert!(ui.contains("root.codex-model-selected(self.current-index)"));
    assert!(ui.contains("root.codex-models-refresh()"));
    assert!(ui.contains("root.codex-rate-status"));
    assert!(ui.contains("in-out property <bool> codex-models-busy: false"));
    assert!(
        ui.matches("enabled: !root.codex-auth-busy && !root.codex-models-busy")
            .count()
            >= 3
    );

    let rust = fs::read_to_string(root.join("src/bin/overlay_host/settings_ai.rs"))
        .expect("read settings_ai.rs");
    assert!(rust.contains("provider_snapshot()"));
    assert!(rust.contains("window.set_codex_models_busy(true)"));
    assert!(rust.contains("window.set_codex_models_busy(false)"));
    assert!(rust.contains("codex_snapshot_ui_is_current(generation) && c.codex_model == saved"));
    assert!(rust.contains("get_codex_model_ids().row_data(index as usize)"));
    assert!(rust.contains("c.codex_model = model.to_string()"));
    assert!(!rust.contains("auth.json"));
}

#[test]
fn active_login_rechecks_language_for_each_event() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rust = fs::read_to_string(root.join("src/bin/overlay_host/settings_ai.rs"))
        .expect("read settings_ai.rs");
    let login = between(
        &rust,
        "win.on_codex_connect_clicked",
        "win.on_codex_model_selected",
    );
    assert!(login.contains("let is_ru = event_cfg.read().ui_is_ru();"));
    assert!(login.contains("codex_account_label(&state, is_ru)"));

    let controller = fs::read_to_string(root.join("src/bin/overlay_host/settings_controller.rs"))
        .expect("read settings_controller.rs");
    assert!(controller.contains("win.set_codex_models_busy(false)"));
}

#[test]
fn safe_model_provider_copy_is_translated_in_english_and_russian() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ui = fs::read_to_string(root.join("ui/settings_panel.slint")).expect("read UI");
    assert!(ui.contains("Codex safe models (ChatGPT subscription)"));
    assert!(ui.contains("Experimental safe mode: the model is pinned"));

    let po = fs::read_to_string(root.join("translations/ru/LC_MESSAGES/slint-replay.po"))
        .expect("read RU catalog");
    for msgid in [
        "Copy code",
        "Copy one-time code",
        "Codex safe models (ChatGPT subscription)",
        "Account model:",
        "Experimental safe mode: the model is pinned; model-command file and network access are denied, and unexpected tools are stopped.",
        "Loading account models...",
        "No account models are available. Sign in or refresh.",
    ] {
        assert!(po.contains(&format!("msgid \"{msgid}\"\nmsgstr \"")));
    }
}
