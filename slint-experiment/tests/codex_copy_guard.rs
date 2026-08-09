//! RC15 guard for the explicit, local-only device-code copy action.
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
            >= 2
    );
    assert!(
        controller
            .matches("set_codex_copy_status(SharedString::default())")
            .count()
            >= 2
    );
    assert!(controller.matches("invalidate_codex_login_ui()").count() >= 2);
}

#[test]
fn connection_only_copy_is_translated_in_english_and_russian() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ui = fs::read_to_string(root.join("ui/settings_panel.slint")).expect("read UI");
    assert!(ui.contains("Codex connection (ChatGPT subscription)"));
    assert!(ui.contains("RC15 connection only: live answers are disabled"));

    let po = fs::read_to_string(root.join("translations/ru/LC_MESSAGES/slint-replay.po"))
        .expect("read RU catalog");
    for (msgid, msgstr) in [
        ("Copy code", "Копировать код"),
        ("Copy one-time code", "Копировать одноразовый код"),
        (
            "Codex connection (ChatGPT subscription)",
            "Подключение Codex (подписка ChatGPT)",
        ),
    ] {
        assert!(po.contains(&format!("msgid \"{msgid}\"\nmsgstr \"{msgstr}\"")));
    }
}
